//! Shared data of both passes for ff-unmute: the shots, the tracker and the camera, all
//! driven by the music. `generated.rs` (from music/gen_hud.py) holds the measured events.
//!
//! Every position is in the flat pass's logical space, `W` x `H`.

mod generated;
pub use generated::*;

pub const FPS: usize = 30;
pub const W: f32 = 1920.0;
pub const H: f32 = 1080.0;
pub const SHOTS: usize = 22;
pub const TOTAL_FRAMES: usize = LENGTH_FRAMES;

/// JetBrains Mono advances 0.6 em per character.
pub const MONO_ADV: f32 = 0.6;
pub const LOG_SIZE: f32 = 24.0;
pub const LOG_LINE: f32 = 40.0;
pub const LOG_X: f32 = 150.0;
pub const LOG_TOP: f32 = 96.0;
pub const LOG_ROWS: usize = 15;

/// Spectrum panel: log-frequency axis from SPEC_LO to SPEC_HI Hz.
pub const SPEC_X0: f32 = 150.0;
pub const SPEC_X1: f32 = 1770.0;
pub const SPEC_TOP: f32 = 760.0;
pub const SPEC_BASE: f32 = 990.0;
pub const SPEC_LO: f32 = 40.0;
pub const SPEC_HI: f32 = 16_000.0;
pub const SPEC_BANDS: usize = 56;

/// Newest characters with a box on them.
const CHAIN: usize = 4;
/// A lock lasts this many frames from the kick.
pub const LOCK_FRAMES: usize = 2;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Listen,
    Typing,
    Hold,
    Done,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Palette {
    Paper,
    Thermal,
    Mono,
    Phosphor,
    Amber,
    Navy,
    Neon,
}

impl Palette {
    /// Five color stops from black (0) to white (1) of the flat pass, sRGB 0..1.
    pub fn stops(self) -> [[f32; 3]; 5] {
        let c = |hex: u32| {
            [
                ((hex >> 16) & 0xff) as f32 / 255.0,
                ((hex >> 8) & 0xff) as f32 / 255.0,
                (hex & 0xff) as f32 / 255.0,
            ]
        };
        let s = |a, b, cc, d, e| [c(a), c(b), c(cc), c(d), c(e)];
        match self {
            Palette::Paper => s(0xecebe6, 0xc9c8c2, 0x77766f, 0x2a2a28, 0x0c0c0c),
            Palette::Thermal => s(0x050310, 0x3a0b6b, 0xb3306a, 0xf6862a, 0xfcf6b4),
            Palette::Mono => s(0x050506, 0x2b2c2e, 0x8a8c90, 0xd9dadc, 0xffffff),
            Palette::Phosphor => s(0x010604, 0x05361b, 0x13a85a, 0x7dffb4, 0xeafff3),
            Palette::Amber => s(0x070300, 0x3d1a02, 0xb35a09, 0xffae3d, 0xfff3d6),
            Palette::Navy => s(0x071451, 0x0d2a8c, 0x4a63d6, 0xb8c4ff, 0xf4f6ff),
            Palette::Neon => s(0x000502, 0x003d1e, 0x00c46a, 0x5dffae, 0xf0fff6),
        }
    }
}

/// Virtual camera filming the flat screen. Angles in radians, zoom 1 shows the whole screen.
#[derive(Clone, Copy, Debug)]
pub struct Cam {
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub roll: f32,
    pub zoom: (f32, f32),
    pub pan: (f32, f32),
    pub blur: f32,
}

/// What a shot looks like; its timing and text come from `generated.rs`.
pub struct Look {
    pub kind: Kind,
    pub palette: Palette,
    pub cam: Cam,
    /// Baseline start of the hero line.
    pub hero: (f32, f32),
    pub size: f32,
    /// Track a frequency on the spectrum instead of the newest glyph (Hz).
    pub track_hz: Option<f32>,
}

const fn cam(
    tilt_x: f32,
    tilt_y: f32,
    roll: f32,
    z0: f32,
    z1: f32,
    px: f32,
    py: f32,
    blur: f32,
) -> Cam {
    Cam {
        tilt_x,
        tilt_y,
        roll,
        zoom: (z0, z1),
        pan: (px, py),
        blur,
    }
}

