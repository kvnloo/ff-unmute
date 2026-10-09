"""The score: 8 bars at 120 BPM in A minor, 16 steps (16th notes) per bar.

Everything that sounds, and everything the picture reacts to, starts here.
A pattern string has one character per step: "x" hit, "X" accent, "." rest.
"""

BPM = 120
BEAT = 60.0 / BPM  # 0.5 s
STEP = BEAT / 4  # 16th note, 0.125 s
BARS = 8
BAR = BEAT * 4  # 2.0 s
LENGTH = BAR * BARS  # 16.0 s

# Two-bar progression from bar 3 on, one chord per bar.
PROGRESSION = {1: "Am", 2: "Am", 3: "Am", 4: "F", 5: "Dm", 6: "E", 7: "F", 8: "Am"}

# Pad voicings (MIDI notes) and bass roots.
VOICING = {
    "Am": [57, 60, 64, 69],  # A3 C4 E4 A4
    "F": [53, 57, 60, 65],  # F3 A3 C4 F4
    "Dm": [50, 57, 62, 65],  # D3 A3 D4 F4
    "E": [52, 56, 59, 64],  # E3 G#3 B3 E4
}
ROOT = {"Am": 33, "F": 29, "Dm": 26, "E": 28}  # A1 F1 D1 E1

# Drum patterns per section. Bars are 1-based.
KICK = {
    "groove": "x...x...x...x...",
}
CLAP = {
    "groove": "....x.......x...",
}
HAT = {
    "intro": "......x.......x.",
    "groove": "..x...x...x...x.",
    "busy": "xxXxxxXxxxXxxxXx",
}
OPEN_HAT = {
    "busy": "..x...x...x...x.",
}
# Off-beat rolling bass, "x" = play the chord root, "o" = octave up.
BASS = {
    "groove": "..x.ox.x..x.ox.x",
}
# Arp from bar 5: chord tones up an octave, pluck.
ARP = "x.x.x.x.x.x.x.x."

SECTIONS = {
    1: "intro",
    2: "intro",
    3: "groove",
    4: "groove",
    5: "full",
    6: "full",
    7: "build",
    8: "drop",
}

# Bar 7 snare roll: steps that hit, accelerating (16ths, then 32nds in the last beat).
ROLL_16 = "x.x.x.x.xxxxxxxx"
IMPACT_AT = 7 * BAR  # 14.0 s, the downbeat of bar 8


def bar_start(bar):
    return (bar - 1) * BAR


def steps(pattern, bar):
    """Times of the hits of a pattern in a bar, with accent flags."""
    out = []
    for i, ch in enumerate(pattern):
        if ch in "xXo":
            out.append((bar_start(bar) + i * STEP, ch))
    return out
