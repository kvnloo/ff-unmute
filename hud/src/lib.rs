//! Shared data of both passes for ff-unmute: a DAW timeline that scrolls under a fixed
//! playhead, the tracker that locks onto every event the playhead touches, and one
//! continuous camera move. `generated.rs` (music/gen_hud.py) holds the measured events.
//!
//! Every position is in the flat pass's logical space, `W` x `H`.

mod generated;
pub use generated::*;

pub const FPS: usize = 30;
pub const W: f32 = 1920.0;
pub const H: f32 = 1080.0;
pub const TOTAL_FRAMES: usize = LENGTH_FRAMES;

/// The playhead stays here; the timeline moves left at `PX_PER_S`.
pub const PLAYHEAD_X: f32 = 960.0;
pub const PX_PER_S: f32 = 320.0;
/// After the impact the timeline stops this long later.
pub const STOP_AFTER: f32 = 0.35;

pub const MONO_ADV: f32 = 0.6;
pub const LANE_LABEL_W: f32 = 150.0;

/// A horizontal track of the DAW view.
#[derive(Clone, Copy, Debug)]
pub struct Lane {
    pub name: &'static str,
    pub top: f32,
    pub h: f32,
}

impl Lane {
    pub fn mid(&self) -> f32 {
        self.top + self.h * 0.5
    }
}

pub const PAD: Lane = Lane {
    name: "pad",
    top: 150.0,
    h: 64.0,
};
pub const KICK: Lane = Lane {
    name: "kick",
    top: 226.0,
    h: 58.0,
};
pub const CLAP: Lane = Lane {
    name: "clap · hat",
    top: 296.0,
    h: 48.0,
};
pub const BASS_LANE: Lane = Lane {
    name: "bass",
    top: 356.0,
    h: 112.0,
};
pub const WAVE_LANE: Lane = Lane {
    name: "master",
    top: 480.0,
    h: 260.0,
};
pub const DATA: Lane = Lane {
    name: "agent",
    top: 752.0,
    h: 150.0,
};
pub const LANES: [Lane; 6] = [PAD, KICK, CLAP, BASS_LANE, WAVE_LANE, DATA];
pub const RULER_Y: f32 = 128.0;

/// FFT strip on the right of the playhead: frequency up, level to the right.
pub const FFT_X: f32 = PLAYHEAD_X + 8.0;
pub const FFT_W: f32 = 130.0;
pub const FFT_TOP: f32 = 150.0;
pub const FFT_BOTTOM: f32 = 902.0;
pub const FFT_LO: f32 = 40.0;
pub const FFT_HI: f32 = 16_000.0;
pub const FFT_BANDS: usize = 48;

pub const LOCK_FRAMES: usize = 2;

pub fn seconds(global: usize) -> f32 {
    global as f32 / FPS as f32
}

pub fn frame_of(t: f32) -> usize {
    (t * FPS as f32).round() as usize
}

/// Time used to lay out the timeline: it stops shortly after the impact.
pub fn layout_time(global: usize) -> f32 {
    seconds(global).min(IMPACT + STOP_AFTER)
}

/// Length of the track in seconds.
pub fn length_s() -> f32 {
    LENGTH_FRAMES as f32 / FPS as f32
}

/// The ending has no fixed card: after the impact the view zooms out until the whole track
/// fits the screen, so the last image is this track's own waveform, notes and locks.
/// 0 while following the playhead, 1 once the whole track is in view.
pub fn end_mix(global: usize) -> f32 {
    ease_in_out(((seconds(global) - IMPACT - 0.25) / 1.1).clamp(0.0, 1.0))
}

fn follow_x(t: f32, global: usize) -> f32 {
    PLAYHEAD_X + (t - layout_time(global)) * PX_PER_S
}

fn full_x(t: f32) -> f32 {
    LANE_LABEL_W + 30.0 + t / length_s() * (W - LANE_LABEL_W - 80.0)
}

/// x of a timeline time on the screen at a video frame.
pub fn time_x(t: f32, global: usize) -> f32 {
    let e = end_mix(global);
    follow_x(t, global) + (full_x(t) - follow_x(t, global)) * e
}

