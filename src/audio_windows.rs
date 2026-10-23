//! Native Windows WASAPI capture, including system-output loopback.
//!
//! The device and COM interfaces live entirely on the dedicated capture thread.
//! This worker returns on device removal, format failure or default-device
//! change; Source::spawn restarts it while keeping the recording timeline.
use std::collections::VecDeque;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use wasapi::{DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat, initialize_mta};

use super::{CHUNK_BYTES, FRAME_BYTES, Inner, Input, ingest_chunk};

const BUFFER_HNS: i64 = 400_000; // 40 ms
const POLL: Duration = Duration::from_millis(10);
const CHECK_DEVICE: Duration = Duration::from_secs(2);
// Keep a bounded queue; an unexpected WASAPI backlog must not grow forever.
const MAX_QUEUE_BYTES: usize = CHUNK_BYTES * 100;

fn configured_device(input: Input) -> String {
    match input {
        Input::Microphone => crate::settings::microphone_device(),
        Input::Computer => crate::settings::output_device(),
    }
}

fn direction(input: Input) -> Direction {
    match input {
        Input::Microphone => Direction::Capture,
        Input::Computer => Direction::Render,
    }
}

/// Called exactly once on the dedicated capture thread, before device retries.
/// COM interfaces are not moved across threads.
pub(super) fn initialize_thread() -> Result<(), String> {
    initialize_mta()
        .ok()
        .map_err(|e| format!("WASAPI COM initialization: {e}"))
}

pub(super) fn capture(input: Input, shared: &Mutex<Inner>) -> Result<(), String> {
    let enumerator = DeviceEnumerator::new().map_err(|e| format!("WASAPI enumerator: {e}"))?;
    let wanted = configured_device(input);
    let device = if wanted == "default" {
        enumerator.get_default_device(&direction(input))
    } else {
        enumerator.get_device(&wanted)
    }
    .map_err(|e| format!("WASAPI endpoint {}: {e}", wanted))?;
    let device_id = device
        .get_id()
        .map_err(|e| format!("WASAPI device id: {e}"))?;
    let mut client = device
        .get_iaudioclient()
        .map_err(|e| format!("WASAPI audio client: {e}"))?;

    // Shared-mode AUTOCONVERTPCM and SRC let Windows adapt mono mics and mix
    // formats to the raw stereo 48 kHz PCM format of both existing tracks.
    let format = WaveFormat::new(16, 16, &SampleType::Int, 48_000, 2, None);
    let mode = StreamMode::PollingShared {
        autoconvert: true,
        buffer_duration_hns: BUFFER_HNS,
    };
    client
        .initialize_client(&format, &Direction::Capture, &mode)
        .map_err(|e| format!("WASAPI initialization (48k stereo s16): {e}"))?;
    let reader = client
        .get_audiocaptureclient()
        .map_err(|e| format!("WASAPI capture interface: {e}"))?;
    client
        .start_stream()
        .map_err(|e| format!("WASAPI start: {e}"))?;

    let mut queue = VecDeque::<u8>::with_capacity(CHUNK_BYTES * 3);
    let mut last_chunk = Instant::now();
    let mut last_device_check = Instant::now();
    let mut chunks = 0u64;

    let result = (|| -> Result<(), String> {
        loop {
            // Users can change default devices in Settings while the app is open.
            // Reconnect without ever carrying COM interfaces to another thread.
            if last_device_check.elapsed() >= CHECK_DEVICE {
                last_device_check = Instant::now();
                let current = configured_device(input);
                if current != wanted {
                    return Err("audio device selection changed".into());
                }
                if current == "default" {
                    let current_device = enumerator
                        .get_default_device(&direction(input))
                        .map_err(|e| format!("default audio endpoint unavailable: {e}"))?;
                    if current_device
                        .get_id()
                        .map_err(|e| format!("WASAPI default device id: {e}"))?
                        != device_id
                    {
                        return Err("default audio endpoint changed".into());
                    }
                }
            }

            while let Some(frames) = reader
                .get_next_packet_size()
                .map_err(|e| format!("WASAPI packet: {e}"))?
            {
                if frames == 0 {
                    break;
                }
                let bytes = frames as usize * FRAME_BYTES;
                // A malicious or broken driver should not cause unbounded alloc.
                if bytes > MAX_QUEUE_BYTES {
                    return Err("WASAPI packet is implausibly large".into());
                }
                let mut packet = vec![0u8; bytes];
                let (received, _info) = reader
                    .read_from_device(&mut packet)
                    .map_err(|e| format!("WASAPI read: {e}"))?;
                queue.extend(&packet[..received as usize * FRAME_BYTES]);
                if queue.len() > MAX_QUEUE_BYTES {
                    return Err("WASAPI capture fell behind".into());
                }

                while queue.len() >= CHUNK_BYTES {
                    let mut chunk = [0u8; CHUNK_BYTES];
                    for byte in &mut chunk {
                        *byte = queue.pop_front().expect("checked complete audio chunk");
                    }
                    let now = Instant::now();
                    chunks += 1;
                    ingest_chunk(
                        shared,
                        &chunk,
                        now.saturating_duration_since(last_chunk),
                        chunks == 1,
                        chunks,
                    );
                    last_chunk = now;
                }
            }
            thread::sleep(POLL);
        }
    })();
    let _ = client.stop_stream();
    result
}
