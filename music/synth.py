"""Render the score to out/pass/music.wav and every note to out/events.json.

Run from the workspace root: python3 -I music/synth.py
Needs numpy, scipy and ffmpeg (for the loudness measurement).
"""

import json
import re
import subprocess
import sys
import wave
from pathlib import Path

import numpy as np
from scipy.signal import butter, fftconvolve, sosfilt, sosfilt_zi

sys.path.insert(0, str(Path(__file__).resolve().parent))
import score as S  # noqa: E402

SR = 48_000
ROOT = Path(__file__).resolve().parent.parent
OUT_WAV = ROOT / "out" / "pass" / "music.wav"
OUT_EVENTS = ROOT / "out" / "events.json"
STEMS = ROOT / "out" / "stems"
TARGET_LUFS = -14.0
CEILING_DB = -1.5
N = int(SR * S.LENGTH)
rng = np.random.default_rng(1209)

events = []


def note(kind, t, **kw):
    events.append({"t": round(t, 4), "kind": kind, **kw})


def midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def tt(seconds):
    return np.arange(int(SR * seconds)) / SR


# ---------- oscillators and filters ----------


def poly_blep(phase, dt):
    """Band-limited step correction for a saw; phase and dt are arrays (0..1, cycles per sample)."""
    out = np.zeros_like(phase)
    a = phase < dt
    x = phase[a] / dt[a]
    out[a] = x + x - x * x - 1.0
    b = phase > 1.0 - dt
    x = (phase[b] - 1.0) / dt[b]
    out[b] = x * x + x + x + 1.0
    return out


def saw(freq, n, phase0=0.0):
    f = np.broadcast_to(np.asarray(freq, dtype=float), (n,))
    dt = f / SR
    phase = (phase0 + np.cumsum(dt)) % 1.0
    return 2.0 * phase - 1.0 - poly_blep(phase, dt)


def sine(freq, n, phase0=0.0):
    f = np.broadcast_to(np.asarray(freq, dtype=float), (n,))
    return np.sin(2 * np.pi * (phase0 + np.cumsum(f) / SR))


def env_adsr(n, a, d, s, r, hold=None):
    """Attack, decay, sustain level, release (s). Note held for `hold` s (default: until release)."""
    t = np.arange(n) / SR
    hold = (n / SR - r) if hold is None else hold
    e = np.where(t < a, t / max(a, 1e-4), 1.0)
    e = np.where((t >= a) & (t < a + d), 1.0 - (1.0 - s) * (t - a) / max(d, 1e-4), e)
    e = np.where((t >= a + d) & (t < hold), s, e)
    rel = np.clip(1.0 - (t - hold) / max(r, 1e-4), 0.0, 1.0)
    e = np.where(t >= hold, s * rel, e)
    return e


def exp_env(n, attack, decay):
    t = np.arange(n) / SR
    return np.clip(t / max(attack, 1e-4), 0, 1) * np.exp(-t / max(decay, 1e-4))


def filt(x, kind, cutoff, order=2, q=None):
    sos = butter(order, cutoff, btype=kind, fs=SR, output="sos")
    return sosfilt(sos, x)