const fn look(kind: Kind, palette: Palette, cam: Cam, hero: (f32, f32), size: f32) -> Look {
    Look {
        kind,
        palette,
        cam,
        hero,
        size,
        track_hz: None,
    }
}

const fn on_hz(
    kind: Kind,
    palette: Palette,
    cam: Cam,
    hero: (f32, f32),
    size: f32,
    hz: f32,
) -> Look {
    Look {
        kind,
        palette,
        cam,
        hero,
        size,
        track_hz: Some(hz),
    }
}

use Kind::*;
use Palette::*;

pub const LOOKS: [Look; SHOTS] = [
    // bars 1-2: listening, slow
    look(
        Listen,
        Mono,
        cam(0.30, 0.42, -0.04, 1.6, 2.1, 80.0, 0.0, 1.0),
        (560.0, 520.0),
        120.0,
    ),
    look(
        Listen,
        Navy,
        cam(0.18, -0.38, 0.03, 1.9, 2.3, -60.0, 0.0, 1.0),
        (420.0, 520.0),
        100.0,
    ),
    // bars 3-4: the kick arrives
    look(
        Typing,
        Thermal,
        cam(0.34, -0.24, 0.04, 2.3, 2.6, 40.0, 0.0, 1.2),
        (420.0, 500.0),
        112.0,
    ),
    on_hz(
        Typing,
        Thermal,
        cam(0.62, 0.12, -0.08, 2.0, 2.3, 0.0, 0.0, 1.3),
        (460.0, 500.0),
        108.0,
        KICK_HZ,
    ),
    look(
        Typing,
        Phosphor,
        cam(0.14, 0.55, -0.05, 2.2, 2.5, 30.0, 0.0, 1.1),
        (300.0, 520.0),
        100.0,
    ),
    on_hz(
        Typing,
        Phosphor,
        cam(0.70, -0.20, 0.06, 1.9, 2.2, -20.0, 0.0, 1.2),
        (420.0, 500.0),
        108.0,
        BASS_HZ,
    ),
    // bars 5-6: a cut per beat
    look(
        Typing,
        Amber,
        cam(0.28, -0.50, 0.05, 2.6, 2.8, 0.0, 0.0, 1.2),
        (330.0, 520.0),
        104.0,
    ),
    look(
        Typing,
        Amber,
        cam(0.20, 0.48, -0.06, 3.0, 3.2, 0.0, 0.0, 1.3),
        (620.0, 520.0),
        120.0,
    ),
    look(
        Typing,
        Thermal,
        cam(0.40, 0.10, 0.02, 2.6, 2.9, 20.0, 0.0, 1.1),
        (520.0, 520.0),
        116.0,
    ),
    on_hz(
        Typing,
        Thermal,
        cam(0.66, -0.30, -0.04, 2.1, 2.4, 0.0, 0.0, 1.3),
        (420.0, 500.0),
        104.0,
        880.0,
    ),
    look(
        Typing,
        Navy,
        cam(0.22, -0.40, 0.04, 3.0, 3.2, 0.0, 0.0, 1.2),
        (620.0, 520.0),
        120.0,
    ),
    look(
        Typing,
        Mono,
        cam(0.10, 0.52, -0.03, 2.4, 2.6, 30.0, 0.0, 1.1),
        (260.0, 520.0),
        96.0,
    ),
    look(
        Typing,
        Phosphor,
        cam(0.36, -0.18, 0.05, 2.5, 2.7, 0.0, 0.0, 1.2),
        (300.0, 520.0),
        100.0,
    ),
    look(
        Typing,
        Thermal,
        cam(0.24, 0.40, -0.05, 2.7, 3.0, 0.0, 0.0, 1.2),
        (420.0, 520.0),
        112.0,
    ),
    // bar 7: an 8th per shot, closer every time
    look(
        Typing,
        Phosphor,
        cam(0.30, -0.45, 0.06, 3.0, 3.2, 0.0, 0.0, 1.3),
        (640.0, 520.0),
        130.0,
    ),
    look(
        Typing,
        Thermal,
        cam(0.30, 0.45, -0.06, 3.1, 3.3, 0.0, 0.0, 1.3),
        (640.0, 520.0),
        130.0,
    ),
    look(
        Typing,
        Amber,
        cam(0.50, -0.30, 0.04, 3.2, 3.4, 0.0, 0.0, 1.3),
        (580.0, 520.0),
        130.0,
    ),
    look(
        Typing,
        Navy,
        cam(0.50, 0.30, -0.04, 3.3, 3.5, 0.0, 0.0, 1.4),
        (640.0, 520.0),
        130.0,
    ),
    look(
        Typing,
        Paper,
        cam(0.20, -0.20, 0.08, 3.4, 3.6, 0.0, 0.0, 1.4),
        (640.0, 520.0),
        130.0,
    ),
    look(
        Typing,
        Thermal,
        cam(0.20, 0.20, -0.08, 3.6, 4.0, 0.0, 0.0, 1.5),
        (760.0, 520.0),
        140.0,
    ),
    // bar 7 beat 4: the held breath
    look(
        Hold,
        Mono,
        cam(0.40, 0.00, 0.00, 2.4, 2.2, 0.0, 0.0, 1.8),
        (900.0, 520.0),
        120.0,
    ),
    // bar 8: impact, collapse, unmuted
    look(
        Done,
        Neon,
        cam(0.44, -0.30, 0.05, 3.2, 2.0, 0.0, 0.0, 1.2),
        (960.0, 540.0),
        80.0,
    ),
];

