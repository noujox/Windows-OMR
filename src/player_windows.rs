//! WASAPI playback of FFmpeg-decoded PCM on Windows.
//! The COM objects stay in one audio thread; dropping the playback kills
//! FFmpeg first so its stdout unblocks, then joins this thread.
use std::io::{ErrorKind, Read};
use std::process::ChildStdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat, initialize_mta};

const FRAME_BYTES: usize = 4; // 48 kHz stereo signed 16 bit
const POLL: Duration = Duration::from_millis(10);
const BUFFER_HNS: i64 = 800_000; // 80 ms
const MAX_WRITE_FRAMES: u32 = 960; // 20 ms

pub(super) struct Sink {
    stop: Arc<AtomicBool>,
    done: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Sink {
    pub(super) fn start(audio: ChildStdout) -> Option<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let finished = done.clone();
        let (ready, initialized) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || {
            let result = playback(audio, &stopping, ready);
            if let Err(message) = result {
                eprintln!("Meeting Recorder: WASAPI playback: {message}");
            }
            finished.store(true, Ordering::Release);
        });
        let sink = Sink {
            stop,
            done,
            handle: Some(handle),
        };
        if !matches!(initialized.recv_timeout(Duration::from_secs(3)), Ok(Ok(()))) {
            // Thread normally already finished; caller kills ffmpeg on failure.
            return None;
        }
        Some(sink)
    }

    pub(super) fn ended(&self) -> bool {
        self.done.load(Ordering::Acquire)
    }

    pub(super) fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.handle.take() {
            let _ = thread.join();
        }
    }
}

/// COM initialization, stream creation and destruction remain in this thread.
fn playback(
    mut audio: ChildStdout,
    stop: &AtomicBool,
    ready: mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    // HRESULT::ok() checks for both S_OK and S_FALSE.
    if let Err(error) = initialize_mta().ok() {
        let message = format!("COM initialization: {error}");
        let _ = ready.send(Err(message.clone()));
        return Err(message);
    }

    let setup = (|| -> Result<_, String> {
        let enumerator = DeviceEnumerator::new().map_err(|e| e.to_string())?;
        let selected = crate::settings::output_device();
        let device = if selected == "default" {
            enumerator.get_default_device(&Direction::Render)
        } else {
            enumerator.get_device(&selected)
        }
        .map_err(|e| format!("output device {selected}: {e}"))?;
        let mut client = device.get_iaudioclient().map_err(|e| e.to_string())?;
        let fmt = WaveFormat::new(16, 16, &SampleType::Int, 48_000, 2, None);
        client
            .initialize_client(
                &fmt,
                &Direction::Render,
                &StreamMode::PollingShared {
                    autoconvert: true,
                    buffer_duration_hns: BUFFER_HNS,
                },
            )
            .map_err(|e| format!("initialize output: {e}"))?;
        let render = client.get_audiorenderclient().map_err(|e| e.to_string())?;
        client.start_stream().map_err(|e| e.to_string())?;
        Ok((client, render))
    })();
    let (client, render) = match setup {
        Ok(value) => value,
        Err(message) => {
            let _ = ready.send(Err(message.clone()));
            return Err(message);
        }
    };
    let _ = ready.send(Ok(()));
    let mut raw = [0u8; MAX_WRITE_FRAMES as usize * FRAME_BYTES];
    let mut pending = Vec::<u8>::with_capacity(raw.len());
    let result = (|| -> Result<(), String> {
        while !stop.load(Ordering::Acquire) {
            let free = client
                .get_available_space_in_frames()
                .map_err(|e| format!("output buffer: {e}"))?;
            if free == 0 {
                thread::sleep(POLL);
                continue;
            }
            let requested = free.min(MAX_WRITE_FRAMES) as usize * FRAME_BYTES;

            // A pipe read can stop between audio frames. Save any leftover
            // bytes until the next read instead of dropping half a sample.
            while pending.len() < requested {
                match audio.read(&mut raw[..requested - pending.len()]) {
                    Ok(0) => break,
                    Ok(n) => pending.extend_from_slice(&raw[..n]),
                    Err(e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(e) => return Err(format!("decoder pipe: {e}")),
                }
            }
            let frames = (pending.len() / FRAME_BYTES).min(requested / FRAME_BYTES);
            if frames == 0 {
                break;
            }
            let count = frames * FRAME_BYTES;
            render
                .write_to_device(frames, &pending[..count], None)
                .map_err(|e| format!("write output: {e}"))?;
            pending.drain(..count);
            if frames < requested / FRAME_BYTES {
                break; // decoder EOF: remaining buffer already queued in WASAPI
            }
            thread::sleep(POLL);
        }
        Ok(())
    })();
    let _ = client.stop_stream();
    result
}
