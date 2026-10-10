# ff-unmute: "This one, unmute" (design)

Date: 2026-10-09
Status: approved (brainstorming)

## Why

ff-tracking got a quote from the author of fframes: "what a fire … BUT DO NOT UNMUTE".
The picture worked, the sound did not (it was measured, never listened to). The second video
answers that: sound comes first, and the picture reacts to it.

## Decisions

| Topic | Decision |
|---|---|
| Purpose | Keep the fframes momentum: a second piece, as a reply to "DO NOT UNMUTE" |
| Concept | Sound first; the picture reacts to its own music |
| Music | Composed in code (Python synth), approved by ear by the owner before any picture work |
| Picture | ff-tracking continued: the agent listens to the track and writes what it measures |
| Length | 16 s, 120 BPM (beat = 0.5 s = 15 frames, bar = 2 s), 8 bars, dark electronic, A minor |
| Repo | New repo `ff-unmute`, based on ff-tracking's hud / flat / lens / tools |
| Sync | Score first: one score → WAV + events.json; analysis of the rendered WAV → measured.json |

## Structure

| Bars | Time | Music | Picture |
|---|---|---|---|
| 1–2 | 0–4 s | Pad (Am) opening from a closed filter, sparse hi-hats, no kick | `listening…`, the spectrum rises from flat, slow camera, a cut per bar |
| 3–4 | 4–8 s | Four-on-the-floor kick, bass line in A1 | Each kick locks a box; measured lines: `kick 55 Hz @ 4.00 s`, `tempo 120.0 BPM` |
| 5–6 | 8–12 s | Clap on 2 and 4, 16th hats, chords Am–F–Dm–E | A cut per beat, tear on each clap, palette per bar |
| 7 | 12–14 s | Riser, snare roll, filter opens | Boxes multiply, push-in, cuts every 0.25 s |
| 8 | 14–16 s | Big hit at 14.0 s, full mix, tail | Every box collapses into one → `✓ unmuted` pill, fade to black |

Every number on screen (tempo, kick frequency, peak, loudness) is measured from the rendered
WAV, not copied from the score.

## Architecture

```
music/score.py ──► music/synth.py ──► out/pass/music.wav + out/events.json
                                   └► music/analyze.py ──► out/measured.json
                                   └► hud/src/generated.rs (const arrays; hud stays dependency-free)
hud ──► flat (pass 1: terminal + spectrum + scope + tracker) ──► out/pass/flat.mp4
hud ──► lens (pass 2: lens.sksl, beat-driven uniforms, camera tracker) ──► out/unmute.mp4 (+ music.wav)
```

- **score.py**: patterns per bar and instrument (step, note, velocity).
- **synth.py**: kick (pitch-swept sine + click), clap (filtered noise bursts), hats, bass (saw
  through a resonant low-pass with envelope), pad (detuned saws, slow filter), riser, impact.
  Light bus compression and a soft clip; exports every note to events.json.
- **analyze.py**: onsets (spectral flux), tempo (onset autocorrelation), kick fundamental (FFT
  of the first kick), peak, LUFS (ffmpeg ebur128), energy in 2–5 kHz relative to the mix.
- **flat**: agent log with measured lines, a spectrum analyzer (fframes `visualize_audio_frame`
  on music.wav) and a waveform scope; boxes lock onto the newest text and onto the spectrum peak.
- **lens**: same shader. Kick → zoom punch and exposure, clap → tear, bar → palette. Labels show
  measured values next to the coordinates.

## Checks

1. Music gate: waveform, loudness (-14 LUFS ±0.5, true peak ≤ -1 dBTP), 2–5 kHz balance, then
   the owner listens and approves. No picture work before that.
2. Sync test in hud: every kick frame is a lock frame; every cut lands on a beat.
3. Contact sheet per bar, full-size frames of a lock and of the ending, `audio analyze`.
4. Result: `out/unmute.mp4`, 1920×1080, 480 frames, 16.0 s.

## Delivery

English README in the same quality as ff-tracking (preview, how it works, stills, link to the
video with sound), a GitHub release, `docs/PROMPT.md` with the prompt that makes this video.
The repo goes to GitHub after the video is approved.

## Out of scope

- Vocals, sampled instruments, third-party audio.
- Changes to fframes itself (fframes#210 stays a separate track).

## Revision, 2026-10-10: the playhead flight

The first full draft (a cut per bar, then per beat, with big terminal text over a spectrum panel)
looked too much like ff-tracking. The owner asked what had changed; the honest answer was mostly
the concept, not the picture. The picture was rebuilt, the music stayed bit-identical:

- **Subject:** a DAW timeline that follows its playhead (ruler, markers, lanes for pad, kick,
  clap and hats, bass, master waveform, agent; a live FFT strip at the playhead).
- **Camera:** one 16 s shot, Catmull-Rom over 12 keyframes, no cuts; palettes blend over 0.5 s.
- **Tracker:** boxes on the events the playhead touches; kicks, claps and the impact lock red on
  their frame; labels carry measured values.
- **Ending:** the timeline stops 0.35 s after the impact and dims; "✓ unmuted" pill.

The structure table above describes the first draft; `hud/src/lib.rs` describes what ships.

## Revision, 2026-10-10 (v1.1.0): an ending made of data

The owner rejected the closing pill: both videos ended on the same kind of button, and every
video made with this pipeline should end differently, from its own data. The ending is now a
zoom-out to the whole track (waveform, notes, the boxes on every kick, clap and the impact),
and the only bright marker is the loudest moment measured from the per-frame RMS
(`loudest -9.9 dBFS @ 4.500 s` for this track). ff-tracking keeps its "Done" pill.
