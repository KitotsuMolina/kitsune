use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use kitsune::color_resolver::{ColorMode, Palette, PaletteChannel, RgbaColor, load_palette};
use std::collections::HashMap;
use std::env;
use std::f64::consts::PI;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpectrumMode {
    Single,
    Group,
}

impl SpectrumMode {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "group" => Self::Group,
            _ => Self::Single,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisualMode {
    Bars,
    Ring,
}

impl VisualMode {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ring" => Self::Ring,
            _ => Self::Bars,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderStyle {
    Bars,
    BarsFill,
    Waves,
    WavesKwy,
    WavesOcean,
    WavesOceanFill,
    WavesFill,
    Dots,
    Triangle,
    Polygon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlendMode {
    Normal,
    Add,
    Screen,
    Multiply,
}

impl BlendMode {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "add" | "plus" | "additive" => Self::Add,
            "screen" => Self::Screen,
            "multiply" | "mul" => Self::Multiply,
            _ => Self::Normal,
        }
    }

    fn cairo_operator(self) -> gtk::cairo::Operator {
        match self {
            Self::Normal => gtk::cairo::Operator::Over,
            Self::Add => gtk::cairo::Operator::Add,
            Self::Screen => gtk::cairo::Operator::Screen,
            Self::Multiply => gtk::cairo::Operator::Multiply,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderQuality {
    Performance,
    Balanced,
    High,
}

impl RenderQuality {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "performance" | "perf" | "low" => Self::Performance,
            "high" | "ultra" => Self::High,
            _ => Self::Balanced,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpectrumZone {
    Full,
    Bass,
    Mid,
    Treble,
    BassMid,
    MidTreble,
}

impl SpectrumZone {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bass" | "low" | "graves" => Self::Bass,
            "mid" | "mids" | "medios" => Self::Mid,
            "treble" | "high" | "agudos" => Self::Treble,
            "bass_mid" | "bass-mid" | "low_mid" | "low-mid" => Self::BassMid,
            "mid_treble" | "mid-treble" | "high_mid" | "high-mid" => Self::MidTreble,
            _ => Self::Full,
        }
    }
}

impl RenderStyle {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bars_fill" | "bars-fill" => Self::BarsFill,
            "waves" | "wave" => Self::Waves,
            "waves_kwy" | "waves-kwy" | "kwy_waves" | "kwy-waves" | "ribbon" => Self::WavesKwy,
            "waves_ocean" | "waves-ocean" | "ocean" | "ocean_waves" | "ocean-waves" => Self::WavesOcean,
            "waves_ocean_fill" | "waves-ocean-fill" | "ocean_fill" | "ocean-fill" => Self::WavesOceanFill,
            "waves_fill" | "waves-fill" | "wavefill" => Self::WavesFill,
            "dots" | "dot" => Self::Dots,
            "triangle" => Self::Triangle,
            "polygon" => Self::Polygon,
            _ => Self::Bars,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayLayout {
    Line,
    Radial,
    Polygon,
}

impl OverlayLayout {
    fn from_mode_style(mode: &str, style: &str) -> Self {
        let style = style.trim().to_ascii_lowercase();
        if style == "triangle" || style == "polygon" {
            return OverlayLayout::Polygon;
        }
        match mode.trim().to_ascii_lowercase().as_str() {
            "ring" => OverlayLayout::Radial,
            _ => OverlayLayout::Line,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarsAnchor {
    Bottom,
    Top,
    Left,
    Right,
}

impl BarsAnchor {
    fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "top" => Self::Top,
            "left" => Self::Left,
            "right" => Self::Right,
            _ => Self::Bottom,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BarsDirection {
    Up,
    Down,
    Left,
    Right,
}

impl BarsDirection {
    fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "down" => Self::Down,
            "left" => Self::Left,
            "right" => Self::Right,
            _ => Self::Up,
        }
    }
}

#[derive(Debug, Clone)]
struct Config {
    monitor: String,
    width: i32,
    height: i32,
    bars: usize,
    fps: u32,
    color: RgbaColor,
    color2: RgbaColor,
    color_mode: ColorMode,
    color2_mode: ColorMode,
    color_palette_file: PathBuf,
    dynamic_contrast_guard: bool,
    dynamic_contrast_threshold: f64,
    render_quality: RenderQuality,
    bar_width: f64,
    bar_gap: f64,
    bar_corner_radius: f64,
    segmented_bars: bool,
    segment_length: f64,
    segment_gap: f64,
    line_max_height_ratio: f64,
    bars_anchor: BarsAnchor,
    bars_direction: BarsDirection,
    ring_inner_ratio: f64,
    ring_length_ratio: f64,
    bars_wave_thickness: f64,
    bars_dot_radius: f64,
    ring_wave_thickness: f64,
    ring_dot_radius: f64,
    bars_wave_roundness: f64,
    ring_wave_roundness: f64,
    ring_fill_softness: f64,
    ring_fill_overlap_px: f64,
    layout: OverlayLayout,
    polygon_sides: usize,
    position: String,
    config_path: PathBuf,
    spectrum_mode: SpectrumMode,
    visual_mode: VisualMode,
    bars_style: RenderStyle,
    ring_style: RenderStyle,
    group_file: PathBuf,
    group_poll_ms: u64,
    ring_show_threshold: f64,
    ring_hide_threshold: f64,
    ring_fade_in_sec: f64,
    ring_fade_out_sec: f64,
    neon_enabled: bool,
    neon_strength: f64,
    neon_layers: usize,
    afterglow_enabled: bool,
    afterglow_decay: f64,
    afterglow_alpha: f64,
    particles_enabled: bool,
    particles_max: usize,
    particles_spawn_rate: f64,
    particles_life_min: f64,
    particles_life_max: f64,
    particles_speed_min: f64,
    particles_speed_max: f64,
    particles_size_min: f64,
    particles_size_max: f64,
    particles_alpha: f64,
    particles_drift: f64,
    particles_mode: String,
    particles_color: RgbaColor,
    particles_color_mode: ColorMode,
    particles_glow_strength: f64,
    particle_glow_pass_cap: usize,
    particles_update_divisor: u32,
    afterglow_update_divisor: u32,
    base_light_enabled: bool,
    base_light_height: f64,
    base_light_alpha: f64,
    base_light_color: RgbaColor,
    base_light_color_mode: ColorMode,
}

#[derive(Debug, Clone)]
struct GroupLayer {
    enabled: bool,
    mode: VisualMode,
    style: RenderStyle,
    profile: LayerProfile,
    zone: SpectrumZone,
    static_color: RgbaColor,
    color_mode: ColorMode,
    alpha: f64,
    auto_hide: bool,
    blend_mode: BlendMode,
    palette_channel: PaletteChannel,
    target_luma: Option<f64>,
    bars_anchor: Option<BarsAnchor>,
    bars_direction: Option<BarsDirection>,
    bar_width: Option<f64>,
    bar_gap: Option<f64>,
    bar_corner_radius: Option<f64>,
    segmented_bars: Option<bool>,
    segment_length: Option<f64>,
    segment_gap: Option<f64>,
    line_max_height_ratio: Option<f64>,
    ring_inner_ratio: Option<f64>,
    ring_length_ratio: Option<f64>,
    bars_wave_thickness: Option<f64>,
    bars_dot_radius: Option<f64>,
    ring_wave_thickness: Option<f64>,
    ring_dot_radius: Option<f64>,
    bars_wave_roundness: Option<f64>,
    ring_wave_roundness: Option<f64>,
    ring_fill_softness: Option<f64>,
    ring_fill_overlap_px: Option<f64>,
    polygon_sides: Option<usize>,
    particles_enabled: bool,
    particles_mode: ParticleMode,
    particles_style: ParticleStyle,
    particles_color_mode: ColorMode,
    particles_static_color: RgbaColor,
    particles_glow_strength: Option<f64>,
    particles_alpha_mult: Option<f64>,
    particles_size_mult: Option<f64>,
    glow_style: Option<GlowStyle>,
    neon_enabled: Option<bool>,
    neon_strength: Option<f64>,
    neon_layers: Option<usize>,
    afterglow_enabled: Option<bool>,
    afterglow_decay: Option<f64>,
    afterglow_alpha: Option<f64>,
    base_light_enabled: bool,
    base_light_height: Option<f64>,
    base_light_alpha: Option<f64>,
    base_light_color_mode: ColorMode,
    base_light_static_color: RgbaColor,
}

#[derive(Debug, Clone, Copy)]
struct LayerVisibility {
    alpha: f64,
    target_visible: bool,
}

#[derive(Debug, Clone, Copy)]
struct LayerProfile {
    gain: f64,
    gamma: f64,
    curve_drive: f64,
    bass_boost: f64,
    bass_power: f64,
    low_band_gain: f64,
    mid_band_gain: f64,
    high_band_gain: f64,
    height_scale: f64,
    loud_floor: f64,
    loud_floor_curve: f64,
}

#[derive(Debug, Clone, Copy)]
struct OverlayParticle {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    life: f64,
    age: f64,
    size: f64,
    alpha: f64,
    color: Option<RgbaColor>,
    color2: Option<RgbaColor>,
    glow_strength: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParticleMode {
    Auto,
    BarsBase,
    RingCenter,
}

impl ParticleMode {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bars_base" | "bars-base" | "bars" => Self::BarsBase,
            "ring_center" | "ring-center" | "ring" => Self::RingCenter,
            _ => Self::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParticleStyle {
    Soft,
    Spark,
    Dust,
    Neon,
    Orb,
    Trail,
    Burst,
    Orbit,
}

impl ParticleStyle {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "spark" => Self::Spark,
            "dust" => Self::Dust,
            "neon" => Self::Neon,
            "orb" => Self::Orb,
            "trail" => Self::Trail,
            "burst" => Self::Burst,
            "orbit" => Self::Orbit,
            _ => Self::Soft,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlowStyle {
    Neon,
    Inner,
    Outer,
    SoftBloom,
}

impl GlowStyle {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "inner" | "inner_glow" | "inner-glow" => Self::Inner,
            "outer" | "outer_glow" | "outer-glow" => Self::Outer,
            "soft_bloom" | "soft-bloom" | "bloom" => Self::SoftBloom,
            _ => Self::Neon,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ParticleStyleTuning {
    alpha_mult: f64,
    size_mult: f64,
    glow_mult: f64,
    speed_mult: f64,
    drift_mult: f64,
    life_mult: f64,
}

fn particle_style_tuning(style: ParticleStyle) -> ParticleStyleTuning {
    match style {
        ParticleStyle::Soft => ParticleStyleTuning {
            alpha_mult: 1.0,
            size_mult: 1.0,
            glow_mult: 1.0,
            speed_mult: 1.0,
            drift_mult: 1.0,
            life_mult: 1.0,
        },
        ParticleStyle::Spark => ParticleStyleTuning {
            alpha_mult: 1.15,
            size_mult: 0.88,
            glow_mult: 1.25,
            speed_mult: 1.18,
            drift_mult: 1.05,
            life_mult: 0.82,
        },
        ParticleStyle::Dust => ParticleStyleTuning {
            alpha_mult: 0.92,
            size_mult: 1.45,
            glow_mult: 0.85,
            speed_mult: 0.72,
            drift_mult: 1.35,
            life_mult: 1.32,
        },
        ParticleStyle::Neon => ParticleStyleTuning {
            alpha_mult: 1.20,
            size_mult: 1.18,
            glow_mult: 1.55,
            speed_mult: 1.05,
            drift_mult: 0.95,
            life_mult: 1.08,
        },
        ParticleStyle::Orb => ParticleStyleTuning {
            alpha_mult: 1.08,
            size_mult: 1.85,
            glow_mult: 1.40,
            speed_mult: 0.70,
            drift_mult: 0.75,
            life_mult: 1.45,
        },
        ParticleStyle::Trail => ParticleStyleTuning {
            alpha_mult: 0.95,
            size_mult: 0.86,
            glow_mult: 1.05,
            speed_mult: 1.10,
            drift_mult: 0.90,
            life_mult: 1.80,
        },
        ParticleStyle::Burst => ParticleStyleTuning {
            alpha_mult: 1.28,
            size_mult: 0.92,
            glow_mult: 1.35,
            speed_mult: 1.75,
            drift_mult: 1.10,
            life_mult: 0.62,
        },
        ParticleStyle::Orbit => ParticleStyleTuning {
            alpha_mult: 1.02,
            size_mult: 1.06,
            glow_mult: 1.18,
            speed_mult: 0.92,
            drift_mult: 0.52,
            life_mult: 1.22,
        },
    }
}

impl LayerProfile {
    fn defaults_for(mode: VisualMode) -> Self {
        match mode {
            VisualMode::Ring => Self {
                gain: 2.10,
                gamma: 0.70,
                curve_drive: 0.95,
                bass_boost: 0.22,
                bass_power: 2.1,
                low_band_gain: 1.0,
                mid_band_gain: 1.0,
                high_band_gain: 1.0,
                height_scale: 0.52,
                loud_floor: 0.22,
                loud_floor_curve: 1.18,
            },
            VisualMode::Bars => Self {
                gain: 1.82,
                gamma: 0.80,
                curve_drive: 0.84,
                bass_boost: 0.14,
                bass_power: 2.0,
                low_band_gain: 1.0,
                mid_band_gain: 1.0,
                high_band_gain: 1.0,
                height_scale: 0.44,
                loud_floor: 0.18,
                loud_floor_curve: 1.14,
            },
        }
    }
}

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "
        window#kitsune-overlay,
        window#kitsune-overlay.background,
        window#kitsune-overlay:backdrop,
        window#kitsune-overlay > *,
        window#kitsune-overlay > *.background,
        drawingarea#kitsune-bars,
        drawingarea#kitsune-bars.background {
            background-color: transparent;
            background-image: none;
            box-shadow: none;
            border: none;
        }
        ",
    );

    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );
    }
}

fn strip_background_classes(widget: &impl IsA<gtk::Widget>) {
    widget.remove_css_class("background");
}

fn default_config_path() -> PathBuf {
    if let Ok(path) = env::var("KITSUNE_CFG") {
        return PathBuf::from(path);
    }
    PathBuf::from("./config/base.conf")
}

fn args_config_path() -> PathBuf {
    let mut config_path = default_config_path();
    let mut it = env::args().skip(1);
    while let Some(arg) = it.next() {
        if arg == "--config" {
            if let Some(value) = it.next() {
                config_path = PathBuf::from(value);
            }
        }
    }
    config_path
}

fn cfg_map(path: &Path) -> std::io::Result<std::collections::HashMap<String, String>> {
    let raw = fs::read_to_string(path)?;
    let mut map = std::collections::HashMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        map.insert(key.trim().to_string(), value.trim().to_string());
    }
    Ok(map)
}

fn cfg_get_string(map: &HashMap<String, String>, key: &str, default: &str) -> String {
    map.get(key).cloned().unwrap_or_else(|| default.to_string())
}

fn map_get_num_f64(map: &HashMap<String, String>, key: &str, default: f64) -> f64 {
    map.get(key)
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(default)
}

fn effective_neon_layers(mode: VisualMode, requested_layers: usize, quality: RenderQuality) -> usize {
    let requested_layers = requested_layers.max(1);
    match quality {
        RenderQuality::High => requested_layers,
        RenderQuality::Balanced => match mode {
            VisualMode::Ring => requested_layers.min(2),
            VisualMode::Bars => requested_layers.min(4),
        },
        RenderQuality::Performance => match mode {
            VisualMode::Ring => requested_layers.min(1),
            VisualMode::Bars => requested_layers.min(2),
        },
    }
}

fn parse_config(path: &Path) -> Config {
    let map = match cfg_map(path) {
        Ok(map) => map,
        Err(err) => {
            eprintln!("[overlay] failed to load config {}: {}", path.display(), err);
            HashMap::new()
        }
    };
    let monitor = cfg_get_string(&map, "monitor", "DP-1");
    let width = map
        .get("width")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1920)
        .max(1);
    let height = map
        .get("height")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1080)
        .max(1);
    let bars = map
        .get("bars")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(48)
        .max(1);
    let fps = map
        .get("fps")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(60)
        .max(1);
    let spectrum_mode = SpectrumMode::from_str(&cfg_get_string(&map, "spectrum_mode", "single"));
    let mode_raw = cfg_get_string(&map, "mode", "bars");
    let bars_style_raw = cfg_get_string(&map, "bars_style", "bars");
    let ring_style_raw = cfg_get_string(&map, "ring_style", "waves_fill");
    let color = RgbaColor::from_hex_with_alpha(
        map.get("color").map(String::as_str).unwrap_or("#ff2f8f"),
        0.96,
    );
    let color2 = RgbaColor::from_hex_with_alpha(
        map.get("color2").map(String::as_str).unwrap_or("#19f0ff"),
        0.92,
    );
    let dynamic_color = map
        .get("dynamic_color")
        .map(|v| parse_boolish(v))
        .unwrap_or(false);
    let color_mode = map
        .get("color_mode")
        .map(|v| ColorMode::from_str(v))
        .unwrap_or(if dynamic_color { ColorMode::AccentMid } else { ColorMode::Static });
    let color2_mode = map
        .get("color2_mode")
        .map(|v| ColorMode::from_str(v))
        .unwrap_or(if dynamic_color { ColorMode::AccentLight } else { ColorMode::Static });
    let color_palette_file =
        PathBuf::from(cfg_get_string(&map, "color_palette_file", "/tmp/kitsune-accent.palette"));
    let dynamic_contrast_guard = map
        .get("dynamic_contrast_guard")
        .map(|v| parse_boolish(v))
        .unwrap_or(false);
    let dynamic_contrast_threshold = map
        .get("dynamic_contrast_threshold")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.68)
        .clamp(0.35, 0.95);
    let render_quality = map
        .get("render_quality")
        .map(|v| RenderQuality::from_str(v))
        .unwrap_or(RenderQuality::Balanced);
    let bar_width = map
        .get("bar_width")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(8.0)
        .clamp(1.0, 64.0);
    let bar_gap = map
        .get("bar_gap")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(3.0)
        .clamp(0.0, 48.0);
    let bar_corner_radius = map
        .get("bar_corner_radius")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(4.0)
        .clamp(0.0, 64.0);
    let segmented_bars = map
        .get("segmented_bars")
        .map(|v| parse_boolish(v))
        .unwrap_or(false);
    let segment_length = map
        .get("segment_length")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(10.0)
        .clamp(1.0, 96.0);
    let segment_gap = map
        .get("segment_gap")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(4.0)
        .clamp(0.0, 48.0);
    let line_max_height_ratio = map
        .get("line_max_height_ratio")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.68)
        .clamp(0.05, 1.0);
    let bars_anchor = map
        .get("bars_anchor")
        .map(|v| BarsAnchor::from_str(v))
        .unwrap_or(BarsAnchor::Bottom);
    let raw_bars_direction = map
        .get("bars_direction")
        .map(|v| BarsDirection::from_str(v))
        .unwrap_or(BarsDirection::Up);
    let bars_direction = match bars_anchor {
        BarsAnchor::Bottom => match raw_bars_direction {
            BarsDirection::Up | BarsDirection::Down => raw_bars_direction,
            _ => BarsDirection::Up,
        },
        BarsAnchor::Top => match raw_bars_direction {
            BarsDirection::Up | BarsDirection::Down => raw_bars_direction,
            _ => BarsDirection::Down,
        },
        BarsAnchor::Left => match raw_bars_direction {
            BarsDirection::Left | BarsDirection::Right => raw_bars_direction,
            _ => BarsDirection::Right,
        },
        BarsAnchor::Right => match raw_bars_direction {
            BarsDirection::Left | BarsDirection::Right => raw_bars_direction,
            _ => BarsDirection::Left,
        },
    };
    let ring_inner_ratio = map
        .get("ring_inner_ratio")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.20)
        .clamp(0.05, 0.75);
    let ring_length_ratio = map
        .get("ring_length_ratio")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.22)
        .clamp(0.05, 0.60);
    let bars_wave_thickness = map
        .get("bars_wave_thickness")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(3.0)
        .clamp(1.0, 24.0);
    let bars_dot_radius = map
        .get("bars_dot_radius")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(2.0)
        .clamp(1.0, 24.0);
    let ring_wave_thickness = map
        .get("ring_wave_thickness")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(2.0)
        .clamp(1.0, 24.0);
    let ring_dot_radius = map
        .get("ring_dot_radius")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(2.0)
        .clamp(1.0, 24.0);
    let bars_wave_roundness = map
        .get("bars_wave_roundness")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.70)
        .clamp(0.05, 1.0);
    let ring_wave_roundness = map
        .get("ring_wave_roundness")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.65)
        .clamp(0.05, 1.0);
    let ring_fill_softness = map
        .get("ring_fill_softness")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.35)
        .clamp(0.0, 1.0);
    let ring_fill_overlap_px = map
        .get("ring_fill_overlap_px")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(1.8)
        .clamp(0.0, 32.0);
    let polygon_sides = map
        .get("polygon_sides")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(3)
        .max(3);
    let position = cfg_get_string(&map, "overlay_position", "bottom");
    let group_file = PathBuf::from(cfg_get_string(&map, "group_file", "default.group"));
    let group_poll_ms = map
        .get("group_poll_ms")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(400)
        .max(50);
    let ring_show_threshold = map
        .get("ring_show_threshold")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.030)
        .clamp(0.001, 1.0);
    let ring_hide_threshold = map
        .get("ring_hide_threshold")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.012)
        .clamp(0.0, 1.0);
    let ring_fade_in_sec = map
        .get("ring_fade_in_sec")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.25)
        .clamp(0.01, 5.0);
    let ring_fade_out_sec = map
        .get("ring_fade_out_sec")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.85)
        .clamp(0.01, 10.0);
    let neon_enabled = map
        .get("neon_enabled")
        .map(|v| parse_boolish(v))
        .unwrap_or(map.get("postfx_enabled").map(|v| parse_boolish(v)).unwrap_or(true));
    let neon_strength = map
        .get("neon_strength")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(map.get("postfx_glow_strength").and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.35))
        .clamp(0.0, 4.0);
    let neon_layers = map
        .get("neon_layers")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(3)
        .clamp(1, 6);
    let afterglow_enabled = map
        .get("afterglow_enabled")
        .map(|v| parse_boolish(v))
        .unwrap_or(true);
    let afterglow_decay = map
        .get("afterglow_decay")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.78)
        .clamp(0.0, 0.98);
    let afterglow_alpha = map
        .get("afterglow_alpha")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.26)
        .clamp(0.0, 1.0);
    let particles_enabled = map
        .get("particles_enabled")
        .map(|v| parse_boolish(v))
        .unwrap_or(false);
    let particles_max = map
        .get("particles_max")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(100)
        .clamp(0, 4000);
    let particles_spawn_rate = map
        .get("particles_spawn_rate")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(24.0)
        .clamp(0.0, 4000.0);
    let particles_life_min = map
        .get("particles_life_min")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.45)
        .clamp(0.02, 8.0);
    let particles_life_max = map
        .get("particles_life_max")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(1.20)
        .clamp(0.03, 10.0);
    let particles_speed_min = map
        .get("particles_speed_min")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(24.0)
        .clamp(1.0, 4000.0);
    let particles_speed_max = map
        .get("particles_speed_max")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(96.0)
        .clamp(1.0, 4000.0);
    let particles_size_min = map
        .get("particles_size_min")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(1.0)
        .clamp(1.0, 24.0);
    let particles_size_max = map
        .get("particles_size_max")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(2.0)
        .clamp(1.0, 32.0);
    let particles_alpha = map
        .get("particles_alpha")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.58)
        .clamp(0.0, 1.0);
    let particles_drift = map
        .get("particles_drift")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(22.0)
        .clamp(0.0, 1200.0);
    let particles_mode = cfg_get_string(&map, "particles_mode", "auto");
    let particles_color = RgbaColor::from_hex_with_alpha(
        map.get("particles_color").map(String::as_str).unwrap_or("#FFFFFF"),
        0.96,
    );
    let particles_color_mode = map
        .get("particles_color_mode")
        .map(|v| ColorMode::from_str(v))
        .unwrap_or(ColorMode::Static);
    let particles_glow_strength = map
        .get("particles_glow_strength")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(1.15)
        .clamp(0.0, 4.0);
    let particle_glow_pass_cap = map
        .get("particle_glow_pass_cap")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(match render_quality {
            RenderQuality::Performance => 1,
            RenderQuality::Balanced => 2,
            RenderQuality::High => 3,
        })
        .clamp(0, 4);
    let particles_update_divisor = map
        .get("particles_update_divisor")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(match render_quality {
            RenderQuality::Performance => 3,
            RenderQuality::Balanced => 2,
            RenderQuality::High => 1,
        })
        .clamp(1, 8);
    let afterglow_update_divisor = map
        .get("afterglow_update_divisor")
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(match render_quality {
            RenderQuality::Performance => 3,
            RenderQuality::Balanced => 2,
            RenderQuality::High => 1,
        })
        .clamp(1, 8);
    let base_light_enabled = map
        .get("base_light_enabled")
        .map(|v| parse_boolish(v))
        .unwrap_or(false);
    let base_light_height = map
        .get("base_light_height")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(18.0)
        .clamp(1.0, 160.0);
    let base_light_alpha = map
        .get("base_light_alpha")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.26)
        .clamp(0.0, 1.0);
    let base_light_color = RgbaColor::from_hex_with_alpha(
        map.get("base_light_color").map(String::as_str).unwrap_or("#19f0ff"),
        0.96,
    );
    let base_light_color_mode = map
        .get("base_light_color_mode")
        .map(|v| ColorMode::from_str(v))
        .unwrap_or(ColorMode::AccentLight);

    Config {
        monitor,
        width,
        height,
        bars,
        fps,
        color,
        color2,
        color_mode,
        color2_mode,
        color_palette_file,
        dynamic_contrast_guard,
        dynamic_contrast_threshold,
        render_quality,
        bar_width,
        bar_gap,
        bar_corner_radius,
        segmented_bars,
        segment_length,
        segment_gap,
        line_max_height_ratio,
        bars_anchor,
        bars_direction,
        ring_inner_ratio,
        ring_length_ratio,
        bars_wave_thickness,
        bars_dot_radius,
        ring_wave_thickness,
        ring_dot_radius,
        bars_wave_roundness,
        ring_wave_roundness,
        ring_fill_softness,
        ring_fill_overlap_px,
        layout: OverlayLayout::from_mode_style(&mode_raw, &bars_style_raw),
        polygon_sides,
        position,
        config_path: path.to_path_buf(),
        spectrum_mode,
        visual_mode: VisualMode::from_str(&mode_raw),
        bars_style: RenderStyle::from_str(&bars_style_raw),
        ring_style: RenderStyle::from_str(&ring_style_raw),
        group_file,
        group_poll_ms,
        ring_show_threshold,
        ring_hide_threshold,
        ring_fade_in_sec,
        ring_fade_out_sec,
        neon_enabled,
        neon_strength,
        neon_layers,
        afterglow_enabled,
        afterglow_decay,
        afterglow_alpha,
        particles_enabled,
        particles_max,
        particles_spawn_rate,
        particles_life_min,
        particles_life_max,
        particles_speed_min,
        particles_speed_max,
        particles_size_min,
        particles_size_max,
        particles_alpha,
        particles_drift,
        particles_mode,
        particles_color,
        particles_color_mode,
        particles_glow_strength,
        particle_glow_pass_cap,
        particles_update_divisor,
        afterglow_update_divisor,
        base_light_enabled,
        base_light_height,
        base_light_alpha,
        base_light_color,
        base_light_color_mode,
    }
}

