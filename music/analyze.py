"""Measure the rendered track, so every number the agent writes on screen is real.

Run from the workspace root after music/synth.py:
    python3 -I music/analyze.py   -> out/measured.json

It only reads out/pass/music.wav and, to know where to look, the event times in
out/events.json. Every value is measured from the audio, none is copied from the score.
"""

import json
import re
import subprocess
import wave
from pathlib import Path

import numpy as np
from scipy.signal import butter, sosfilt

ROOT = Path(__file__).resolve().parent.parent
WAV = ROOT / "out" / "pass" / "music.wav"
EVENTS = ROOT / "out" / "events.json"
OUT = ROOT / "out" / "measured.json"
STEMS = ROOT / "out" / "stems"
FPS = 30


def load(path=WAV):
    w = wave.open(str(path))
    sr = w.getframerate()
    x = (
        np.frombuffer(w.readframes(w.getnframes()), "<i2").reshape(-1, w.getnchannels())
        / 32768.0
    )
    return sr, x


def peak_hz(sig, sr, lo, hi, pad=1 << 16):
    win = np.hanning(len(sig))
    spec = np.abs(np.fft.rfft(sig * win, pad))
    f = np.fft.rfftfreq(pad, 1 / sr)
    band = (f >= lo) & (f <= hi)
    i = np.argmax(spec * band)
    # parabolic interpolation around the peak bin
    if 0 < i < len(spec) - 1:
        a, b, c = (
            np.log(spec[i - 1] + 1e-12),
            np.log(spec[i] + 1e-12),
            np.log(spec[i + 1] + 1e-12),
        )
        i = i + 0.5 * (a - c) / (a - 2 * b + c)
    return float(i * sr / pad)


def onsets(mono, sr, lo=None, hi=None, threshold=1.6):
    """Spectral-flux onsets (s) in a band."""
    hop, n = 256, 1024
    if lo or hi:
        sos = butter(
            4, [lo or 20, hi or sr / 2 - 1], btype="bandpass", fs=sr, output="sos"
        )
        mono = sosfilt(sos, mono)
    frames = np.lib.stride_tricks.sliding_window_view(mono, n)[::hop] * np.hanning(n)
    mag = np.abs(np.fft.rfft(frames, axis=1))
    flux = np.maximum(0, np.diff(mag, axis=0)).sum(axis=1)
    flux = flux / (np.median(flux) + 1e-9)
    hits = []
    for i in range(1, len(flux) - 1):
        if flux[i] > threshold * 4 and flux[i] >= flux[i - 1] and flux[i] > flux[i + 1]:
            t = (i + 1) * hop / sr
            if not hits or t - hits[-1] > 0.08:
                hits.append(t)
    return hits


def envelope_hits(sig, sr, rel=0.3, gap=0.25):
    """Times where the RMS envelope peaks above rel * its maximum, at least gap s apart."""
    hop = int(0.005 * sr)
    env = np.sqrt(np.convolve(sig**2, np.ones(hop) / hop, mode="same"))[::hop]
    thr = rel * env.max()
    hits = []
    for i in range(1, len(env) - 1):
        if env[i] > thr and env[i] >= env[i - 1] and env[i] > env[i + 1]:
            t = i * hop / sr
            if not hits or t - hits[-1] > gap:
                hits.append(t)
    # report the rise, not the top: step back to where the envelope crosses half the peak
    out = []
    for t in hits:
        i = int(t * sr / hop)
        while i > 0 and env[i - 1] > 0.5 * env[int(t * sr / hop)]:
            i -= 1
        t0 = round(i * hop / sr, 3)
        if not out or t0 - out[-1] > gap:
            out.append(t0)
    return out


def loudness():
    out = subprocess.run(
        [
            "ffmpeg",
            "-nostats",
            "-i",
            str(WAV),
            "-af",
            "ebur128=peak=true",
            "-f",
            "null",
            "-",
        ],
        capture_output=True,
        text=True,
    ).stderr
    return (
        float(re.findall(r"I:\s+(-?[\d.]+) LUFS", out)[-1]),
        float(re.findall(r"Peak:\s+(-?[\d.]+) dBFS", out)[-1]),
    )


