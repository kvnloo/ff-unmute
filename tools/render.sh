#!/usr/bin/env bash
# Whole pipeline: music -> measurements -> hud constants -> flat pass -> lens pass.
# Run from anywhere: tools/render.sh   (-> out/unmute.mp4)
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p out/pass
python3 -I music/synth.py      # out/pass/music.wav, out/events.json, out/stems/
python3 -I music/analyze.py    # out/measured.json
python3 -I music/gen_hud.py    # hud/src/generated.rs
cargo build --release -p hud -p flat -p lens
target/release/flat render -o out/pass/flat.mp4
target/release/lens render -o out/unmute.mp4
ffprobe -v error -show_entries stream=codec_type,width,height,nb_frames,duration -of compact out/unmute.mp4