/// Shot index and frame inside it for a video frame.
pub fn split(global: usize) -> (usize, usize) {
    let g = global.min(TOTAL_FRAMES - 1);
    let si = (0..SHOTS).rfind(|&i| SHOT_FRAMES[i] <= g).unwrap_or(0);
    (si, g - SHOT_FRAMES[si])
}

pub fn shot_len(si: usize) -> usize {
    SHOT_FRAMES[si + 1] - SHOT_FRAMES[si]
}

pub fn look_of(si: usize) -> &'static Look {
    &LOOKS[si.min(SHOTS - 1)]
}

pub fn text(si: usize) -> &'static str {
    SHOT_TEXT[si.min(SHOTS - 1)]
}

pub fn seconds(global: usize) -> f32 {
    global as f32 / FPS as f32
}

pub fn frame_of(t: f32) -> usize {
    (t * FPS as f32).round() as usize
}

/// Deterministic noise in 0..1.
pub fn rnd(a: u32, b: u32) -> f32 {
    let mut h = a.wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77) ^ 0x27d4_eb2f;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^= h >> 15;
    h as f32 / u32::MAX as f32
}

/// x of a frequency on the spectrum panel.
pub fn hz_x(hz: f32) -> f32 {
    let p = (hz.max(SPEC_LO) / SPEC_LO).ln() / (SPEC_HI / SPEC_LO).ln();
    SPEC_X0 + p.clamp(0.0, 1.0) * (SPEC_X1 - SPEC_X0)
}

/// Envelope of the most recent event before `t` with an exponential decay.
fn env(events: &[f32], t: f32, decay: f32) -> f32 {
    events
        .iter()
        .filter(|&&e| e <= t + 1e-4)
        .map(|&e| (-(t - e) / decay).exp())
        .fold(0.0, f32::max)
}

pub fn kick_env(t: f32) -> f32 {
    env(&KICKS, t, 0.12).max(env(&[IMPACT], t, 0.35))
}

pub fn clap_env(t: f32) -> f32 {
    env(&CLAPS, t, 0.07)
}

pub fn stutter_on(t: f32) -> bool {
    STUTTERS.iter().any(|&s| t >= s && t < s + 0.25)
}

/// Frames at which a kick (or the impact) lands.
pub fn is_lock(global: usize) -> bool {
    KICKS.iter().chain([IMPACT].iter()).any(|&k| {
        let f = frame_of(k);
        global >= f && global < f + LOCK_FRAMES
    })
}