fn parse_boolish(raw: &str) -> bool {
    matches!(raw.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

fn resolve_group_path(base_config: &Path, raw: &str) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else {
        let primary = base_config
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(&path);
        if primary.exists() {
            return primary;
        }

        let cwd_joined = Path::new(".").join(&path);
        if cwd_joined.exists() {
            return cwd_joined;
        }

        if let Some(file_name) = path.file_name() {
            let xdg_home = env::var("XDG_CONFIG_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| env::var("HOME").ok().map(|home| Path::new(&home).join(".config")));
            if let Some(config_home) = xdg_home {
                let fallback = config_home.join("kitsune/groups").join(file_name);
                if fallback.exists() {
                    return fallback;
                }
            }
            let fallback = Path::new("./config/groups").join(file_name);
            if fallback.exists() {
                return fallback;
            }
        }

        primary
    }
}

fn resolve_profile_path(config_path: &Path, raw: &str) -> PathBuf {
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        return candidate;
    }

    let with_ext = if candidate.extension().is_some() {
        candidate.clone()
    } else {
        candidate.with_extension("profile")
    };

    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    let roots = [
        config_dir.join(&with_ext),
        Path::new(".").join(&with_ext),
        Path::new("./config/profiles").join(with_ext.file_name().unwrap_or_default()),
    ];

    for root in roots {
        if root.exists() {
            return root;
        }
    }

    config_dir.join(with_ext)
}

fn load_layer_profile(config_path: &Path, mode: VisualMode, raw: &str) -> LayerProfile {
    let defaults = LayerProfile::defaults_for(mode);
    let profile_path = resolve_profile_path(config_path, raw);
    let Ok(map) = cfg_map(&profile_path) else {
        return defaults;
    };

    LayerProfile {
        gain: map_get_num_f64(&map, "gain", defaults.gain).max(0.0),
        gamma: map_get_num_f64(&map, "gamma", defaults.gamma).max(0.0001),
        curve_drive: map_get_num_f64(&map, "curve_drive", defaults.curve_drive).max(0.1),
        bass_boost: map_get_num_f64(&map, "bass_boost", defaults.bass_boost).clamp(0.0, 4.0),
        bass_power: map_get_num_f64(&map, "bass_power", defaults.bass_power).clamp(1.0, 8.0),
        low_band_gain: map_get_num_f64(&map, "low_band_gain", defaults.low_band_gain).clamp(0.0, 4.0),
        mid_band_gain: map_get_num_f64(&map, "mid_band_gain", defaults.mid_band_gain).clamp(0.0, 4.0),
        high_band_gain: map_get_num_f64(&map, "high_band_gain", defaults.high_band_gain).clamp(0.0, 4.0),
        height_scale: map_get_num_f64(&map, "height_scale", defaults.height_scale).clamp(0.05, 1.0),
        loud_floor: map_get_num_f64(&map, "loud_floor", defaults.loud_floor).clamp(0.0, 1.0),
        loud_floor_curve: map_get_num_f64(&map, "loud_floor_curve", defaults.loud_floor_curve).max(0.5),
    }
}

fn parse_group_layers(config_path: &Path, group_path: &Path) -> Vec<GroupLayer> {
    let Ok(raw) = fs::read_to_string(group_path) else {
        eprintln!("[overlay] failed to read group file {}", group_path.display());
        return Vec::new();
    };
    let mut layers = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        if key.trim() != "layer" {
            continue;
        }
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        if parts.len() < 6 {
            continue;
        }
        let enabled = parse_boolish(parts[0]);
        let mode = VisualMode::from_str(parts[1]);
        let style = RenderStyle::from_str(parts[2]);
        let profile = load_layer_profile(config_path, mode, parts[3]);
        let alpha = parts[5].parse::<f64>().unwrap_or(1.0).clamp(0.0, 1.0);
        let mut color_mode = ColorMode::Static;
        let mut auto_hide = mode == VisualMode::Ring;
        let mut blend_mode = BlendMode::Normal;
        let mut palette_channel = PaletteChannel::Auto;
        let mut target_luma = None;
        let mut zone = SpectrumZone::Full;
        let mut bars_anchor = None;
        let mut bars_direction = None;
        let mut bar_width = None;
        let mut bar_gap = None;
        let mut bar_corner_radius = None;
        let mut segmented_bars = None;
        let mut segment_length = None;
        let mut segment_gap = None;
        let mut line_max_height_ratio = None;
        let mut ring_inner_ratio = None;
        let mut ring_length_ratio = None;
        let mut bars_wave_thickness = None;
        let mut bars_dot_radius = None;
        let mut ring_wave_thickness = None;
        let mut ring_dot_radius = None;
        let mut bars_wave_roundness = None;
        let mut ring_wave_roundness = None;
        let mut ring_fill_softness = None;
        let mut ring_fill_overlap_px = None;
        let mut polygon_sides = None;
        let mut particles_enabled = false;
        let mut particles_mode = ParticleMode::Auto;
        let mut particles_style = ParticleStyle::Soft;
        let mut particles_color_mode = color_mode;
        let mut particles_static_color = RgbaColor::from_hex_with_alpha(parts[4], alpha);
        let mut particles_glow_strength = None;
        let mut particles_alpha_mult = None;
        let mut particles_size_mult = None;
        let mut glow_style = None;
        let mut neon_enabled = None;
        let mut neon_strength = None;
        let mut neon_layers = None;
        let mut afterglow_enabled = None;
        let mut afterglow_decay = None;
        let mut afterglow_alpha = None;
        let mut base_light_enabled = false;
        let mut base_light_height = None;
        let mut base_light_alpha = None;
        let mut base_light_color_mode = color_mode;
        let mut base_light_static_color = RgbaColor::from_hex_with_alpha(parts[4], alpha);
        if matches!(
            parts[4].to_ascii_lowercase().as_str(),
            "accent_light" | "accent_mid" | "accent_dark" | "static"
        ) {
            color_mode = ColorMode::from_str(parts[4]);
            particles_color_mode = color_mode;
            base_light_color_mode = color_mode;
        }
        for extra in parts.iter().skip(6) {
            if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("color_mode")
            {
                color_mode = ColorMode::from_str(value);
                particles_color_mode = color_mode;
                base_light_color_mode = color_mode;
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("auto_hide")
            {
                auto_hide = parse_boolish(value);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("blend_mode")
            {
                blend_mode = BlendMode::from_str(value);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("palette_channel")
            {
                palette_channel = PaletteChannel::from_str(value);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("target_luma")
            {
                target_luma = value.parse::<f64>().ok().map(|v| v.clamp(0.0, 1.0));
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("zone")
            {
                zone = SpectrumZone::from_str(value);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("bars_anchor")
            {
                bars_anchor = Some(BarsAnchor::from_str(value));
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("bars_direction")
            {
                bars_direction = Some(BarsDirection::from_str(value));
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("bar_width")
            {
                bar_width = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("bar_gap")
            {
                bar_gap = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("bar_corner_radius")
            {
                bar_corner_radius = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("segmented_bars")
            {
                segmented_bars = Some(parse_boolish(value));
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("segment_length")
            {
                segment_length = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("segment_gap")
            {
                segment_gap = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("line_max_height_ratio")
            {
                line_max_height_ratio = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("ring_inner_ratio")
            {
                ring_inner_ratio = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("ring_length_ratio")
            {
                ring_length_ratio = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("bars_wave_thickness")
            {
                bars_wave_thickness = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("bars_dot_radius")
            {
                bars_dot_radius = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("ring_wave_thickness")
            {
                ring_wave_thickness = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("ring_dot_radius")
            {
                ring_dot_radius = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("bars_wave_roundness")
            {
                bars_wave_roundness = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("ring_wave_roundness")
            {
                ring_wave_roundness = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("ring_fill_softness")
            {
                ring_fill_softness = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("ring_fill_overlap_px")
            {
                ring_fill_overlap_px = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("polygon_sides")
            {
                polygon_sides = value.parse::<usize>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("particles")
            {
                particles_enabled = parse_boolish(value);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("particles_mode")
            {
                particles_mode = ParticleMode::from_str(value);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("particles_style")
            {
                particles_style = ParticleStyle::from_str(value);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("particles_color_mode")
            {
                particles_color_mode = ColorMode::from_str(value);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("particles_color")
            {
                particles_static_color = RgbaColor::from_hex_with_alpha(value, alpha);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("particles_glow_strength")
            {
                particles_glow_strength = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("particles_alpha_mult")
            {
                particles_alpha_mult = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("particles_size_mult")
            {
                particles_size_mult = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && (key.trim().eq_ignore_ascii_case("glow_style")
                    || key.trim().eq_ignore_ascii_case("neon_style"))
            {
                glow_style = Some(GlowStyle::from_str(value));
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("neon")
            {
                neon_enabled = Some(parse_boolish(value));
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("neon_strength")
            {
                neon_strength = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("neon_layers")
            {
                neon_layers = value.parse::<usize>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("afterglow")
            {
                afterglow_enabled = Some(parse_boolish(value));
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("afterglow_decay")
            {
                afterglow_decay = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("afterglow_alpha")
            {
                afterglow_alpha = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("base_light")
            {
                base_light_enabled = parse_boolish(value);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("base_light_height")
            {
                base_light_height = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("base_light_alpha")
            {
                base_light_alpha = value.parse::<f64>().ok();
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("base_light_color_mode")
            {
                base_light_color_mode = ColorMode::from_str(value);
            } else if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("base_light_color")
            {
                base_light_static_color = RgbaColor::from_hex_with_alpha(value, alpha);
            }
        }
        let static_color = if color_mode == ColorMode::Static {
            RgbaColor::from_hex_with_alpha(parts[4], alpha)
        } else {
            RgbaColor::from_hex_with_alpha("#ffffff", alpha)
        };
        layers.push(GroupLayer {
            enabled,
            mode,
            style,
            profile,
            zone,
            static_color,
            color_mode,
            alpha,
            auto_hide,
            blend_mode,
            palette_channel,
            target_luma,
            bars_anchor,
            bars_direction,
            bar_width,
            bar_gap,
            bar_corner_radius,
            segmented_bars,
            segment_length,
            segment_gap,
            line_max_height_ratio,
            ring_inner_ratio,
            ring_length_ratio,
            bars_wave_thickness,
            bars_dot_radius,
            ring_wave_thickness,
            ring_dot_radius,
            bars_wave_roundness,
            ring_wave_roundness,
            ring_fill_softness,
            ring_fill_overlap_px,
            polygon_sides,
            particles_enabled,
            particles_mode,
            particles_style,
            particles_color_mode,
            particles_static_color,
            particles_glow_strength,
            particles_alpha_mult,
            particles_size_mult,
            glow_style,
            neon_enabled,
            neon_strength,
            neon_layers,
            afterglow_enabled,
            afterglow_decay,
            afterglow_alpha,
            base_light_enabled,
            base_light_height,
            base_light_alpha,
            base_light_color_mode,
            base_light_static_color,
        });
    }
    layers
}

fn apply_layer_profile(values: &[f64], mode: VisualMode, profile: &LayerProfile) -> Vec<f64> {
    if values.is_empty() {
        return Vec::new();
    }

    let denom = (values.len().saturating_sub(1)).max(1) as f64;
    let energy = values.iter().copied().sum::<f64>() / values.len() as f64;
    let loud_floor = profile.loud_floor * energy.powf(profile.loud_floor_curve);
    let height_gain = (profile.height_scale / 0.52).clamp(0.25, 2.0);

    values
        .iter()
        .enumerate()
        .map(|(i, src)| {
            let pos = i as f64 / denom;
            let low = 1.0 - pos;
            let band_gain = if pos < 0.33 {
                profile.low_band_gain
            } else if pos < 0.66 {
                profile.mid_band_gain
            } else {
                profile.high_band_gain
            };
            let boosted = *src * band_gain * (1.0 + profile.bass_boost * low.powf(profile.bass_power));
            let raw = (boosted * profile.gain).max(0.0).powf(profile.gamma);
            let mut curved = 1.0 - (-raw * profile.curve_drive).exp();
            curved *= height_gain;
            let curved = curved.clamp(0.0, 1.0);
            match mode {
                VisualMode::Ring => curved.max(loud_floor * 0.85).clamp(0.0, 1.0),
                VisualMode::Bars => {
                    let center_dist = (pos - 0.5).abs() * 2.0;
                    let floor_bar = loud_floor * (0.85 + 0.15 * (1.0 - center_dist));
                    curved.max(floor_bar).clamp(0.0, 1.0)
                }
            }
        })
        .collect()
}

fn zone_weight(zone: SpectrumZone, pos: f64) -> f64 {
    let clamp_band = |start: f64, end: f64, p: f64| -> f64 {
        if p < start || p > end {
            return 0.0;
        }
        let center = (start + end) * 0.5;
        let half = ((end - start) * 0.5).max(1e-6);
        let dist = ((p - center).abs() / half).clamp(0.0, 1.0);
        (1.0 - dist * dist).clamp(0.0, 1.0)
    };

    match zone {
        SpectrumZone::Full => 1.0,
        SpectrumZone::Bass => clamp_band(0.00, 0.22, pos),
        SpectrumZone::Mid => clamp_band(0.20, 0.68, pos),
        SpectrumZone::Treble => clamp_band(0.62, 1.00, pos),
        SpectrumZone::BassMid => clamp_band(0.00, 0.56, pos),
        SpectrumZone::MidTreble => clamp_band(0.36, 1.00, pos),
    }
}

fn apply_spectrum_zone(values: &[f64], zone: SpectrumZone) -> Vec<f64> {
    if values.is_empty() || zone == SpectrumZone::Full {
        return values.to_vec();
    }
    let denom = (values.len().saturating_sub(1)).max(1) as f64;
    values
        .iter()
        .enumerate()
        .map(|(i, value)| {
            let pos = i as f64 / denom;
            value * zone_weight(zone, pos)
        })
        .collect()
}

fn compute_layer_energy(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().copied().sum::<f64>() / values.len() as f64
    }
}

fn monitor_by_name(name: &str) -> Option<gdk::Monitor> {
    let display = gdk::Display::default()?;
    let model = display.monitors();
    for idx in 0..model.n_items() {
        let item = model.item(idx)?;
        let mon = item.downcast::<gdk::Monitor>().ok()?;
        if mon.connector().as_deref() == Some(name) {
            return Some(mon);
        }
    }
    None
}

fn apply_layer_shell(window: &gtk::ApplicationWindow, cfg: &Config, monitor: Option<&gdk::Monitor>) {
    if !gtk4_layer_shell::is_supported() {
        eprintln!("[overlay] gtk4-layer-shell unsupported in current compositor/session");
        return;
    }

    window.init_layer_shell();
    window.set_monitor(monitor);
    window.set_namespace(Some("kitsune"));
    window.set_layer(Layer::Bottom);
    window.set_keyboard_mode(KeyboardMode::None);
    window.set_exclusive_zone(0);

    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, false);
        window.set_margin(edge, 0);
    }

    match cfg.position.as_str() {
        "top" => {
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Left, true);
            window.set_anchor(Edge::Right, true);
        }
        "left" => {
            window.set_anchor(Edge::Left, true);
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Bottom, true);
        }
        "right" => {
            window.set_anchor(Edge::Right, true);
            window.set_anchor(Edge::Top, true);
            window.set_anchor(Edge::Bottom, true);
        }
        _ => {
            window.set_anchor(Edge::Bottom, true);
            window.set_anchor(Edge::Left, true);
            window.set_anchor(Edge::Right, true);
        }
    }
}

fn write_cava_config(bar_count: usize, framerate: u32) -> std::io::Result<PathBuf> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let path = env::temp_dir().join(format!("kitsune-overlay-cava-{timestamp}.conf"));
    let content = format!(
        "[general]\nframerate = {framerate}\nbars = {bar_count}\n[output]\nmethod = raw\nraw_target = /dev/stdout\ndata_format = ascii\nascii_max_range = 1000\nchannels = mono\n"
    );
    fs::write(&path, content)?;
    Ok(path)
}

fn parse_cava_line(line: &str, expected_bars: usize) -> Option<Vec<f64>> {
    let values: Vec<f64> = line
        .trim()
        .split(';')
        .filter(|p| !p.is_empty())
        .filter_map(|part| part.parse::<f64>().ok())
        .map(|value| (value / 1000.0).clamp(0.0, 1.0))
        .collect();
    if values.len() < expected_bars {
        return None;
    }
    Some(values.into_iter().take(expected_bars).collect())
}

fn spawn_cava_stream(bar_count: usize, framerate: u32) -> std::io::Result<Arc<Mutex<Vec<f64>>>> {
    let latest = Arc::new(Mutex::new(vec![0.0; bar_count]));
    let config_path = write_cava_config(bar_count, framerate)?;
    let latest_for_thread = Arc::clone(&latest);

    thread::spawn(move || {
        let mut command = Command::new("cava");
        command
            .arg("-p")
            .arg(&config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(err) => {
                eprintln!("[overlay] failed to start cava: {err}");
                let _ = fs::remove_file(&config_path);
                return;
            }
        };

        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                eprintln!("[overlay] cava produced no stdout");
                let _ = child.kill();
                let _ = fs::remove_file(&config_path);
                return;
            }
        };

        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let mut smoothed = vec![0.0; bar_count];

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if let Some(values) = parse_cava_line(&line, bar_count) {
                        for (slot, input) in smoothed.iter_mut().zip(values.iter()) {
                            let attack = 0.24;
                            let decay = 0.84;
                            if *input > *slot {
                                *slot = *slot + ((*input - *slot) * attack);
                            } else {
                                *slot *= decay;
                            }
                        }
                        if let Ok(mut target) = latest_for_thread.lock() {
                            *target = smoothed.clone();
                        }
                    }
                }
                Err(err) => {
                    eprintln!("[overlay] cava read error: {err}");
                    break;
                }
            }
        }

        let _ = child.kill();
        let _ = fs::remove_file(&config_path);
    });

    Ok(latest)
}

fn draw_round_bar(ctx: &gtk::cairo::Context, x: f64, y: f64, width: f64, height: f64, radius: f64) {
    let x = (x * 2.0).round() * 0.5;
    let y = (y * 2.0).round() * 0.5;
    let width = (width * 2.0).round() * 0.5;
    let height = (height * 2.0).round() * 0.5;
    let radius = radius.max(0.0).min(width * 0.5).min(height * 0.5);
    if radius <= 0.0 {
        ctx.rectangle(x, y, width, height);
        return;
    }
    ctx.new_sub_path();
    ctx.move_to(x + radius, y);
    ctx.line_to(x + width - radius, y);
    ctx.arc(x + width - radius, y + radius, radius, -PI / 2.0, 0.0);
    ctx.line_to(x + width, y + height - radius);
    ctx.arc(x + width - radius, y + height - radius, radius, 0.0, PI / 2.0);
    ctx.line_to(x + radius, y + height);
    ctx.arc(x + radius, y + height - radius, radius, PI / 2.0, PI);
    ctx.line_to(x, y + radius);
    ctx.arc(x + radius, y + radius, radius, PI, 3.0 * PI / 2.0);
    ctx.close_path();
}

#[derive(Clone, Copy)]
enum BarOrientation {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
struct BarRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Clone, Copy)]
struct BarStyle {
    corner_radius: f64,
    segmented: bool,
    segment_length: f64,
    segment_gap: f64,
}

fn for_each_segment_span(
    total_length: f64,
    segment_length: f64,
    segment_gap: f64,
    from_start: bool,
    mut segment: impl FnMut(f64, f64),
) {
    let total_length = total_length.max(0.0);
    if total_length <= 0.0 {
        return;
    }
    let segment_length = segment_length.max(1.0);
    let segment_gap = segment_gap.max(0.0);
    let step = segment_length + segment_gap;

    if from_start {
        let mut cursor = 0.0;
        while cursor < total_length {
            let length = (total_length - cursor).min(segment_length);
            if length <= 0.0 {
                break;
            }
            segment(cursor, length);
            cursor += step;
        }
        return;
    }

    let mut cursor = total_length;
    while cursor > 0.0 {
        let start = (cursor - segment_length).max(0.0);
        let length = cursor - start;
        if length <= 0.0 {
            break;
        }
        segment(start, length);
        if start <= 0.0 {
            break;
        }
        cursor = (start - segment_gap).max(0.0);
    }
}

fn append_bar_path(
    ctx: &gtk::cairo::Context,
    rect: BarRect,
    style: BarStyle,
    orientation: BarOrientation,
    forward: bool,
) {
    if style.segmented {
        match orientation {
            BarOrientation::Horizontal => {
                for_each_segment_span(
                    rect.height,
                    style.segment_length,
                    style.segment_gap,
                    forward,
                    |offset, len| {
                        draw_round_bar(ctx, rect.x, rect.y + offset, rect.width, len, style.corner_radius);
                    },
                );
            }
            BarOrientation::Vertical => {
                for_each_segment_span(
                    rect.width,
                    style.segment_length,
                    style.segment_gap,
                    forward,
                    |offset, len| {
                        draw_round_bar(ctx, rect.x + offset, rect.y, len, rect.height, style.corner_radius);
                    },
                );
            }
        }
        return;
    }

    draw_round_bar(ctx, rect.x, rect.y, rect.width, rect.height, style.corner_radius);
}

fn append_directed_bar_path(
    ctx: &gtk::cairo::Context,
    center_x: f64,
    center_y: f64,
    angle: f64,
    length: f64,
    thickness: f64,
    style: BarStyle,
) {
    ctx.save().ok();
    ctx.translate(center_x, center_y);
    ctx.rotate(angle);
    append_bar_path(
        ctx,
        BarRect {
            x: 0.0,
            y: -(thickness * 0.5),
            width: length.max(2.0),
            height: thickness.max(1.0),
        },
        style,
        BarOrientation::Vertical,
        true,
    );
    ctx.restore().ok();
}

#[derive(Clone, Copy, Debug)]
struct RadialDistribution {
    first_angle: f64,
    angle_step: f64,
    tangential_thickness: f64,
}

fn radial_distribution(
    count: usize,
    inner_radius: f64,
    thickness: f64,
    gap: f64,
    start_angle: f64,
    arc_radians: f64,
) -> Option<RadialDistribution> {
    if count == 0 {
        return None;
    }
    let inner_radius = inner_radius.max(1.0);
    let arc_magnitude = (PI * 2.0_f64).min(arc_radians.abs().max(0.001));
    let full_circle = (arc_magnitude - (PI * 2.0)).abs() < 0.001;
    let gap_count = if count <= 1 {
        0
    } else if full_circle {
        count
    } else {
        count.saturating_sub(1)
    } as f64;
    let total_nominal = (count as f64 * thickness.max(1.0)) + (gap_count * gap.max(0.0));
    let available_arc_length = arc_magnitude * inner_radius;
    let scale = if total_nominal > available_arc_length {
        available_arc_length / total_nominal
    } else {
        1.0
    };
    let tangential_thickness = (thickness * scale).max(1.0);
    let base_gap = gap.max(0.0) * scale;
    let occupied_length = (count as f64 * tangential_thickness) + (gap_count * base_gap);
    let extra_gap = if gap_count > 0.0 {
        (available_arc_length - occupied_length).max(0.0) / gap_count
    } else {
        0.0
    };
    let effective_gap = base_gap + extra_gap;
    let angle_step = if count <= 1 {
        0.0
    } else {
        (tangential_thickness + effective_gap) / inner_radius
    };
    let first_angle = if full_circle {
        start_angle
    } else if count == 1 {
        start_angle + (arc_radians * 0.5)
    } else {
        start_angle + (tangential_thickness * 0.5 / inner_radius)
    };
    Some(RadialDistribution {
        first_angle,
        angle_step,
        tangential_thickness,
    })
}

fn build_drawing_area(cfg: &Config, stream: Arc<Mutex<Vec<f64>>>) -> gtk::DrawingArea {
    let drawing_area = gtk::DrawingArea::new();
    drawing_area.set_widget_name("kitsune-bars");
    strip_background_classes(&drawing_area);
    drawing_area.set_content_width(cfg.width);
    drawing_area.set_content_height(cfg.height);
    let color = cfg.color;
    let color2 = cfg.color2;
    let default_palette = Palette::from_base(color, color2);
    let palette = Arc::new(Mutex::new(load_palette(&cfg.color_palette_file, default_palette)));
    let bar_width = cfg.bar_width;
    let bar_gap = cfg.bar_gap;
    let bar_corner_radius = cfg.bar_corner_radius;
    let segmented_bars = cfg.segmented_bars;
    let segment_length = cfg.segment_length;
    let segment_gap = cfg.segment_gap;
    let line_max_height_ratio = cfg.line_max_height_ratio;
    let bars_anchor = cfg.bars_anchor;
    let bars_direction = cfg.bars_direction;
    let ring_inner_ratio = cfg.ring_inner_ratio;
    let ring_length_ratio = cfg.ring_length_ratio;
    let bars_wave_thickness = cfg.bars_wave_thickness;
    let bars_dot_radius = cfg.bars_dot_radius;
    let ring_wave_thickness = cfg.ring_wave_thickness;
    let ring_dot_radius = cfg.ring_dot_radius;
    let bars_wave_roundness = cfg.bars_wave_roundness;
    let ring_wave_roundness = cfg.ring_wave_roundness;
    let ring_fill_softness = cfg.ring_fill_softness;
    let ring_fill_overlap_px = cfg.ring_fill_overlap_px;
    let polygon_sides = cfg.polygon_sides;
    let spectrum_mode = cfg.spectrum_mode;
    let single_mode = cfg.visual_mode;
    let single_bars_style = cfg.bars_style;
    let single_ring_style = cfg.ring_style;
    let single_color_mode = cfg.color_mode;
    let single_color2_mode = cfg.color2_mode;
    let config_path = cfg.config_path.clone();
    let group_raw = cfg.group_file.to_string_lossy().to_string();
    let group_path = resolve_group_path(&config_path, &group_raw);
    eprintln!(
        "[overlay] spectrum_mode={:?} group_file_raw={} group_file_resolved={}",
        spectrum_mode,
        group_raw,
        group_path.display()
    );
    let group_layers = Arc::new(Mutex::new(parse_group_layers(&config_path, &group_path)));
    let group_layers_for_timer = Arc::clone(&group_layers);
    let group_last_mtime = Arc::new(Mutex::new(fs::metadata(&group_path).and_then(|m| m.modified()).ok()));
    let group_last_mtime_for_timer = Arc::clone(&group_last_mtime);
    let group_visibility = Arc::new(Mutex::new(Vec::<LayerVisibility>::new()));
    let group_visibility_for_draw = Arc::clone(&group_visibility);
    let group_visibility_for_timer = Arc::clone(&group_visibility);
    let group_poll_ms = cfg.group_poll_ms;
    let ring_show_threshold = cfg.ring_show_threshold;
    let ring_hide_threshold = cfg.ring_hide_threshold;
    let ring_fade_in_sec = cfg.ring_fade_in_sec;
    let ring_fade_out_sec = cfg.ring_fade_out_sec;
    let palette_file = cfg.color_palette_file.clone();
    let palette_for_timer = Arc::clone(&palette);
    let stream_for_draw = Arc::clone(&stream);
    let stream_for_timer = Arc::clone(&stream);
    let stream_for_tick = Arc::clone(&stream);
    let group_layers_for_tick = Arc::clone(&group_layers);
    let afterglow_state = Arc::new(Mutex::new(vec![0.0; cfg.bars]));
    let afterglow_state_for_draw = Arc::clone(&afterglow_state);
    let afterglow_state_for_timer = Arc::clone(&afterglow_state);
    let group_afterglow_state = Arc::new(Mutex::new(Vec::<Vec<f64>>::new()));
    let group_afterglow_state_for_draw = Arc::clone(&group_afterglow_state);
    let group_afterglow_state_for_timer = Arc::clone(&group_afterglow_state);
    let particles = Arc::new(Mutex::new(Vec::<OverlayParticle>::new()));
    let particles_for_draw = Arc::clone(&particles);
    let particles_for_timer = Arc::clone(&particles);
    let group_particles = Arc::new(Mutex::new(Vec::<Vec<OverlayParticle>>::new()));
    let group_particles_for_draw = Arc::clone(&group_particles);
    let group_particles_for_timer = Arc::clone(&group_particles);
    let palette_for_particles_tick = Arc::clone(&palette);
    let particle_accum = Arc::new(Mutex::new(0.0_f64));
    let particle_accum_for_timer = Arc::clone(&particle_accum);
    let group_particle_rr = Arc::new(Mutex::new(0_usize));
    let group_particle_rr_for_timer = Arc::clone(&group_particle_rr);
    let particle_frame_counter = Arc::new(Mutex::new(0_u64));
    let particle_frame_counter_for_timer = Arc::clone(&particle_frame_counter);
    let afterglow_frame_counter = Arc::new(Mutex::new(0_u64));
    let afterglow_frame_counter_for_timer = Arc::clone(&afterglow_frame_counter);
    let rng_state = Arc::new(Mutex::new(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xC0DEC0DE),
    ));
    let rng_state_for_timer = Arc::clone(&rng_state);
    let particle_mode = ParticleMode::from_str(&cfg.particles_mode);
    let neon_enabled = cfg.neon_enabled;
    let neon_strength = cfg.neon_strength;
    let neon_layers = cfg.neon_layers;
    let afterglow_enabled = cfg.afterglow_enabled;
    let afterglow_alpha = cfg.afterglow_alpha;
    let afterglow_decay = cfg.afterglow_decay;
    let particles_enabled = cfg.particles_enabled;
    let particles_max = cfg.particles_max;
    let particles_spawn_rate = cfg.particles_spawn_rate;
    let particles_speed_max = cfg.particles_speed_max;
    let particles_color = cfg.particles_color;
    let particles_color_mode = cfg.particles_color_mode;
    let particles_glow_strength = cfg.particles_glow_strength;
    let particle_glow_pass_cap = cfg.particle_glow_pass_cap;
    let particles_update_divisor = cfg.particles_update_divisor.max(1);
    let afterglow_update_divisor = cfg.afterglow_update_divisor.max(1);
    let render_quality = cfg.render_quality;
    let base_light_enabled = cfg.base_light_enabled;
    let base_light_height = cfg.base_light_height;
    let base_light_alpha = cfg.base_light_alpha;
    let base_light_color = cfg.base_light_color;
    let base_light_color_mode = cfg.base_light_color_mode;
    let dynamic_contrast_guard = cfg.dynamic_contrast_guard;
    let dynamic_contrast_threshold = cfg.dynamic_contrast_threshold;
    let cfg_for_particles = cfg.clone();
    let cfg_for_draw_layers = cfg.clone();

    drawing_area.set_draw_func(move |_, ctx, width, height| {
        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        let _ = ctx.paint();

        let values = stream_for_draw.lock().map(|v| v.clone()).unwrap_or_default();
        if values.is_empty() {
            return;
        }
        let afterglow_values = afterglow_state_for_draw
            .lock()
            .map(|v| v.clone())
            .unwrap_or_else(|_| vec![0.0; values.len()]);
        let current_palette = palette.lock().map(|v| *v).unwrap_or(default_palette);
        let resolved_base_light_color = current_palette.resolve_with_contrast_guard(
            base_light_color_mode,
            base_light_color,
            dynamic_contrast_guard,
            dynamic_contrast_threshold,
        );
        if spectrum_mode == SpectrumMode::Group {
            let layers = group_layers.lock().map(|v| v.clone()).unwrap_or_default();
            let vis = group_visibility_for_draw
                .lock()
                .map(|v| v.clone())
                .unwrap_or_default();
            let layer_particles = group_particles_for_draw
                .lock()
                .map(|v| v.clone())
                .unwrap_or_default();
            let layer_afterglow_states = group_afterglow_state_for_draw
                .lock()
                .map(|v| v.clone())
                .unwrap_or_default();
            let mut particle_draw_queue: Vec<(Vec<OverlayParticle>, RgbaColor, RgbaColor, f64, BlendMode)> = Vec::new();
            for (layer_index, layer) in layers.iter().enumerate().rev() {
                if !layer.enabled {
                    continue;
                }
                let base_color = if layer.color_mode == ColorMode::Static {
                    layer.static_color
                } else {
                    current_palette.resolve_custom_dynamic(
                        layer.color_mode,
                        layer.static_color,
                        layer.palette_channel,
                        layer.target_luma,
                        dynamic_contrast_guard,
                        dynamic_contrast_threshold,
                    )
                };
                let layer_base_light_color = if layer.base_light_color_mode == ColorMode::Static {
                    layer.base_light_static_color
                } else {
                    current_palette.resolve_custom_dynamic(
                        layer.base_light_color_mode,
                        layer.base_light_static_color,
                        layer.palette_channel,
                        layer.target_luma,
                        dynamic_contrast_guard,
                        dynamic_contrast_threshold,
                    )
                };
                let (layer_bars_anchor, layer_bars_direction) =
                    effective_bars_orientation_for_layer(layer, &cfg_for_draw_layers);
                let layer_neon_enabled = layer.neon_enabled.unwrap_or(neon_enabled);
                let layer_glow_style = layer.glow_style.unwrap_or(GlowStyle::Neon);
                let layer_neon_strength = layer.neon_strength.unwrap_or(neon_strength);
                let layer_neon_layers = effective_neon_layers(
                    layer.mode,
                    layer.neon_layers.unwrap_or(neon_layers),
                    render_quality,
                );
                let layer_afterglow_enabled = layer.afterglow_enabled.unwrap_or(afterglow_enabled);
                let layer_afterglow_alpha = layer.afterglow_alpha.unwrap_or(afterglow_alpha);
                let layer_base_light_enabled = layer.base_light_enabled;
                let layer_base_light_height = layer.base_light_height.unwrap_or(base_light_height);
                let layer_base_light_alpha = layer.base_light_alpha.unwrap_or(base_light_alpha);
                let layer_bar_width = layer.bar_width.unwrap_or(bar_width);
                let layer_bar_gap = layer.bar_gap.unwrap_or(bar_gap);
                let layer_bar_corner_radius = layer.bar_corner_radius.unwrap_or(bar_corner_radius);
                let layer_segmented_bars = layer.segmented_bars.unwrap_or(segmented_bars);
                let layer_segment_length = layer.segment_length.unwrap_or(segment_length);
                let layer_segment_gap = layer.segment_gap.unwrap_or(segment_gap);
                let layer_line_max_height_ratio =
                    layer.line_max_height_ratio.unwrap_or(line_max_height_ratio);
                let layer_ring_inner_ratio = layer.ring_inner_ratio.unwrap_or(ring_inner_ratio);
                let layer_ring_length_ratio = layer.ring_length_ratio.unwrap_or(ring_length_ratio);
                let layer_bars_wave_thickness =
                    layer.bars_wave_thickness.unwrap_or(bars_wave_thickness);
                let layer_bars_dot_radius = layer.bars_dot_radius.unwrap_or(bars_dot_radius);
                let layer_ring_wave_thickness =
                    layer.ring_wave_thickness.unwrap_or(ring_wave_thickness);
                let layer_ring_dot_radius = layer.ring_dot_radius.unwrap_or(ring_dot_radius);
                let layer_bars_wave_roundness =
                    layer.bars_wave_roundness.unwrap_or(bars_wave_roundness);
                let layer_ring_wave_roundness =
                    layer.ring_wave_roundness.unwrap_or(ring_wave_roundness);
                let layer_ring_fill_softness =
                    layer.ring_fill_softness.unwrap_or(ring_fill_softness);
                let layer_ring_fill_overlap_px =
                    layer.ring_fill_overlap_px.unwrap_or(ring_fill_overlap_px);
                let layer_polygon_sides = layer.polygon_sides.unwrap_or(polygon_sides);
                let zoned_values = apply_spectrum_zone(&values, layer.zone);
                let layer_values = apply_layer_profile(&zoned_values, layer.mode, &layer.profile);
                let visibility_alpha = vis
                    .get(layer_index)
                    .map(|entry| entry.alpha)
                    .unwrap_or(if layer.auto_hide { 0.0 } else { 1.0 });
                if visibility_alpha <= 0.001 {
                    continue;
                }
                let layer_color2 = gradient_color(
                    base_color,
                    current_palette.resolve_custom_dynamic(
                        ColorMode::AccentLight,
                        current_palette.accent_light,
                        layer.palette_channel,
                        layer.target_luma.map(|v| (v + 0.12).clamp(0.0, 1.0)),
                        dynamic_contrast_guard,
                        dynamic_contrast_threshold,
                    ),
                    0.35,
                );
                let layer_alpha = (layer.alpha * visibility_alpha * 1.8).clamp(0.0, 1.0);
                let _ = ctx.save();
                ctx.set_operator(layer.blend_mode.cairo_operator());
                if layer_afterglow_enabled && layer_afterglow_alpha > 0.001 {
                    let ghost_values = layer_afterglow_states
                        .get(layer_index)
                        .cloned()
                        .unwrap_or_default();
                    draw_visual_layer_with_effects(
                        ctx,
                        width as f64,
                        height as f64,
                        &ghost_values,
                        layer.mode,
                        layer.style,
                        gradient_color(base_color, layer_color2, 0.45),
                        layer_color2,
                        layer_bar_width,
                        layer_bar_gap,
                        layer_bar_corner_radius,
                        layer_segmented_bars,
                        layer_segment_length,
                        layer_segment_gap,
                        layer_bars_wave_thickness,
                        layer_bars_dot_radius,
                        layer_ring_wave_thickness,
                        layer_ring_dot_radius,
                        layer_bars_wave_roundness,
                        layer_ring_wave_roundness,
                        layer_ring_fill_softness,
                        layer_ring_fill_overlap_px,
                        layer_line_max_height_ratio,
                        layer_bars_anchor,
                        layer_bars_direction,
                        layer_ring_inner_ratio,
                        layer_ring_length_ratio,
                        layer_polygon_sides,
                        (layer_alpha * layer_afterglow_alpha).clamp(0.0, 1.0),
                        false,
                        layer_glow_style,
                        layer_neon_strength,
                        layer_neon_layers,
                        false,
                        layer_base_light_height,
                        layer_base_light_alpha,
                        layer_base_light_color,
                    );
                }
                draw_visual_layer_with_effects(
                    ctx,
                    width as f64,
                    height as f64,
                    &layer_values,
                    layer.mode,
                    layer.style,
                    base_color,
                    layer_color2,
                    layer_bar_width,
                    layer_bar_gap,
                    layer_bar_corner_radius,
                    layer_segmented_bars,
                    layer_segment_length,
                    layer_segment_gap,
                    layer_bars_wave_thickness,
                    layer_bars_dot_radius,
                    layer_ring_wave_thickness,
                    layer_ring_dot_radius,
                    layer_bars_wave_roundness,
                    layer_ring_wave_roundness,
                    layer_ring_fill_softness,
                    layer_ring_fill_overlap_px,
                    layer_line_max_height_ratio,
                    layer_bars_anchor,
                    layer_bars_direction,
                    layer_ring_inner_ratio,
                    layer_ring_length_ratio,
                    layer_polygon_sides,
                    layer_alpha,
                    layer_neon_enabled,
                    layer_glow_style,
                    layer_neon_strength,
                    layer_neon_layers,
                    layer_base_light_enabled,
                    layer_base_light_height,
                    layer_base_light_alpha,
                    layer_base_light_color,
                );
                let _ = ctx.restore();
                if layer.particles_enabled
                    && let Some(live_particles) = layer_particles.get(layer_index)
                {
                    let (particle_color, particle_color2) = resolve_group_layer_particle_colors(
                        current_palette,
                        layer,
                        color2,
                        dynamic_contrast_guard,
                        dynamic_contrast_threshold,
                    );
                    particle_draw_queue.push((
                        live_particles.clone(),
                        particle_color,
                        particle_color2,
                        layer
                            .particles_glow_strength
                            .unwrap_or(particles_glow_strength),
                        layer.blend_mode,
                    ));
                }
            }
            for (live_particles, particle_color, particle_color2, glow_strength, blend_mode) in particle_draw_queue {
                let _ = ctx.save();
                ctx.set_operator(blend_mode.cairo_operator());
                draw_particles(
                    ctx,
                    &live_particles,
                    particle_color,
                    particle_color2,
                    glow_strength,
                    particle_glow_pass_cap,
                );
                let _ = ctx.restore();
            }
            return;
        }

        let single_color = current_palette.resolve_with_contrast_guard(
            single_color_mode,
            color,
            dynamic_contrast_guard,
            dynamic_contrast_threshold,
        );
        let single_color2 = current_palette.resolve_with_contrast_guard(
            single_color2_mode,
            color2,
            dynamic_contrast_guard,
            dynamic_contrast_threshold,
        );
        let style = match single_mode {
            VisualMode::Ring => single_ring_style,
            VisualMode::Bars => single_bars_style,
        };
        let single_neon_layers = effective_neon_layers(single_mode, neon_layers, render_quality);
        if afterglow_enabled && afterglow_alpha > 0.001 {
            draw_visual_layer_with_effects(
                ctx,
                width as f64,
                height as f64,
                &afterglow_values,
                single_mode,
                style,
                gradient_color(single_color, single_color2, 0.45),
                single_color2,
                bar_width,
                bar_gap,
                bar_corner_radius,
                segmented_bars,
                segment_length,
                segment_gap,
                bars_wave_thickness,
                bars_dot_radius,
                ring_wave_thickness,
                ring_dot_radius,
                bars_wave_roundness,
                ring_wave_roundness,
                ring_fill_softness,
                ring_fill_overlap_px,
                line_max_height_ratio,
                bars_anchor,
                bars_direction,
                ring_inner_ratio,
                ring_length_ratio,
                polygon_sides,
                afterglow_alpha.clamp(0.0, 1.0),
                false,
                GlowStyle::Neon,
                neon_strength,
                single_neon_layers,
                false,
                base_light_height,
                base_light_alpha,
                resolved_base_light_color,
            );
        }
        draw_visual_layer_with_effects(
            ctx,
            width as f64,
            height as f64,
            &values,
            single_mode,
            style,
            single_color,
            single_color2,
            bar_width,
            bar_gap,
            bar_corner_radius,
            segmented_bars,
            segment_length,
            segment_gap,
            bars_wave_thickness,
            bars_dot_radius,
            ring_wave_thickness,
            ring_dot_radius,
            bars_wave_roundness,
            ring_wave_roundness,
            ring_fill_softness,
            ring_fill_overlap_px,
            line_max_height_ratio,
            bars_anchor,
            bars_direction,
            ring_inner_ratio,
            ring_length_ratio,
            polygon_sides,
            1.0,
            neon_enabled,
            GlowStyle::Neon,
            neon_strength,
            single_neon_layers,
            base_light_enabled,
            base_light_height,
            base_light_alpha,
            resolved_base_light_color,
        );
        if particles_enabled {
            let particle_color = current_palette.resolve_with_contrast_guard(
                particles_color_mode,
                particles_color,
                dynamic_contrast_guard,
                dynamic_contrast_threshold,
            );
            if let Ok(live_particles) = particles_for_draw.lock() {
                draw_particles(
                    ctx,
                    &live_particles,
                    particle_color,
                    single_color2,
                    particles_glow_strength,
                    particle_glow_pass_cap,
                );
            }
        }
    });

    let area_weak = drawing_area.downgrade();
    let tick_ms = (1000_u64 / u64::from(cfg.fps)).max(1);
    glib::timeout_add_local(Duration::from_millis(tick_ms), move || {
        if let Some(area) = area_weak.upgrade() {
            let dt = tick_ms as f64 / 1000.0;
            let snapshot = stream_for_tick.lock().map(|v| v.clone()).unwrap_or_default();
            let afterglow_tick_ready = if let Ok(mut frame_counter) = afterglow_frame_counter_for_timer.lock() {
                *frame_counter = frame_counter.saturating_add(1);
                (*frame_counter % u64::from(afterglow_update_divisor)) == 0
            } else {
                true
            };
            if afterglow_tick_ready {
                let afterglow_dt = dt * afterglow_update_divisor as f64;
                let afterglow_decay_step = afterglow_decay.powf(afterglow_dt.max(0.0001) / dt.max(0.0001));
                if spectrum_mode != SpectrumMode::Group
                    && afterglow_enabled
                    && !snapshot.is_empty()
                    && let Ok(mut ghost) = afterglow_state_for_timer.lock()
                {
                    if ghost.len() != snapshot.len() {
                        ghost.resize(snapshot.len(), 0.0);
                    }
                    for (target, sample) in ghost.iter_mut().zip(snapshot.iter()) {
                        *target = (sample.max(*target * afterglow_decay_step)).clamp(0.0, 1.0);
                    }
                }
                if spectrum_mode == SpectrumMode::Group
                    && !snapshot.is_empty()
                    && let Ok(layers_snapshot) = group_layers_for_tick.lock().map(|v| v.clone())
                    && let Ok(mut per_layer_ghosts) = group_afterglow_state_for_timer.lock()
                {
                    if per_layer_ghosts.len() != layers_snapshot.len() {
                        per_layer_ghosts.resize_with(layers_snapshot.len(), Vec::new);
                    }
                    for (layer_index, layer) in layers_snapshot.iter().enumerate() {
                        let layer_afterglow_enabled = layer.afterglow_enabled.unwrap_or(afterglow_enabled);
                        let layer_afterglow_decay = layer.afterglow_decay.unwrap_or(afterglow_decay);
                        let layer_afterglow_decay_step =
                            layer_afterglow_decay.powf(afterglow_dt.max(0.0001) / dt.max(0.0001));
                        if !layer.enabled || !layer_afterglow_enabled {
                            per_layer_ghosts[layer_index].clear();
                            continue;
                        }
                        let zoned_values = apply_spectrum_zone(&snapshot, layer.zone);
                        let layer_values = apply_layer_profile(&zoned_values, layer.mode, &layer.profile);
                        let ghost = &mut per_layer_ghosts[layer_index];
                        if ghost.len() != layer_values.len() {
                            ghost.resize(layer_values.len(), 0.0);
                        }
                        for (target, sample) in ghost.iter_mut().zip(layer_values.iter()) {
                            *target = (sample.max(*target * layer_afterglow_decay_step)).clamp(0.0, 1.0);
                        }
                    }
                }
            }
            let particles_tick_ready = if let Ok(mut frame_counter) = particle_frame_counter_for_timer.lock() {
                *frame_counter = frame_counter.saturating_add(1);
                (*frame_counter % u64::from(particles_update_divisor)) == 0
            } else {
                true
            };
            if particles_tick_ready
                && particles_enabled
                && particles_max > 0
                && width_height_valid(cfg_for_particles.width, cfg_for_particles.height)
            {
                let particle_dt = dt * particles_update_divisor as f64;
                let layers_snapshot = group_layers_for_tick.lock().map(|v| v.clone()).unwrap_or_default();
                let mode = effective_particle_mode(
                    particle_mode,
                    spectrum_mode,
                    single_mode,
                    &layers_snapshot,
                );
                let palette_snapshot = palette_for_particles_tick
                    .lock()
                    .map(|v| *v)
                    .unwrap_or(default_palette);
                let mut spawn_budget = particles_spawn_rate * particle_dt;
                if let Ok(mut accum) = particle_accum_for_timer.lock() {
                    *accum += spawn_budget;
                    spawn_budget = accum.floor();
                    *accum -= spawn_budget;
                }
                if let Ok(mut rng) = rng_state_for_timer.lock() {
                    let particle_layers: Vec<GroupLayer> = if spectrum_mode == SpectrumMode::Group {
                        layers_snapshot
                            .iter()
                            .filter(|layer| layer.enabled && layer.particles_enabled)
                            .cloned()
                            .collect()
                    } else {
                        Vec::new()
                    };
                    if spectrum_mode == SpectrumMode::Group && !particle_layers.is_empty() {
                        if let Ok(mut per_layer_particles) = group_particles_for_timer.lock() {
                            if per_layer_particles.len() != layers_snapshot.len() {
                                per_layer_particles.resize_with(layers_snapshot.len(), Vec::new);
                            }
                            let enabled_indices: Vec<usize> = layers_snapshot
                                .iter()
                                .enumerate()
                                .filter_map(|(idx, layer)| if layer.enabled && layer.particles_enabled { Some(idx) } else { None })
                                .collect();
                            let mut rr_cursor = group_particle_rr_for_timer
                                .lock()
                                .map(|v| *v)
                                .unwrap_or(0);
                            for spawn_index in 0..spawn_budget as usize {
                                if enabled_indices.is_empty() {
                                    break;
                                }
                                let layer_index = enabled_indices[(rr_cursor + spawn_index) % enabled_indices.len()];
                                let layer = &layers_snapshot[layer_index];
                                let total_count: usize = per_layer_particles.iter().map(Vec::len).sum();
                                if total_count >= particles_max {
                                    break;
                                }
                                let (primary, secondary) = resolve_group_layer_particle_colors(
                                    palette_snapshot,
                                    layer,
                                    color2,
                                    dynamic_contrast_guard,
                                    dynamic_contrast_threshold,
                                );
                                let glow = layer
                                    .particles_glow_strength
                                    .unwrap_or(cfg_for_particles.particles_glow_strength);
                                per_layer_particles[layer_index].push(spawn_group_layer_particle(
                                    &mut rng,
                                    layer,
                                    cfg_for_particles.width as f64,
                                    cfg_for_particles.height as f64,
                                    &cfg_for_particles,
                                    primary,
                                    secondary,
                                    glow,
                                ));
                            }
                            if !enabled_indices.is_empty()
                                && let Ok(mut rr) = group_particle_rr_for_timer.lock()
                            {
                                rr_cursor = (rr_cursor + spawn_budget as usize) % enabled_indices.len();
                                *rr = rr_cursor;
                            }
                            for particles_for_layer in per_layer_particles.iter_mut() {
                                for particle in particles_for_layer.iter_mut() {
                                    particle.age += particle_dt;
                                    particle.x += particle.vx * particle_dt;
                                    particle.y += particle.vy * particle_dt;
                                    particle.vx += lerp(-cfg_for_particles.particles_drift, cfg_for_particles.particles_drift, pseudo_rand01(&mut rng)) * particle_dt * 0.18;
                                    if particle.vx.abs() <= particle.vy.abs() {
                                        particle.vy += particle.vy.signum() * particles_speed_max * particle_dt * 0.06;
                                    } else {
                                        particle.vx += particle.vx.signum() * particles_speed_max * particle_dt * 0.06;
                                    }
                                }
                                particles_for_layer.retain(|particle| {
                                    particle.age < particle.life
                                        && particle.x >= -32.0
                                        && particle.x <= cfg_for_particles.width as f64 + 32.0
                                        && particle.y >= -64.0
                                        && particle.y <= cfg_for_particles.height as f64 + 64.0
                                });
                            }
                        }
                    } else if let Ok(mut live_particles) = particles_for_timer.lock() {
                        for _ in 0..spawn_budget as usize {
                            if live_particles.len() >= particles_max {
                                break;
                            }
                            live_particles.push(spawn_overlay_particle(
                                &mut rng,
                                mode,
                                cfg_for_particles.width as f64,
                                cfg_for_particles.height as f64,
                                &cfg_for_particles,
                            ));
                        }
                        for particle in live_particles.iter_mut() {
                            particle.age += particle_dt;
                            particle.x += particle.vx * particle_dt;
                            particle.y += particle.vy * particle_dt;
                            particle.vx += lerp(-cfg_for_particles.particles_drift, cfg_for_particles.particles_drift, pseudo_rand01(&mut rng)) * particle_dt * 0.18;
                            if particle.vx.abs() <= particle.vy.abs() {
                                particle.vy += particle.vy.signum() * particles_speed_max * particle_dt * 0.06;
                            } else {
                                particle.vx += particle.vx.signum() * particles_speed_max * particle_dt * 0.06;
                            }
                        }
                        live_particles.retain(|particle| {
                            particle.age < particle.life
                                && particle.x >= -32.0
                                && particle.x <= cfg_for_particles.width as f64 + 32.0
                                && particle.y >= -64.0
                                && particle.y <= cfg_for_particles.height as f64 + 64.0
                        });
                    }
                }
            }
            area.queue_draw();
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });

    let area_weak_reload = drawing_area.downgrade();
    glib::timeout_add_local(Duration::from_millis(group_poll_ms), move || {
        let new_mtime = fs::metadata(&group_path).and_then(|m| m.modified()).ok();
        let mut changed = false;
        if let Ok(mut current) = group_last_mtime_for_timer.lock()
            && *current != new_mtime
        {
            *current = new_mtime;
            changed = true;
        }
        if changed {
            let layers = parse_group_layers(&config_path, &group_path);
            if let Ok(mut target) = group_layers_for_timer.lock() {
                *target = layers;
            }
            if let Ok(mut vis) = group_visibility_for_timer.lock() {
                vis.clear();
            }
            if let Some(area) = area_weak_reload.upgrade() {
                area.queue_draw();
            }
        }
        let loaded_palette = load_palette(&palette_file, default_palette);
        if let Ok(mut target) = palette_for_timer.lock() {
            *target = loaded_palette;
        }
        let snapshot = stream_for_timer.lock().map(|v| v.clone()).unwrap_or_default();
        let layers = group_layers_for_timer
            .lock()
            .map(|v| v.clone())
            .unwrap_or_default();
        if let Ok(mut vis) = group_visibility_for_timer.lock() {
            if vis.len() != layers.len() {
                vis.resize(
                    layers.len(),
                    LayerVisibility {
                        alpha: 1.0,
                        target_visible: true,
                    },
                );
            }
            for (entry, layer) in vis.iter_mut().zip(layers.iter()) {
                if !layer.enabled {
                    entry.alpha = 0.0;
                    entry.target_visible = false;
                    continue;
                }
                if !layer.auto_hide {
                    entry.alpha = 1.0;
                    entry.target_visible = true;
                    continue;
                }
                let zoned = apply_spectrum_zone(&snapshot, layer.zone);
                let profiled = apply_layer_profile(&zoned, layer.mode, &layer.profile);
                let energy = compute_layer_energy(&profiled);
                if entry.target_visible {
                    if energy <= ring_hide_threshold {
                        entry.target_visible = false;
                    }
                } else if energy >= ring_show_threshold {
                    entry.target_visible = true;
                }

                if entry.target_visible {
                    let step = (group_poll_ms as f64 / 1000.0 / ring_fade_in_sec).clamp(0.0, 1.0);
                    entry.alpha = (entry.alpha + step).clamp(0.0, 1.0);
                } else {
                    let step = (group_poll_ms as f64 / 1000.0 / ring_fade_out_sec).clamp(0.0, 1.0);
                    entry.alpha = (entry.alpha - step).clamp(0.0, 1.0);
                }
            }
        }
        glib::ControlFlow::Continue
    });

    drawing_area
}

fn width_height_valid(width: i32, height: i32) -> bool {
    width > 0 && height > 0
}

fn gradient_color(a: RgbaColor, b: RgbaColor, t: f64) -> RgbaColor {
    RgbaColor {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

fn vivid_color(mut color: RgbaColor) -> RgbaColor {
    let max = color.r.max(color.g).max(color.b);
    let min = color.r.min(color.g).min(color.b);
    let saturation = (max - min).clamp(0.0, 1.0);
    let boost = if saturation < 0.35 { 1.22 } else { 1.10 };
    color.r = (color.r * boost).clamp(0.0, 1.0);
    color.g = (color.g * boost).clamp(0.0, 1.0);
    color.b = (color.b * boost).clamp(0.0, 1.0);
    color.a = color.a.clamp(0.92, 1.0);
    color
}

fn pseudo_rand01(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    let value = (*state >> 11) as f64;
    let max = (u64::MAX >> 11) as f64;
    (value / max).clamp(0.0, 1.0)
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn effective_particle_mode(
    configured: ParticleMode,
    spectrum_mode: SpectrumMode,
    single_mode: VisualMode,
    layers: &[GroupLayer],
) -> ParticleMode {
    match configured {
        ParticleMode::Auto => {
            if spectrum_mode == SpectrumMode::Group {
                if layers.iter().any(|layer| layer.enabled && layer.mode == VisualMode::Ring) {
                    ParticleMode::RingCenter
                } else {
                    ParticleMode::BarsBase
                }
            } else if single_mode == VisualMode::Ring {
                ParticleMode::RingCenter
            } else {
                ParticleMode::BarsBase
            }
        }
        explicit => explicit,
    }
}

fn effective_particle_mode_for_layer(layer: &GroupLayer, cfg: &Config) -> ParticleMode {
    match layer.particles_mode {
        ParticleMode::Auto => match layer.mode {
            VisualMode::Ring => ParticleMode::RingCenter,
            VisualMode::Bars => match cfg.bars_anchor {
                BarsAnchor::Top | BarsAnchor::Bottom => ParticleMode::BarsBase,
                BarsAnchor::Left => ParticleMode::BarsBase,
                BarsAnchor::Right => ParticleMode::BarsBase,
            },
        },
        explicit => explicit,
    }
}

fn effective_bars_orientation_for_layer(layer: &GroupLayer, cfg: &Config) -> (BarsAnchor, BarsDirection) {
    let anchor = layer.bars_anchor.unwrap_or(cfg.bars_anchor);
    let raw_direction = layer.bars_direction.unwrap_or(cfg.bars_direction);
    let direction = match anchor {
        BarsAnchor::Bottom => match raw_direction {
            BarsDirection::Up | BarsDirection::Down => raw_direction,
            _ => BarsDirection::Up,
        },
        BarsAnchor::Top => match raw_direction {
            BarsDirection::Up | BarsDirection::Down => raw_direction,
            _ => BarsDirection::Down,
        },
        BarsAnchor::Left => match raw_direction {
            BarsDirection::Left | BarsDirection::Right => raw_direction,
            _ => BarsDirection::Right,
        },
        BarsAnchor::Right => match raw_direction {
            BarsDirection::Left | BarsDirection::Right => raw_direction,
            _ => BarsDirection::Left,
        },
    };
    (anchor, direction)
}

fn resolve_group_layer_particle_colors(
    palette: Palette,
    layer: &GroupLayer,
    fallback_secondary: RgbaColor,
    dynamic_contrast_guard: bool,
    dynamic_contrast_threshold: f64,
) -> (RgbaColor, RgbaColor) {
    let primary = if layer.particles_color_mode == ColorMode::Static {
        layer.particles_static_color
    } else {
        palette.resolve_custom_dynamic(
            layer.particles_color_mode,
            layer.particles_static_color,
            layer.palette_channel,
            layer.target_luma,
            dynamic_contrast_guard,
            dynamic_contrast_threshold,
        )
    };
    let secondary = gradient_color(
        primary,
        palette.resolve_custom_dynamic(
            ColorMode::AccentLight,
            fallback_secondary,
            layer.palette_channel,
            layer.target_luma.map(|v| (v + 0.14).clamp(0.0, 1.0)),
            dynamic_contrast_guard,
            dynamic_contrast_threshold,
        ),
        0.35,
    );
    (primary, secondary)
}

fn spawn_group_layer_particle(
    rng: &mut u64,
    layer: &GroupLayer,
    width: f64,
    height: f64,
    cfg: &Config,
    primary: RgbaColor,
    secondary: RgbaColor,
    glow_strength: f64,
) -> OverlayParticle {
    let mode = effective_particle_mode_for_layer(layer, cfg);
    let (layer_anchor, _) = effective_bars_orientation_for_layer(layer, cfg);
    let tuning = particle_style_tuning(layer.particles_style);
    let mut particle = match mode {
        ParticleMode::RingCenter => {
            let mut particle = spawn_overlay_particle(rng, mode, width, height, cfg);
            let inner = width.min(height) * layer.ring_inner_ratio.unwrap_or(cfg.ring_inner_ratio);
            let angle = (particle.y - (height * 0.5)).atan2(particle.x - (width * 0.5));
            let spread = lerp(0.0, width.min(height) * 0.018, pseudo_rand01(rng));
            let origin_radius = inner + spread;
            particle.x = (width * 0.5) + angle.cos() * origin_radius;
            particle.y = (height * 0.5) + angle.sin() * origin_radius;
            particle.life *= tuning.life_mult;
            if layer.particles_style == ParticleStyle::Orbit {
                let cx = width * 0.5;
                let cy = height * 0.5;
                let dx = particle.x - cx;
                let dy = particle.y - cy;
                let angle = dy.atan2(dx);
                let tangential_speed = (particle.vx.abs() + particle.vy.abs()).max(cfg.particles_speed_min * 0.65);
                particle.vx = (angle + PI * 0.5).cos() * tangential_speed;
                particle.vy = (angle + PI * 0.5).sin() * tangential_speed;
            }
            particle
        }
        ParticleMode::BarsBase => {
            let life = lerp(cfg.particles_life_min, cfg.particles_life_max, pseudo_rand01(rng))
                * tuning.life_mult;
            let speed = lerp(cfg.particles_speed_min, cfg.particles_speed_max, pseudo_rand01(rng))
                * tuning.speed_mult;
            let size = lerp(cfg.particles_size_min, cfg.particles_size_max, pseudo_rand01(rng))
                * tuning.size_mult;
            let drift = cfg.particles_drift * tuning.drift_mult;
            let (x, y, vx, vy) = match layer_anchor {
                BarsAnchor::Top => (
                    pseudo_rand01(rng) * width,
                    lerp(4.0, height * 0.08, pseudo_rand01(rng)),
                    lerp(-drift, drift, pseudo_rand01(rng)),
                    speed,
                ),
                BarsAnchor::Left => (
                    lerp(4.0, width * 0.08, pseudo_rand01(rng)),
                    pseudo_rand01(rng) * height,
                    speed,
                    lerp(-drift, drift, pseudo_rand01(rng)),
                ),
                BarsAnchor::Right => (
                    width - lerp(4.0, width * 0.08, pseudo_rand01(rng)),
                    pseudo_rand01(rng) * height,
                    -speed,
                    lerp(-drift, drift, pseudo_rand01(rng)),
                ),
                BarsAnchor::Bottom => (
                    pseudo_rand01(rng) * width,
                    height - lerp(4.0, height * 0.08, pseudo_rand01(rng)),
                    lerp(-drift, drift, pseudo_rand01(rng)),
                    -speed,
                ),
            };
            OverlayParticle {
                x,
                y,
                vx,
                vy,
                life,
                age: 0.0,
                size,
                alpha: cfg.particles_alpha,
                color: None,
                color2: None,
                glow_strength: None,
            }
        }
        ParticleMode::Auto => spawn_overlay_particle(rng, mode, width, height, cfg),
    };
    particle.alpha = (particle.alpha * tuning.alpha_mult).clamp(0.0, 1.0);
    if let Some(alpha_mult) = layer.particles_alpha_mult {
        particle.alpha = (particle.alpha * alpha_mult).clamp(0.0, 1.0);
    }
    if let Some(size_mult) = layer.particles_size_mult {
        particle.size = (particle.size * size_mult).clamp(0.5, 24.0);
    }
    particle.color = Some(primary);
    particle.color2 = Some(secondary);
    particle.glow_strength = Some(glow_strength * tuning.glow_mult);
    particle
}

fn spawn_overlay_particle(
    rng: &mut u64,
    mode: ParticleMode,
    width: f64,
    height: f64,
    cfg: &Config,
) -> OverlayParticle {
    let life = lerp(cfg.particles_life_min, cfg.particles_life_max, pseudo_rand01(rng));
    let speed = lerp(cfg.particles_speed_min, cfg.particles_speed_max, pseudo_rand01(rng));
    let size = lerp(cfg.particles_size_min, cfg.particles_size_max, pseudo_rand01(rng));
    let drift = cfg.particles_drift;
    match mode {
        ParticleMode::RingCenter => {
            let angle = pseudo_rand01(rng) * PI * 2.0;
            let inner = width.min(height) * cfg.ring_inner_ratio;
            let spread = lerp(0.0, width.min(height) * 0.018, pseudo_rand01(rng));
            let origin_radius = inner + spread;
            let x = (width * 0.5) + angle.cos() * origin_radius;
            let y = (height * 0.5) + angle.sin() * origin_radius;
            let drift_angle = angle + lerp(-0.35, 0.35, pseudo_rand01(rng));
            OverlayParticle {
                x,
                y,
                vx: drift_angle.cos() * speed + lerp(-drift, drift, pseudo_rand01(rng)),
                vy: drift_angle.sin() * speed + lerp(-drift, drift, pseudo_rand01(rng)),
                life,
                age: 0.0,
                size,
                alpha: cfg.particles_alpha,
                color: None,
                color2: None,
                glow_strength: None,
            }
        }
        _ => {
            let (x, y, vx, vy) = match cfg.bars_anchor {
                BarsAnchor::Top => (
                    pseudo_rand01(rng) * width,
                    lerp(4.0, height * 0.08, pseudo_rand01(rng)),
                    lerp(-drift, drift, pseudo_rand01(rng)),
                    speed,
                ),
                BarsAnchor::Left => (
                    lerp(4.0, width * 0.08, pseudo_rand01(rng)),
                    pseudo_rand01(rng) * height,
                    speed,
                    lerp(-drift, drift, pseudo_rand01(rng)),
                ),
                BarsAnchor::Right => (
                    width - lerp(4.0, width * 0.08, pseudo_rand01(rng)),
                    pseudo_rand01(rng) * height,
                    -speed,
                    lerp(-drift, drift, pseudo_rand01(rng)),
                ),
                BarsAnchor::Bottom => (
                    pseudo_rand01(rng) * width,
                    height - lerp(4.0, height * 0.08, pseudo_rand01(rng)),
                    lerp(-drift, drift, pseudo_rand01(rng)),
                    -speed,
                ),
            };
            OverlayParticle {
                x,
                y,
                vx,
                vy,
                life,
                age: 0.0,
                size,
                alpha: cfg.particles_alpha,
                color: None,
                color2: None,
                glow_strength: None,
            }
        }
    }
}

fn draw_particles(
    ctx: &gtk::cairo::Context,
    particles: &[OverlayParticle],
    color: RgbaColor,
    color2: RgbaColor,
    glow_strength: f64,
    glow_pass_cap: usize,
) {
    for (index, particle) in particles.iter().enumerate() {
        let t = if particle.life <= 0.0 {
            1.0
        } else {
            (particle.age / particle.life).clamp(0.0, 1.0)
        };
        let blend = ((index as f64 * 0.137) % 1.0).clamp(0.0, 1.0);
        let base_color = particle.color.unwrap_or(color);
        let base_color2 = particle.color2.unwrap_or(color2);
        let glow_strength = particle.glow_strength.unwrap_or(glow_strength);
        let c = vivid_color(gradient_color(base_color, base_color2, blend));
        let alpha = (particle.alpha * (1.0 - t)).clamp(0.0, 1.0);
        if alpha <= 0.001 {
            continue;
        }
        let glow_color = vivid_color(gradient_color(base_color2, base_color, 0.35));
        let glow_layers = if glow_strength <= 0.01 {
            0
        } else if glow_strength < 1.2 {
            1
        } else if glow_strength < 2.0 {
            2
        } else {
            3
        }
        .min(glow_pass_cap);
        for pass in (1..=glow_layers).rev() {
            let pass_scale = 1.0 + (pass as f64 * 0.75 * glow_strength);
            let pass_alpha = (alpha * 0.18 * glow_strength / pass as f64).clamp(0.0, 0.55);
            ctx.set_source_rgba(glow_color.r, glow_color.g, glow_color.b, pass_alpha);
            ctx.arc(
                particle.x,
                particle.y,
                (particle.size.max(0.5) * pass_scale).max(0.8),
                0.0,
                PI * 2.0,
            );
            let _ = ctx.fill();
        }
        ctx.set_source_rgba(c.r, c.g, c.b, alpha);
        ctx.arc(particle.x, particle.y, particle.size.max(0.5), 0.0, PI * 2.0);
        let _ = ctx.fill();
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_visual_layer_with_effects(
    ctx: &gtk::cairo::Context,
    width: f64,
    height: f64,
    values: &[f64],
    mode: VisualMode,
    style: RenderStyle,
    color: RgbaColor,
    color2: RgbaColor,
    bar_thickness: f64,
    bar_gap: f64,
    bar_corner_radius: f64,
    segmented_bars: bool,
    segment_length: f64,
    segment_gap: f64,
    bars_wave_thickness: f64,
    bars_dot_radius: f64,
    ring_wave_thickness: f64,
    ring_dot_radius: f64,
    bars_wave_roundness: f64,
    ring_wave_roundness: f64,
    ring_fill_softness: f64,
    ring_fill_overlap_px: f64,
    line_max_height_ratio: f64,
    bars_anchor: BarsAnchor,
    bars_direction: BarsDirection,
    ring_inner_ratio: f64,
    ring_length_ratio: f64,
    polygon_sides: usize,
    alpha_scale: f64,
    neon_enabled: bool,
    glow_style: GlowStyle,
    neon_strength: f64,
    neon_layers: usize,
    base_light_enabled: bool,
    base_light_height: f64,
    base_light_alpha: f64,
    base_light_color: RgbaColor,
) {
    if neon_enabled && neon_strength > 0.01 {
        let glow_layers = neon_layers.max(1);
        for pass in (1..=glow_layers).rev() {
            let (widen, glow_alpha, glow_color) = match glow_style {
                GlowStyle::Inner => (
                    (1.0 - (pass as f64 * 0.08 * neon_strength)).max(0.32),
                    (alpha_scale * 0.10 * neon_strength / pass as f64).clamp(0.0, 0.34),
                    vivid_color(gradient_color(color, color2, 0.22)),
                ),
                GlowStyle::Outer => (
                    1.0 + (pass as f64 * 0.34 * neon_strength),
                    (alpha_scale * 0.14 * neon_strength / pass as f64).clamp(0.0, 0.56),
                    vivid_color(gradient_color(color2, color, 0.40)),
                ),
                GlowStyle::SoftBloom => (
                    1.0 + (pass as f64 * 0.52 * neon_strength),
                    (alpha_scale * 0.08 * neon_strength / (pass as f64 * 0.9)).clamp(0.0, 0.28),
                    vivid_color(gradient_color(color2, color, 0.50)),
                ),
                GlowStyle::Neon => (
                    1.0 + (pass as f64 * 0.28 * neon_strength),
                    (alpha_scale * 0.12 * neon_strength / pass as f64).clamp(0.0, 0.5),
                    vivid_color(gradient_color(color2, color, 0.35)),
                ),
            };
            draw_visual_layer(
                ctx,
                width,
                height,
                values,
                mode,
                style,
                glow_color,
                glow_color,
                bar_thickness * widen,
                bar_gap,
                bar_corner_radius * widen,
                segmented_bars,
                segment_length * widen,
                segment_gap,
                bars_wave_thickness * widen,
                bars_dot_radius * widen,
                ring_wave_thickness * widen,
                ring_dot_radius * widen,
                bars_wave_roundness,
                ring_wave_roundness,
                ring_fill_softness,
                ring_fill_overlap_px * widen,
                line_max_height_ratio,
                bars_anchor,
                bars_direction,
                ring_inner_ratio,
                ring_length_ratio,
                polygon_sides,
                glow_alpha,
                false,
                base_light_height,
                base_light_alpha,
                base_light_color,
            );
        }
    }

    draw_visual_layer(
        ctx,
        width,
        height,
        values,
        mode,
        style,
        color,
        color2,
        bar_thickness,
        bar_gap,
        bar_corner_radius,
        segmented_bars,
        segment_length,
        segment_gap,
        bars_wave_thickness,
        bars_dot_radius,
        ring_wave_thickness,
        ring_dot_radius,
        bars_wave_roundness,
        ring_wave_roundness,
        ring_fill_softness,
        ring_fill_overlap_px,
        line_max_height_ratio,
        bars_anchor,
        bars_direction,
        ring_inner_ratio,
        ring_length_ratio,
        polygon_sides,
        alpha_scale,
        base_light_enabled,
        base_light_height,
        base_light_alpha,
        base_light_color,
    );
}

fn stroke_smooth_path(
    ctx: &gtk::cairo::Context,
    points: &[(f64, f64)],
    closed: bool,
    line_width: f64,
    roundness: f64,
) {
    if points.is_empty() {
        return;
    }
    let first = points[0];
    ctx.new_path();
    ctx.move_to(first.0, first.1);

    if points.len() == 1 {
        ctx.set_line_width(line_width);
        let _ = ctx.stroke();
        return;
    }

    let roundness = roundness.clamp(0.05, 1.0);
    for index in 1..points.len() {
        let (prev_x, prev_y) = points[index - 1];
        let (x, y) = points[index];
        let dx = x - prev_x;
        let ctrl_dx = dx * 0.5 * roundness;
        ctx.curve_to(prev_x + ctrl_dx, prev_y, x - ctrl_dx, y, x, y);
    }

    if closed {
        let (last_x, last_y) = points[points.len() - 1];
        let dx = first.0 - last_x;
        let ctrl_dx = dx * 0.5 * roundness;
        ctx.curve_to(last_x + ctrl_dx, last_y, first.0 - ctrl_dx, first.1, first.0, first.1);
        ctx.close_path();
    }

    ctx.set_line_cap(gtk::cairo::LineCap::Round);
    ctx.set_line_join(gtk::cairo::LineJoin::Round);
    ctx.set_line_width(line_width);
    let _ = ctx.stroke();
}

fn append_smooth_path(
    ctx: &gtk::cairo::Context,
    points: &[(f64, f64)],
    closed: bool,
    roundness: f64,
) {
    if points.is_empty() {
        return;
    }
    let first = points[0];
    ctx.new_path();
    ctx.move_to(first.0, first.1);

    if points.len() == 1 {
        if closed {
            ctx.close_path();
        }
        return;
    }

    let roundness = roundness.clamp(0.05, 1.0);
    for index in 1..points.len() {
        let (prev_x, prev_y) = points[index - 1];
        let (x, y) = points[index];
        let dx = x - prev_x;
        let ctrl_dx = dx * 0.5 * roundness;
        ctx.curve_to(prev_x + ctrl_dx, prev_y, x - ctrl_dx, y, x, y);
    }

    if closed {
        let (last_x, last_y) = points[points.len() - 1];
        let dx = first.0 - last_x;
        let ctrl_dx = dx * 0.5 * roundness;
        ctx.curve_to(last_x + ctrl_dx, last_y, first.0 - ctrl_dx, first.1, first.0, first.1);
        ctx.close_path();
    }
}

fn build_ocean_wave_points(
    wave_points: &[(f64, f64)],
    height: f64,
    max_height_ratio: f64,
) -> Vec<(f64, f64)> {
    if wave_points.len() < 2 {
        return wave_points.to_vec();
    }
    let crest_room = (height * max_height_ratio).max(8.0);
    let amplitude = (crest_room * 0.08).clamp(4.0, 26.0);
    let count = wave_points.len();
    let mut out = Vec::with_capacity(count * 2 - 1);
    for (index, point) in wave_points.iter().enumerate() {
        let t = if count <= 1 { 0.0 } else { index as f64 / (count - 1) as f64 };
        let base_phase = t * PI * 2.6;
        let sway = base_phase.sin() * amplitude;
        out.push((point.0, (point.1 + sway).clamp(0.0, height)));
        if let Some(next) = wave_points.get(index + 1).copied() {
            let mid_x = (point.0 + next.0) * 0.5;
            let mid_t = if count <= 1 { t } else { (index as f64 + 0.5) / (count - 1) as f64 };
            let mid_phase = mid_t * PI * 2.6;
            let mid_y = ((point.1 + next.1) * 0.5)
                + (mid_phase.sin() * amplitude * 1.2)
                - (mid_phase.cos() * amplitude * 0.35);
            out.push((mid_x, mid_y.clamp(0.0, height)));
        }
    }
    out
}

fn draw_line_layout(
    ctx: &gtk::cairo::Context,
    width: f64,
    height: f64,
    values: &[f64],
    color: RgbaColor,
    color2: RgbaColor,
    style: RenderStyle,
    bar_thickness: f64,
    gap: f64,
    corner_radius: f64,
    segmented_bars: bool,
    segment_length: f64,
    segment_gap: f64,
    wave_thickness: f64,
    dot_radius: f64,
    wave_roundness: f64,
    max_height_ratio: f64,
    bars_anchor: BarsAnchor,
    bars_direction: BarsDirection,
    alpha_scale: f64,
    base_light_enabled: bool,
    base_light_height: f64,
    base_light_alpha: f64,
    base_light_color: RgbaColor,
) {
    let vertical_layout = matches!(bars_anchor, BarsAnchor::Bottom | BarsAnchor::Top);
    let count = values.len().max(1) as f64;
    let total_nominal = (count * bar_thickness) + ((count - 1.0).max(0.0) * gap);
    let axis_span = if vertical_layout { width } else { height };
    let cross_span = if vertical_layout { height } else { width };
    let scale = if total_nominal > axis_span {
        axis_span / total_nominal
    } else {
        1.0
    };
    let bar_width = (bar_thickness * scale).max(1.0);
    let gap_width = gap * scale;
    let rendered_total = (count * bar_width) + ((count - 1.0).max(0.0) * gap_width);
    let start_axis = (axis_span - rendered_total).max(0.0) * 0.5;
    let max_bar_extent = (cross_span * max_height_ratio).max(2.0);
    let bar_style = BarStyle {
        corner_radius,
        segmented: segmented_bars,
        segment_length,
        segment_gap,
    };
    if base_light_enabled && base_light_alpha > 0.001 {
        let c = vivid_color(base_light_color);
        if vertical_layout {
            let light_h = base_light_height.clamp(2.0, height * 0.35);
            let (y0, y1) = match bars_anchor {
                BarsAnchor::Top => (0.0, light_h),
                _ => ((height - light_h).max(0.0), height),
            };
            let gradient = gtk::cairo::LinearGradient::new(0.0, y1, 0.0, y0);
            gradient.add_color_stop_rgba(0.0, c.r, c.g, c.b, (c.a * alpha_scale * base_light_alpha).clamp(0.0, 1.0));
            gradient.add_color_stop_rgba(0.45, c.r, c.g, c.b, (c.a * alpha_scale * base_light_alpha * 0.42).clamp(0.0, 0.72));
            gradient.add_color_stop_rgba(1.0, c.r, c.g, c.b, 0.0);
            ctx.rectangle(0.0, y0, width, light_h);
            let _ = ctx.set_source(&gradient);
            let _ = ctx.fill();
        } else {
            let light_w = base_light_height.clamp(2.0, width * 0.35);
            let (x0, x1) = match bars_anchor {
                BarsAnchor::Left => (0.0, light_w),
                _ => ((width - light_w).max(0.0), width),
            };
            let gradient = gtk::cairo::LinearGradient::new(x1, 0.0, x0, 0.0);
            gradient.add_color_stop_rgba(0.0, c.r, c.g, c.b, (c.a * alpha_scale * base_light_alpha).clamp(0.0, 1.0));
            gradient.add_color_stop_rgba(0.45, c.r, c.g, c.b, (c.a * alpha_scale * base_light_alpha * 0.42).clamp(0.0, 0.72));
            gradient.add_color_stop_rgba(1.0, c.r, c.g, c.b, 0.0);
            ctx.rectangle(x0, 0.0, light_w, height);
            let _ = ctx.set_source(&gradient);
            let _ = ctx.fill();
        }
    }
    let mut wave_points: Vec<(f64, f64)> = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let normalized = value.clamp(0.0, 1.0);
        let bar_extent = (max_bar_extent * normalized).max(2.0);
        let axis_pos = start_axis + (index as f64 * (bar_width + gap_width));
        let c = vivid_color(gradient_color(color, color2, index as f64 / count));
        let effective_alpha = (c.a * alpha_scale).clamp(0.0, 1.0);
        ctx.set_source_rgba(c.r, c.g, c.b, effective_alpha);
        match style {
            RenderStyle::Dots => {
                let radius = dot_radius.max((bar_width * 0.32).max(2.0));
                let (cx, cy) = if vertical_layout {
                    let tip_y = match bars_direction {
                        BarsDirection::Down => bar_extent - radius,
                        _ => height - bar_extent + radius,
                    };
                    (axis_pos + (bar_width * 0.5), tip_y)
                } else {
                    let tip_x = match bars_direction {
                        BarsDirection::Right => bar_extent - radius,
                        _ => width - bar_extent + radius,
                    };
                    (tip_x, axis_pos + (bar_width * 0.5))
                };
                ctx.arc(cx, cy, radius, 0.0, PI * 2.0);
                let _ = ctx.fill();
            }
            RenderStyle::Waves | RenderStyle::WavesKwy | RenderStyle::WavesOcean | RenderStyle::WavesOceanFill | RenderStyle::WavesFill => {
                if vertical_layout {
                    let tip_y = match bars_direction {
                        BarsDirection::Down => bar_extent,
                        _ => height - bar_extent,
                    };
                    wave_points.push((axis_pos + (bar_width * 0.5), tip_y));
                } else {
                    let tip_x = match bars_direction {
                        BarsDirection::Right => bar_extent,
                        _ => width - bar_extent,
                    };
                    wave_points.push((tip_x, axis_pos + (bar_width * 0.5)));
                }
            }
            _ => {
                let (rect, orientation, forward) = if vertical_layout {
                    let y = match bars_direction {
                        BarsDirection::Down => 0.0,
                        _ => height - bar_extent,
                    };
                    (
                        BarRect { x: axis_pos, y, width: bar_width, height: bar_extent },
                        BarOrientation::Horizontal,
                        bars_direction == BarsDirection::Down,
                    )
                } else {
                    let x = match bars_direction {
                        BarsDirection::Right => 0.0,
                        _ => width - bar_extent,
                    };
                    (
                        BarRect { x, y: axis_pos, width: bar_extent, height: bar_width },
                        BarOrientation::Vertical,
                        bars_direction == BarsDirection::Right,
                    )
                };
                append_bar_path(
                    ctx,
                    rect,
                    BarStyle {
                        corner_radius: bar_style.corner_radius.min(rect.width * 0.5).min(rect.height * 0.5),
                        ..bar_style
                    },
                    orientation,
                    forward,
                );
                let _ = ctx.fill();
            }
        }
    }
    if matches!(style, RenderStyle::Waves | RenderStyle::WavesKwy | RenderStyle::WavesOcean | RenderStyle::WavesOceanFill | RenderStyle::WavesFill) && !wave_points.is_empty() {
        let ocean_points = if vertical_layout && matches!(style, RenderStyle::WavesOcean | RenderStyle::WavesOceanFill) {
            build_ocean_wave_points(&wave_points, height, max_height_ratio)
        } else {
            wave_points.clone()
        };
        if style == RenderStyle::WavesKwy {
            let wave_color = vivid_color(color2);
            ctx.set_source_rgba(
                wave_color.r,
                wave_color.g,
                wave_color.b,
                (wave_color.a * alpha_scale).clamp(0.0, 1.0),
            );
            stroke_smooth_path(ctx, &ocean_points, false, wave_thickness.max((bar_width * 1.05).max(3.5)), wave_roundness.max(0.82));
        } else if style == RenderStyle::WavesOcean {
            let wave_color = vivid_color(color2);
            ctx.set_source_rgba(
                wave_color.r,
                wave_color.g,
                wave_color.b,
                (wave_color.a * alpha_scale * 0.95).clamp(0.0, 1.0),
            );
            stroke_smooth_path(
                ctx,
                &ocean_points,
                false,
                wave_thickness.max((bar_width * 0.96).max(3.1)),
                wave_roundness.max(0.90),
            );
        } else if style == RenderStyle::WavesOceanFill {
            let fill_color = vivid_color(color2);
            append_smooth_path(ctx, &ocean_points, false, wave_roundness.max(0.90));
            if let (Some((first_x, first_y)), Some((last_x, last_y))) = (ocean_points.first().copied(), ocean_points.last().copied()) {
                if vertical_layout {
                    let baseline_y = if bars_direction == BarsDirection::Down { 0.0 } else { height };
                    ctx.line_to(last_x, baseline_y);
                    ctx.line_to(first_x, baseline_y);
                } else {
                    let baseline_x = if bars_direction == BarsDirection::Right { 0.0 } else { width };
                    ctx.line_to(baseline_x, last_y);
                    ctx.line_to(baseline_x, first_y);
                }
                ctx.close_path();
                let gradient = if vertical_layout {
                    gtk::cairo::LinearGradient::new(0.0, height, 0.0, height * 0.12)
                } else {
                    gtk::cairo::LinearGradient::new(width, 0.0, width * 0.12, 0.0)
                };
                gradient.add_color_stop_rgba(
                    0.00,
                    fill_color.r,
                    fill_color.g,
                    fill_color.b,
                    (fill_color.a * alpha_scale * 0.78).clamp(0.0, 0.88),
                );
                gradient.add_color_stop_rgba(
                    0.50,
                    fill_color.r,
                    fill_color.g,
                    fill_color.b,
                    (fill_color.a * alpha_scale * 0.48).clamp(0.0, 0.58),
                );
                gradient.add_color_stop_rgba(
                    1.00,
                    fill_color.r,
                    fill_color.g,
                    fill_color.b,
                    (fill_color.a * alpha_scale * 0.18).clamp(0.0, 0.26),
                );
                let _ = ctx.set_source(&gradient);
                let _ = ctx.fill();
            }
            let stroke_color = vivid_color(color);
            ctx.set_source_rgba(
                stroke_color.r,
                stroke_color.g,
                stroke_color.b,
                (stroke_color.a * alpha_scale).clamp(0.0, 1.0),
            );
            stroke_smooth_path(
                ctx,
                &ocean_points,
                false,
                wave_thickness.max((bar_width * 0.94).max(3.0)),
                wave_roundness.max(0.92),
            );
        } else if style == RenderStyle::WavesFill {
            let fill_color = vivid_color(color2);
            append_smooth_path(ctx, &ocean_points, false, wave_roundness.max(0.78));
            if let (Some((first_x, first_y)), Some((last_x, last_y))) = (ocean_points.first().copied(), ocean_points.last().copied()) {
                if vertical_layout {
                    let baseline_y = if bars_direction == BarsDirection::Down { 0.0 } else { height };
                    ctx.line_to(last_x, baseline_y);
                    ctx.line_to(first_x, baseline_y);
                } else {
                    let baseline_x = if bars_direction == BarsDirection::Right { 0.0 } else { width };
                    ctx.line_to(baseline_x, last_y);
                    ctx.line_to(baseline_x, first_y);
                }
                ctx.close_path();
                let gradient = if vertical_layout {
                    gtk::cairo::LinearGradient::new(0.0, height, 0.0, height * 0.18)
                } else {
                    gtk::cairo::LinearGradient::new(width, 0.0, width * 0.18, 0.0)
                };
                gradient.add_color_stop_rgba(
                    0.00,
                    fill_color.r,
                    fill_color.g,
                    fill_color.b,
                    (fill_color.a * alpha_scale * 0.72).clamp(0.0, 0.82),
                );
                gradient.add_color_stop_rgba(
                    0.55,
                    fill_color.r,
                    fill_color.g,
                    fill_color.b,
                    (fill_color.a * alpha_scale * 0.42).clamp(0.0, 0.52),
                );
                gradient.add_color_stop_rgba(
                    1.00,
                    fill_color.r,
                    fill_color.g,
                    fill_color.b,
                    (fill_color.a * alpha_scale * 0.16).clamp(0.0, 0.24),
                );
                let _ = ctx.set_source(&gradient);
                let _ = ctx.fill();
            }
            let stroke_color = vivid_color(color);
            ctx.set_source_rgba(
                stroke_color.r,
                stroke_color.g,
                stroke_color.b,
                (stroke_color.a * alpha_scale).clamp(0.0, 1.0),
            );
            stroke_smooth_path(
                ctx,
                &ocean_points,
                false,
                wave_thickness.max((bar_width * 0.82).max(2.8)),
                wave_roundness.max(0.78),
            );
        } else {
            let wave_color = vivid_color(color2);
            ctx.set_source_rgba(
                wave_color.r,
                wave_color.g,
                wave_color.b,
                (wave_color.a * alpha_scale).clamp(0.0, 1.0),
            );
            ctx.new_path();
            for (idx, (x, y)) in ocean_points.iter().enumerate() {
                if idx == 0 {
                    ctx.move_to(*x, *y);
                } else {
                    ctx.line_to(*x, *y);
                }
            }
            stroke_smooth_path(ctx, &ocean_points, false, wave_thickness.max((bar_width * 0.75).max(2.5)), wave_roundness);
        }
    }
}

fn draw_radial_layout(
    ctx: &gtk::cairo::Context,
    width: f64,
    height: f64,
    values: &[f64],
    color: RgbaColor,
    color2: RgbaColor,
    style: RenderStyle,
    bar_thickness: f64,
    bar_gap: f64,
    corner_radius: f64,
    segmented_bars: bool,
    segment_length: f64,
    segment_gap: f64,
    wave_thickness: f64,
    dot_radius: f64,
    wave_roundness: f64,
    fill_softness: f64,
    fill_overlap_px: f64,
    inner_ratio: f64,
    length_ratio: f64,
    alpha_scale: f64,
) {
    if matches!(style, RenderStyle::Waves | RenderStyle::WavesKwy | RenderStyle::WavesOcean | RenderStyle::WavesOceanFill | RenderStyle::WavesFill) {
        let cx = width * 0.5;
        let cy = height * 0.5;
        let inner = width.min(height) * inner_ratio;
        let span = PI * 2.0;
        let step = span / values.len().max(1) as f64;
        let mut wave_points: Vec<(f64, f64)> = Vec::with_capacity(values.len());
        let mut inner_points: Vec<(f64, f64)> = Vec::with_capacity(values.len());
        for (index, value) in values.iter().enumerate() {
            let angle = -PI / 2.0 + (index as f64 * step);
            let len = (width.min(height) * length_ratio) * value.clamp(0.0, 1.0);
            wave_points.push((cx + angle.cos() * (inner + len), cy + angle.sin() * (inner + len)));
            let fill_inner = (inner - fill_overlap_px - (length_ratio * width.min(height) * fill_softness * 0.12)).max(8.0);
            inner_points.push((cx + angle.cos() * fill_inner, cy + angle.sin() * fill_inner));
        }
        let wave_color = vivid_color(gradient_color(color, color2, 0.58));
        ctx.set_source_rgba(
            wave_color.r,
            wave_color.g,
            wave_color.b,
            (wave_color.a * alpha_scale).clamp(0.0, 1.0),
        );
        if matches!(style, RenderStyle::WavesFill | RenderStyle::WavesOceanFill) {
            ctx.new_path();
            if let Some((first_x, first_y)) = wave_points.first().copied() {
                ctx.move_to(first_x, first_y);
                for &(x, y) in wave_points.iter().skip(1) {
                    ctx.line_to(x, y);
                }
                for &(x, y) in inner_points.iter().rev() {
                    ctx.line_to(x, y);
                }
                ctx.close_path();
                let _ = ctx.fill();
            }
        } else {
            let thickness = if style == RenderStyle::WavesKwy {
                wave_thickness.max((bar_thickness * 1.05).max(3.5))
            } else if style == RenderStyle::WavesOceanFill {
                wave_thickness.max((bar_thickness * 0.95).max(3.1))
            } else if style == RenderStyle::WavesOcean {
                wave_thickness.max((bar_thickness * 0.95).max(3.1))
            } else {
                wave_thickness.max((bar_thickness * 0.85).max(2.75))
            };
            let roundness = if style == RenderStyle::WavesKwy {
                wave_roundness.max(0.82)
            } else if style == RenderStyle::WavesOceanFill {
                wave_roundness.max(0.90)
            } else if style == RenderStyle::WavesOcean {
                wave_roundness.max(0.90)
            } else {
                wave_roundness
            };
            stroke_smooth_path(ctx, &wave_points, true, thickness, roundness);
        }
        return;
    }
    let cx = width * 0.5;
    let cy = height * 0.5;
    let inner = width.min(height) * inner_ratio;
    let span = PI * 2.0;
    let Some(distribution) = radial_distribution(
        values.len(),
        inner,
        bar_thickness,
        bar_gap.max((bar_thickness * 0.42).max(1.0)),
        -PI / 2.0,
        span,
    ) else {
        return;
    };
    let bar_style = BarStyle {
        corner_radius,
        segmented: segmented_bars,
        segment_length,
        segment_gap,
    };
    for (index, value) in values.iter().enumerate() {
        let angle = distribution.first_angle + (index as f64 * distribution.angle_step);
        let len = (width.min(height) * length_ratio) * value.clamp(0.0, 1.0);
        let c = vivid_color(gradient_color(color, color2, index as f64 / values.len().max(1) as f64));
        ctx.set_source_rgba(c.r, c.g, c.b, (c.a * alpha_scale).clamp(0.0, 1.0));
        match style {
            RenderStyle::Dots => {
                let px = cx + angle.cos() * (inner + len);
                let py = cy + angle.sin() * (inner + len);
                ctx.arc(px, py, dot_radius.max(2.0), 0.0, PI * 2.0);
                let _ = ctx.fill();
            }
            _ => {
                append_directed_bar_path(
                    ctx,
                    cx + angle.cos() * inner,
                    cy + angle.sin() * inner,
                    angle,
                    len,
                    distribution.tangential_thickness,
                    BarStyle {
                        corner_radius: bar_style
                            .corner_radius
                            .min(distribution.tangential_thickness * 0.45),
                        ..bar_style
                    },
                );
                let _ = ctx.fill();
            }
        }
    }
}

fn polygon_points(cx: f64, cy: f64, radius: f64, sides: usize) -> Vec<(f64, f64)> {
    let mut points = Vec::with_capacity(sides);
    for idx in 0..sides {
        let angle = -PI / 2.0 + (idx as f64 * ((PI * 2.0) / sides as f64));
        points.push((cx + angle.cos() * radius, cy + angle.sin() * radius));
    }
    points
}

fn point_distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

fn polygon_point_and_normal(vertices: &[(f64, f64)], distance: f64) -> ((f64, f64), (f64, f64)) {
    let edge_length = point_distance(vertices[0], vertices[1]).max(1.0);
    let edge_index = ((distance / edge_length).floor() as usize) % vertices.len();
    let edge_start = vertices[edge_index];
    let edge_end = vertices[(edge_index + 1) % vertices.len()];
    let along = (distance % edge_length) / edge_length;
    let point = (
        edge_start.0 + ((edge_end.0 - edge_start.0) * along),
        edge_start.1 + ((edge_end.1 - edge_start.1) * along),
    );
    let midpoint = ((edge_start.0 + edge_end.0) * 0.5, (edge_start.1 + edge_end.1) * 0.5);
    let length = (midpoint.0.powi(2) + midpoint.1.powi(2)).sqrt().max(1.0);
    (point, (midpoint.0 / length, midpoint.1 / length))
}

fn draw_polygon_layout(
    ctx: &gtk::cairo::Context,
    width: f64,
    height: f64,
    values: &[f64],
    color: RgbaColor,
    color2: RgbaColor,
    bar_thickness: f64,
    bar_gap: f64,
    corner_radius: f64,
    segmented_bars: bool,
    segment_length: f64,
    segment_gap: f64,
    sides: usize,
    alpha_scale: f64,
) {
    let cx = width * 0.5;
    let cy = height * 0.5;
    let radius = width.min(height) * 0.28;
    let local_points = polygon_points(0.0, 0.0, radius, sides.max(3));
    let edge_length = point_distance(local_points[0], local_points[1]).max(1.0);
    let perimeter = edge_length * local_points.len() as f64;
    let gap_count = if values.len() <= 1 { 0 } else { values.len() } as f64;
    let total_nominal = (values.len() as f64 * bar_thickness.max(1.0)) + (gap_count * bar_gap.max(0.0));
    let scale = if total_nominal > perimeter {
        perimeter / total_nominal
    } else {
        1.0
    };
    let tangential_thickness = (bar_thickness * scale).max(1.0);
    let base_gap = bar_gap.max(0.0) * scale;
    let occupied_length = (values.len() as f64 * tangential_thickness) + (gap_count * base_gap);
    let extra_gap = if gap_count > 0.0 {
        (perimeter - occupied_length).max(0.0) / gap_count
    } else {
        0.0
    };
    let step_distance = tangential_thickness + base_gap + extra_gap;
    let bar_style = BarStyle {
        corner_radius,
        segmented: segmented_bars,
        segment_length,
        segment_gap,
    };

    for (index, value) in values.iter().enumerate() {
        let center_distance = (tangential_thickness * 0.5) + (index as f64 * step_distance);
        let (point, normal) = polygon_point_and_normal(&local_points, center_distance % perimeter);
        let len = 8.0 + value.clamp(0.0, 1.0) * 42.0;
        let c = vivid_color(gradient_color(color, color2, index as f64 / values.len().max(1) as f64));
        ctx.set_source_rgba(c.r, c.g, c.b, (c.a * alpha_scale).clamp(0.0, 1.0));
        append_directed_bar_path(
            ctx,
            cx + point.0,
            cy + point.1,
            normal.1.atan2(normal.0),
            len,
            tangential_thickness,
            BarStyle {
                corner_radius: bar_style.corner_radius.min(tangential_thickness * 0.42),
                ..bar_style
            },
        );
        let _ = ctx.fill();
    }
}

fn draw_visual_layer(
    ctx: &gtk::cairo::Context,
    width: f64,
    height: f64,
    values: &[f64],
    mode: VisualMode,
    style: RenderStyle,
    color: RgbaColor,
    color2: RgbaColor,
    bar_thickness: f64,
    bar_gap: f64,
    bar_corner_radius: f64,
    segmented_bars: bool,
    segment_length: f64,
    segment_gap: f64,
    bars_wave_thickness: f64,
    bars_dot_radius: f64,
    ring_wave_thickness: f64,
    ring_dot_radius: f64,
    bars_wave_roundness: f64,
    ring_wave_roundness: f64,
    ring_fill_softness: f64,
    ring_fill_overlap_px: f64,
    line_max_height_ratio: f64,
    bars_anchor: BarsAnchor,
    bars_direction: BarsDirection,
    ring_inner_ratio: f64,
    ring_length_ratio: f64,
    polygon_sides: usize,
    alpha_scale: f64,
    base_light_enabled: bool,
    base_light_height: f64,
    base_light_alpha: f64,
    base_light_color: RgbaColor,
) {
    match mode {
        VisualMode::Bars => match style {
            RenderStyle::Triangle | RenderStyle::Polygon => {
                draw_polygon_layout(
                    ctx,
                    width,
                    height,
                    values,
                    color,
                    color2,
                    bar_thickness,
                    bar_gap,
                    bar_corner_radius,
                    segmented_bars,
                    segment_length,
                    segment_gap,
                    polygon_sides,
                    alpha_scale,
                )
            }
            _ => draw_line_layout(
                ctx,
                width,
                height,
                values,
                color,
                color2,
                style,
                bar_thickness,
                bar_gap,
                bar_corner_radius,
                segmented_bars,
                segment_length,
                segment_gap,
                bars_wave_thickness,
                bars_dot_radius,
                bars_wave_roundness,
                line_max_height_ratio,
                bars_anchor,
                bars_direction,
                alpha_scale,
                base_light_enabled,
                base_light_height,
                base_light_alpha,
                base_light_color,
            ),
        },
        VisualMode::Ring => match style {
            RenderStyle::Triangle | RenderStyle::Polygon => {
                draw_polygon_layout(
                    ctx,
                    width,
                    height,
                    values,
                    color,
                    color2,
                    bar_thickness,
                    bar_gap,
                    bar_corner_radius,
                    segmented_bars,
                    segment_length,
                    segment_gap,
                    polygon_sides,
                    alpha_scale,
                )
            }
            _ => draw_radial_layout(
                ctx,
                width,
                height,
                values,
                color,
                color2,
                style,
                bar_thickness,
                bar_gap,
                bar_corner_radius,
                segmented_bars,
                segment_length,
                segment_gap,
                ring_wave_thickness,
                ring_dot_radius,
                ring_wave_roundness,
                ring_fill_softness,
                ring_fill_overlap_px,
                ring_inner_ratio,
                ring_length_ratio,
                alpha_scale,
            ),
        },
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = parse_config(&args_config_path());
    let stream = spawn_cava_stream(cfg.bars, cfg.fps)?;
    let app = gtk::Application::builder()
        .application_id("dev.kitotsu.kitsune.overlay")
        .build();
    let cfg_for_activate = cfg.clone();

    app.connect_activate(move |app| {
        install_css();
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Kitsune Overlay")
            .build();

        window.set_widget_name("kitsune-overlay");
        strip_background_classes(&window);
        window.set_decorated(false);
        window.set_resizable(false);
        window.set_focusable(false);
        window.set_default_size(cfg_for_activate.width, cfg_for_activate.height);

        let drawing_area = build_drawing_area(&cfg_for_activate, Arc::clone(&stream));
        window.set_child(Some(&drawing_area));

        let monitor = monitor_by_name(&cfg_for_activate.monitor);
        apply_layer_shell(&window, &cfg_for_activate, monitor.as_ref());

        eprintln!(
            "[overlay] direct gtk4-layer-shell frontend config={} monitor={} layout={:?}",
            cfg_for_activate.config_path.display(),
            cfg_for_activate.monitor,
            cfg_for_activate.layout
        );
        window.present();
    });

    app.run();
    Ok(())
}
