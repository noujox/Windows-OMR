# Meeting Recorder for Windows

**An unofficial Windows port of [jankeesvw/omarchy-meeting-recorder](https://github.com/jankeesvw/omarchy-meeting-recorder).**

The original [Meeting Recorder](https://github.com/jankeesvw/omarchy-meeting-recorder) is a beautifully simple meeting recorder built for [Omarchy](https://omarchy.org) on Linux: record both sides of a conversation, transcribe it locally, and revisit the result in an editable, searchable transcript.

**This fork brings that experience to Windows.** The goal is not to reinvent the recorder, but to keep what makes it useful—its interface, local-first approach, and meeting workflow—while replacing its Linux-specific plumbing with native Windows integrations.

> **Project status: Windows preview / active development.** The Windows application builds and has been tested on a Windows machine, but this is not yet a polished, signed release. Some hardware-dependent features and edge cases still need validation. This fork is maintained independently from the original project.

![Original Meeting Recorder interface on Omarchy (reference screenshot; Windows appearance may differ)](screenshots/hero.webp)

## What works in this port

At its heart, it is still Meeting Recorder: **your microphone, the other side of the call, and a transcript you control**—without a meeting bot joining the call.

| Area | Windows port |
| --- | --- |
| **Audio capture** | Native **WASAPI** recording from the default microphone, plus **WASAPI loopback** for the computer's default playback device. Both sides are kept as separate tracks. |
| **Local transcription** | **Whisper / whisper.cpp** through `whisper-rs`; the current Windows preview is built with a CPU baseline. Models are downloaded when needed. |
| **Transcripts & playback** | Keep the original editing, speaker naming, waveform playback, imports, and saved-meeting workflow. |
| **Desktop integration** | Windows notification-area icon, minimize-to-tray support, and optional automatic startup with Windows. |
| **Windows internals** | Native **Named Pipes** for local control, Windows audio playback, and webcam capture through FFmpeg/DirectShow. |
| **Troubleshooting** | Persistent application logs, recovery of saved meetings, and fixes for Windows-specific GTK and IPC issues. |
| **Browser automation** | An **optional** satellite extension for browser-based Google Meet, Microsoft Teams, and Zoom. |

We have also removed the integrations that made little sense outside Omarchy—such as the Hyprland-specific window rules and the Omarchy bar plugin—while keeping the Rust, GTK4/libadwaita application and the existing meeting format.

## Exclusive to this fork: Meeting Recorder Satellite 🧪 BETA

**A new, optional feature developed for the Windows fork—not part of the original Omarchy project.**

Ever joined a meeting, started talking, and only remembered to press **Record** halfway through? **Meeting Recorder Satellite** explores a less intrusive approach: a small browser extension that recognizes when you enter a meeting and asks the desktop recorder to start **automatically, only when you have enabled it**.

> **BETA — experimental browser automation.** Detection depends on meeting-site interfaces and can fail or trigger at the wrong moment. The feature is disabled by default in **both** the extension and Meeting Recorder. Use the manual recording controls whenever reliability matters, and test with a non-sensitive call first.

### How it works

```text
Meet / Teams / Zoom (browser)
       ↓ detect an active call
Satellite browser extension
       ↓ Native Messaging
Local Windows satellite host
       ↓ restricted Named Pipe
Meeting Recorder → Start / Pause / Resume
```

- **Join a browser meeting:** When Satellite recognizes an active Google Meet, Microsoft Teams (web), or Zoom (web) call, it sends a start request to Meeting Recorder **if both automation switches are on**.
- **Leave the meeting:** It **pauses** your recording instead of stopping or finalizing it. After about **10 seconds**, the app comes to the foreground and lets you choose whether to stop or continue.
- **Temporary disconnection:** The extension uses periodic heartbeats and a conservative timeout, so brief reconnects do not immediately end your recording.
- **You stay in control:** Manual recordings and manual pauses take precedence. Satellite never ends a recording on its own.

Satellite **does not record browser audio or video itself**, read the meeting chat, or invite a bot to your call. It only looks for visible indicators of an active meeting and sends local control messages; the Windows application handles the actual capture. It does **not** launch Meeting Recorder, so the app must be running (it may be minimized to the tray).

### Install the Satellite extension (Windows)

**You need:** a Windows build of this fork with Satellite support, the `satellite` package containing `extension/`, `windows/install.ps1`, and `meeting-recorder-satellite-host.exe`, plus a Chromium-based browser (Chrome, Edge, or Vivaldi).

1. **Extract the Satellite package into a permanent folder.** Keep the native host executable and `windows/` installer alongside the `extension/` folder. Do not move the package after registration without reinstalling the host.
2. **Load the browser extension:** open `chrome://extensions`, `edge://extensions`, or `vivaldi://extensions`; enable **Developer mode**, select **Load unpacked / Cargar descomprimida**, and choose the **`extension/` subfolder** (the one containing `manifest.json`, **not** the package root).
3. **Copy your extension ID** from the browser's extensions page (32 lowercase letters). Each browser may assign a different ID.
4. **Register Native Messaging:** open PowerShell **in the extracted Satellite package folder** and run the command below, replacing the placeholder with the ID from step 3. The example is for **Edge**:

   ```powershell
   powershell -ExecutionPolicy Bypass -File ".\windows\install.ps1" -ExtensionId YOUR_32_CHARACTER_EXTENSION_ID -Browser Edge
   ```

   Change `-Browser Edge` to `Chrome` or `Vivaldi` for your browser. `Both` registers the Chrome and Edge registry locations, and `All` registers all three; however, the current installer authorizes **one extension ID per installation**, so use the specific browser option when IDs differ. No administrator privileges are required.
5. **Reload the extension** from your browser's extensions page (or restart the browser). You **do not run** `meeting-recorder-satellite-host.exe` manually: the browser launches it through Native Messaging when needed.

The installer only registers a per-user Windows host manifest. It does **not** install or launch the desktop app, open network ports, or start a recording. If you are building from source instead of using a prepared Satellite package, you must also compile the `meeting-recorder-satellite-host` binary and include it at the package root. See the [Satellite technical notes](satellite/README.md) for its architecture and files.

### How to use it

1. Open **Meeting Recorder** and enable **Automatic browser meetings** in its settings.
2. Open the **Meeting Recorder Satellite** browser popup and turn on **Automatic detection / Detección automática**.
3. Join a supported meeting **in a browser tab**. The extension will attempt to detect an active call and ask the running app to record; confirm recording actually started by checking Meeting Recorder's own timer and meters.
4. When you leave, Satellite pauses the recording. Review it in Meeting Recorder and choose **Stop** to finish/transcribe or **Resume** to continue.
5. If the meeting was already active before you enabled automation, the popup provides **Reenviar inicio de reunión / Resend meeting start**. Click it **only when you want to start recording**, as it may trigger a new recording.

The popup's **host connected** status means the browser can reach the native host—it **does not guarantee recording has started**. If something goes wrong, check `%LOCALAPPDATA%\omarchy-meeting-recorder\logs\app.log` and see [Satellite troubleshooting and known limitations](satellite/README.md).

**BETA limitations:** Meet/Teams/Zoom may change their UI and break detection; desktop meeting applications are not supported; multi-browser setups with distinct extension IDs need extra care; and unexpected disconnects or reconnects may require manual intervention. Please obtain any necessary participant consent and follow applicable recording laws.

## Getting started on Windows

**This is currently a developer/preview project, not a one-click Windows installer.** No official Windows release download is provided here yet.

If you have a packaged preview build:

1. Extract the **entire** package to a folder; keep its DLLs, FFmpeg and application resources together.
2. Start it using **`Start Meeting Recorder.cmd`** from that folder.
3. Check the microphone and computer-audio meters, then record or import audio as needed.
4. Install the satellite **only if you want** automatic detection of browser meetings.

To build from source, use **MSYS2 UCRT64** on Windows with the GTK4/libadwaita development libraries, Rust GNU toolchain, CMake/Ninja, Clang and the relevant multimedia dependencies:

```bash
bash scripts/build-windows-msys2.sh
```

The build script produces a Windows executable; **the executable alone is not a portable distribution**. The GTK and multimedia runtime DLLs must be packaged as well. See the [Windows build workflow](.github/workflows/windows-ci.yml), [preview packager](scripts/package-windows-preview.py), and [technical port notes](docs/PORT_WINDOWS.md) for details.

## What is still being worked on

- Making installation and distribution simpler, with reproducible Windows releases.
- Verifying audio-device changes, webcam behavior, tray integration and different hardware configurations.
- Improving GPU-accelerated transcription on Windows; CPU operation is the current baseline.
- Restoring optional agent-generated chapters **only** when a safe Windows execution model is available. This integration is deliberately disabled for now.
- Refining browser-meeting detection, Native Messaging and reconnection handling across supported browsers.

The project is evolving, so expect rough edges and please treat preview builds accordingly.

## Why a fork?

This work exists because the original recorder is worth bringing to another platform—not because it needs replacing. The aim is to offer Windows users the same thoughtful, privacy-conscious workflow while allowing Windows-specific changes to develop at their own pace.

**All credit for the original project and its design goes to [Jankees van Woezik](https://github.com/jankeesvw) and the upstream contributors.** This is a community-maintained, **unofficial** port, not an upstream Windows release or an affiliation with the original maintainer.

- **Original project:** [jankeesvw/omarchy-meeting-recorder](https://github.com/jankeesvw/omarchy-meeting-recorder)
- **This Windows fork:** [noujox/Windows-OMR](https://github.com/noujox/Windows-OMR)
- **Windows development notes:** [docs/PORT_WINDOWS.md](docs/PORT_WINDOWS.md)
- **License:** [MIT](LICENSE), retaining the original copyright notice

The original Linux/Omarchy documentation is preserved below for reference. **Its Arch/Omarchy installation commands do not install this Windows port.**

---

<details>
<summary><strong>Original upstream README — Linux / Omarchy reference</strong></summary>

# Original upstream README

A meeting recorder for [Omarchy](https://omarchy.org). It records your microphone and the computer audio as two tracks, and when you stop you get a transcript with speakers, chapters and a player. You can also drop in a recording you already have. Everything is transcribed on your own machine.

No bot joins your call, and no audio leaves your computer. It works with any meeting app, because it simply listens to what your computer plays and what you say.

![The done screen in Tokyo Night: chapters on the left, the transcript on the right, a waveform player above it](screenshots/hero.webp)

Open the app, check that both meters move, and press **Start recording**. When you stop, [whisper.cpp](https://github.com/ggml-org/whisper.cpp) transcribes the meeting while a 90s animation keeps you company. You get the transcript with who said what, a player to listen back from any line, and chapters written by the coding agent you already use. Everything takes the colours of your Omarchy theme.

Built for Omarchy on Hyprland (GTK 4 and libadwaita, written in Rust).

## Install

Meeting Recorder is in the [Omarchy package repository](https://github.com/omacom/omarchy-pkgs):

```bash
yay -S omarchy-meeting-recorder
```

For now it is in the edge channel, so this works if you run Omarchy's edge packages; everyone else gets it with the next Omarchy release. Until then, this one line builds the same pacman package from the [PKGBUILD](packaging/aur/PKGBUILD) in this repository ([install.sh](install.sh) is ten lines, read it first if you like):

```bash
curl -fsSL https://raw.githubusercontent.com/jankeesvw/omarchy-meeting-recorder/main/install.sh | bash
```

Either way `sudo pacman -R omarchy-meeting-recorder-bin` removes it again.

Then open **Meeting Recorder** from the launcher. The first transcription downloads the whisper model (about 1.6 GB, once), and the app shows you how far along it is. It offers to put a live waveform in your bar the first time, and the package prints the Hyprland rules for a floating window (also [below](#build-from-source)).

Prefer to build it yourself? See [Build from source](#build-from-source), or grab the binary from the [latest release](https://github.com/jankeesvw/omarchy-meeting-recorder/releases/latest).

<p align="center"><img src="screenshots/transcribing-animation.webp" alt="The transcribing animation: a neon sun over a scrolling grid, the progress bar and the lines as they are recognised, with the speakers' names" width="420"></p>

## What it does

### Checks the sound before you start

The app opens ready, not recording. The two meters are live from the start, the same moving waves as while recording, so you can see that both your microphone and the computer audio arrive before the meeting begins. Type a name if you like (otherwise it becomes "Meeting 14:30"), pick the audio format and the transcript language, and press **Start recording**.

<p align="center"><img src="screenshots/ready.webp" alt="The ready page: meeting name, audio file, language, two meters that say Not recording, the Start recording button and Import an audio file, or drop one here" width="440"></p>

### Records both sides of the call

Your microphone and whatever your computer plays are recorded as two separate tracks. The name, the audio format and the language can all still be changed during the call.

<p align="center"><img src="screenshots/recording.webp" alt="Recording: both meters moving, the clock, Pause and Stop recording" width="440">&nbsp;&nbsp;<img src="screenshots/paused.webp" alt="Paused: both waves frozen and dimmed with a PAUSED sign, Resume and Stop recording" width="440"></p>

**Pause** freezes both waves under a "❚❚ PAUSED" sign and stops the clock; nothing is written to either track until you press **Resume**.

### Records your webcam too, if you like

Turn on **Webcam** and your camera is recorded next to the audio, as `webcam.mp4` (picture only) in the meeting folder, and a small live picture in its row shows what it sees. The video keeps time with the audio: with the camera on, a recording (or a resume) starts its audio once the first frame is in, pauses are left out as in the audio, and you can turn the camera on or off during the recording, black filling the time it was off. The toggle is remembered, and so is the camera, which is the first real one the system has. Open the meeting later and the video plays small in the corner of the transcript, in step with the audio.

### Keeps your meeting notes next to the recording

The **a** in the corner of the ready page opens meeting notes next to the recorder, empty in a new window: the script of a voice-over, or what you want to say in a meeting. The paragraph with the cursor is lit and kept in the middle; ↓ and ↑ (or Ctrl+↓ and Ctrl+↑ while you type) move to the next and the previous one, and every edit is saved right away. `omarchy-meeting-recorder --notes <file>` opens a file of your own there. A recording made with meeting notes open keeps a copy as `notes.md` in its meeting folder, and the done page shows it next to the transcript.

![Recording Launch sync with the meeting notes beside it: the agenda for the call, with the paragraph about Northwind lit and the others dimmed](screenshots/notes.webp)

### Stays out of the way

Press Ctrl+M, or the button in the header bar, and the window shrinks to a strip with only the clock and the two waves. Drag the strip anywhere; the small button on its right, or Ctrl+M again, brings the full window back.

<p align="center"><img src="screenshots/compact.webp" alt="The compact strip: a red dot, the elapsed time, two small waves and an expand button" width="420"></p>

The bar widget shows the same while you record: a pulsing dot, a small waveform with the mic above the line and the computer audio below it, and the time. Paused it says "paused 01:23", and while the meeting is transcribed it shows the progress. Clicking it brings the recorder window back.

<p align="center"><img src="screenshots/bar-widget.webp" alt="The bar widget recording, paused and transcribing" width="600"></p>

### Transcribes on your own machine

When you stop, the window switches straight to the transcribing animation: it saves the audio, then [whisper-rs](https://github.com/tazz4843/whisper-rs) transcribes the meeting, and the lines type themselves out with the speakers' names as they are recognised. It ends on 100% and DONE, and stays at least ten seconds, also for a short recording. Nothing is sent anywhere.

<p align="center"><img src="screenshots/transcribing.webp" alt="The transcribing animation at 77 percent with lines from Maya and Tom" width="440"></p>

### Imports any recording

Drop an audio file on the window, or click **Import an audio file**: a phone memo, a call you recorded elsewhere, anything ffmpeg can read. Pick the language and how many people speak, or leave Speakers on Automatic, and the file is transcribed the same way. Since one file has no second track, the voices themselves are told apart, and each speaker gets a colour from your theme.

<p align="center"><img src="screenshots/drop-overlay.webp" alt="Dragging an mp3 from Nautilus onto the window: a dashed border and Drop to import" width="360">&nbsp;&nbsp;<img src="screenshots/import-dialog.webp" alt="The Import audio dialog with Language and Speakers set to Automatic" width="360"></p>

![An imported design review: three speakers, each in their own colour](screenshots/import-speakers.webp)

### Gives you a transcript you can listen to

The done screen puts the transcript on the right: the time, the speaker and the text in their own columns, one paragraph per turn. Above it sits a player with a waveform of both sides, your side above the line and the other side below it. Click or drag in the waveform to seek, or click any line to play from there. The line that is playing is highlighted and the transcript scrolls along.

On the left: the meeting name and one row per speaker, which you can rename at any time (the folder, the transcript and the manifest follow, and your own name is remembered for next time; give two speakers the same name and they become one), the chapters, **Copy transcript** (also Enter), Open folder, New recording, and the language to transcribe again in.

![The done screen while playing: the current chapter selected and the current line highlighted](screenshots/done.webp)

### Lets you fix it where you read it

Hover a line and three buttons appear: edit the text in place, give the line to the next speaker, or delete it. A deleted line comes back with Undo.

![Hovering a line: edit, next speaker and delete](screenshots/row-actions.webp)

![Editing a line in place](screenshots/inline-edit.webp)

### Chapters by your default agent

When Omarchy has a default coding agent set (`omarchy default agent`, for instance Claude Code or Codex) and the meeting is three minutes or longer, the agent divides the transcript into chapters once it is done. They show up as a list on the left, as headings in the transcript and as markers on the waveform (hover for the title), and `transcript.md` gets a `## Chapters` list at the top, so a copied transcript carries them too. The Chapters header on the done page makes them again.

<p align="center"><img src="screenshots/chapters.webp" alt="Close-up of the chapters list with the current chapter selected" width="600"></p>

Chapters are an extra, not a requirement: without an agent the button is simply not there and everything else works the same. The agent runs without any tools. It gets the transcript and the instructions, and can only answer with text.

### Runs your own actions

Put a few scripts of your own under **Actions** on the done page: store the transcript in your notes, publish it, mail it around. They go in `~/.config/omarchy-meeting-recorder/config.toml`, each with a name for the menu and a command. An action can also run on its own, as soon as the transcript is done. When none are configured, this optional menu stays hidden. See [Actions](#actions).

### Wears your Omarchy theme

The app reads the palette of the current theme (`colors.toml`): the background, the accent, and the theme's own colours for the speakers, the waves and the animation. Switch themes while it is open and it follows.

![The done screen in Tokyo Night, Osaka Jade, Catppuccin Latte, Gruvbox, Kanagawa and Everforest](screenshots/themes.webp)

![The done screen on Catppuccin Latte](screenshots/done-light.webp)

### Keeps your recording safe

If the app quits while it records (a crash, a logout, a power cut), the next start finds the unfinished recording and offers to save it as a meeting, keep it for later, or discard it.

<p align="center"><img src="screenshots/recovery.webp" alt="Unfinished recording found, with Save, Later and Discard" width="440"></p>

## Handy to know

- **Keyboard.** Ctrl+M switches between the full window and the compact strip. Ctrl+N opens another window. On the done page Enter copies the transcript. Ctrl+F finds text in it: the match is marked, Enter and Shift+Enter move between matches, and Escape closes the bar. Ctrl+W closes the window and Ctrl+Q all of them, and they ask first while recording or transcribing.
- **The name** stays editable all the time. After the transcript is done, changing it (Enter, or leaving the field) renames the meeting folder and the heading in the transcript.
- **Closing** while recording or transcribing asks first. You can stop and close, let the transcription finish in the background and quit afterwards, or cancel the transcription; the audio is kept either way.
- **Opening a meeting later.** Double-click its `.meeting-recorder` file, or run `omarchy-meeting-recorder <folder>`. It opens on the done page with the settings it was made with. When the window is busy recording or transcribing, the meeting opens in a window of its own.
- **More than one window.** Ctrl+N, `omarchy-meeting-recorder new-window` or New Window in the launcher opens another one, to read an older meeting or start the next while the last one is still transcribing. Every window shows its own meeting; one records at a time. The bar widget and the keybindings follow the window that is recording, else the one transcribing.
- **Keybindings.** `omarchy-meeting-recorder start`, `pause`, `stop` and `compact` control the running app, so you can bind them to keys in Hyprland.

## What it writes to disk

Every meeting is a plain folder in `~/Documents/Meetings`, named `<YYYYMMDDHHMM> <name>`, so they sort by date:

![Nautilus showing four meeting folders](screenshots/files-meetings.webp)

Inside, the audio in the format you picked, the transcript, a `.meeting-recorder` file that opens the meeting in the app when you double-click it, and (hidden) `.tracks`, the two separate tracks the app keeps so it can transcribe the meeting again:

![The inside of a meeting folder with hidden files shown: audio.ogg, Launch sync.meeting-recorder, transcript.md and .tracks](screenshots/files-meeting-folder.webp)

- `<name>.meeting-recorder`, a small JSON file with the title, start time, duration, audio format, language, speaker names, the model that transcribed it and the chapters. It has its own MIME type (`application/x-omarchy-meeting`), so double-clicking it opens the meeting in the app on the done page, with the settings the meeting was made with. The folder itself stays a plain folder.
- `transcript.md`, with the speaker and a timestamp on every line (and the chapters, when there are any)
- the audio in the format you picked:
  - **Mono**: `audio.ogg`, mic and computer audio mixed
  - **Stereo**: `audio.ogg`, mic on the left channel, computer audio on the right
  - **Separate files**: `mic.ogg` and `computer.ogg`
- `.tracks/mic.ogg` and `.tracks/computer.ogg`, a hidden copy of both tracks in mono. This is what Transcribe again uses, so the speakers stay apart whatever audio format you chose. Delete the directory if you do not need that.
- `notes.md`, the meeting notes that were open while you recorded, when there were any
- `webcam.mp4`, the camera, picture only and as long as the audio, when the webcam was on
- For an imported file: `audio.ogg`, the transcript and the `.meeting-recorder` file; the original file is left where it was.

Both tracks are always recorded separately, and each is levelled to the same speech loudness when it is saved, so a quiet microphone and a loud call end up equally easy to hear. The format can be switched until the moment you press stop.

## How it works

- **Recording.** The mic (`@DEFAULT_SOURCE@`) and the monitor of the default output (`@DEFAULT_MONITOR@`) are captured with `parec`. Because it follows the default output, switching to a headset during a call keeps working. When the audio stops coming in for a moment, say while a Bluetooth headset switches to its headset mode, that track gets silence for the time, so the two tracks stay in step. `ffmpeg` encodes the audio to Opus when you stop.
- **Transcription.** After the call both tracks are mixed and transcribed in one pass with whisper-rs, using the `large-v3-turbo` model unless you pick another, so there is a single timeline. Long silences are skipped, which keeps whisper from inventing text in them, and word times come from whisper's attention alignment (DTW).
- **Who said what.** The mic and the computer audio are transcribed one at a time, so the side of each line is simply its track, and two people talking at once, or someone talking over music, are both kept. Through speakers the other side leaks into your mic; the app leaves out what is only that echo, also when it is loud enough to be heard as a voice of its own: a voice on the mic that only speaks while the computer audio does is echo. When several voices share one side, a colleague next to you or three people on the other end, they are told apart as well (see below): "You 1", "You 2" and "Remote 1", "Remote 2" and so on, each with its own name field.
- **Imported files.** A single audio file has no second track to tell the speakers apart, so the voices themselves are told apart with NVIDIA's [Nemotron 3 Diarization](https://huggingface.co/nvidia/Nemotron-3-Diarization), run locally through ONNX Runtime. It follows up to eight speakers, also when they talk at the same time, and numbers them "Speaker 1", "Speaker 2" and so on in the order they first speak. The number of speakers is found automatically (voices heard for only a few seconds are folded into the nearest real speaker) or can be fixed. A sentence always goes to one speaker as a whole. Similar voices and fast back-and-forth can still land on the wrong speaker, which the swap-speaker button fixes per line.
- **Chapters.** The recorder runs `omarchy-default-agent`'s agent headless and with every tool switched off, in an empty working directory, bounded in time and size. Agents that cannot run without tools are not used.
- **Playback.** `ffmpeg` decodes into `pacat`, so playing a meeting back needs nothing beyond what recording already uses.
- **Crash recovery.** While recording, both tracks are written to a cache directory as they come in. A recording that was not stopped properly is still there on the next start.
- **The bar widget.** The app serves its live state on a Unix socket in `$XDG_RUNTIME_DIR`. `omarchy-meeting-recorder watch` relays it as NDJSON, which is what the widget reads.

### The model

The default is whisper's `large-v3-turbo`. To use another, set it in `~/.config/omarchy-meeting-recorder/config.toml`:

```toml
model = "small"   # tiny, tiny.en, base, base.en, small, small.en, medium, medium.en, large-v3, large-v3-turbo, or a path to a .bin file
```

The command-line `transcribe` and `transcribe-file` take `--model` instead. When the configured model is not on disk yet, the start screen says so, with its size, and a Download button:

<p align="center"><img src="screenshots/model-banner.webp" alt="The banner: The speech model (tiny, 75 MB) is needed to transcribe, with Download" width="600"></p>

The app looks for `ggml-<model>.bin`, for instance `ggml-large-v3-turbo.bin`, in `~/.local/share/omarchy-meeting-recorder/models/`. If you use [voxtype](https://voxtype.io) and it already downloaded that model to `~/.local/share/voxtype/models/`, that copy is used. Otherwise it is downloaded (about 1.6 GB for `large-v3-turbo`) from [Hugging Face](https://huggingface.co/ggerganov/whisper.cpp). Finding speakers downloads the speaker model on first use (about 120 MB) to `nemotron-3-diarization/` in the same directory: the int8 ONNX export of Nemotron 3 Diarization from the [Hugging Face ONNX community](https://huggingface.co/onnx-community/Nemotron-3-Diarization-ONNX), pinned to one revision. The model is NVIDIA's, under the [OpenMDW license](https://huggingface.co/nvidia/Nemotron-3-Diarization). ONNX Runtime is compiled into the binary, so nothing else is needed at runtime.

### Languages

The Language dropdowns offer Auto-detect, English, Dutch, German, French, Spanish, Italian and Portuguese. Whisper knows many more; add the ones you need in `~/.config/omarchy-meeting-recorder/config.toml`, by code or by English name:

```toml
languages = ["th", "ja", "Korean"]
```

They are added after the built-in ones, named the way whisper names them (Thai, Japanese, Korean), and picked up the next time the app starts. A name whisper does not know is skipped. The `.en` models only transcribe English, so use a multilingual model, like the default `large-v3-turbo`, for other languages.

## Actions

Your own scripts, picked from the **Actions** menu on the done page: store the transcript in Obsidian, publish it, mail it around. Each is a name and a command in `~/.config/omarchy-meeting-recorder/config.toml`; the command gets the meeting folder and the meeting's details, and what it prints last shows up in the app, with an **Open** button for a link. Give one `auto = true` and it runs by itself every time a transcript is done, after the chapters when there are any: a meeting is filed in your notes, or handed to a pipeline of your own, without a click.

<p align="center"><img src="screenshots/actions-menu.webp" alt="Clicking Actions on the done page: the view zooms in on the menu with Store transcript in Obsidian and Publish as public transcript" width="700"></p>

[docs/actions.md](docs/actions.md) explains it all, with two complete examples (Store transcript in Obsidian, and Publish as public transcript, where your default agent writes the summary) and a section for your agent, so you can ask it to write actions for you.

## Testing

`bench/` holds a small test suite: recordings of calls with people talking at once, echo through speakers, two people at one mic, music in the background and silence, plus a real meeting from the AMI corpus. `bench/run.py` runs the app over them and scores the transcript and the speakers; CI runs it on every pull request and posts the scores there. See [bench/README.md](bench/README.md).

## Privacy

The audio, the transcript and everything else stay on your computer. The only thing that leaves it is the transcript text for the chapters, and only when you have set a default agent: it goes to that agent's service, the one you already chose and pay for. No agent, no chapters, nothing sent. Actions are yours: they send whatever your scripts send, and only when you pick one.

## Requirements

- PipeWire with `parec` and `pacat` (both from `libpulse`), for recording and for playing a meeting back
- `ffmpeg` with libopus
- GTK 4 and libadwaita 1.6 or newer
- Rust, CMake, Vulkan headers and `glslc`, to build it (whisper.cpp is compiled along)
- Vulkan loader at runtime; a compatible Vulkan GPU driver enables accelerated transcription
- Optional: a default agent in Omarchy for chapters

## Build from source

```bash
cargo build --release
ln -s "$PWD/target/release/omarchy-meeting-recorder" ~/.local/bin/omarchy-meeting-recorder
ln -s "$PWD/data/omarchy-meeting-recorder.desktop" ~/.local/share/applications/
mkdir -p ~/.local/share/mime/packages
ln -s "$PWD/data/omarchy-meeting-recorder.xml" ~/.local/share/mime/packages/
update-mime-database ~/.local/share/mime
xdg-mime default omarchy-meeting-recorder.desktop application/x-omarchy-meeting
```

The last three lines register the `.meeting-recorder` file type, so a double-click opens the meeting in the app. File managers that go through GIO (Nautilus) pick that up right away; restart Nautilus if it still opens the file as text. `xdg-open`, which most launchers and terminals use on Hyprland, looks at the contents with `file` instead and sees JSON, so it opens the file in your text editor. Install `perl-file-mimeinfo` (`yay -S perl-file-mimeinfo`) and `xdg-open` goes by the registered type too.

The default build includes whisper.cpp's Vulkan backend and uses a compatible GPU when available. With no usable GPU it falls back to the CPU. GPU support accelerates speech-to-text; speaker identification still runs on the CPU. Processing time depends on the model, audio and hardware, and can be substantial for long recordings on CPU.

On Arch, the default build needs `vulkan-headers` and `shaderc`. The installed binary needs `vulkan-icd-loader`, plus the Vulkan driver appropriate for your GPU to use acceleration. Existing CPU-only release binaries do not gain GPU support from installing a driver: rebuild or install a release compiled with Vulkan enabled. Restart the recorder after replacing its binary, once any recording or processing has finished.

For a CPU-only binary without Vulkan build or runtime dependencies:

```bash
cargo build --release --no-default-features
```

To check which support was compiled into a binary:

```bash
omarchy-meeting-recorder --version
```

This reports build support, not whether a particular transcription actually ran on a GPU. The `vulkan` feature remains available explicitly for build scripts using `--no-default-features --features vulkan`.

The window floats nicely with a Hyprland rule on its class:

```lua
o.window("^com\\.jankeesvw\\.OmarchyMeetingRecorder$", { float = true })
o.window("^com\\.jankeesvw\\.OmarchyMeetingRecorder$", { size = { 480, 700 } })
o.window("^com\\.jankeesvw\\.OmarchyMeetingRecorder$", { center = true })
```

### Bar widget

The `plugin` directory is an Omarchy Quattro bar widget. It stays hidden until a recording starts. Installed as a package, the app offers to add it the first time you open it. From source, link it yourself:

```bash
ln -s "$PWD/plugin" ~/.config/omarchy/plugins/jankeesvw.meeting-recorder
omarchy-shell shell rescanPlugins
omarchy plugin enable jankeesvw.meeting-recorder
omarchy bar move jankeesvw.meeting-recorder --section right
```

The shell discovers plugins asynchronously. If enabling immediately after a rescan says the plugin is not known, wait until `omarchy-shell shell listPlugins` includes `jankeesvw.meeting-recorder`, then run:

```bash
omarchy plugin enable jankeesvw.meeting-recorder --section right
```

This also recovers a failed first-start “Add to Bar” attempt in version 1.0.2, which leaves the widget linked but does not offer again on restart.

## Command line

| Command | What it does |
|---|---|
| `omarchy-meeting-recorder [--name <name>] [--notes <file>]` | Open the recorder, ready to record. `--name` names the meeting about to be recorded, whoever presses Start; `--notes` shows a file as meeting notes next to the recorder, to read from and edit while recording |
| `omarchy-meeting-recorder <folder or .meeting-recorder file>` | Open a saved meeting on the done page |
| `omarchy-meeting-recorder start [--name <name>] [--notes <file>]` | Start recording, for a keybinding; opens the recorder first when it is not running. With a name, the meeting gets that name (a script can pass a calendar event's title); the name without `--name` works too. With notes, they open next to the recorder |
| `omarchy-meeting-recorder pause` | Pause or resume the running recording |
| `omarchy-meeting-recorder stop` | Stop the running recording |
| `omarchy-meeting-recorder compact` | Switch the recording window between full and compact |
| `omarchy-meeting-recorder new-window` | Open another window, or the app when it is not running |
| `omarchy-meeting-recorder watch` | Stream the recorder state as NDJSON, for the bar widget; a line carries the meeting's `folder` once it has one |
| `omarchy-meeting-recorder wait [--audio]` | Wait until the meeting being recorded is done and print its folder, for a script or an agent; with `--audio` as soon as the audio is saved, before the transcript |
| `omarchy-meeting-recorder transcribe <mic> <computer> [--language xx] [--model name]` | Transcribe two tracks and print the transcript as Markdown |
| `omarchy-meeting-recorder transcribe-file <audio> [--speakers N] [--language xx] [--model name]` | Transcribe one file, telling the voices apart, and print the transcript as Markdown |
| `omarchy-meeting-recorder ask "<prompt>" < text` | Run a prompt over stdin through the default agent, without tools (`ask --agent` shows which agent that is) |
| `omarchy-meeting-recorder action "<name>" <meeting folder>` | Run one of your [actions](docs/actions.md) on a meeting, as the done page does; without arguments it lists them |

Every flag takes its value after a space or an equals sign: `--name "Weekly sync"` and `--name="Weekly sync"` are the same. For example:

```bash
omarchy-meeting-recorder --notes script.md --name "Voice-over"
omarchy-meeting-recorder transcribe mic.ogg computer.ogg --language en > transcript.md
omarchy-meeting-recorder transcribe-file interview.mp3 --speakers 2 > transcript.md
```

Any format ffmpeg can read works. `--language` takes `auto` (the default) or any language code whisper knows, such as `en`, `nl` or `th`.

## The screenshots

The meetings in the screenshots and clips are invented and were voiced with [piper](https://github.com/rhasspy/piper). `demo/` has the scripts and a step-by-step guide to shoot them again.

## License

MIT

</details>
