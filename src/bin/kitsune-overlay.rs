use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
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
    WavesFill,
    Dots,
    Triangle,
    Polygon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColorMode {
    Static,
    AccentLight,
    AccentMid,
    AccentDark,
}

impl ColorMode {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "accent_light" | "light" => Self::AccentLight,
            "accent_dark" | "dark" => Self::AccentDark,
            "accent_mid" | "mid" => Self::AccentMid,
            _ => Self::Static,
        }
    }
}

impl RenderStyle {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "bars_fill" | "bars-fill" => Self::BarsFill,
            "waves" | "wave" => Self::Waves,
            "waves_kwy" | "waves-kwy" | "kwy_waves" | "kwy-waves" | "ribbon" => Self::WavesKwy,
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
    bar_width: f64,
    bar_gap: f64,
    bar_corner_radius: f64,
    segmented_bars: bool,
    segment_length: f64,
    segment_gap: f64,
    line_max_height_ratio: f64,
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
}

#[derive(Debug, Clone)]
struct GroupLayer {
    enabled: bool,
    mode: VisualMode,
    style: RenderStyle,
    static_color: RgbaColor,
    color_mode: ColorMode,
    alpha: f64,
}

#[derive(Debug, Clone, Copy)]
struct Palette {
    accent_light: RgbaColor,
    accent_mid: RgbaColor,
    accent_dark: RgbaColor,
}

impl Palette {
    fn from_base(base: RgbaColor, alt: RgbaColor) -> Self {
        Self {
            accent_light: vivid_color(gradient_color(base, alt, 0.55)),
            accent_mid: vivid_color(gradient_color(base, alt, 0.25)),
            accent_dark: vivid_color(gradient_color(
                base,
                RgbaColor::from_hex_with_alpha("#101820", base.a),
                0.65,
            )),
        }
    }