/// Pixels per second of the timeline at a video frame.
pub fn px_per_s(global: usize) -> f32 {
    let full = (W - LANE_LABEL_W - 80.0) / length_s();
    PX_PER_S + (full - PX_PER_S) * end_mix(global)
}

/// Where the playhead is drawn: fixed while following, at its place in the track at the end.
pub fn playhead_x(global: usize) -> f32 {
    time_x(layout_time(global), global)
}

pub fn fft_y(hz: f32) -> f32 {
    let p = (hz.max(FFT_LO) / FFT_LO).ln() / (FFT_HI / FFT_LO).ln();
    FFT_BOTTOM - p.clamp(0.0, 1.0) * (FFT_BOTTOM - FFT_TOP)
}

pub fn bass_y(midi: u8) -> f32 {
    let p = (midi as f32 - 26.0) / (45.0 - 26.0);
    BASS_LANE.top + 8.0 + (1.0 - p.clamp(0.0, 1.0)) * (BASS_LANE.h - 26.0)
}

/// Bleep height by frequency (the agent lane is a little piano roll of its own).
pub fn bleep_y(hz: f32) -> f32 {
    let p = ((hz / 700.0).ln() / (5000.0_f32 / 700.0).ln()).clamp(0.0, 1.0);
    DATA.top + 10.0 + (1.0 - p) * (DATA.h - 60.0)
}

pub fn arp_y(midi: u8) -> f32 {
    let p = (midi as f32 - 62.0) / (81.0 - 62.0);
    PAD.top + 6.0 + (1.0 - p.clamp(0.0, 1.0)) * (PAD.h - 18.0)
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

/// An event clip on the timeline: what the tracker can lock onto.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventKind {
    Kick,
    Clap,
    Bleep(f32),
    Token(usize),
    Impact,
}

#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub t: f32,
    pub kind: EventKind,
}

impl Event {
    /// The clip's rectangle at a time offset 0 (x is relative: add time_x).
    pub fn rect(&self) -> (f32, f32, f32, f32) {
        match self.kind {
            EventKind::Kick => (0.0, KICK.top + 6.0, 0.12 * PX_PER_S, KICK.h - 12.0),
            EventKind::Clap => (0.0, CLAP.top + 6.0, 0.09 * PX_PER_S, CLAP.h - 20.0),
            EventKind::Bleep(hz) => (0.0, bleep_y(hz), 0.05 * PX_PER_S, 12.0),
            EventKind::Token(_) => (-1.0, DATA.top + DATA.h - 36.0, 3.0, 26.0),
            EventKind::Impact => (
                0.0,
                WAVE_LANE.top + 10.0,
                0.5 * PX_PER_S,
                WAVE_LANE.h - 20.0,
            ),
        }
    }

    pub fn value(&self) -> String {
        match self.kind {
            EventKind::Kick => format!("kick {KICK_HZ:.1} Hz"),
            EventKind::Clap => "clap".to_string(),
            EventKind::Bleep(hz) => format!("bleep {hz:.1} Hz"),
            EventKind::Token(n) => format!("token #{:02}", n + 1),
            EventKind::Impact => format!("impact {IMPACT:.3} s"),
        }
    }
}

