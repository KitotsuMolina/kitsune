use kitsune::color_resolver::{
    ColorMode, Palette, PaletteChannel, RgbaColor, load_palette, palette_path_for_monitor,
};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct LayerColorRequest {
    color_mode: ColorMode,
    static_color: RgbaColor,
    palette_monitor: Option<String>,
    palette_channel: PaletteChannel,
    target_luma: Option<f64>,
    particles_color_mode: ColorMode,
    particles_static_color: RgbaColor,
    particles_palette_channel: Option<PaletteChannel>,
    particles_target_luma: Option<f64>,
    base_light_color_mode: ColorMode,
    base_light_static_color: RgbaColor,
    base_light_palette_channel: Option<PaletteChannel>,
    base_light_target_luma: Option<f64>,
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

fn parse_layer_parts(parts: &[&str]) -> Result<LayerColorRequest, String> {
    if parts.len() < 6 {
        return Err("layer spec missing required fields".to_string());
    }
    let alpha = parts[5].parse::<f64>().unwrap_or(1.0).clamp(0.0, 1.0);
    let mut color_mode = ColorMode::Static;
    let mut particles_color_mode = ColorMode::Static;
    let mut base_light_color_mode = ColorMode::Static;
    let mut palette_monitor = None;
    let mut palette_channel = PaletteChannel::Auto;
    let mut target_luma = None;
    let mut particles_static_color = RgbaColor::from_hex_with_alpha(parts[4], alpha);
    let mut base_light_static_color = RgbaColor::from_hex_with_alpha(parts[4], alpha);
    let mut particles_palette_channel = None;
    let mut particles_target_luma = None;
    let mut base_light_palette_channel = None;
    let mut base_light_target_luma = None;
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
            } else if key.eq_ignore_ascii_case("palette_monitor") {
                palette_monitor = Some(value.trim().to_string()).filter(|v| !v.is_empty());
            } else if key.eq_ignore_ascii_case("palette_channel") {
                palette_channel = PaletteChannel::from_str(value);
            } else if key.eq_ignore_ascii_case("target_luma") {
                target_luma = value.parse::<f64>().ok().map(|v| v.clamp(0.0, 1.0));
            } else if key.eq_ignore_ascii_case("particles_color_mode") {
                particles_color_mode = ColorMode::from_str(value);
            } else if key.eq_ignore_ascii_case("particles_color") {
                particles_static_color = RgbaColor::from_hex_with_alpha(value, alpha);
            } else if key.eq_ignore_ascii_case("particles_palette_channel") {
                particles_palette_channel = Some(PaletteChannel::from_str(value));
            } else if key.eq_ignore_ascii_case("particles_target_luma") {
                particles_target_luma = value.parse::<f64>().ok().map(|v| v.clamp(0.0, 1.0));
            } else if key.eq_ignore_ascii_case("base_light_color_mode") {
                base_light_color_mode = ColorMode::from_str(value);
            } else if key.eq_ignore_ascii_case("base_light_color") {
                base_light_static_color = RgbaColor::from_hex_with_alpha(value, alpha);
            } else if key.eq_ignore_ascii_case("base_light_palette_channel") {
                base_light_palette_channel = Some(PaletteChannel::from_str(value));
            } else if key.eq_ignore_ascii_case("base_light_target_luma") {
                base_light_target_luma = value.parse::<f64>().ok().map(|v| v.clamp(0.0, 1.0));
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
        palette_monitor,
        palette_channel,
        target_luma,
        particles_color_mode,
        particles_static_color,
        particles_palette_channel,
        particles_target_luma,
        base_light_color_mode,
        base_light_static_color,
        base_light_palette_channel,
        base_light_target_luma,
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
    let base_palette_path = PathBuf::from(
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
    let layer = if let Some(spec) = spec_override.as_deref() {
        parse_layer_from_spec(spec)?
    } else {
        let group_path = group_path.ok_or("missing --group-file")?;
        parse_layer_from_group(&group_path, layer_index)?
    };
    let palette_path = palette_path_for_monitor(&base_palette_path, layer.palette_monitor.as_deref());
    let palette = load_palette(&palette_path, fallback);

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
            layer.particles_palette_channel.unwrap_or(layer.palette_channel),
            layer.particles_target_luma.or(layer.target_luma),
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
            layer.base_light_palette_channel.unwrap_or(layer.palette_channel),
            layer
                .base_light_target_luma
                .or_else(|| layer.target_luma.map(|v| (v + 0.12).clamp(0.0, 1.0))),
            contrast_guard,
            contrast_threshold,
        )
    };

    println!(
        "{{\"ok\":true,\"layer_color\":\"{}\",\"particles_color\":\"{}\",\"base_light_color\":\"{}\",\"debug\":{{\"palette_file\":\"{}\",\"palette_monitor\":{},\"channel\":\"{}\",\"target_luma\":{},\"contrast_guard\":{},\"contrast_threshold\":{}}}}}",
        layer_color.to_hex(),
        particles_color.to_hex(),
        base_light_color.to_hex(),
        palette_path.display(),
        layer
            .palette_monitor
            .as_ref()
            .map(|value| format!("\"{}\"", value.replace('\"', "\\\"")))
            .unwrap_or_else(|| "null".to_string()),
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
