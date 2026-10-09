//! Pass 1: the agent's screen while it listens, flat and colorless, with the tracker on top.
//!
//! White on black only. The lens pass maps brightness to each shot's palette, so the one
//! color here is pure red (#ff0000): it marks a locked box and survives the palette.
//! Drawn in `hud`'s 1920x1080 logical space at 2x, so the lens can magnify it.
use fframes::{
    AudioMap, Color, Duration, FFramesContext, Frame, SampleSize, Svgr, Video, VisualizeFrameInput,
    WindowFunction, include_media_dir,
};
use hud::{Kind, LOG_SIZE, TrackBox};
use std::sync::OnceLock;

include_media_dir!(pub struct FlatMedia, "media");

pub const WIDTH: usize = 3840;
pub const HEIGHT: usize = 2160;
const SCALE: f32 = 2.0;

const MONO: &str = "JetBrains Mono";
const WHITE: &str = "#ffffff";
const LOG_GRAY: &str = "#5c5c5c";
const UI_GRAY: &str = "#3a3a3a";
const RED: &str = "#ff0000";

/// fframes hands the FFT 1024 contiguous samples from the start of each frame.
const FFT_SIZE: usize = 1024;
const SAMPLE_RATE: f32 = 48_000.0;
/// Bar height mapping of the FFT magnitude, dB.
const DB_FLOOR: f32 = -6.0;
const DB_CEIL: f32 = 48.0;

#[derive(Default)]
pub struct FlatVideo;

impl FlatVideo {
    pub fn new() -> Self {
        Self
    }
}

impl Video for FlatVideo {
    const FPS: usize = hud::FPS;
    const WIDTH: usize = WIDTH;
    const HEIGHT: usize = HEIGHT;
    const BACKGROUND_COLOR: Color = Color::BLACK;

    fn duration(&self) -> Duration<'_> {
        Duration::Frames(hud::TOTAL_FRAMES)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let g = frame.index;
        let (si, f) = hud::split(g);
        let content = match hud::look_of(si).kind {
            Kind::Listen | Kind::Typing => hero(si, f, true),
            Kind::Hold => hero(si, f, (f / 4) % 2 == 0),
            Kind::Done => done(f),
        };
        let bands = spectrum(&frame, ctx);
        fframes::svgr!(
            <svg xmlns="http://www.w3.org/2000/svg" width={WIDTH} height={HEIGHT}>
                <defs>
                    <pattern id="hatch" width="7" height="7" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
                        <rect x="0" y="0" width="2.6" height="7" fill="#ffffff" />
                    </pattern>
                    <pattern id="hatch-red" width="7" height="7" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
                        <rect x="0" y="0" width="3.2" height="7" fill="#ff0000" />
                    </pattern>
                </defs>
                <rect width={WIDTH} height={HEIGHT} fill="#000000" />
                <g transform={format!("scale({SCALE})")}>
                    {chrome(g)}
                    {log(g)}
                    {spectrum_panel(&bands)}
                    {content}
                    {tracker(g)}
                </g>
            </svg>
        )
    }
}

/// FFT bin ranges of the log-spaced bands (computed once).
fn band_bins() -> &'static Vec<(usize, usize)> {
    static BINS: OnceLock<Vec<(usize, usize)>> = OnceLock::new();
    BINS.get_or_init(|| {
        let bin_hz = SAMPLE_RATE / FFT_SIZE as f32;
        (0..hud::SPEC_BANDS)
            .map(|b| {
                let edge = |i: usize| {
                    hud::SPEC_LO
                        * (hud::SPEC_HI / hud::SPEC_LO).powf(i as f32 / hud::SPEC_BANDS as f32)
                };
                let lo = (edge(b) / bin_hz).floor() as usize;
                let hi = ((edge(b + 1) / bin_hz).ceil() as usize).max(lo + 1);
                (lo.max(1), hi.min(FFT_SIZE / 2))
            })
            .collect()
    })
}

/// Band levels 0..1 from fframes' FFT of music.wav at this frame.
fn spectrum(frame: &Frame, ctx: &FFramesContext) -> Vec<f32> {
    let Some(audio) = ctx.get_audio("music.wav") else {
        return vec![0.0; hud::SPEC_BANDS];
    };
    let mags = frame.visualize_audio_frame(VisualizeFrameInput {
        audio,
        sample_size: SampleSize::S1024,
        smooth_level: 1,
        window: Some(WindowFunction::Hann),
    });
    band_bins()
        .iter()
        .map(|&(lo, hi)| {
            let m = mags
                .get(lo..hi.min(mags.len()))
                .map(|s| s.iter().cloned().fold(0.0, f32::max))
                .unwrap_or(0.0);
            let db = 20.0 * (m + 1e-6).log10();
            ((db - DB_FLOOR) / (DB_CEIL - DB_FLOOR)).clamp(0.0, 1.0)
        })
        .collect()
}

