use std::path::Path;
use std::{cmp::Ordering, fs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Static,
    AccentLight,
    AccentMid,
    AccentDark,
}

impl ColorMode {
    pub fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "accent_light" | "light" => Self::AccentLight,
            "accent_dark" | "dark" => Self::AccentDark,
            "accent_mid" | "mid" | "dynamic" | "wallpaper" => Self::AccentMid,
            _ => Self::Static,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteChannel {
    Auto,
    Red,
    Green,
    Blue,
}

impl PaletteChannel {
    pub fn from_str(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "r" | "red" => Self::Red,
            "g" | "green" => Self::Green,
            "b" | "blue" => Self::Blue,
            _ => Self::Auto,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RgbaColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl RgbaColor {
    pub fn from_hex_with_alpha(input: &str, alpha: f64) -> Self {
        let clean = input.trim().trim_start_matches('#');
        if clean.len() != 6 || !clean.chars().all(|c| c.is_ascii_hexdigit()) {
            return Self {
                r: 0.96,
                g: 0.97,
                b: 0.98,
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

    pub fn to_hex(self) -> String {
        format!(
            "#{:02X}{:02X}{:02X}",
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub accent_light: RgbaColor,
    pub accent_mid: RgbaColor,
    pub accent_dark: RgbaColor,
    pub candidates: [RgbaColor; 8],
    pub candidate_count: usize,
    pub red_candidates: [RgbaColor; 4],
    pub red_candidate_count: usize,
    pub green_candidates: [RgbaColor; 4],
    pub green_candidate_count: usize,
    pub blue_candidates: [RgbaColor; 4],
    pub blue_candidate_count: usize,
    pub red_dark: RgbaColor,
    pub red_mid: RgbaColor,
    pub red_light: RgbaColor,
    pub has_red_dark: bool,
    pub has_red_mid: bool,
    pub has_red_light: bool,
    pub green_dark: RgbaColor,
    pub green_mid: RgbaColor,
    pub green_light: RgbaColor,
    pub has_green_dark: bool,
    pub has_green_mid: bool,
    pub has_green_light: bool,
    pub blue_dark: RgbaColor,
    pub blue_mid: RgbaColor,
    pub blue_light: RgbaColor,
    pub has_blue_dark: bool,
    pub has_blue_mid: bool,
    pub has_blue_light: bool,
}

impl Palette {
    pub fn from_base(base: RgbaColor, alt: RgbaColor) -> Self {
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
            red_dark: accent_dark,
            red_mid: accent_mid,
            red_light: accent_light,
            has_red_dark: false,
            has_red_mid: false,
            has_red_light: false,
            green_dark: accent_dark,
            green_mid: accent_mid,
            green_light: accent_light,
            has_green_dark: false,
            has_green_mid: false,
            has_green_light: false,
            blue_dark: accent_dark,
            blue_mid: accent_mid,
            blue_light: accent_light,
            has_blue_dark: false,
            has_blue_mid: false,
            has_blue_light: false,
        }
    }

    pub fn resolve(&self, mode: ColorMode, fallback: RgbaColor) -> RgbaColor {
        match mode {
            ColorMode::Static => fallback,
            ColorMode::AccentLight => self.accent_light,
            ColorMode::AccentMid => self.accent_mid,
            ColorMode::AccentDark => self.accent_dark,
        }
    }

    pub fn resolve_with_contrast_guard(
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

    pub fn is_bright_palette(&self, threshold: f64) -> bool {
        let weighted = (color_luma(self.accent_light) * 0.55)
            + (color_luma(self.accent_mid) * 0.35)
            + (color_luma(self.accent_dark) * 0.10);
        weighted >= (threshold - 0.08)
            || (color_luma(self.accent_light) >= threshold
                && color_luma(self.accent_mid) >= (threshold - 0.12))
    }

    pub fn resolve_custom_dynamic(
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
        let target_luma = target_luma.unwrap_or(color_luma(base)).clamp(0.0, 1.0);
        if channel != PaletteChannel::Auto {
            if let Some(color) = self.resolve_banded_channel(channel, target_luma) {
                return color;
            }
        }
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
        pool.iter()
            .copied()
            .min_by(|a, b| {
                let a_score = (color_luma(*a) - target_luma).abs();
                let b_score = (color_luma(*b) - target_luma).abs();
                a_score.partial_cmp(&b_score).unwrap_or(Ordering::Equal)
            })
            .unwrap_or(base)
    }

    fn resolve_banded_channel(&self, channel: PaletteChannel, target_luma: f64) -> Option<RgbaColor> {
        let (dark, has_dark, mid, has_mid, light, has_light) = match channel {
            PaletteChannel::Red => (
                self.red_dark,
                self.has_red_dark,
                self.red_mid,
                self.has_red_mid,
                self.red_light,
                self.has_red_light,
            ),
            PaletteChannel::Green => (
                self.green_dark,
                self.has_green_dark,
                self.green_mid,
                self.has_green_mid,
                self.green_light,
                self.has_green_light,
            ),
            PaletteChannel::Blue => (
                self.blue_dark,
                self.has_blue_dark,
                self.blue_mid,
                self.has_blue_mid,
                self.blue_light,
                self.has_blue_light,
            ),
            PaletteChannel::Auto => return None,
        };

        let mut points: Vec<(f64, RgbaColor)> = Vec::new();
        if has_dark {
            points.push((0.26, dark));
        }
        if has_mid {
            points.push((0.52, mid));
        }
        if has_light {
            points.push((0.78, light));
        }
        if points.is_empty() {
            return None;
        }
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        if points.len() == 1 {
            return Some(points[0].1);
        }
        if target_luma <= points[0].0 {
            return Some(points[0].1);
        }
        if target_luma >= points[points.len() - 1].0 {
            return Some(points[points.len() - 1].1);
        }
        for window in points.windows(2) {
            let (l0, c0) = window[0];
            let (l1, c1) = window[1];
            if target_luma >= l0 && target_luma <= l1 {
                let t = if (l1 - l0).abs() < f64::EPSILON {
                    0.0
                } else {
                    (target_luma - l0) / (l1 - l0)
                };
                return Some(interpolate_color(c0, c1, t));
            }
        }
        Some(points[0].1)
    }
}

pub fn load_palette(path: &Path, fallback: Palette) -> Palette {
    let Ok(raw) = fs::read_to_string(path) else {
        return fallback;
    };
    let mut light = None;
    let mut mid = None;
    let mut dark = None;
    let mut candidates = [fallback.accent_light; 8];
    let mut candidate_count = 0_usize;
    let mut red_candidates = [fallback.accent_mid; 4];
    let mut red_candidate_count = 0_usize;
    let mut green_candidates = [fallback.accent_mid; 4];
    let mut green_candidate_count = 0_usize;
    let mut blue_candidates = [fallback.accent_mid; 4];
    let mut blue_candidate_count = 0_usize;
    let mut red_dark = fallback.red_dark;
    let mut red_mid = fallback.red_mid;
    let mut red_light = fallback.red_light;
    let mut has_red_dark = false;
    let mut has_red_mid = false;
    let mut has_red_light = false;
    let mut green_dark = fallback.green_dark;
    let mut green_mid = fallback.green_mid;
    let mut green_light = fallback.green_light;
    let mut has_green_dark = false;
    let mut has_green_mid = false;
    let mut has_green_light = false;
    let mut blue_dark = fallback.blue_dark;
    let mut blue_mid = fallback.blue_mid;
    let mut blue_light = fallback.blue_light;
    let mut has_blue_dark = false;
    let mut has_blue_mid = false;
    let mut has_blue_light = false;
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
                if k.starts_with("candidate_r_dark_") {
                    red_dark = parsed;
                    has_red_dark = true;
                } else if k.starts_with("candidate_r_mid_") {
                    red_mid = parsed;
                    has_red_mid = true;
                } else if k.starts_with("candidate_r_light_") {
                    red_light = parsed;
                    has_red_light = true;
                }
                if red_candidate_count < red_candidates.len() {
                    red_candidates[red_candidate_count] = parsed;
                    red_candidate_count += 1;
                }
            }
            k if k.starts_with("candidate_g_") => {
                if k.starts_with("candidate_g_dark_") {
                    green_dark = parsed;
                    has_green_dark = true;
                } else if k.starts_with("candidate_g_mid_") {
                    green_mid = parsed;
                    has_green_mid = true;
                } else if k.starts_with("candidate_g_light_") {
                    green_light = parsed;
                    has_green_light = true;
                }
                if green_candidate_count < green_candidates.len() {
                    green_candidates[green_candidate_count] = parsed;
                    green_candidate_count += 1;
                }
            }
            k if k.starts_with("candidate_b_") => {
                if k.starts_with("candidate_b_dark_") {
                    blue_dark = parsed;
                    has_blue_dark = true;
                } else if k.starts_with("candidate_b_mid_") {
                    blue_mid = parsed;
                    has_blue_mid = true;
                } else if k.starts_with("candidate_b_light_") {
                    blue_light = parsed;
                    has_blue_light = true;
                }
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
        candidates: if candidate_count > 0 {
            candidates
        } else {
            fallback.candidates
        },
        candidate_count: if candidate_count > 0 {
            candidate_count
        } else {
            fallback.candidate_count
        },
        red_candidates: if red_candidate_count > 0 {
            red_candidates
        } else {
            fallback.red_candidates
        },
        red_candidate_count: if red_candidate_count > 0 {
            red_candidate_count
        } else {
            fallback.red_candidate_count
        },
        green_candidates: if green_candidate_count > 0 {
            green_candidates
        } else {
            fallback.green_candidates
        },
        green_candidate_count: if green_candidate_count > 0 {
            green_candidate_count
        } else {
            fallback.green_candidate_count
        },
        blue_candidates: if blue_candidate_count > 0 {
            blue_candidates
        } else {
            fallback.blue_candidates
        },
        blue_candidate_count: if blue_candidate_count > 0 {
            blue_candidate_count
        } else {
            fallback.blue_candidate_count
        },
        red_dark,
        red_mid,
        red_light,
        has_red_dark,
        has_red_mid,
        has_red_light,
        green_dark,
        green_mid,
        green_light,
        has_green_dark,
        has_green_mid,
        has_green_light,
        blue_dark,
        blue_mid,
        blue_light,
        has_blue_dark,
        has_blue_mid,
        has_blue_light,
    }
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
        r: (color.r * factor).clamp(0.0, 1.0),
        g: (color.g * factor).clamp(0.0, 1.0),
        b: (color.b * factor).clamp(0.0, 1.0),
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

fn interpolate_color(a: RgbaColor, b: RgbaColor, t: f64) -> RgbaColor {
    let t = t.clamp(0.0, 1.0);
    RgbaColor {
        r: a.r + ((b.r - a.r) * t),
        g: a.g + ((b.g - a.g) * t),
        b: a.b + ((b.b - a.b) * t),
        a: a.a + ((b.a - a.a) * t),
    }
}