pub fn events() -> &'static Vec<Event> {
    static EV: std::sync::OnceLock<Vec<Event>> = std::sync::OnceLock::new();
    EV.get_or_init(|| {
        let mut v: Vec<Event> = KICKS
            .iter()
            .map(|&t| Event {
                t,
                kind: EventKind::Kick,
            })
            .collect();
        v.extend(CLAPS.iter().map(|&t| Event {
            t,
            kind: EventKind::Clap,
        }));
        v.extend(BLEEPS.iter().map(|&(t, hz)| Event {
            t,
            kind: EventKind::Bleep(hz),
        }));
        v.extend(TOKENS.iter().enumerate().map(|(i, &t)| Event {
            t,
            kind: EventKind::Token(i),
        }));
        v.push(Event {
            t: IMPACT,
            kind: EventKind::Impact,
        });
        v.sort_by(|a, b| a.t.total_cmp(&b.t));
        v
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrackBox {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub hatch: bool,
    pub locked: bool,
    /// A measured value the camera writes next to the box (e.g. "kick 55.7 Hz").
    pub value: Option<String>,
}

impl TrackBox {
    pub fn center(&self) -> (f32, f32) {
        (self.x + self.w * 0.5, self.y + self.h * 0.5)
    }
}

pub struct Track {
    pub boxes: Vec<TrackBox>,
    /// Index of the box the camera focuses on, if any.
    pub target: Option<usize>,
    pub tails: Vec<((f32, f32), (f32, f32))>,
    pub focus: (f32, f32),
    pub lock: bool,
}

/// How long a box stays on an event after the playhead passed it (s).
const TRAIL: f32 = 0.55;

fn jitter(k: usize, g: usize, amp: f32) -> (f32, f32) {
    let a = (k * 37 + 11) as u32;
    (
        (rnd(a, g as u32) - 0.5) * 2.0 * amp,
        (rnd(a + 1, g as u32 + 999) - 0.5) * 2.0 * amp,
    )
}

pub fn is_lock(global: usize) -> bool {
    events()
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::Kick | EventKind::Clap | EventKind::Impact
            )
        })
        .any(|e| {
            let f = frame_of(e.t);
            global >= f && global < f + LOCK_FRAMES
        })
}

pub fn track(global: usize) -> Track {
    let t = seconds(global);
    let cam = camera(t);
    let mut boxes = vec![];
    let mut kinds = vec![];
    if t < IMPACT {
        for (k, e) in events().iter().enumerate() {
            if e.t > t + 1e-4 || t - e.t > TRAIL {
                continue;
            }
            let (rx, ry, rw, rh) = e.rect();
            let x = time_x(e.t, global) + rx;
            let rw = rw * px_per_s(global) / PX_PER_S;
            let pad = match e.kind {
                EventKind::Token(_) => 6.0,
                _ => 5.0 + rnd(k as u32, 3) * 6.0,
            };
            let j = jitter(k, global, 1.4);
            let f = frame_of(e.t);
            let locked =
                global >= f && global < f + LOCK_FRAMES && !matches!(e.kind, EventKind::Token(_));
            boxes.push(TrackBox {
                x: x - pad + j.0,
                y: ry - pad + j.1,
                w: rw + 2.0 * pad,
                h: rh + 2.0 * pad,
                hatch: !matches!(e.kind, EventKind::Token(_)) && rnd(k as u32, 7) > 0.3,
                locked,
                value: match e.kind {
                    EventKind::Token(n) if n % 3 != 0 => None,
                    _ => Some(e.value()),
                },
            });
            kinds.push(e.kind);
        }
    } else {
        boxes = memory_boxes(global);
        kinds = vec![EventKind::Impact; boxes.len()];
    }

    // Focus: the newest box near where the camera looks, else the playhead at that height.
    let reach = 0.75 * 540.0 / cam.zoom;
    let target = (0..boxes.len()).rev().find(|&i| {
        let (cx, cy) = boxes[i].center();
        (cy - cam.look_y).abs() < reach
            && (cx - playhead_x(global)).abs() < reach * 1.6
            && !matches!(kinds[i], EventKind::Token(_))
    });
    let focus = target
        .map(|i| boxes[i].center())
        .unwrap_or((playhead_x(global), cam.look_y));

    let mut tails = vec![];
    if let Some(first) = boxes.first() {
        let c = first.center();
        let a = rnd(frame_of(t) as u32 / 15, 1) * std::f32::consts::TAU;
        tails.push((c, (c.0 + a.cos() * 2600.0, c.1 + a.sin() * 2600.0)));
    }
    Track {
        boxes,
        target,
        tails,
        focus,
        lock: is_lock(global),
    }
}

