//! Pass 1: a DAW timeline that follows its playhead, flat and colorless, with the tracker.
//!
//! White on black only. The lens pass maps brightness to the current palette, so the one
//! color here is pure red (#ff0000): it marks a locked box and survives the palette.
//! Drawn in `hud`'s 1920x1080 logical space at 2x, so the lens can magnify it.
use fframes::{
    AudioMap, Color, Duration, FFramesContext, Frame, SampleSize, Svgr, Video, VisualizeFrameInput,
    WindowFunction, include_media_dir,
};
use hud::{Lane, TrackBox};
use std::sync::OnceLock;

include_media_dir!(pub struct FlatMedia, "media");

pub const WIDTH: usize = 3840;
pub const HEIGHT: usize = 2160;
const SCALE: f32 = 2.0;

const MONO: &str = "JetBrains Mono";
const WHITE: &str = "#ffffff";
const LIT: &str = "#e6e6e6";
const UNLIT: &str = "#666666";
const LOG_GRAY: &str = "#5c5c5c";
const UI_GRAY: &str = "#333333";
const RED: &str = "#ff0000";

/// fframes hands the FFT 1024 contiguous samples from the start of each frame.
const FFT_SIZE: usize = 1024;
const SAMPLE_RATE: f32 = 48_000.0;
const DB_FLOOR: f32 = -6.0;
const DB_CEIL: f32 = 48.0;

/// Visible range of the timeline in logical x.
const VIEW_X0: f32 = hud::LANE_LABEL_W;
const VIEW_X1: f32 = hud::W + 40.0;

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
        let t = hud::seconds(g);
        let bands = spectrum(&frame, ctx);
        // After the impact the timeline steps back so the pill can take over.
        let _ = t;
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
                    <g>
                        {ruler(g)}
                        {markers(g)}
                        {lanes(g)}
                        {waveform(g)}
                        {fft_strip(&bands, g)}
                        {lane_labels()}
                        {playhead(g)}
                    </g>
                    {chrome(g)}
                    {tracker(g)}
                </g>
            </svg>
        )
    }
}

fn visible(x: f32, w: f32) -> bool {
    x + w > VIEW_X0 - 10.0 && x < VIEW_X1
}

fn ruler<'a>(g: usize) -> Svgr<'a> {
    let beat = 60.0 / hud::BPM;
    let mut items = vec![];
    for b in 0..=32usize {
        let tb = b as f32 * beat;
        let x = hud::time_x(tb, g);
        if !visible(x, 1.0) {
            continue;
        }
        let bar = b % 4 == 0;
        let h = if bar { 16.0 } else { 8.0 };
        items.push(fframes::svgr!(<rect x={x} y={hud::RULER_Y - h} width="1.5" height={h} fill={if bar { "#8a8a8a" } else { LOG_GRAY }} />));
        if bar {
            let label = format!("{}   {:.1} s", b / 4 + 1, tb);
            items.push(fframes::svgr!(<text x={x + 6.0} y={hud::RULER_Y - 4.0} fill={LOG_GRAY}>{label}</text>));
        }
    }
    fframes::svgr!(
        <g font-family={MONO} font-weight="400" font-size="15">
            <rect x={VIEW_X0} y={hud::RULER_Y} width={hud::W} height="1.5" fill={UI_GRAY} />
            {items}
        </g>
    )
}

/// What the agent writes on the timeline: each marker types itself as the playhead reaches it.
fn markers<'a>(g: usize) -> Svgr<'a> {
    let now = hud::seconds(g);
    let items: Vec<Svgr> = hud::MARKERS
        .iter()
        .filter(|(t, _)| *t <= now)
        .filter_map(|(t, text)| {
            let x = hud::time_x(*t, g);
            let n = text.chars().count();
            let shown = ((now - t) / 0.25 * n as f32).ceil().min(n as f32) as usize;
            let s: String = text.chars().take(shown.max(1)).collect();
            // At the end the view holds the whole track: markers shrink, and only the one
            // picked from the data (its loudest moment) keeps full brightness.
            let e = hud::end_mix(g);
            let key = text.starts_with("loudest");
            let size = 26.0 * (1.0 - 0.45 * e);
            let w = n as f32 * size * hud::MONO_ADV + 16.0;
            let opacity = if key { 1.0 } else { 1.0 - 0.6 * e };
            visible(x, w).then(|| {
                fframes::svgr!(<g opacity={opacity}>
                    <rect x={x} y="58" width="2" height={hud::RULER_Y - 58.0} fill={LIT} />
                    <rect x={x} y="58" width={w} height={8.0 + size} fill="#1a1a1a" />
                    <text x={x + 8.0} y={62.0 + size * 0.85} font-size={size} fill={LIT}>{s}</text>
                </g>)
            })
        })
        .collect();
    fframes::svgr!(<g font-family={MONO} font-weight="700" font-size="26">{items}</g>)
}

fn clip<'a>(x: f32, y: f32, w: f32, h: f32, lit: bool) -> Svgr<'a> {
    let fill = if lit { LIT } else { UNLIT };
    fframes::svgr!(<rect x={x} y={y} width={w.max(1.0)} height={h.max(1.0)} rx="2" fill={fill} />)
}