fn last_lock_before(global: usize) -> Option<usize> {
    KICKS
        .iter()
        .chain([IMPACT].iter())
        .map(|&k| frame_of(k))
        .filter(|&f| f <= global)
        .max()
}

fn chars(si: usize) -> Vec<char> {
    text(si).chars().collect()
}

/// Characters of the hero line on screen at shot frame `f`: types in the first half.
pub fn visible(si: usize, f: usize) -> usize {
    let n = chars(si).len();
    let l = look_of(si);
    match l.kind {
        Listen | Typing => {
            let p = (f as f32 / (shot_len(si) as f32 * 0.5).max(1.0)).min(1.0);
            ((n as f32) * (0.3 + 0.7 * p)).ceil() as usize
        }
        _ => n,
    }
    .min(n)
}

/// Cell of character `i` of the hero line: x, y (top), w, h.
pub fn cell(si: usize, i: usize) -> (f32, f32, f32, f32) {
    let l = look_of(si);
    let adv = l.size * MONO_ADV;
    (
        l.hero.0 + i as f32 * adv,
        l.hero.1 - l.size * 0.78,
        adv,
        l.size * 0.98,
    )
}

/// The agent's log on screen at a video frame: (text, x, baseline y), newest at the bottom.
pub fn log_rows(global: usize) -> Vec<(&'static str, f32, f32)> {
    let t = seconds(global);
    let heard: Vec<&'static str> = LOG
        .iter()
        .filter(|(lt, _)| *lt <= t)
        .map(|(_, s)| *s)
        .collect();
    let from = heard.len().saturating_sub(LOG_ROWS);
    heard[from..]
        .iter()
        .enumerate()
        .map(|(r, s)| (*s, LOG_X, LOG_TOP + r as f32 * LOG_LINE))
        .filter(|(_, _, y)| (*y - 520.0).abs() > 110.0 || *y < 300.0)
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub hatch: bool,
    pub locked: bool,
    /// A measured value the camera writes next to the box (e.g. "55.7 Hz").
    pub value: Option<String>,
}

impl TrackBox {
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
}

pub struct Track {
    /// Boxes in chain order; the focus is on `target`.
    pub boxes: Vec<TrackBox>,
    pub target: usize,
    pub tails: Vec<((f32, f32), (f32, f32))>,
    pub focus: (f32, f32),
    pub lock: bool,
}

fn jitter(si: usize, f: usize, k: usize, amp: f32) -> (f32, f32) {
    let a = (si * 1000 + k * 37) as u32;
    (
        (rnd(a, f as u32) - 0.5) * 2.0 * amp,
        (rnd(a + 1, f as u32 + 999) - 0.5) * 2.0 * amp,
    )
}

fn boxed(x: f32, y: f32, w: f32, h: f32, pad: f32, j: (f32, f32), hatch: bool) -> TrackBox {
    TrackBox {
        x: x - pad + j.0,
        y: y - pad + j.1,
        w: w + 2.0 * pad,
        h: h + 2.0 * pad,
        hatch,
        locked: false,
        value: None,
    }
}

fn spectrum_box(hz: f32, j: (f32, f32), value: bool) -> TrackBox {
    let x = hz_x(hz);
    let mut b = boxed(
        x - 16.0,
        SPEC_TOP + 30.0,
        32.0,
        SPEC_BASE - SPEC_TOP - 40.0,
        0.0,
        j,
        true,
    );
    if value {
        b.value = Some(format!("{hz:.1} Hz"));
    }
    b
}