/// After the impact: one small box on every kick, clap and the impact, where they sit in the
/// track. As the view zooms out they become the track's own pattern of locks.
fn memory_boxes(global: usize) -> Vec<TrackBox> {
    let scale = px_per_s(global) / PX_PER_S;
    let f = frame_of(IMPACT);
    events()
        .iter()
        .filter(|e| matches!(e.kind, EventKind::Kick | EventKind::Clap | EventKind::Impact))
        .map(|e| {
            let (rx, ry, rw, rh) = e.rect();
            let pad = 3.0 + 3.0 * (1.0 - end_mix(global));
            TrackBox {
                x: time_x(e.t, global) + rx - pad,
                y: ry - pad,
                w: (rw * scale).max(3.0) + 2.0 * pad,
                h: rh + 2.0 * pad,
                hatch: e.kind == EventKind::Impact,
                locked: e.kind == EventKind::Impact && global >= f && global < f + LOCK_FRAMES,
                value: (e.kind == EventKind::Impact).then(|| e.value()),
            }
        })
        .collect()
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

// ---------- the camera: one move, keyframed ----------

/// (time s, look_y, zoom, tilt_x, tilt_y, roll, x offset from the playhead, blur)
const KEYS: [(f32, f32, f32, f32, f32, f32, f32, f32); 12] = [
    (0.0, 610.0, 3.4, 0.38, -0.48, 0.05, -40.0, 1.2),
    (2.0, 600.0, 2.7, 0.24, 0.34, -0.04, -80.0, 1.1),
    (4.0, 255.0, 2.5, 0.56, -0.22, 0.03, -30.0, 1.3),
    (6.0, 412.0, 2.3, 0.30, 0.46, -0.05, -60.0, 1.2),
    (8.0, 800.0, 2.6, 0.62, -0.30, 0.04, -40.0, 1.3),
    (10.0, 620.0, 2.0, 0.22, 0.55, -0.06, -90.0, 1.1),
    (11.0, 330.0, 2.4, 0.42, -0.40, 0.05, -40.0, 1.2),
    (12.0, 790.0, 2.8, 0.50, 0.30, -0.04, -30.0, 1.3),
    (13.5, 610.0, 3.9, 0.32, 0.10, 0.00, -20.0, 1.5),
    (14.0, 610.0, 3.5, 0.30, 0.00, 0.00, 0.0, 1.4),
    (15.2, 560.0, 1.12, 0.10, -0.06, 0.01, 0.0, 0.6),
    (16.0, 560.0, 1.06, 0.08, -0.05, 0.01, 0.0, 0.5),
];

#[derive(Clone, Copy, Debug)]
pub struct CamKey {
    pub look_y: f32,
    pub zoom: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub roll: f32,
    pub dx: f32,
    pub blur: f32,
}

fn catmull(p0: f32, p1: f32, p2: f32, p3: f32, u: f32) -> f32 {
    0.5 * ((2.0 * p1)
        + (-p0 + p2) * u
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * u * u
        + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * u * u * u)
}

pub fn camera(t: f32) -> CamKey {
    let n = KEYS.len();
    let i = (0..n - 1).rfind(|&i| KEYS[i].0 <= t).unwrap_or(0);
    let k = |j: isize| KEYS[(j.clamp(0, n as isize - 1)) as usize];
    let (a, b, c, d) = (
        k(i as isize - 1),
        k(i as isize),
        k(i as isize + 1),
        k(i as isize + 2),
    );
    let u = ((t - b.0) / (c.0 - b.0).max(1e-3)).clamp(0.0, 1.0);
    let s = |f: fn(&(f32, f32, f32, f32, f32, f32, f32, f32)) -> f32| {
        catmull(f(&a), f(&b), f(&c), f(&d), u)
    };
    CamKey {
        look_y: s(|x| x.1),
        zoom: s(|x| x.2).max(1.0),
        tilt_x: s(|x| x.3),
        tilt_y: s(|x| x.4),
        roll: s(|x| x.5),
        dx: s(|x| x.6),
        blur: s(|x| x.7),
    }
}

// ---------- palettes, blended over time ----------

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

/// From this time on, this palette (blended in over `BLEND` s).
pub const PALETTES: [(f32, Palette); 9] = [
    (0.0, Palette::Mono),
    (2.0, Palette::Navy),
    (4.0, Palette::Thermal),
    (8.0, Palette::Phosphor),
    (10.0, Palette::Amber),
    (12.0, Palette::Thermal),
    (13.0, Palette::Navy),
    (13.5, Palette::Mono),
    (14.0, Palette::Neon),
];
const BLEND: f32 = 0.5;

pub fn palette_stops(t: f32) -> [[f32; 3]; 5] {
    let i = (0..PALETTES.len())
        .rfind(|&i| PALETTES[i].0 <= t)
        .unwrap_or(0);
    let cur = PALETTES[i].1.stops();
    if i == 0 {
        return cur;
    }
    let prev = PALETTES[i - 1].1.stops();
    let u = ((t - PALETTES[i].0) / BLEND).clamp(0.0, 1.0);
    let u = u * u * (3.0 - 2.0 * u);
    let mut out = cur;
    for s in 0..5 {
        for c in 0..3 {
            out[s][c] = prev[s][c] + (cur[s][c] - prev[s][c]) * u;
        }
    }
    out
}

// ---------- envelopes ----------

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
    pub stops: [[f32; 3]; 5],
    pub lock: bool,
    /// Exposure flash: the first frames and the impact.
    pub cut: f32,
    pub punch: f32,
    pub tear: f32,
    pub out: f32,
}

