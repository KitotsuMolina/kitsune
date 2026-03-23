use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

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
            "accent_mid" | "mid" | "dynamic" | "wallpaper" => Self::AccentMid,
            _ => Self::Static,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaletteChannel {
    Auto,
    Red,
    Green,
    Blue,
}

impl PaletteChannel {
    fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "r" | "red" => Self::Red,
            "g" | "green" => Self::Green,
            "b" | "blue" => Self::Blue,
            _ => Self::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RgbaColor {
    r: f64,
    g: f64,
    b: f64,
    a: f64,
}

impl RgbaColor {
    fn from_hex_with_alpha(input: &str, alpha: f64) -> Self {
        let clean = input.trim().trim_start_matches('#');
        if clean.len() != 6 || !clean.chars().all(|c| c.is_ascii_hexdigit()) {
            return Self {
                r: 1.0,
                g: 1.0,
                b: 1.0,
                a: alpha,
            };
        }
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(255) as f64 / 255.0;
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255) as f64 / 255.0;
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255) as f64 / 255.0;
        Self {
            r,
            g,
            b,
            a: alpha.clamp(0.0, 1.0),
        }
    }

    fn to_hex(self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8
        )
    }
}

#[derive(Debug, Clone, Copy)]
struct Palette {
    accent_light: RgbaColor,
    accent_mid: RgbaColor,
    accent_dark: RgbaColor,
    candidates: [RgbaColor; 8],
    candidate_count: usize,
    red_candidates: [RgbaColor; 4],
    red_candidate_count: usize,
    green_candidates: [RgbaColor; 4],
    green_candidate_count: usize,
    blue_candidates: [RgbaColor; 4],
    blue_candidate_count: usize,
}