pub fn track(global: usize) -> Track {
    let (si, f) = split(global);
    let l = look_of(si);
    let t = seconds(global);
    let lock = is_lock(global);
    let mut boxes = vec![];
    let mut target = 0;

    match l.kind {
        Listen | Typing => {
            let ch = chars(si);
            let n = visible(si, f);
            let idx: Vec<usize> = (0..n).filter(|&i| !ch[i].is_whitespace()).collect();
            let from = idx.len().saturating_sub(CHAIN);
            for (k, &i) in idx[from..].iter().enumerate() {
                let (x, y, w, h) = cell(si, i);
                let pad = 2.0 + rnd(si as u32 + 3, i as u32) * 8.0;
                let hatch = rnd(si as u32 + 9, i as u32) > 0.35;
                boxes.push(boxed(x, y, w, h, pad, jitter(si, f, k, 1.6), hatch));
            }
            // After a kick in this shot, the newest box holds the last word; red on the kick.
            // In short shots (bar 7) the word box shows only while it is locked, so the word stays readable.
            let persist = shot_len(si) >= 15 || lock;
            if let Some(lf) = last_lock_before(global).filter(|&lf| lf >= SHOT_FRAMES[si] && persist) {
                let _ = lf;
                let last_word_start = (0..n)
                    .rev()
                    .take_while(|&i| !ch[i].is_whitespace())
                    .last()
                    .unwrap_or(0);
                let (x0, y, _, h) = cell(si, last_word_start);
                let (x1, _, w1, _) = cell(si, n.saturating_sub(1));
                if let Some(b) = boxes.last_mut() {
                    *b = boxed(x0, y, x1 + w1 - x0, h, 10.0, jitter(si, f, 77, 0.8), true);
                    b.locked = lock;
                }
            }
            target = boxes.len().saturating_sub(1);
        }
        Hold => {
            let (x, y, w, h) = cell(si, 0);
            boxes.push(boxed(x, y, w * 0.8, h, 8.0, jitter(si, f, 1, 0.6), false));
        }
        Done => {}
    }

    // Spectrum: the tracked frequency, every kick and every bleep of the last 6 frames.
    if let Some(hz) = l.track_hz {
        let mut b = spectrum_box(hz, jitter(si, f, 50, 1.0), true);
        b.locked = lock;
        boxes.push(b);
        target = boxes.len() - 1;
    } else if lock && l.kind != Done {
        boxes.push(spectrum_box(KICK_HZ, jitter(si, f, 51, 1.0), true));
    }
    for (k, (bt, hz)) in BLEEPS.iter().enumerate() {
        if *bt <= t && t - *bt < 6.0 / FPS as f32 {
            boxes.push(spectrum_box(*hz, jitter(si, f, 60 + k, 1.4), true));
        }
    }

    // Tokens: a small box on a glyph of the log for 3 frames.
    let rows = log_rows(global);
    if !rows.is_empty() {
        for (k, tt) in TOKENS.iter().enumerate() {
            if *tt <= t && t - *tt < 3.0 / FPS as f32 {
                let (s, x, y) = rows[k % rows.len()];
                let col = (k * 7) % s.chars().count().max(1);
                let adv = LOG_SIZE * MONO_ADV;
                boxes.push(boxed(
                    x + col as f32 * adv,
                    y - LOG_SIZE * 0.8,
                    adv,
                    LOG_SIZE,
                    3.0,
                    (0.0, 0.0),
                    k % 2 == 0,
                ));
            }
        }
    }

    if l.kind == Done {
        boxes = done_boxes(f);
        target = boxes.len().saturating_sub(1);
        if let Some(b) = boxes.last_mut() {
            b.locked = lock;
        }
    }

    let mut tails = vec![];
    if let Some(first) = boxes.first() {
        let c = first.center();
        let a = rnd(si as u32 + 200, 1) * std::f32::consts::TAU;
        tails.push((c, (c.0 + a.cos() * 2600.0, c.1 + a.sin() * 2600.0)));
    }
    let focus = boxes.get(target).map(|b| b.center()).unwrap_or(l.hero);
    Track {
        boxes,
        target,
        tails,
        focus,
        lock,
    }
}

