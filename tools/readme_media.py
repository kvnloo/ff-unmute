"""Build the README images from the rendered video and the music.

Run from the workspace root after tools/render.sh:
    python3 -I tools/readme_media.py            # all
    python3 -I tools/readme_media.py hero flow  # some
Needs: Pillow, ffmpeg, img2webp (libwebp). Writes docs/media/{hero.webp, flow.jpg, stages.jpg, music.png}.
"""

import os
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "docs" / "media"
WORK = ROOT / "frames" / "readme"
FONT = ImageFont.truetype(str(ROOT / "media" / "JetBrainsMono-Bold.ttf"), 30)
SMALL = ImageFont.truetype(str(ROOT / "media" / "JetBrainsMono-Bold.ttf"), 22)
VIDEO = ROOT / "out" / "unmute.mp4"

# A kick lands on frame 270 (9.0 s): the lock is visible in every stage.
STAGE_FRAME = 270
STAGES = [
    ("flat", "1  flat pass: the DAW"),
    ("0", "2  perspective"),
    ("1", "3  + depth of field"),
    ("2", "4  + palette, aberration"),
    ("3", "5  + bloom, LED grid"),
    ("4", "6  + tear, grain, camera tracker"),
]
# One frame in the middle of every bar.
FLOW = [
    (30, "bar 1  listening"),
    (90, "bar 2  the pad opens"),
    (150, "bar 3  kick 55.7 Hz"),
    (210, "bar 4  bass 54.8 Hz"),
    (270, "bar 5  clap, bleeps"),
    (330, "bar 6  tokens"),
    (390, "bar 7  build"),
    (468, "bar 8  the whole track"),
]


def run(cmd, env=None):
    subprocess.run(
        cmd,
        check=True,
        cwd=ROOT,
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def label(img, text, font=FONT):
    d = ImageDraw.Draw(img, "RGBA")
    h = font.size + 30
    d.rectangle([0, 0, img.width, h], fill=(0, 0, 0, 150))
    d.text((22, 14), text, font=font, fill=(255, 255, 255, 255))
    return img


def grid(tiles, cols, w, h):
    rows = (len(tiles) + cols - 1) // cols
    out = Image.new("RGB", (cols * w, rows * h), "black")
    for i, t in enumerate(tiles):
        out.paste(t, ((i % cols) * w, (i // cols) * h))
    return out


def video_frame(n, path):
    run(
        [
            "ffmpeg",
            "-y",
            "-i",
            str(VIDEO),
            "-vf",
            f"select=eq(n\\,{n})",
            "-frames:v",
            "1",
            "-fps_mode",
            "vfr",
            str(path),
        ]
    )
    return Image.open(path).convert("RGB")


def stages():
    tiles = []
    for key, text in STAGES:
        d = WORK / f"stage-{key}"
        if key == "flat":
            run(["target/release/flat", "frame", str(STAGE_FRAME), "-o", str(d)])
        else:
            run(
                ["target/release/lens", "frame", str(STAGE_FRAME), "-o", str(d)],
                env={**os.environ, "LENS_STAGE": key},
            )
        img = (
            Image.open(d / f"{STAGE_FRAME}.png")
            .convert("RGB")
            .resize((960, 540), Image.LANCZOS)
        )
        tiles.append(label(img, text))
    grid(tiles, 3, 960, 540).save(OUT / "stages.jpg", quality=88)


def flow():
    tiles = [
        label(
            video_frame(n, WORK / f"flow-{n}.png").resize((640, 360), Image.LANCZOS),
            text,
            SMALL,
        )
        for n, text in FLOW
    ]
    grid(tiles, 4, 640, 360).save(OUT / "flow.jpg", quality=86)


def music():
    run(
        [
            "ffmpeg",
            "-y",
            "-i",
            "out/pass/music.wav",
            "-lavfi",
            "showspectrumpic=s=1600x420:legend=1:scale=log:fscale=log:stop=16000:color=magma,format=rgb24",
            str(OUT / "music.png"),
        ]
    )


def hero():
    """Animated WebP (GitHub renders it inline): 800 px, 12 fps, silent."""
    d = WORK / "hero"
    d.mkdir(parents=True, exist_ok=True)
    for p in d.glob("f*.png"):
        p.unlink()
    run(
        [
            "ffmpeg",
            "-y",
            "-i",
            str(VIDEO),
            "-vf",
            "fps=12,scale=800:-1:flags=lanczos",
            str(d / "f%03d.png"),
        ]
    )
    frames = sorted(str(p) for p in d.glob("f*.png"))
    run(
        [
            "img2webp",
            "-loop",
            "0",
            "-lossy",
            "-q",
            "68",
            "-m",
            "6",
            "-d",
            "83",
            *frames,
            "-o",
            str(OUT / "hero.webp"),
        ]
    )


if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    WORK.mkdir(parents=True, exist_ok=True)
    for step in sys.argv[1:] or ["stages", "flow", "music", "hero"]:
        globals()[step]()
    for p in sorted(OUT.iterdir()):
        print(f"{p.relative_to(ROOT)}  {p.stat().st_size / 1e6:.1f} MB")