    fn resolve(&self, mode: ColorMode, fallback: RgbaColor) -> RgbaColor {
        match mode {
            ColorMode::Static => fallback,
            ColorMode::AccentLight => self.accent_light,
            ColorMode::AccentMid => self.accent_mid,
            ColorMode::AccentDark => self.accent_dark,
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

#[derive(Debug, Clone, Copy)]
struct RgbaColor {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

impl RgbaColor {
    fn from_hex_with_alpha(raw: &str, alpha: f64) -> Self {
        let value = raw.trim().trim_start_matches('#');
        if value.len() == 6 && value.chars().all(|c| c.is_ascii_hexdigit()) {
            let r = u8::from_str_radix(&value[0..2], 16).unwrap_or(255) as f64 / 255.0;
            let g = u8::from_str_radix(&value[2..4], 16).unwrap_or(255) as f64 / 255.0;
            let b = u8::from_str_radix(&value[4..6], 16).unwrap_or(255) as f64 / 255.0;
            return Self { r, g, b, a: alpha };
        }
        Self {
            r: 0.96,
            g: 0.97,
            b: 0.98,
            a: alpha,
        }
    }
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

fn parse_config(path: &Path) -> Config {
    let map = cfg_map(path).unwrap_or_default();
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
    let group_file = PathBuf::from(cfg_get_string(&map, "group_file", "./config/groups/default.group"));
    let group_poll_ms = map
        .get("group_poll_ms")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(400)
        .max(50);

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
        base_config
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(path)
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
        let alpha = parts[5].parse::<f64>().unwrap_or(1.0).clamp(0.0, 1.0);
        let mut color_mode = ColorMode::Static;
        if matches!(
            parts[4].to_ascii_lowercase().as_str(),
            "accent_light" | "accent_mid" | "accent_dark" | "static"
        ) {
            color_mode = ColorMode::from_str(parts[4]);
        }
        for extra in parts.iter().skip(6) {
            if let Some((key, value)) = extra.split_once('=')
                && key.trim().eq_ignore_ascii_case("color_mode")
            {
                color_mode = ColorMode::from_str(value);
            }
        }
        let static_color = if color_mode == ColorMode::Static {
            RgbaColor::from_hex_with_alpha(parts[4], alpha)
        } else {
            RgbaColor::from_hex_with_alpha("#ffffff", alpha)
        };
        let _profile = parts[3];
        let _resolved_group_path = resolve_group_path(config_path, &group_path.to_string_lossy());
        layers.push(GroupLayer {
            enabled,
            mode,
            style,
            static_color,
            color_mode,
            alpha,
        });
    }
    layers
}

fn load_palette(path: &Path, fallback: Palette) -> Palette {
    let Ok(raw) = fs::read_to_string(path) else {
        return fallback;
    };
    let mut light = None;
    let mut mid = None;
    let mut dark = None;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let parsed = RgbaColor::from_hex_with_alpha(value.trim(), 0.96);
        match key.trim() {
            "accent_light" => light = Some(parsed),
            "accent_mid" => mid = Some(parsed),
            "accent_dark" => dark = Some(parsed),
            _ => {}
        }
    }
    Palette {
        accent_light: light.unwrap_or(fallback.accent_light),
        accent_mid: mid.unwrap_or(fallback.accent_mid),
        accent_dark: dark.unwrap_or(fallback.accent_dark),
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
    let ring_inner_ratio = cfg.ring_inner_ratio;
    let ring_length_ratio = cfg.ring_length_ratio;
    let polygon_sides = cfg.polygon_sides;
    let spectrum_mode = cfg.spectrum_mode;
    let single_mode = cfg.visual_mode;
    let single_bars_style = cfg.bars_style;
    let single_ring_style = cfg.ring_style;
    let single_color_mode = cfg.color_mode;
    let single_color2_mode = cfg.color2_mode;
    let config_path = cfg.config_path.clone();
    let group_path = resolve_group_path(&config_path, &cfg.group_file.to_string_lossy());
    let group_layers = Arc::new(Mutex::new(parse_group_layers(&config_path, &group_path)));
    let group_layers_for_timer = Arc::clone(&group_layers);
    let group_last_mtime = Arc::new(Mutex::new(fs::metadata(&group_path).and_then(|m| m.modified()).ok()));
    let group_last_mtime_for_timer = Arc::clone(&group_last_mtime);
    let group_poll_ms = cfg.group_poll_ms;
    let palette_file = cfg.color_palette_file.clone();
    let palette_for_timer = Arc::clone(&palette);

    drawing_area.set_draw_func(move |_, ctx, width, height| {
        ctx.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        let _ = ctx.paint();

        let values = stream.lock().map(|v| v.clone()).unwrap_or_default();
        if values.is_empty() {
            return;
        }
        let current_palette = palette.lock().map(|v| *v).unwrap_or(default_palette);

        if spectrum_mode == SpectrumMode::Group {
            let layers = group_layers.lock().map(|v| v.clone()).unwrap_or_default();
            for layer in layers.iter().filter(|layer| layer.enabled) {
                let base_color = current_palette.resolve(layer.color_mode, layer.static_color);
                draw_visual_layer(
                    ctx,
                    width as f64,
                    height as f64,
                    &values,
                    layer.mode,
                    layer.style,
                    base_color,
                    gradient_color(base_color, current_palette.accent_light, 0.35),
                    bar_width,
                    bar_gap,
                    bar_corner_radius,
                    segmented_bars,
                    segment_length,
                    segment_gap,
                    line_max_height_ratio,
                    ring_inner_ratio,
                    ring_length_ratio,
                    polygon_sides,
                    (layer.alpha * 1.8).clamp(0.0, 1.0),
                );
            }
            return;
        }

        let single_color = current_palette.resolve(single_color_mode, color);
        let single_color2 = current_palette.resolve(single_color2_mode, color2);
        let style = match single_mode {
            VisualMode::Ring => single_ring_style,
            VisualMode::Bars => single_bars_style,
        };
        draw_visual_layer(
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
            line_max_height_ratio,
            ring_inner_ratio,
            ring_length_ratio,
            polygon_sides,
            1.0,
        );
    });

    let area_weak = drawing_area.downgrade();
    let tick_ms = (1000_u64 / u64::from(cfg.fps)).max(1);
    glib::timeout_add_local(Duration::from_millis(tick_ms), move || {
        if let Some(area) = area_weak.upgrade() {
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
            if let Some(area) = area_weak_reload.upgrade() {
                area.queue_draw();
            }
        }
        let loaded_palette = load_palette(&palette_file, default_palette);
        if let Ok(mut target) = palette_for_timer.lock() {
            *target = loaded_palette;
        }
        glib::ControlFlow::Continue
    });

    drawing_area
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
    alpha_scale: f64,
) {
    let count = values.len().max(1) as f64;
    let total_nominal = (count * bar_thickness) + ((count - 1.0).max(0.0) * gap);
    let scale = if total_nominal > width {
        width / total_nominal
    } else {
        1.0
    };
    let bar_width = (bar_thickness * scale).max(1.0);
    let gap_width = gap * scale;
    let rendered_total = (count * bar_width) + ((count - 1.0).max(0.0) * gap_width);
    let start_x = (width - rendered_total).max(0.0) * 0.5;
    let max_bar_height = (height * max_height_ratio).max(2.0);
    let bar_style = BarStyle {
        corner_radius,
        segmented: segmented_bars,
        segment_length,
        segment_gap,
    };
    let mut wave_points: Vec<(f64, f64)> = Vec::with_capacity(values.len());
    let mut wave_base_y = height;
    for (index, value) in values.iter().enumerate() {
        let normalized = value.clamp(0.0, 1.0);
        let bar_height = (max_bar_height * normalized).max(2.0);
        let x = start_x + (index as f64 * (bar_width + gap_width));
        let y = height - bar_height;
        let c = vivid_color(gradient_color(color, color2, index as f64 / count));
        let effective_alpha = (c.a * alpha_scale).clamp(0.0, 1.0);
        ctx.set_source_rgba(c.r, c.g, c.b, effective_alpha);
        match style {
            RenderStyle::Dots => {
                let radius = dot_radius.max((bar_width * 0.32).max(2.0));
                ctx.arc(x + (bar_width * 0.5), y + radius, radius, 0.0, PI * 2.0);
                let _ = ctx.fill();
            }
            RenderStyle::Waves | RenderStyle::WavesKwy | RenderStyle::WavesFill => {
                wave_base_y = wave_base_y.min(y + (bar_height * 0.46));
                wave_points.push((x + (bar_width * 0.5), y));
            }
            _ => {
                append_bar_path(
                    ctx,
                    BarRect {
                        x,
                        y,
                        width: bar_width,
                        height: bar_height,
                    },
                    BarStyle {
                        corner_radius: bar_style.corner_radius.min(bar_width * 0.5).min(bar_height * 0.5),
                        ..bar_style
                    },
                    BarOrientation::Horizontal,
                    false,
                );
                let _ = ctx.fill();
            }
        }
    }
    if matches!(style, RenderStyle::Waves | RenderStyle::WavesKwy | RenderStyle::WavesFill) && !wave_points.is_empty() {
        let wave_color = vivid_color(color2);
        ctx.set_source_rgba(
            wave_color.r,
            wave_color.g,
            wave_color.b,
            (wave_color.a * alpha_scale).clamp(0.0, 1.0),
        );
        if style == RenderStyle::WavesKwy {
            stroke_smooth_path(ctx, &wave_points, false, wave_thickness.max((bar_width * 1.05).max(3.5)), wave_roundness.max(0.82));
        } else if style == RenderStyle::WavesFill {
            ctx.new_path();
            for (idx, (x, y)) in wave_points.iter().enumerate() {
                if idx == 0 {
                    ctx.move_to(*x, *y);
                } else {
                    ctx.line_to(*x, *y);
                }
            }
            let fill_y = wave_base_y.max(height * 0.62);
            ctx.line_to(width, fill_y);
            ctx.line_to(0.0, fill_y);
            ctx.close_path();
            let _ = ctx.fill();
        } else {
            ctx.new_path();
            for (idx, (x, y)) in wave_points.iter().enumerate() {
                if idx == 0 {
                    ctx.move_to(*x, *y);
                } else {
                    ctx.line_to(*x, *y);
                }
            }
            stroke_smooth_path(ctx, &wave_points, false, wave_thickness.max((bar_width * 0.75).max(2.5)), wave_roundness);
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
    inner_ratio: f64,
    length_ratio: f64,
    alpha_scale: f64,
) {
    if style == RenderStyle::WavesKwy {
        let cx = width * 0.5;
        let cy = height * 0.5;
        let inner = width.min(height) * inner_ratio;
        let span = PI * 2.0;
        let step = span / values.len().max(1) as f64;
        let mut wave_points: Vec<(f64, f64)> = Vec::with_capacity(values.len());
        for (index, value) in values.iter().enumerate() {
            let angle = -PI / 2.0 + (index as f64 * step);
            let len = (width.min(height) * length_ratio) * value.clamp(0.0, 1.0);
            wave_points.push((cx + angle.cos() * (inner + len), cy + angle.sin() * (inner + len)));
        }
        let wave_color = vivid_color(gradient_color(color, color2, 0.58));
        ctx.set_source_rgba(
            wave_color.r,
            wave_color.g,
            wave_color.b,
            (wave_color.a * alpha_scale).clamp(0.0, 1.0),
        );
        stroke_smooth_path(ctx, &wave_points, true, (bar_thickness * 1.08).max(3.5));
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
                ctx.arc(px, py, 4.0, 0.0, PI * 2.0);
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
    line_max_height_ratio: f64,
    ring_inner_ratio: f64,
    ring_length_ratio: f64,
    polygon_sides: usize,
    alpha_scale: f64,
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
                line_max_height_ratio,
                alpha_scale,
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

    app.run_with_args(&["kitsune-overlay"]);
    Ok(())
}
