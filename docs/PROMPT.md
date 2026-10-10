# The prompt

This is the prompt that makes this video. Paste it into [Claude Code](https://claude.com/claude-code)
with the [`fframes-video`](https://github.com/dmtrKovalenko/fframes/tree/main/skills/fframes-video)
skill installed (`npx skills add dmtrKovalenko/fframes`) on a Mac with Apple silicon.
It holds the decisions the original session made along the way, including the ones it got
wrong first. Change the brief and you get your own version.

````text
Make a 16-second video with fframes (use the fframes-video skill) where the sound comes first:
a camera flies over a DAW timeline in one shot, and a tracker locks onto every beat the
playhead touches. Compose the music in code; measure it; drive the picture from the measurements.

RULE ZERO
You can't hear. So: compose, then measure (band balance per section, loudness, true peak, harsh
2-5 kHz), then hand me a WAV to listen to. Do not start the picture until I approve the music.
Keep every later change bit-identical to the approved WAV (compare the samples).

MUSIC (Python, numpy + scipy, no samples)
- 120 BPM, A minor, 8 bars = 16.0 s, 48 kHz stereo, -14 LUFS, true peak below -1 dBTP.
- music/score.py is the single source of truth: progression Am Am Am F Dm E F Am, voicings,
  16-step patterns per section (intro, groove, full, build, drop).
- music/synth.py: kick (pitch-swept sine + click), clap (four noise bursts), hats, 25 % pulse
  bass from two polyBLEP saws, detuned-saw pad with a filter that opens over the intro, arp
  pluck, riser, impact with sub drop. Sidechain pump from every kick. Stereo reverb from a
  synthetic impulse response. Mono below 120 Hz.
- Make it dithered and agentic: pad at 7 bits, arp and data at 6 bits with sample-and-hold, all
  with TPDF dither; an agent layer of token clicks (32nds, above 8 kHz) and FM bleeps (16ths,
  A minor pentatonic around 0.9-1.8 kHz), denser as the track builds; buffer-repeat stutters
  into bars 5 and 7; the pad gated in 16ths in bars 5-7; a four-bleep signature after the impact.
- Write events.json (every note) and stems (kick, bass, data, tokens). Generate each random
  sound once and place it on both the mix and its stem, or the random sequence shifts.

MEASURE (music/analyze.py, on the stems)
Tempo from kick onsets, first kick time, kick and bass fundamentals (FFT peak with parabolic
interpolation), each bleep's strongest component, token count, centroid, LUFS and true peak
(ffmpeg ebur128). music/gen_hud.py turns events + measurements into hud/src/generated.rs
(constants, so the picture is synced at compile time and hud has no dependencies).

PICTURE: A DAW THAT FOLLOWS ITS PLAYHEAD
- Cargo workspace: hud (timeline, tracker, camera, projection), flat (pass 1), lens (pass 2).
- flat, SVG at 3840x2160 (1920x1080 logical x2), white on black, pure red only for a lock:
  ruler with bars and beats; lanes pad (chord blocks + arp), kick, clap + hats, bass piano roll,
  master waveform from the WAV's peaks, agent lane (bleeps by pitch, tokens as ticks); a fixed
  playhead at x=960 with the timeline scrolling at 320 px/s; a live FFT strip at the playhead
  from frame.visualize_audio_frame (S1024, Hann; bin k = k*sr/1024, group into log bands);
  markers on the ruler that type themselves when the playhead reaches them, carrying the
  measured values ("onset 4.000 s", "kick 55.7 Hz", "tempo 120.0 BPM", "-14.0 LUFS"); clips
  bright once played.
- No fixed closing card: every video must end on its own data. The timeline stops 0.35 s after
  the impact, then the view zooms out (px per second and the playhead blend to a full-track
  layout) until the whole track fits the screen; merge waveform columns so they stay solid;
  boxes stay on every kick, clap and the impact; the FFT strip fades; markers shrink except
  one picked from the data (the loudest moment from the per-frame RMS).
- Tracker: a box on every event within 0.55 s behind the playhead; kicks, claps and the impact
  turn red for 2 frames on their exact frame. Labels carry measured values.
- lens: one SkSL shader films flat.mp4 (bound as iChannel0): tear, perspective (ray vs tilted
  plane), depth of field focused on the tracked box, 5-stop palette, bloom, LED grid, grain.
  Uniforms: kick envelope (zoom punch + bloom), clap/stutter tear, impact flash.
  One continuous camera: Catmull-Rom over about 12 keyframes (time, look height, zoom, tilt,
  roll, offset, blur): waveform → kick lane at 4 s → bass → agent lane at 8 s → close-up in
  bar 7 → pull back after 14 s. Palettes blend over 0.5 s, no cuts.
  Draw a sharp camera-space tracker on top with hud::project (the CPU copy of the shader camera).

KNOWN TRAPS (fframes 1.2.0)
- `flat` is a reserved word in SkSL.
- Never return Svgr::empty() from Video::render_frame; return a root <svg> (fframes#209).
- include_media_dir! panics on files it doesn't know: keep media/ to fonts.
- The lens loads out/pass (flat.mp4, music.wav) at runtime with MediaDirectory and combines it
  with the fonts through CombinedMediaProvider.
- A first version with fast cuts and big terminal text looked too much like ff-tracking. Keep
  the subject the timeline and the camera continuous.

TESTS AND DONE
- hud tests: every kick locks on its frame and its box sits on the playhead; the camera keeps
  the focus inside the frame on every frame; the waveform covers the track.
- After every change: contact sheets, full frames of a lock and of the ending, audio analyze.
- Show me an 8-second draft before the full render.
- Done = out/unmute.mp4, 1920x1080, 480 frames, 16.0 s, -14 LUFS, audio equal to the approved
  WAV, and a README with a preview, one frame per bar, the shader stages and the music.
````

## Notes

- The original session approved the music on its second draft. The first draft's low end was
  almost all of its energy, which the band balance check caught before anyone listened.
- The result won't be identical: the camera keys, palettes and synth details are tuned by eye
  and ear. The code in this repo is one result of this prompt.