fn spectrum_panel<'a>(bands: &[f32]) -> Svgr<'a> {
    let n = bands.len().max(1);
    let step = (hud::SPEC_X1 - hud::SPEC_X0) / n as f32;
    let full = hud::SPEC_BASE - hud::SPEC_TOP - 20.0;
    let bars: Vec<Svgr> = bands
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let h = 1.0 + v * full;
            let x = hud::SPEC_X0 + i as f32 * step + 2.0;
            let fill = if v > 0.75 { "#ffffff" } else { "#bdbdbd" };
            fframes::svgr!(<rect x={x} y={hud::SPEC_BASE - h} width={step - 4.0} height={h} fill={fill} />)
        })
        .collect();
    let ticks: Vec<Svgr> = [50.0_f32, 100.0, 500.0, 1000.0, 5000.0, 10000.0]
        .iter()
        .map(|&hz| {
            let x = hud::hz_x(hz);
            let label = if hz >= 1000.0 {
                format!("{}k", hz / 1000.0)
            } else {
                format!("{hz}")
            };
            fframes::svgr!(<g>
                <rect x={x} y={hud::SPEC_BASE + 2.0} width="1.5" height="8" fill={LOG_GRAY} />
                <text x={x + 4.0} y={hud::SPEC_BASE + 24.0} fill={LOG_GRAY}>{label}</text>
            </g>)
        })
        .collect();
    fframes::svgr!(
        <g font-family={MONO} font-weight="400" font-size="16">
            <rect x={hud::SPEC_X0} y={hud::SPEC_BASE} width={hud::SPEC_X1 - hud::SPEC_X0} height="1.5" fill={UI_GRAY} />
            <text x={hud::SPEC_X0} y={hud::SPEC_TOP - 8.0} fill={LOG_GRAY}>"fft 1024 · hann · 40 Hz – 16 kHz"</text>
            {bars}
            {ticks}
        </g>
    )
}

/// Title bar, status line and a level meter fed by the measured RMS of each frame.
fn chrome<'a>(g: usize) -> Svgr<'a> {
    let t = hud::seconds(g);
    let beat = (t / (60.0 / hud::BPM)) as usize;
    let rms = hud::RMS.get(g).copied().unwrap_or(-90.0);
    let status = format!(
        "{:.0} BPM  ·  bar {}/8  ·  beat {}  ·  {:>6.1} dBFS  ·  frame {:03}/{}",
        hud::BPM,
        (beat / 4 + 1).min(8),
        beat % 4 + 1,
        rms,
        g + 1,
        hud::TOTAL_FRAMES
    );
    let level = ((rms + 48.0) / 48.0).clamp(0.0, 1.0) * 520.0;
    fframes::svgr!(
        <g font-family={MONO} font-weight="400" font-size="18" fill={LOG_GRAY}>
            <circle cx="104" cy="30" r="7" fill={UI_GRAY} />
            <circle cx="128" cy="30" r="7" fill={UI_GRAY} />
            <circle cx="152" cy="30" r="7" fill={UI_GRAY} />
            <text x="190" y="36">"agent — listening  ·  ~/ff-unmute/out/pass/music.wav"</text>
            <rect x="100" y="48" width="1720" height="1" fill={UI_GRAY} />
            <rect x="1250" y="22" width="520" height="14" fill="none" stroke={UI_GRAY} stroke-width="1.5" />
            <rect x="1250" y="22" width={level.max(0.5)} height="14" fill="#9a9a9a" />
            <rect x="100" y="1036" width="1720" height="1" fill={UI_GRAY} />
            <text x="100" y="1064">{status}</text>
        </g>
    )
}

fn log<'a>(g: usize) -> Svgr<'a> {
    let rows = hud::log_rows(g);
    let last = rows.len().saturating_sub(1);
    let rows: Vec<Svgr> = rows
        .into_iter()
        .enumerate()
        .map(|(i, (text, x, y))| {
            let fill = if i == last {
                "#a8a8a8"
            } else if text.starts_with('›') {
                "#7a7a7a"
            } else {
                LOG_GRAY
            };
            fframes::svgr!(<text x={x} y={y} fill={fill}>{text}</text>)
        })
        .collect();
    fframes::svgr!(<g font-family={MONO} font-weight="400" font-size={LOG_SIZE}>{rows}</g>)
}