pub fn shot(global: usize) -> Shot {
    let t = seconds(global);
    let cam = camera(t);
    let base = (PLAYHEAD_X + cam.dx, cam.look_y);
    // The aim leans towards the tracked box, averaged over four frames.
    let mut sum = (0.0, 0.0);
    let mut n = 0.0;
    for k in 0..4usize {
        if global >= k {
            let q = track(global - k).focus;
            sum.0 += q.0;
            sum.1 += q.1;
            n += 1.0;
        }
    }
    let lag = (sum.0 / n, sum.1 / n);
    let punch = kick_env(t);
    let shake = (punch * 3.0, punch * 2.0);
    let aim = (
        base.0 + (lag.0 - base.0) * 0.35 + (rnd(5, global as u32) - 0.5) * shake.0,
        base.1 + (lag.1 - base.1) * 0.35 + (rnd(6, global as u32) - 0.5) * shake.1,
    );
    let impact_flash = env(&[IMPACT], t, 0.18);
    Shot {
        aim,
        focus: track(global).focus,
        zoom: cam.zoom * (1.0 + 0.05 * punch),
        tilt_x: cam.tilt_x,
        tilt_y: cam.tilt_y,
        roll: cam.roll,
        blur: cam.blur,
        stops: palette_stops(t),
        lock: is_lock(global),
        cut: match global {
            0 => 1.0_f32,
            1 => 0.45,
            2 => 0.15,
            _ => 0.0,
        }
        .max(impact_flash),
        punch,
        tear: clap_env(t).max(if stutter_on(t) { 0.8 } else { 0.0 }),
        out: ((t - 15.5) / 0.45).clamp(0.0, 1.0),
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
    fn every_kick_locks_on_its_frame() {
        for &k in &KICKS {
            let g = frame_of(k);
            assert!(is_lock(g), "kick at {k} s does not lock");
            let tr = track(g);
            assert!(tr.boxes.iter().any(|b| b.locked), "no locked box at {k} s");
        }
    }

    #[test]
    fn a_locked_box_sits_on_the_playhead() {
        for &k in &KICKS {
            let tr = track(frame_of(k));
            let b = tr.boxes.iter().find(|b| b.locked).unwrap();
            assert!(
                (b.x - PLAYHEAD_X).abs() < 20.0,
                "kick box at x {} for {k} s",
                b.x
            );
        }
    }

    #[test]
    fn the_camera_keeps_the_focus_in_frame() {
        for g in 0..TOTAL_FRAMES {
            let s = shot(g);
            let (x, y) = project(&s, s.focus, 1920.0, 1080.0).expect("in front of the camera");
            assert!(
                x > 120.0 && x < 1800.0 && y > 90.0 && y < 990.0,
                "frame {g}: focus at {x:.0},{y:.0}"
            );
        }
    }

    #[test]
    fn the_waveform_covers_the_track() {
        assert!(WAVE.len() as f32 / WAVE_COLS_PER_S >= 15.9);
        assert!(MARKERS.iter().all(|(t, _)| *t < 16.0));
    }
}
