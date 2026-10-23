#!/usr/bin/env python3
"""Stage a Windows CPU build as a portable, not-yet-tested distribution.

Run only after successfully linking the Windows executable. This does not launch it.
Collect direct and recursive PE DLL imports from the UCRT64 runtime.
"""
from collections import deque
from pathlib import Path
import argparse, re, shutil, subprocess, sys, zipfile

parser = argparse.ArgumentParser()
parser.add_argument("--repo", type=Path, required=True)
parser.add_argument("--msys", type=Path, default=Path("C:/msys64/ucrt64"))
parser.add_argument("--ort", type=Path, required=True, help="official onnxruntime-win-x64-1.28.3.zip")
args = parser.parse_args()
repo, msys = args.repo, args.msys
exe = repo/"target/x86_64-pc-windows-gnu/debug/omarchy-meeting-recorder.exe"
if not exe.is_file():
    sys.exit(f"No executable at {exe}")
if not args.ort.is_file():
    sys.exit(f"Missing official ORT archive: {args.ort}")
out = repo/"dist/windows/meeting-recorder-windows-preview"
out.mkdir(parents=True, exist_ok=True)
shutil.copy2(exe, out/exe.name)
ffmpeg = msys/"bin/ffmpeg.exe"
if not ffmpeg.is_file():
    sys.exit("MSYS2 FFmpeg missing")
shutil.copy2(ffmpeg, out/"ffmpeg.exe")

with zipfile.ZipFile(args.ort) as z:
    paths = [p for p in z.namelist() if p.lower().endswith("/lib/onnxruntime.dll")]
    if len(paths) != 1:
        sys.exit(f"Expected one official ORT DLL, found {paths}")
    with z.open(paths[0]) as source, (out/"onnxruntime.dll").open("wb") as sink:
        shutil.copyfileobj(source,sink)

native = {p.name.lower():p for p in (msys/"bin").glob("*.dll")}
seen=set()
queue=deque([out/exe.name,out/"ffmpeg.exe",out/"onnxruntime.dll"])
objdump=msys/"bin/objdump.exe"
rx=re.compile(r"DLL Name:\s*(\S+)",re.I)
while queue:
    current=queue.popleft()
    if current.name.lower() in seen:
        continue
    seen.add(current.name.lower())
    data=subprocess.run([str(objdump),"-p",str(current)],capture_output=True,text=True,errors="replace")
    if data.returncode != 0:
        print(f"WARNING cannot inspect imports of {current.name}")
        continue
    for dep in rx.findall(data.stdout):
        dep=dep.lower()
        source=native.get(dep)
        if source and dep not in seen:
            target=out/source.name
            if not target.exists():
                shutil.copy2(source,target)
            queue.append(target)
print(f"Copied {len(list(out.glob('*.dll')))} runtime DLLs")

share=out/"share"
for folder in ("glib-2.0/schemas","icons"):
    src=msys/"share"/folder
    dest=share/folder
    if src.is_dir():
        shutil.copytree(src,dest,dirs_exist_ok=True)
# Windows launcher sets resource locations to the portable tree.
(out/"Start Meeting Recorder.cmd").write_text(
    '@echo off\r\n'
    'set "GSETTINGS_SCHEMA_DIR=%~dp0share\\glib-2.0\\schemas"\r\n'
    'set "XDG_DATA_DIRS=%~dp0share"\r\n'
    'cd /d "%~dp0"\r\n'
    'start "" "%~dp0omarchy-meeting-recorder.exe"\r\n', encoding="utf-8")
(out/"README.txt").write_text(
    "PREVIEW BUILD - NOT FUNCTIONALLY TESTED.\n"
    "For the first user-run, open Start Meeting Recorder.cmd.\n"
    "Windows GNU CPU-only build; models are downloaded when needed.\n"
    "Windows tray/autostart integration has not been manually validated.\n"
    "AI agent chapter generation is disabled until safe Windows isolation exists.\n"
    "Generated automatically; no application execution occurred.\n",encoding="utf-8")
print("STAGED",out)
print("Total MB",round(sum(p.stat().st_size for p in out.rglob("*") if p.is_file())/1024**2,1))