/// The ending: twelve boxes fall into the centre in 10 frames, then the pill's two boxes.
fn done_boxes(f: usize) -> Vec<TrackBox> {
    let c = (960.0, 540.0);
    let p = ease_in_out((f as f32 / 10.0).min(1.0));
    if p < 1.0 {
        return (0..12usize)
            .map(|k| {
                let sx = 160.0 + rnd(77, k as u32) * (W - 320.0);
                let sy = 120.0 + rnd(78, k as u32) * (H - 240.0);
                let sw = 30.0 + rnd(79, k as u32) * 90.0;
                let sh = 26.0 + rnd(80, k as u32) * 60.0;
                boxed(
                    sx + (c.0 - 40.0 - sx) * p,
                    sy + (c.1 - 40.0 - sy) * p,
                    sw + (80.0 - sw) * p,
                    sh + (80.0 - sh) * p,
                    0.0,
                    (0.0, 0.0),
                    rnd(81, k as u32) > 0.4,
                )
            })
            .collect();
    }
    let (ix, iy) = done_icon();
    let (dx, dy) = done_label();
    vec![
        boxed(ix - 38.0, iy - 32.0, 76.0, 64.0, 0.0, (0.0, 0.0), true),
        boxed(dx + 1.0, dy - 62.0, 44.0, 70.0, 3.0, (0.0, 0.0), false),
    ]
}

/// Pill of the last shot: center x, y, width, height.
pub fn done_pill() -> (f32, f32, f32, f32) {
    (960.0, 540.0, 560.0, 132.0)
}

pub fn done_icon() -> (f32, f32) {
    let (cx, cy, w, _) = done_pill();
    (cx - w * 0.5 + 92.0, cy)
}

/// Baseline start of "unmuted".
pub fn done_label() -> (f32, f32) {
    let (cx, cy, _, _) = done_pill();
    (cx - 110.0, cy + 28.0)
}