impl Palette {
    fn from_base(base: RgbaColor, alt: RgbaColor) -> Self {
        let accent_light = vivid_color(gradient_color(base, alt, 0.55));
        let accent_mid = vivid_color(gradient_color(base, alt, 0.25));
        let accent_dark = vivid_color(gradient_color(
            base,
            RgbaColor::from_hex_with_alpha("#101820", base.a),
            0.65,
        ));
        Self {
            accent_light,
            accent_mid,
            accent_dark,
            candidates: [
                accent_light,
                accent_mid,
                accent_dark,
                vivid_color(gradient_color(accent_light, accent_mid, 0.5)),
                vivid_color(gradient_color(accent_mid, accent_dark, 0.5)),
                vivid_color(gradient_color(accent_light, accent_dark, 0.5)),
                vivid_color(gradient_color(
                    accent_light,
                    RgbaColor {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: accent_light.a,
                    },
                    0.20,
                )),
                vivid_color(gradient_color(
                    accent_dark,
                    RgbaColor::from_hex_with_alpha("#101820", accent_dark.a),
                    0.22,
                )),
            ],
            candidate_count: 8,
            red_candidates: [accent_mid; 4],
            red_candidate_count: 0,
            green_candidates: [accent_mid; 4],
            green_candidate_count: 0,
            blue_candidates: [accent_mid; 4],
            blue_candidate_count: 0,
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

    fn resolve_with_contrast_guard(
        &self,
        mode: ColorMode,
        fallback: RgbaColor,
        enabled: bool,
        threshold: f64,
    ) -> RgbaColor {
        if mode == ColorMode::Static {
            return fallback;
        }
        let candidate = self.resolve(mode, fallback);
        if !enabled {
            return candidate;
        }

        let bright_wallpaper = self.is_bright_palette(threshold);
        if bright_wallpaper {
            return match mode {
                ColorMode::AccentLight => {
                    if color_luma(self.accent_dark) <= threshold {
                        self.accent_dark
                    } else {
                        darken_color(self.accent_dark, 0.22)
                    }
                }
                ColorMode::AccentMid => {
                    if color_luma(self.accent_dark) <= threshold {
                        darken_color(self.accent_dark, 0.10)
                    } else {
                        darken_color(self.accent_dark, 0.30)
                    }
                }
                ColorMode::AccentDark => darken_color(self.accent_dark, 0.12),
                ColorMode::Static => fallback,
            };
        }

        if color_luma(candidate) <= threshold {
            return candidate;
        }

        match mode {
            ColorMode::AccentLight => {
                if color_luma(self.accent_mid) <= threshold {
                    self.accent_mid
                } else if color_luma(self.accent_dark) <= threshold {
                    self.accent_dark
                } else {
                    darken_color(self.accent_dark, 0.34)
                }
            }
            ColorMode::AccentMid => {
                if color_luma(self.accent_dark) <= threshold {
                    self.accent_dark
                } else {
                    darken_color(self.accent_dark, 0.28)
                }
            }
            ColorMode::AccentDark => darken_color(candidate, 0.24),
            ColorMode::Static => fallback,
        }
    }

    fn is_bright_palette(&self, threshold: f64) -> bool {
        let weighted = (color_luma(self.accent_light) * 0.55)
            + (color_luma(self.accent_mid) * 0.35)
            + (color_luma(self.accent_dark) * 0.10);
        weighted >= (threshold - 0.08)
            || (color_luma(self.accent_light) >= threshold
                && color_luma(self.accent_mid) >= (threshold - 0.12))
    }

    fn resolve_custom_dynamic(
        &self,
        fallback_mode: ColorMode,
        fallback: RgbaColor,
        channel: PaletteChannel,
        target_luma: Option<f64>,
        contrast_guard_enabled: bool,
        contrast_threshold: f64,
    ) -> RgbaColor {
        let base = self.resolve_with_contrast_guard(
            fallback_mode,
            fallback,
            contrast_guard_enabled,
            contrast_threshold,
        );
        if channel == PaletteChannel::Auto && target_luma.is_none() {
            return base;
        }

        let general_candidates: Vec<RgbaColor> = if self.candidate_count > 0 {
            self.candidates[..self.candidate_count].to_vec()
        } else {
            vec![
                self.resolve_with_contrast_guard(
                    ColorMode::AccentLight,
                    self.accent_light,
                    contrast_guard_enabled,
                    contrast_threshold,
                ),
                self.resolve_with_contrast_guard(
                    ColorMode::AccentMid,
                    self.accent_mid,
                    contrast_guard_enabled,
                    contrast_threshold,
                ),
                self.resolve_with_contrast_guard(
                    ColorMode::AccentDark,
                    self.accent_dark,
                    contrast_guard_enabled,
                    contrast_threshold,
                ),
            ]
        };
        let channel_candidates: Vec<RgbaColor> = match channel {
            PaletteChannel::Red if self.red_candidate_count > 0 => {
                self.red_candidates[..self.red_candidate_count].to_vec()
            }
            PaletteChannel::Green if self.green_candidate_count > 0 => {
                self.green_candidates[..self.green_candidate_count].to_vec()
            }
            PaletteChannel::Blue if self.blue_candidate_count > 0 => {
                self.blue_candidates[..self.blue_candidate_count].to_vec()
            }
            _ => Vec::new(),
        };
        let filtered_general: Vec<RgbaColor> = general_candidates
            .iter()
            .copied()
            .filter(|color| color_matches_channel(*color, channel))
            .collect();
        let pool = if !channel_candidates.is_empty() {
            &channel_candidates
        } else if !filtered_general.is_empty() {
            &filtered_general
        } else {
            &general_candidates
        };
        let target_luma = target_luma.unwrap_or(color_luma(base)).clamp(0.0, 1.0);
        pool.iter()
            .copied()
            .min_by(|a, b| {
                let a_score = (color_luma(*a) - target_luma).abs();
                let b_score = (color_luma(*b) - target_luma).abs();
                a_score
                    .partial_cmp(&b_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(base)
    }
}

#[derive(Debug)]
struct LayerColorRequest {
    color_mode: ColorMode,
    static_color: RgbaColor,
    palette_channel: PaletteChannel,
    target_luma: Option<f64>,
    particles_color_mode: ColorMode,
    particles_static_color: RgbaColor,
    base_light_color_mode: ColorMode,
    base_light_static_color: RgbaColor,
}

fn parse_cfg(path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let Ok(raw) = fs::read_to_string(path) else {
        return map;
    };
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = trimmed.split_once('=') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

fn parse_palette_file(path: &Path, fallback: Palette) -> Palette {
    let Ok(raw) = fs::read_to_string(path) else {
        return fallback;
    };
    let mut light = None;
    let mut mid = None;
    let mut dark = None;
    let mut candidates = [fallback.accent_mid; 8];
    let mut candidate_count = 0_usize;
    let mut red_candidates = [fallback.accent_mid; 4];
    let mut red_candidate_count = 0_usize;
    let mut green_candidates = [fallback.accent_mid; 4];
    let mut green_candidate_count = 0_usize;
    let mut blue_candidates = [fallback.accent_mid; 4];
    let mut blue_candidate_count = 0_usize;
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
            k if k.starts_with("candidate_r_") => {
                if red_candidate_count < red_candidates.len() {
                    red_candidates[red_candidate_count] = parsed;
                    red_candidate_count += 1;
                }
            }
            k if k.starts_with("candidate_g_") => {
                if green_candidate_count < green_candidates.len() {
                    green_candidates[green_candidate_count] = parsed;
                    green_candidate_count += 1;
                }
            }
            k if k.starts_with("candidate_b_") => {
                if blue_candidate_count < blue_candidates.len() {
                    blue_candidates[blue_candidate_count] = parsed;
                    blue_candidate_count += 1;
                }
            }
            k if k.starts_with("candidate_") => {
                if candidate_count < candidates.len() {
                    candidates[candidate_count] = parsed;
                    candidate_count += 1;
                }
            }
            _ => {}
        }
    }
    Palette {
        accent_light: light.unwrap_or(fallback.accent_light),
        accent_mid: mid.unwrap_or(fallback.accent_mid),
        accent_dark: dark.unwrap_or(fallback.accent_dark),
        candidates: if candidate_count > 0 { candidates } else { fallback.candidates },
        candidate_count: if candidate_count > 0 { candidate_count } else { fallback.candidate_count },
        red_candidates: if red_candidate_count > 0 { red_candidates } else { fallback.red_candidates },
        red_candidate_count: if red_candidate_count > 0 { red_candidate_count } else { fallback.red_candidate_count },
        green_candidates: if green_candidate_count > 0 { green_candidates } else { fallback.green_candidates },
        green_candidate_count: if green_candidate_count > 0 { green_candidate_count } else { fallback.green_candidate_count },
        blue_candidates: if blue_candidate_count > 0 { blue_candidates } else { fallback.blue_candidates },
        blue_candidate_count: if blue_candidate_count > 0 { blue_candidate_count } else { fallback.blue_candidate_count },
    }
}

fn parse_layer_parts(parts: &[&str]) -> Result<LayerColorRequest, String> {
    if parts.len() < 6 {
        return Err("layer spec missing required fields".to_string());
    }
    let alpha = parts[5].parse::<f64>().unwrap_or(1.0).clamp(0.0, 1.0);
    let mut color_mode = ColorMode::Static;
    let mut particles_color_mode = ColorMode::Static;
    let mut base_light_color_mode = ColorMode::Static;
    let mut palette_channel = PaletteChannel::Auto;
    let mut target_luma = None;
    let mut particles_static_color = RgbaColor::from_hex_with_alpha(parts[4], alpha);
    let mut base_light_static_color = RgbaColor::from_hex_with_alpha(parts[4], alpha);
    if matches!(
        parts[4].to_ascii_lowercase().as_str(),
        "accent_light" | "accent_mid" | "accent_dark" | "static" | "dynamic" | "wallpaper"
    ) {
        color_mode = ColorMode::from_str(parts[4]);
        particles_color_mode = color_mode;
        base_light_color_mode = color_mode;
    }
    for extra in parts.iter().skip(6) {
        if let Some((key, value)) = extra.split_once('=') {
            let key = key.trim();
            if key.eq_ignore_ascii_case("color_mode") {
                color_mode = ColorMode::from_str(value);
                particles_color_mode = color_mode;
                base_light_color_mode = color_mode;
            } else if key.eq_ignore_ascii_case("palette_channel") {
                palette_channel = PaletteChannel::from_str(value);
            } else if key.eq_ignore_ascii_case("target_luma") {
                target_luma = value.parse::<f64>().ok().map(|v| v.clamp(0.0, 1.0));
            } else if key.eq_ignore_ascii_case("particles_color_mode") {
                particles_color_mode = ColorMode::from_str(value);
            } else if key.eq_ignore_ascii_case("particles_color") {
                particles_static_color = RgbaColor::from_hex_with_alpha(value, alpha);
            } else if key.eq_ignore_ascii_case("base_light_color_mode") {
                base_light_color_mode = ColorMode::from_str(value);
            } else if key.eq_ignore_ascii_case("base_light_color") {
                base_light_static_color = RgbaColor::from_hex_with_alpha(value, alpha);
            }
        }
    }
    let static_color = if color_mode == ColorMode::Static {
        RgbaColor::from_hex_with_alpha(parts[4], alpha)
    } else {
        RgbaColor::from_hex_with_alpha("#ffffff", alpha)
    };
    Ok(LayerColorRequest {
        color_mode,
        static_color,
        palette_channel,
        target_luma,
        particles_color_mode,
        particles_static_color,
        base_light_color_mode,
        base_light_static_color,
    })
}

fn parse_layer_from_spec(spec: &str) -> Result<LayerColorRequest, String> {
    let parts: Vec<&str> = spec.split(',').map(|s| s.trim()).collect();
    parse_layer_parts(&parts)
}

fn parse_layer_from_group(path: &Path, index: usize) -> Result<LayerColorRequest, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("failed to read group file {}: {e}", path.display()))?;
    let mut current = 0_usize;
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
        current += 1;
        if current != index {
            continue;
        }
        let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
        return parse_layer_parts(&parts);
    }
    Err(format!("layer index {} not found", index))
}

fn color_luma(color: RgbaColor) -> f64 {
    (0.2126 * color.r) + (0.7152 * color.g) + (0.0722 * color.b)
}

fn color_matches_channel(color: RgbaColor, channel: PaletteChannel) -> bool {
    match channel {
        PaletteChannel::Auto => true,
        PaletteChannel::Red => color.r >= color.g && color.r >= color.b,
        PaletteChannel::Green => color.g >= color.r && color.g >= color.b,
        PaletteChannel::Blue => color.b >= color.r && color.b >= color.g,
    }
}

fn darken_color(color: RgbaColor, amount: f64) -> RgbaColor {
    let factor = (1.0 - amount).clamp(0.0, 1.0);
    RgbaColor {
        r: color.r * factor,
        g: color.g * factor,
        b: color.b * factor,
        a: color.a,
    }
}

fn gradient_color(a: RgbaColor, b: RgbaColor, t: f64) -> RgbaColor {
    let t = t.clamp(0.0, 1.0);
    RgbaColor {
        r: a.r + ((b.r - a.r) * t),
        g: a.g + ((b.g - a.g) * t),
        b: a.b + ((b.b - a.b) * t),
        a: a.a + ((b.a - a.a) * t),
    }
}

fn vivid_color(mut color: RgbaColor) -> RgbaColor {
    let max = color.r.max(color.g).max(color.b);
    let min = color.r.min(color.g).min(color.b);
    let chroma = max - min;
    if chroma < 0.08 {
        let lift = 0.10;
        color.r = (color.r + lift).clamp(0.0, 1.0);
        color.g = (color.g + lift).clamp(0.0, 1.0);
        color.b = (color.b + lift).clamp(0.0, 1.0);
    } else {
        let boost = 1.10;
        let avg = (color.r + color.g + color.b) / 3.0;
        color.r = (avg + (color.r - avg) * boost).clamp(0.0, 1.0);
        color.g = (avg + (color.g - avg) * boost).clamp(0.0, 1.0);
        color.b = (avg + (color.b - avg) * boost).clamp(0.0, 1.0);
    }
    color
}

fn parse_arg_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2)
        .find_map(|pair| if pair[0] == flag { Some(pair[1].clone()) } else { None })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let cfg_path = PathBuf::from(
        parse_arg_value(&args, "--cfg")
            .or_else(|| env::var("KITSUNE_CFG").ok())
            .unwrap_or_else(|| "./config/base.conf".to_string()),
    );
    let layer_index = parse_arg_value(&args, "--layer")
        .ok_or("missing --layer")?
        .parse::<usize>()
        .map_err(|_| "invalid --layer")?;
    let spec_override = parse_arg_value(&args, "--spec");
    let group_file = parse_arg_value(&args, "--group-file");
    let group_path = group_file.as_ref().map(PathBuf::from);
    let cfg = parse_cfg(&cfg_path);
    let palette_path = PathBuf::from(
        parse_arg_value(&args, "--palette-file")
            .or_else(|| cfg.get("color_palette_file").cloned())
            .unwrap_or_else(|| "/tmp/kitsune-accent.palette".to_string()),
    );
    let base_color = RgbaColor::from_hex_with_alpha(
        cfg.get("color").map(String::as_str).unwrap_or("#ff2f8f"),
        0.96,
    );
    let alt_color = RgbaColor::from_hex_with_alpha(
        cfg.get("color2").map(String::as_str).unwrap_or("#19f0ff"),
        0.96,
    );
    let contrast_guard = matches!(
        cfg.get("dynamic_contrast_guard").map(String::as_str).unwrap_or("0"),
        "1" | "true" | "on" | "yes"
    );
    let contrast_threshold = cfg
        .get("dynamic_contrast_threshold")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.72)
        .clamp(0.0, 1.0);
    let fallback = Palette::from_base(base_color, alt_color);
    let palette = parse_palette_file(&palette_path, fallback);
    let layer = if let Some(spec) = spec_override.as_deref() {
        parse_layer_from_spec(spec)?
    } else {
        let group_path = group_path.ok_or("missing --group-file")?;
        parse_layer_from_group(&group_path, layer_index)?
    };

    let layer_color = if layer.color_mode == ColorMode::Static {
        layer.static_color
    } else {
        palette.resolve_custom_dynamic(
            layer.color_mode,
            layer.static_color,
            layer.palette_channel,
            layer.target_luma,
            contrast_guard,
            contrast_threshold,
        )
    };
    let particles_color = if layer.particles_color_mode == ColorMode::Static {
        layer.particles_static_color
    } else {
        palette.resolve_custom_dynamic(
            layer.particles_color_mode,
            layer.particles_static_color,
            layer.palette_channel,
            layer.target_luma,
            contrast_guard,
            contrast_threshold,
        )
    };
    let base_light_color = if layer.base_light_color_mode == ColorMode::Static {
        layer.base_light_static_color
    } else {
        palette.resolve_custom_dynamic(
            layer.base_light_color_mode,
            layer.base_light_static_color,
            layer.palette_channel,
            layer.target_luma.map(|v| (v + 0.12).clamp(0.0, 1.0)),
            contrast_guard,
            contrast_threshold,
        )
    };

    println!(
        "{{\"ok\":true,\"layer_color\":\"{}\",\"particles_color\":\"{}\",\"base_light_color\":\"{}\",\"debug\":{{\"palette_file\":\"{}\",\"channel\":\"{}\",\"target_luma\":{},\"contrast_guard\":{},\"contrast_threshold\":{}}}}}",
        layer_color.to_hex(),
        particles_color.to_hex(),
        base_light_color.to_hex(),
        palette_path.display(),
        match layer.palette_channel {
            PaletteChannel::Auto => "auto",
            PaletteChannel::Red => "r",
            PaletteChannel::Green => "g",
            PaletteChannel::Blue => "b",
        },
        layer.target_luma.map(|v| format!("{v:.4}")).unwrap_or_else(|| "null".to_string()),
        if contrast_guard { "true" } else { "false" },
        format!("{contrast_threshold:.4}")
    );

    Ok(())
}