def sweep_lowpass(x, cutoff, order=2, block=256):
    """Low-pass with a cutoff per sample (array), applied block by block."""
    y = np.empty_like(x)
    zi = None
    for i in range(0, len(x), block):
        c = float(
            np.clip(cutoff[min(i + block // 2, len(cutoff) - 1)], 30.0, SR * 0.45)
        )
        sos = butter(order, c, btype="lowpass", fs=SR, output="sos")
        if zi is None:
            zi = sosfilt_zi(sos) * x[0]
        seg, zi = sosfilt(sos, x[i : i + block], zi=zi)
        y[i : i + block] = seg
    return y


def place(buf, sig, t, gain_db=0.0, pan=0.0):
    """Add a mono or stereo signal to the stereo buffer at t seconds; equal-power pan."""
    i = int(round(t * SR))
    if i >= N:
        return
    g = 10 ** (gain_db / 20)
    if sig.ndim == 1:
        a = (pan + 1) * np.pi / 4
        sig = np.stack([sig * np.cos(a), sig * np.sin(a)], axis=1)
    sig = sig[: N - i] * g
    buf[i : i + len(sig)] += sig


def reverb_ir(seconds=2.2, damp=6500.0, predelay=0.018):
    n = int(SR * seconds)
    t = np.arange(n) / SR
    decay = np.exp(-t * 6.9 / seconds)  # -60 dB at `seconds`
    ir = (
        np.stack([rng.standard_normal(n), rng.standard_normal(n)], axis=1)
        * decay[:, None]
    )
    ir = np.stack([filt(ir[:, c], "lowpass", damp) for c in range(2)], axis=1)
    pre = np.zeros((int(SR * predelay), 2))
    ir = np.concatenate([pre, ir])
    return ir / np.sqrt(np.sum(ir**2) / 2)


IR = reverb_ir()


def reverb(stereo, wet):
    w = np.stack(
        [fftconvolve(stereo[:, c], IR[:, c])[: len(stereo)] for c in range(2)], axis=1
    )
    w = np.stack([filt(w[:, c], "highpass", 180) for c in range(2)], axis=1)
    return stereo + w * wet


def pingpong(stereo, delay_s, feedback, wet, repeats=6):
    out = stereo.copy()
    d = int(SR * delay_s)
    tap = stereo.copy()
    for k in range(1, repeats + 1):
        tap = filt(tap.T.copy(), "lowpass", 4500).T * feedback
        tap = tap[:, ::-1]  # swap channels each repeat
        shifted = np.zeros_like(stereo)
        if k * d < len(stereo):
            shifted[k * d :] = tap[: len(stereo) - k * d]
        out += shifted * wet
    return out


# ---------- instruments ----------


def kick(accent=1.0):
    n = int(SR * 0.45)
    t = np.arange(n) / SR
    f = 46 + 110 * np.exp(-t / 0.035)
    body = sine(f, n) * exp_env(n, 0.001, 0.17)
    click = filt(rng.standard_normal(n), "bandpass", [1800, 5000]) * exp_env(
        n, 0.0002, 0.004
    )
    return np.tanh((body + 0.18 * click) * 1.6 * accent)


def clap():
    n = int(SR * 0.35)
    noise = filt(rng.standard_normal(n), "bandpass", [900, 2600], order=2)
    e = np.zeros(n)
    for k, off in enumerate((0.0, 0.009, 0.019, 0.031)):
        i = int(off * SR)
        e[i:] += (0.7 if k < 3 else 1.0) * exp_env(
            n - i, 0.0005, 0.012 if k < 3 else 0.11
        )
    return noise * e


def hat(open_=False, accent=1.0):
    n = int(SR * (0.22 if open_ else 0.05))
    metal = sum(
        np.sign(np.sin(2 * np.pi * f * np.arange(n) / SR))
        for f in (2950, 4100, 5300, 6900, 8200)
    )
    sig = filt(metal * 0.15 + rng.standard_normal(n) * 0.6, "highpass", 7200, order=4)
    return sig * exp_env(n, 0.0005, 0.06 if open_ else 0.012) * accent


def snare(vel):
    n = int(SR * 0.18)
    tone = sine(np.full(n, 190.0), n) * exp_env(n, 0.0005, 0.04)
    noise = filt(rng.standard_normal(n), "bandpass", [1500, 6500]) * exp_env(
        n, 0.0005, 0.06
    )
    return (0.5 * tone + 0.8 * noise) * vel


def bass_note(midi, length, cutoff_peak):
    n = int(SR * length)
    f = midi_hz(midi)
    # 25 % pulse from two band-limited saws: colder and more digital than a saw
    s = 0.75 * (saw(np.full(n, f), n) - saw(np.full(n, f), n, 0.25))
    sub = sine(np.full(n, f), n)
    cutoff = 140 + cutoff_peak * np.exp(-np.arange(n) / SR / 0.09)
    s = sweep_lowpass(s, cutoff, order=2, block=128)
    e = env_adsr(n, 0.003, 0.08, 0.7, 0.03)
    return filt(np.tanh((s * 0.9 + sub * 0.45) * e * 1.4), "highpass", 38)


def pad_chord(midis, length, cutoff):
    n = int(SR * length)
    left = np.zeros(n)
    right = np.zeros(n)
    for m in midis:
        f = midi_hz(m)
        for k, det in enumerate((-0.11, -0.05, 0.0, 0.06, 0.12)):
            v = saw(np.full(n, f * 2 ** (det / 12)), n, rng.random())
            if k % 2:
                left += v
            else:
                right += v
            if k == 2:
                left += v * 0.5
                right += v * 0.5
    e = env_adsr(n, 0.35, 0.4, 0.85, 0.6)
    out = np.stack([sweep_lowpass(left, cutoff), sweep_lowpass(right, cutoff)], axis=1)
    return out * e[:, None] / (len(midis) * 3.2)


def pluck(midi):
    n = int(SR * 0.35)
    f = midi_hz(midi)
    phase = np.cumsum(np.full(n, f / SR)) % 1.0
    sq = np.where(phase < 0.5, 1.0, -1.0) * 0.6 + saw(np.full(n, f), n) * 0.4
    cutoff = 600 + 3200 * np.exp(-np.arange(n) / SR / 0.05)
    return sweep_lowpass(sq, cutoff, block=128) * exp_env(n, 0.002, 0.12)


def riser(length):
    n = int(SR * length)
    p = np.arange(n) / n
    noise = rng.standard_normal(n)
    cutoff = 400 + 9000 * p**2.2
    sig = sweep_lowpass(noise, cutoff, order=2)
    tone = saw(110 * 2 ** (p * 2.0), n) * 0.25
    tone = sweep_lowpass(tone, 300 + 3000 * p**2)
    return (sig * 0.5 + tone) * p**1.6


def impact():
    n = int(SR * 2.2)
    t = np.arange(n) / SR
    sub = sine(62 * np.exp(-t / 0.9) + 28, n) * exp_env(n, 0.001, 0.7)
    crash = (
        filt(rng.standard_normal(n), "bandpass", [3000, 9000]) * exp_env(n, 0.001, 0.35) * 0.18
    )
    body = kick(1.2)
    out = sub.copy()
    out[: len(body)] += body
    return np.tanh(out * 1.2) * 0.9 + crash


def crush(x, bits=7, hold=1):
    """Bit depth reduction with TPDF dither; hold > 1 also lowers the sample rate (sample and hold)."""
    q = 2.0 / (2**bits)
    d = (rng.random(x.shape) - rng.random(x.shape)) * q
    y = np.round((x + d) / q) * q
    if hold > 1:
        y = y[(np.arange(len(y)) // hold) * hold]
    return y


def blip(midi, length=0.035):
    """A short FM bleep: the agent computing."""
    n = int(SR * length)
    f = midi_hz(midi)
    t = np.arange(n) / SR
    mod = np.sin(2 * np.pi * f * 3.5 * t) * 2.2 * np.exp(-t / 0.01)
    return np.sin(2 * np.pi * f * t + mod) * exp_env(n, 0.0005, length / 3)


def token_click():
    """One token: a tiny high click."""
    n = int(SR * 0.004)
    return filt(rng.standard_normal(n), "highpass", 8500) * exp_env(n, 0.0001, 0.0008)


def stutter(buf, t_end, slice_s, count):
    """Buffer repeat: the slice that starts count slices before t_end plays count times."""
    sl = int(slice_s * SR)
    end = int(t_end * SR)
    start = end - sl * count
    piece = buf[start : start + sl].copy()
    for k in range(count):
        buf[start + k * sl : start + (k + 1) * sl] = piece * (1 - 0.1 * k)


def gate(n, pattern, t0):
    """A 16th-note volume gate with soft edges, for a signal that starts at bar time t0."""
    g = np.ones(n)
    step = int(S.STEP * SR)
    edge = int(0.004 * SR)
    for k in range(n // step + 1):
        if pattern[k % len(pattern)] == ".":
            a, b = k * step, min(n, (k + 1) * step)
            g[a:b] = 0.25
    return filt(g, "lowpass", 1.0 / (edge / SR))


# Pentatonic A minor, high: the agent's alphabet.
DATA_NOTES = [81, 84, 86, 88, 91, 93]
DENSITY = {"intro": 0.12, "groove": 0.22, "full": 0.32, "build": 0.55, "drop": 0.0}
GATE = "x.xxx.xxx.x.xx.x"


# ---------- arrangement ----------


def render():
    drums = np.zeros((N, 2))
    kick_bus = np.zeros((N, 2))
    bass_bus = np.zeros((N, 2))
    pad_bus = np.zeros((N, 2))
    arp_bus = np.zeros((N, 2))
    fx_bus = np.zeros((N, 2))
    data_bus = np.zeros((N, 2))
    token_stem = np.zeros((N, 2))
    kick_times = []

    for bar in range(1, S.BARS + 1):
        sec = S.SECTIONS[bar]
        chord = S.PROGRESSION[bar]
        t0 = S.bar_start(bar)
        note("bar", t0, bar=bar, chord=chord, section=sec)

        # Pad: every bar, the filter opens over the intro and the build.
        length = S.BAR + (1.4 if bar == S.BARS else 0.25)
        n = int(SR * length)
        p = np.arange(n) / n
        if sec == "intro":
            cutoff = 250 + (bar - 1 + p) / 2 * 2200
        elif sec == "build":
            cutoff = 2400 + p * 3600
        elif sec == "drop":
            cutoff = 6000 - p * 3500
        else:
            cutoff = np.full(n, 2400.0)
        if sec in ("full", "build"):
            pad = pad_chord(S.VOICING[chord], length, cutoff)
            pad *= gate(len(pad), GATE, t0)[:, None]
        else:
            pad = pad_chord(S.VOICING[chord], length, cutoff)
        place(
            pad_bus,
            pad,
            t0,
            gain_db=-2 if sec != "intro" else 0,
        )
        note("chord", t0, chord=chord, notes=S.VOICING[chord])

        # The agent: token clicks on 32nds, FM bleeps on 16ths, denser as it works harder.
        dens = DENSITY[sec] if bar > 1 else 0.0
        for k in range(32):
            t = t0 + k * S.STEP / 2
            if rng.random() < dens:
                click = token_click()
                place(data_bus, click, t, gain_db=-14, pan=float(rng.uniform(-0.8, 0.8)))
                place(token_stem, click, t)
                note("token", t)
            if k % 2 == 0 and rng.random() < dens * 0.85:
                m = int(rng.choice(DATA_NOTES))
                place(data_bus, blip(m), t, gain_db=-17, pan=float(rng.uniform(-0.6, 0.6)))
                note("blip", t, midi=m, hz=round(midi_hz(m), 1))

        # Kick.
        if sec in ("groove", "full", "build"):
            for i, (t, _) in enumerate(S.steps(S.KICK["groove"], bar)):
                if sec == "build" and i == 3:
                    continue  # leave beat 4 of bar 7 empty before the impact
                place(kick_bus, kick(), t, gain_db=-4)
                kick_times.append(t)
                note("kick", t, beat=i + 1)

        # Clap on 2 and 4.
        if sec in ("full",):
            for t, _ in S.steps(S.CLAP["groove"], bar):
                c = clap()
                place(drums, c, t, gain_db=-7, pan=0.05)
                note("clap", t)

        # Hats.
        pat = {
            "intro": S.HAT["intro"],
            "groove": S.HAT["groove"],
            "full": S.HAT["busy"],
            "build": S.HAT["busy"],
        }.get(sec)
        if pat:
            for k, (t, ch) in enumerate(S.steps(pat, bar)):
                acc = 1.0 if ch == "X" else 0.55
                place(
                    drums,
                    hat(False, acc),
                    t + rng.uniform(-0.002, 0.004),
                    gain_db=-15 if sec == "intro" else -13,
                    pan=0.25 if k % 2 else -0.15,
                )
                note("hat", t)
        if sec == "full":
            for t, _ in S.steps(S.OPEN_HAT["busy"], bar):
                place(drums, hat(True), t, gain_db=-17, pan=0.3)

        # Bass.
        if sec in ("groove", "full", "build"):
            root = S.ROOT[chord]
            for k, (t, ch) in enumerate(S.steps(S.BASS["groove"], bar)):
                if sec == "build" and t >= t0 + 3 * S.BEAT:
                    break
                m = root + (12 if ch == "o" else 0)
                peak = 900 if sec == "groove" else 1500 if sec == "full" else 2200
                place(bass_bus, bass_note(m, S.STEP * 1.6, peak), t, gain_db=-12)
                note("bass", t, midi=m, hz=round(midi_hz(m), 2))

        # Arp from bar 5.
        if sec in ("full", "build"):
            tones = [m + 12 for m in S.VOICING[chord]]
            order = [0, 1, 2, 3, 2, 1, 2, 3]
            for k, (t, _) in enumerate(S.steps(S.ARP, bar)):
                m = tones[order[k % len(order)]]
                place(arp_bus, pluck(m), t, gain_db=-10, pan=-0.35 if k % 2 else 0.35)
                note("arp", t, midi=m)

        # Bar 7: snare roll and riser.
        if sec == "build":
            for k, (t, _) in enumerate(S.steps(S.ROLL_16, bar)):
                vel = 0.35 + 0.65 * k / 16
                place(drums, snare(vel), t, gain_db=-9)
                if k >= 12:  # 32nds in the last beat
                    place(drums, snare(vel * 0.8), t + S.STEP / 2, gain_db=-10)
                note("roll", t, vel=round(vel, 2))
            place(fx_bus, riser(S.BAR), t0, gain_db=-9)
            note("riser", t0, length=S.BAR)

        # Bar 8: impact and a held bass.
        if sec == "drop":
            place(drums, impact(), S.IMPACT_AT, gain_db=-4)
            place(bass_bus, bass_note(S.ROOT["Am"], 1.6, 1200), S.IMPACT_AT, gain_db=-11)
            kick_times.append(S.IMPACT_AT)
            note("impact", S.IMPACT_AT)
            for k, m in enumerate((81, 84, 88, 93)):
                t = S.IMPACT_AT + 1.0 + k * S.STEP
                place(data_bus, blip(m, 0.06), t, gain_db=-13, pan=-0.3 + 0.2 * k)
                note("signature", t, midi=m)
            note("kick", S.IMPACT_AT, beat=1)

    # Sidechain: bass, pad and arp duck under every kick.
    sc = np.ones(N)
    duck_len = int(SR * 0.32)
    shape = 1.0 - 0.65 * np.exp(-np.arange(duck_len) / SR / 0.09)
    shape[: int(SR * 0.004)] = np.linspace(1.0, shape[int(SR * 0.004)], int(SR * 0.004))
    for t in kick_times:
        i = int(t * SR)
        j = min(N, i + duck_len)
        sc[i:j] = np.minimum(sc[i:j], shape[: j - i])
    for bus in (bass_bus, pad_bus, arp_bus):
        bus *= sc[:, None]

    # Dithered grain: pad at 7 bits, arp and data at 6 bits and a lower sample rate.
    pad_bus = crush(pad_bus, bits=7)
    arp_bus = crush(arp_bus, bits=6, hold=3)
    data_bus = crush(data_bus, bits=6, hold=2)
    data_bus = pingpong(data_bus, S.STEP * 1.5, 0.3, 0.35, repeats=3)

    pad_bus = reverb(pad_bus, 0.35)
    arp_bus = reverb(pingpong(arp_bus, S.BEAT * 0.75, 0.42, 0.55), 0.25)
    fx_bus = reverb(fx_bus, 0.3)

    # Mono below 120 Hz keeps the low end solid.
    # Stems for music/analyze.py: it measures each part on its own, the mix is unchanged.
    for name, bus in (("kick", kick_bus), ("bass", bass_bus), ("data", data_bus), ("tokens", token_stem)):
        write(STEMS / f"{name}.wav", bus / (np.max(np.abs(bus)) + 1e-9) * 0.9)

    drums = drums + kick_bus
    mix = drums + bass_bus + pad_bus + arp_bus + fx_bus + data_bus
    low = np.stack([filt(mix[:, c], "lowpass", 120) for c in range(2)], axis=1)
    high = mix - low
    mix = high + low.mean(axis=1, keepdims=True)
    mix = np.stack([filt(mix[:, c], "highpass", 28) for c in range(2)], axis=1)

    # A gentle dip at 3 kHz takes the edge off the hats and claps.
    edge = np.stack(
        [filt(mix[:, c], "bandpass", [2200, 4800]) for c in range(2)], axis=1
    )
    mix = mix - 0.25 * edge

    # Buffer-repeat stutters into bars 5 and 7.
    for t_end in (S.bar_start(5), S.bar_start(7)):
        stutter(mix, t_end, S.STEP / 2, 4)
        note("stutter", t_end - 4 * S.STEP / 2, slices=4)
    return mix


def bus_compress(x, threshold_db=-14.0, ratio=2.0, attack=0.01, release=0.15):
    level = np.sqrt(filt(np.mean(x**2, axis=1), "lowpass", 30) + 1e-12)
    db = 20 * np.log10(level + 1e-9)
    over = np.maximum(0.0, db - threshold_db)
    gain_db = -over * (1 - 1 / ratio)
    # smooth the gain with attack/release
    g = np.empty_like(gain_db)
    a = np.exp(-1 / (SR * attack))
    r = np.exp(-1 / (SR * release))
    prev = 0.0
    for i, v in enumerate(gain_db[::32]):
        coef = a if v < prev else r
        prev = coef * prev + (1 - coef) * v
        g[i * 32 : (i + 1) * 32] = prev
    return x * (10 ** (g / 20))[:, None]


def write(path, x):
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes((np.clip(x, -1, 1) * 32767).astype("<i2").tobytes())


def lufs(path):
    out = subprocess.run(
        [
            "ffmpeg",
            "-nostats",
            "-i",
            str(path),
            "-af",
            "ebur128=peak=true",
            "-f",
            "null",
            "-",
        ],
        capture_output=True,
        text=True,
    ).stderr
    i = float(re.findall(r"I:\s+(-?[\d.]+) LUFS", out)[-1])
    tp = float(re.findall(r"Peak:\s+(-?[\d.]+) dBFS", out)[-1])
    return i, tp


def master(mix):
    mix = bus_compress(mix)
    mix /= np.max(np.abs(mix)) + 1e-9
    fade = int(SR * 0.6)
    mix[-fade:] *= np.linspace(1, 0, fade)[:, None] ** 1.5
    ceiling = 10 ** (CEILING_DB / 20)
    base = mix * 10 ** (-10 / 20)
    gain = 1.0
    for _ in range(4):
        out = np.tanh(base * gain / ceiling) * ceiling
        write(OUT_WAV, out)
        i, _ = lufs(OUT_WAV)
        gain *= 10 ** ((TARGET_LUFS - i) / 20)
    return out


if __name__ == "__main__":
    mix = render()
    master(mix)
    i, tp = lufs(OUT_WAV)
    OUT_EVENTS.write_text(
        json.dumps({"bpm": S.BPM, "length": S.LENGTH, "events": events}, indent=1)
    )
    print(
        f"wrote {OUT_WAV.relative_to(ROOT)}  {S.LENGTH:.1f} s  {i:.1f} LUFS  true peak {tp:.1f} dBTP  "
        f"{len(events)} events -> {OUT_EVENTS.relative_to(ROOT)}"
    )