pub fn ease_in_out(t: f32) -> f32 {
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

pub fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

/// Camera state for one video frame.
#[derive(Clone, Copy, Debug)]
pub struct Shot {
    pub aim: (f32, f32),
    pub focus: (f32, f32),
    pub zoom: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub roll: f32,
    pub blur: f32,
    pub palette: Palette,
    pub lock: bool,
    /// Exposure kick right after a cut, 0..1.
    pub cut: f32,
    /// Kick envelope 0..1 (zoom punch, exposure).
    pub punch: f32,
    /// Clap and stutter tear, 0..1.
    pub tear: f32,
    /// Fade to black at the end, 0..1.
    pub out: f32,
}

pub fn shot(global: usize) -> Shot {
    let (si, f) = split(global);
    let l = look_of(si);
    let t = seconds(global);
    let len = shot_len(si);
    let p = f as f32 / (len.max(2) - 1) as f32;
    // The aim trails the target over four frames (within the shot) and leans to the hero line.
    let mut sum = (0.0, 0.0);
    let mut n = 0.0;
    for k in 0..4 {
        if f >= k {
            let q = track(global - k).focus;
            sum.0 += q.0;
            sum.1 += q.1;
            n += 1.0;
        }
    }
    let lag = (sum.0 / n, sum.1 / n);
    let hero_mid = match l.kind {
        Listen | Typing if l.track_hz.is_none() => {
            let (x, y, w, h) = cell(si, visible(si, f).saturating_sub(1) / 2);
            (x + w * 0.5, y + h * 0.5)
        }
        _ => lag,
    };
    let punch = kick_env(t);
    let aim = (
        lag.0 * 0.72 + hero_mid.0 * 0.28 + l.cam.pan.0 * (p - 0.5),
        lag.1 * 0.72 + hero_mid.1 * 0.28 + l.cam.pan.1 * (p - 0.5),
    );
    let sway = (si as f32 * 1.7 + p * 1.3).sin() * 0.015;
    let zoom = (l.cam.zoom.0 + (l.cam.zoom.1 - l.cam.zoom.0) * ease_out(p)) * (1.0 + 0.06 * punch);
    Shot {
        aim,
        focus: track(global).focus,
        zoom,
        tilt_x: l.cam.tilt_x + sway,
        tilt_y: l.cam.tilt_y - sway * 0.7,
        roll: l.cam.roll + sway * 0.4,
        blur: l.cam.blur,
        palette: l.palette,
        lock: is_lock(global),
        cut: match f {
            0 => 1.0,
            1 => 0.45,
            2 => 0.15,
            _ => 0.0,
        },
        punch,
        tear: clap_env(t).max(if stutter_on(t) { 0.8 } else { 0.0 }),
        out: if si == SHOTS - 1 {
            ((t - 15.3) / 0.6).clamp(0.0, 1.0)
        } else {
            0.0
        },
    }
}

/// Must match `FOCAL` in lens.sksl.
pub const FOCAL: f32 = 1.7;

type V3 = [f32; 3];

fn mat_mul_vec(cols: &[V3; 3], v: V3) -> V3 {
    [
        cols[0][0] * v[0] + cols[1][0] * v[1] + cols[2][0] * v[2],
        cols[0][1] * v[0] + cols[1][1] * v[1] + cols[2][1] * v[2],
        cols[0][2] * v[0] + cols[1][2] * v[1] + cols[2][2] * v[2],
    ]
}

/// Same rotation as `rotation()` in lens.sksl (SkSL matrices are column-major): Rz * Ry * Rx.
fn rotate(s: &Shot, v: V3) -> V3 {
    let (cx, sx) = (s.tilt_x.cos(), s.tilt_x.sin());
    let (cy, sy) = (s.tilt_y.cos(), s.tilt_y.sin());
    let (cz, sz) = (s.roll.cos(), s.roll.sin());
    let rx = [[1.0, 0.0, 0.0], [0.0, cx, sx], [0.0, -sx, cx]];
    let ry = [[cy, 0.0, -sy], [0.0, 1.0, 0.0], [sy, 0.0, cy]];
    let rz = [[cz, sz, 0.0], [-sz, cz, 0.0], [0.0, 0.0, 1.0]];
    mat_mul_vec(&rz, mat_mul_vec(&ry, mat_mul_vec(&rx, v)))
}

/// Where a logical point of the flat screen lands in the lens output. `None` behind the camera.
pub fn project(s: &Shot, q: (f32, f32), res_w: f32, res_h: f32) -> Option<(f32, f32)> {
    let k = FOCAL * H / s.zoom;
    let (lx, ly) = ((q.0 - s.aim.0) / k, (q.1 - s.aim.1) / k);
    let eu = rotate(s, [1.0, 0.0, 0.0]);
    let ev = rotate(s, [0.0, 1.0, 0.0]);
    let p = [
        eu[0] * lx + ev[0] * ly,
        eu[1] * lx + ev[1] * ly,
        1.0 + eu[2] * lx + ev[2] * ly,
    ];
    if p[2] <= 0.05 {
        return None;
    }
    Some((
        FOCAL * p[0] / p[2] * res_h + 0.5 * res_w,
        FOCAL * p[1] / p[2] * res_h + 0.5 * res_h,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_matches_the_music() {
        assert_eq!(SHOT_FRAMES.len(), SHOTS + 1);
        assert_eq!(SHOT_TEXT.len(), SHOTS);
        assert_eq!(*SHOT_FRAMES.last().unwrap(), TOTAL_FRAMES);
        // every cut lands on an 8th note (0.25 s = 7.5 frames, rounded)
        for &f in &SHOT_FRAMES {
            let beats = f as f32 / FPS as f32 / (60.0 / BPM) * 2.0;
            assert!(
                (beats - beats.round()).abs() < 0.07,
                "cut at frame {f} is off the grid"
            );
        }
    }

    #[test]
    fn every_kick_is_a_lock() {
        for &k in &KICKS {
            assert!(is_lock(frame_of(k)), "kick at {k} s does not lock");
            assert!(track(frame_of(k)).lock);
        }
    }

    #[test]
    fn the_camera_keeps_the_target_in_frame() {
        for g in 0..TOTAL_FRAMES {
            let s = shot(g);
            let (x, y) = project(&s, s.focus, 1920.0, 1080.0).expect("in front of the camera");
            assert!(
                x > 150.0 && x < 1770.0 && y > 120.0 && y < 960.0,
                "frame {g}: target at {x:.0},{y:.0}"
            );
        }
    }

    #[test]
    fn hero_lines_fit_the_canvas() {
        for si in 0..SHOTS {
            let n = text(si).chars().count();
            if n > 0 {
                let (x, _, w, _) = cell(si, n - 1);
                assert!(x + w < W - 60.0, "shot {si} too wide");
            }
        }
    }
}