def main():
    sr, x = load()
    mono = x.mean(axis=1)
    ev = json.loads(EVENTS.read_text())["events"]
    times = lambda kind: [e["t"] for e in ev if e["kind"] == kind]

    # Each part is measured on its own stem (out/stems, written by synth.py).
    _, kick = load(STEMS / "kick.wav")
    _, bass = load(STEMS / "bass.wav")
    _, data = load(STEMS / "data.wav")
    kick, bass, data = kick.mean(axis=1), bass.mean(axis=1), data.mean(axis=1)

    kicks = envelope_hits(kick, sr, rel=0.3, gap=0.25)
    iois = np.diff(kicks[:16])
    tempo = 60.0 / float(np.median(iois)) if len(iois) else 0.0

    first_kick = kicks[0] if kicks else 0.0
    i0 = int((first_kick + 0.03) * sr)
    kick_hz = peak_hz(kick[i0 : i0 + int(0.12 * sr)], sr, 30, 150)

    b0 = times("bass")[0]
    j0 = int((b0 + 0.01) * sr)
    bass_hz = peak_hz(bass[j0 : j0 + int(0.1 * sr)], sr, 30, 200)

    # A bleep's strongest component (FM puts it on a sideband, which is what you hear).
    blips = []
    for e in ev:
        if e["kind"] in ("blip", "signature"):
            k = int((e["t"] + 0.002) * sr)
            hz = peak_hz(data[k : k + int(0.03 * sr)], sr, 500, 6000)
            blips.append({"t": e["t"], "hz": round(hz, 1)})

    # Spectral centroid of bars 5-6.
    seg = mono[int(8 * sr) : int(12 * sr)]
    spec = np.abs(np.fft.rfft(seg * np.hanning(len(seg))))
    f = np.fft.rfftfreq(len(seg), 1 / sr)
    centroid = float((f * spec).sum() / (spec.sum() + 1e-12))

    # Level per video frame (RMS in dBFS) for the meter.
    hop = sr // FPS
    rms = [
        float(
            20 * np.log10(np.sqrt(np.mean(mono[i * hop : (i + 1) * hop] ** 2)) + 1e-9)
        )
        for i in range(int(len(mono) / hop))
    ]

    integrated, true_peak = loudness()
    peak_db = float(20 * np.log10(np.max(np.abs(x)) + 1e-12))
    m = {
        "sample_rate": sr,
        "channels": int(x.shape[1]),
        "seconds": round(len(mono) / sr, 3),
        "onsets": len(onsets(mono, sr)),
        "kicks": [round(t, 3) for t in kicks],
        "first_kick": round(first_kick, 3),
        "tempo_bpm": round(tempo, 2),
        "kick_hz": round(kick_hz, 1),
        "bass_hz": round(bass_hz, 1),
        "blips": blips,
        "tokens": len(envelope_hits(load(STEMS / "tokens.wav")[1].mean(axis=1), sr, rel=0.3, gap=0.02)),
        "tokens_in_score": len(times("token")),
        "centroid_hz": round(centroid, 0),
        "lufs": integrated,
        "true_peak_db": true_peak,
        "peak_db": round(peak_db, 2),
        "rms_per_frame": [round(v, 1) for v in rms],
    }
    OUT.write_text(json.dumps(m, indent=1))
    print(
        f"tempo {m['tempo_bpm']} BPM, first kick {m['first_kick']} s, kick {m['kick_hz']} Hz, bass {m['bass_hz']} Hz, "
        f"{len(m['kicks'])} kicks, {len(blips)} bleeps, {m['tokens']} tokens, centroid {m['centroid_hz']:.0f} Hz, "
        f"{m['lufs']} LUFS, true peak {m['true_peak_db']} dBTP"
    )


if __name__ == "__main__":
    main()