fn lanes<'a>(g: usize) -> Svgr<'a> {
    let pxs = hud::px_per_s(g);
    let now = hud::seconds(g);
    let mut items: Vec<Svgr> = vec![];
    // lane separators
    for l in hud::LANES.iter() {
        items.push(fframes::svgr!(<rect x={VIEW_X0} y={l.top + l.h + 4.0} width={hud::W} height="1" fill={UI_GRAY} />));
    }
    // pad: a block per bar with the chord name, the arp notes inside
    for (t, chord) in hud::CHORDS.iter() {
        let x = hud::time_x(*t, g);
        let w = 2.0 * pxs - 6.0;
        if visible(x, w) {
            let lit = *t <= now;
            items.push(fframes::svgr!(<g>
                <rect x={x} y={hud::PAD.top} width={w} height={hud::PAD.h} rx="4" fill="#161616" stroke={if lit { "#7a7a7a" } else { "#3a3a3a" }} stroke-width="1.5" />
                <text x={x + 10.0} y={hud::PAD.top + 24.0} font-family={MONO} font-weight="700" font-size="20" fill={if lit { LIT } else { UNLIT }}>{*chord}</text>
            </g>));
        }
    }
    for (t, m) in hud::ARP.iter() {
        let x = hud::time_x(*t, g);
        if visible(x, 20.0) {
            items.push(clip(x, hud::arp_y(*m), 0.1 * pxs, 6.0, *t <= now));
        }
    }
    // kick, clap, hats
    for t in hud::KICKS.iter().chain([hud::IMPACT].iter()) {
        let x = hud::time_x(*t, g);
        if visible(x, 40.0) {
            items.push(clip(
                x,
                hud::KICK.top + 6.0,
                0.12 * pxs,
                hud::KICK.h - 12.0,
                *t <= now,
            ));
        }
    }
    for t in hud::CLAPS.iter() {
        let x = hud::time_x(*t, g);
        if visible(x, 30.0) {
            items.push(clip(
                x,
                hud::CLAP.top + 6.0,
                0.09 * pxs,
                hud::CLAP.h - 20.0,
                *t <= now,
            ));
        }
    }
    for t in hud::HATS.iter() {
        let x = hud::time_x(*t, g);
        if visible(x, 4.0) {
            items.push(clip(
                x,
                hud::CLAP.top + hud::CLAP.h - 10.0,
                3.0,
                8.0,
                *t <= now,
            ));
        }
    }
    // bass piano roll
    for (t, m) in hud::BASS.iter() {
        let x = hud::time_x(*t, g);
        if visible(x, 70.0) {
            items.push(clip(
                x,
                hud::bass_y(*m),
                0.125 * 1.6 * pxs - 4.0,
                14.0,
                *t <= now,
            ));
        }
    }
    // the agent: bleeps by pitch, tokens as ticks
    for (t, hz) in hud::BLEEPS.iter() {
        let x = hud::time_x(*t, g);
        if visible(x, 20.0) {
            items.push(clip(
                x,
                hud::bleep_y(*hz),
                0.05 * pxs,
                12.0,
                *t <= now,
            ));
        }
    }
    for t in hud::TOKENS.iter() {
        let x = hud::time_x(*t, g);
        if visible(x, 4.0) {
            items.push(clip(
                x - 1.0,
                hud::DATA.top + hud::DATA.h - 36.0,
                3.0,
                26.0,
                *t <= now,
            ));
        }
    }
    fframes::svgr!(<g>{items}</g>)
}

/// Master waveform from music.wav's peaks, bright where the playhead has passed.
fn waveform<'a>(g: usize) -> Svgr<'a> {
    let pxs = hud::px_per_s(g);
    let now = hud::seconds(g);
    let mid = hud::WAVE_LANE.mid();
    let half = hud::WAVE_LANE.h * 0.5 - 8.0;
    // When the view zooms out, columns get thinner than a pixel: merge them in groups and
    // draw each group's peak, so the waveform stays solid at every zoom.
    let col_w = pxs / hud::WAVE_COLS_PER_S;
    let stride = (1.6 / col_w).ceil().max(1.0) as usize;
    let w = col_w * stride as f32;
    let cols: Vec<Svgr> = hud::WAVE
        .chunks(stride)
        .enumerate()
        .filter_map(|(k, chunk)| {
            let i = k * stride;
            let p = chunk.iter().cloned().fold(0.0, f32::max);
            let t = i as f32 / hud::WAVE_COLS_PER_S;
            let x = hud::time_x(t, g);
            visible(x, w).then(|| {
                let h = 1.0 + p * half;
                let fill = if t <= now { LIT } else { UNLIT };
                fframes::svgr!(<rect x={x} y={mid - h} width={(w - 0.6).max(0.8)} height={2.0 * h} fill={fill} />)
            })
        })
        .collect();
    fframes::svgr!(<g>
        <rect x={VIEW_X0} y={mid} width={hud::W} height="1" fill={UI_GRAY} />
        {cols}
    </g>)
}

