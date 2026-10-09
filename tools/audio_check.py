"""Band balance per section of out/pass/music.wav (numbers I can read instead of listening).

    python3 -I tools/audio_check.py
"""

from pathlib import Path
import wave

import numpy as np
from scipy.signal import welch

ROOT = Path(__file__).resolve().parent.parent
SECTIONS = [("intro", 0, 4), ("groove", 4, 8), ("full", 8, 12), ("build", 12, 14), ("drop", 14, 16)]
BANDS = [("<120", 20, 120), ("120-2k", 120, 2000), ("2-5k", 2000, 5000), (">5k", 5000, 20000)]

w = wave.open(str(ROOT / "out" / "pass" / "music.wav"))
sr = w.getframerate()
x = np.frombuffer(w.readframes(w.getnframes()), "<i2").reshape(-1, 2) / 32768
m = x.mean(1)
print("section   rms dBFS  " + "  ".join(f"{b:>7s}" for b, _, _ in BANDS) + "   (dB of the section's total)")
for name, a, b in SECTIONS:
    seg = m[int(a * sr) : int(b * sr)]
    rms = 20 * np.log10(np.sqrt((seg**2).mean()) + 1e-9)
    f, p = welch(seg, sr, nperseg=8192)
    tot = p.sum()
    vals = [10 * np.log10(p[(f >= lo) & (f < hi)].sum() / tot + 1e-12) for _, lo, hi in BANDS]
    print(f"{name:8s} {rms:8.1f}  " + "  ".join(f"{v:7.1f}" for v in vals))
print("clipped samples", int((np.abs(x) > 0.999).sum()), "  L/R correlation", round(float(np.corrcoef(x[:, 0], x[:, 1])[0, 1]), 2))