/// The hero line, one glyph per cell so the tracker's coordinates are exact.
fn hero<'a>(si: usize, f: usize, cursor_on: bool) -> Svgr<'a> {
    let l = hud::look_of(si);
    let n = hud::visible(si, f);
    let typing = f as f32 <= hud::shot_len(si) as f32 * 0.5;
    let glyphs: Vec<Svgr> = hud::text(si)
        .chars()
        .take(n)
        .enumerate()
        .map(|(i, c)| {
            let (x, _, _, _) = hud::cell(si, i);
            // The newest glyph arrives overexposed and settles.
            let fill = if typing && n - 1 - i < 2 {
                "#ffffff"
            } else {
                "#e9e9e9"
            };
            fframes::svgr!(<text x={x} y={l.hero.1} fill={fill}>{c.to_string()}</text>)
        })
        .collect();
    let (cx, cy, cw, ch) = hud::cell(si, n);
    let cursor = if cursor_on {
        fframes::svgr!(<rect x={cx + cw * 0.12} y={cy} width={cw * 0.62} height={ch} fill={WHITE} />)
    } else {
        Svgr::empty()
    };
    fframes::svgr!(
        <g font-family={MONO} font-weight="700" font-size={l.size}>
            {glyphs}
            {cursor}
        </g>
    )
}

fn done<'a>(f: usize) -> Svgr<'a> {
    let (cx, cy, w, h) = hud::done_pill();
    let (ix, iy) = hud::done_icon();
    let (lx, ly) = hud::done_label();
    let grow = hud::ease_out(((f as f32 - 8.0) / 6.0).clamp(0.0, 1.0));
    if f < 8 {
        return Svgr::empty();
    }
    let pw = 80.0 + (w - 80.0) * grow;
    let ph = 80.0 + (h - 80.0) * grow;
    let label_opacity = ((f as f32 - 11.0) / 4.0).clamp(0.0, 1.0);
    fframes::svgr!(
        <g>
            <rect x={cx - pw * 0.5} y={cy - ph * 0.5} width={pw} height={ph} rx={ph * 0.5}
                fill="#1d1d1d" stroke={WHITE} stroke-width="4" />
            <rect x={cx - pw * 0.5 - 14.0} y={cy - ph * 0.5 - 14.0} width={pw + 28.0} height={ph + 28.0} rx={ph * 0.5 + 14.0}
                fill="none" stroke="#6a6a6a" stroke-width="1.5" />
            <g opacity={label_opacity}>
                <circle cx={ix} cy={iy} r="30" fill="#f2f2f2" />
                <path d={format!("M {} {} l 10 11 l 20 -24", ix - 14.0, iy + 1.0)} fill="none" stroke="#111111"
                    stroke-width="7" stroke-linecap="round" stroke-linejoin="round" />
                <text x={lx} y={ly} font-family={MONO} font-weight="700" font-size="80" fill={WHITE}>"unmuted"</text>
            </g>
        </g>
    )
}

/// The tracker's boxes, on the screen itself so they take the LED grid and the blur.
/// Labels, links and the crosshair belong to the camera and are drawn by the lens pass.
fn tracker<'a>(g: usize) -> Svgr<'a> {
    let boxes: Vec<Svgr> = hud::track(g).boxes.iter().map(tracker_box).collect();
    fframes::svgr!(<g>{boxes}</g>)
}

fn tracker_box<'a>(b: &TrackBox) -> Svgr<'a> {
    let stroke = if b.locked { RED } else { WHITE };
    // Hatch in difference mode: stripes turn dark where they cross a glyph.
    let fill = if !b.hatch {
        Svgr::empty()
    } else if b.locked {
        fframes::svgr!(<rect x={b.x} y={b.y} width={b.w} height={b.h} fill="url(#hatch-red)" />)
    } else {
        fframes::svgr!(<g style="mix-blend-mode:difference">
            <rect x={b.x} y={b.y} width={b.w} height={b.h} fill="url(#hatch)" />
        </g>)
    };
    fframes::svgr!(
        <g>
            {fill}
            <rect x={b.x} y={b.y} width={b.w} height={b.h} fill="none" stroke={stroke} stroke-width="2.2" />
        </g>
    )
}