/// FFT bin ranges of the log-spaced bands (computed once).
fn band_bins() -> &'static Vec<(usize, usize)> {
    static BINS: OnceLock<Vec<(usize, usize)>> = OnceLock::new();
    BINS.get_or_init(|| {
        let bin_hz = SAMPLE_RATE / FFT_SIZE as f32;
        (0..hud::FFT_BANDS)
            .map(|b| {
                let edge = |i: usize| {
                    hud::FFT_LO * (hud::FFT_HI / hud::FFT_LO).powf(i as f32 / hud::FFT_BANDS as f32)
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
        return vec![0.0; hud::FFT_BANDS];
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
            ((20.0 * (m + 1e-6).log10() - DB_FLOOR) / (DB_CEIL - DB_FLOOR)).clamp(0.0, 1.0)
        })
        .collect()
}

fn fft_strip<'a>(bands: &[f32], g: usize) -> Svgr<'a> {
    // It belongs to the live playhead: it fades out as the view zooms out to the whole track.
    let x0 = hud::playhead_x(g) + 8.0;
    let fade = 1.0 - hud::end_mix(g);
    let n = bands.len().max(1);
    let step = (hud::FFT_BOTTOM - hud::FFT_TOP) / n as f32;
    let bars: Vec<Svgr> = bands
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let y = hud::FFT_BOTTOM - (i + 1) as f32 * step + 1.5;
            fframes::svgr!(<rect x={x0} y={y} width={0.5 + v * hud::FFT_W} height={step - 3.0} fill="#9a9a9a" />)
        })
        .collect();
    fframes::svgr!(<g opacity={fade}>{bars}</g>)
}

fn lane_labels<'a>() -> Svgr<'a> {
    let rows: Vec<Svgr> = hud::LANES
        .iter()
        .map(|l: &Lane| fframes::svgr!(<text x="24" y={l.mid() + 6.0} fill={LOG_GRAY}>{l.name}</text>))
        .collect();
    fframes::svgr!(
        <g font-family={MONO} font-weight="400" font-size="18">
            <rect x="0" y="50" width={hud::LANE_LABEL_W - 8.0} height="900" fill="#000000" />
            <rect x={hud::LANE_LABEL_W - 8.0} y="50" width="1.5" height="900" fill={UI_GRAY} />
            {rows}
        </g>
    )
}

fn playhead<'a>(g: usize) -> Svgr<'a> {
    let x = hud::playhead_x(g);
    fframes::svgr!(<g>
        <path d={format!("M {} {} l 9 -12 l -18 0 z", x, hud::RULER_Y)} fill={WHITE} />
        <rect x={x - 1.2} y={hud::RULER_Y} width="2.4" height={hud::DATA.top + hud::DATA.h - hud::RULER_Y} fill={WHITE} />
    </g>)
}

/// Title bar, status line and a level meter fed by the measured RMS of each frame.
fn chrome<'a>(g: usize) -> Svgr<'a> {
    let t = hud::seconds(g);
    let beat = (t / (60.0 / hud::BPM)) as usize;
    let rms = hud::RMS.get(g).copied().unwrap_or(-90.0);
    let status = format!(
        "{:.0} BPM  ·  bar {}/8  ·  beat {}  ·  {:06.3} s  ·  {:>6.1} dBFS  ·  frame {:03}/{}",
        hud::BPM,
        (beat / 4 + 1).min(8),
        beat % 4 + 1,
        t,
        rms,
        g + 1,
        hud::TOTAL_FRAMES
    );
    let level = ((rms + 48.0) / 48.0).clamp(0.0, 1.0) * 520.0;
    fframes::svgr!(
        <g font-family={MONO} font-weight="400" font-size="18" fill={LOG_GRAY}>
            <circle cx="24" cy="26" r="6" fill={UI_GRAY} />
            <circle cx="44" cy="26" r="6" fill={UI_GRAY} />
            <circle cx="64" cy="26" r="6" fill={UI_GRAY} />
            <text x="96" y="32">"agent — listening  ·  ~/ff-unmute/out/pass/music.wav  ·  follow playhead"</text>
            <rect x="1250" y="18" width="520" height="14" fill="none" stroke={UI_GRAY} stroke-width="1.5" />
            <rect x="1250" y="18" width={level.max(0.5)} height="14" fill="#9a9a9a" />
            <rect x="0" y="1012" width={hud::W} height="1" fill={UI_GRAY} />
            <text x="24" y="1046">{status}</text>
        </g>
    )
}

/// The tracker's boxes, on the screen itself so they take the LED grid and the blur.
fn tracker<'a>(g: usize) -> Svgr<'a> {
    let boxes: Vec<Svgr> = hud::track(g).boxes.iter().map(tracker_box).collect();
    fframes::svgr!(<g>{boxes}</g>)
}

fn tracker_box<'a>(b: &TrackBox) -> Svgr<'a> {
    let stroke = if b.locked { RED } else { WHITE };
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
