use std::collections::{HashMap, VecDeque};
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use wgpu::util::DeviceExt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Bars,
    Ring,
}

impl Mode {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "ring" => Mode::Ring,
            _ => Mode::Bars,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeMode {
    Standard,
    Test,
}

impl RuntimeMode {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "test" => RuntimeMode::Test,
            _ => RuntimeMode::Standard,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenderBackend {
    Cpu,
    Gpu,
}

impl RenderBackend {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "gpu" => RenderBackend::Gpu,
            _ => RenderBackend::Cpu,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpectrumMode {
    Single,
    Group,
}

impl SpectrumMode {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "group" => SpectrumMode::Group,
            _ => SpectrumMode::Single,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PostFxScope {
    Final,
    Layer,
    Mixed,
}

impl PostFxScope {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "layer" => PostFxScope::Layer,
            "mixed" => PostFxScope::Mixed,
            _ => PostFxScope::Final,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParticleLayer {
    Front,
    Back,
}

impl ParticleLayer {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "back" | "behind" => ParticleLayer::Back,
            _ => ParticleLayer::Front,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParticleColorMode {
    Static,
    Spectrum,
}

impl ParticleColorMode {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "spectrum" | "dynamic" => ParticleColorMode::Spectrum,
            _ => ParticleColorMode::Static,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenderStyle {
    Bars,
    BarsFill,
    Waves,
    WavesFill,
    Dots,
}

impl RenderStyle {
    fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "bars_fill" | "barfill" | "bars-fill" => RenderStyle::BarsFill,
            "wave" | "waves" => RenderStyle::Waves,
            "waves_fill" | "wavefill" | "waves-fill" | "filled" => RenderStyle::WavesFill,
            "dot" | "dots" => RenderStyle::Dots,
            _ => RenderStyle::Bars,
        }
    }
}

#[derive(Clone, Copy)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

fn parse_hex_color(s: &str) -> Color {
    let h = s.trim().trim_start_matches('#');
    if h.len() == 6 && h.chars().all(|c| c.is_ascii_hexdigit()) {
        let r = u8::from_str_radix(&h[0..2], 16).unwrap_or(0xA6);
        let g = u8::from_str_radix(&h[2..4], 16).unwrap_or(0x0C);
        let b = u8::from_str_radix(&h[4..6], 16).unwrap_or(0xDB);
        return Color { r, g, b, a: 255 };
    }
    Color {
        r: 0xA6,
        g: 0x0C,
        b: 0xDB,
        a: 255,
    }
}

fn color_from_file(path: &Path) -> io::Result<Color> {
    let f = File::open(path)?;
    let mut r = BufReader::new(f);
    let mut line = String::new();
    let _ = r.read_line(&mut line)?;
    Ok(parse_hex_color(line.trim()))
}

fn parse_boolish(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "on" | "yes" => Some(true),
        "0" | "false" | "off" | "no" => Some(false),
        _ => None,
    }
}

#[derive(Clone)]
struct AppConfig {
    backend: RenderBackend,
    spectrum_mode: SpectrumMode,
    group_file: PathBuf,
    group_poll_ms: u64,
    mode: Mode,
    runtime_mode: RuntimeMode,
    monitor: String,
    width: usize,
    height: usize,
    fps: u32,
    bars: usize,
    fifo_video: String,
    fifo_cava: String,
    color: Color,
    dynamic_color: bool,
    color_source_file: PathBuf,
    color_poll_seconds: u64,
    color_smooth: f32,
    color_instant_apply: bool,
    ring_auto_hide: bool,
    ring_show_threshold: f32,
    ring_hide_threshold: f32,
    ring_fade_in_sec: f32,
    ring_fade_out_sec: f32,
    rotate_profiles: bool,
    rotation_seconds: u64,
    static_profile: String,
    test_profile_file: PathBuf,
    test_profile_poll_ms: u64,
    profile_dir: PathBuf,
    bars_profiles: Vec<String>,
    ring_profiles: Vec<String>,
    bars_style: RenderStyle,
    ring_style: RenderStyle,
    bars_wave_thickness: i32,
    bars_dot_radius: i32,
    ring_wave_thickness: i32,
    ring_dot_radius: i32,
    bars_wave_roundness: f32,
    ring_wave_roundness: f32,
    ring_fill_softness: f32,
    ring_fill_overlap_px: f32,
    postfx_scope: PostFxScope,
    postfx_enabled: bool,
    postfx_blur_passes: usize,
    postfx_blur_mix: f32,
    postfx_glow_strength: f32,
    postfx_glow_mix: f32,
    postfx_skip_plain_bars: bool,
    particles_enabled: bool,
    particles_max: usize,
    particles_spawn_rate: f32,
    particles_life_min: f32,
    particles_life_max: f32,
    particles_speed_min: f32,
    particles_speed_max: f32,
    particles_size_min: i32,
    particles_size_max: i32,
    particles_size_scale: f32,
    particles_alpha: f32,
    particles_drift: f32,
    particles_fade_jitter: f32,
    particles_layer: ParticleLayer,
    particles_color: Color,
    particles_color_mode: ParticleColorMode,
}

#[derive(Clone, Copy)]
struct PostFxParams {
    enabled: bool,
    blur_passes: usize,
    blur_mix: f32,
    glow_strength: f32,
    glow_mix: f32,
}

#[derive(Clone)]
struct SpectrumLayer {
    enabled: bool,
    mode: Mode,
    style: RenderStyle,
    profile: Profile,
    color: Color,
    alpha: f32,
    runtime_mode: RuntimeMode,
    rotate_profiles: bool,
    profiles: Vec<String>,
    profile_index: usize,
    profile_last_switch: Instant,
    test_profile_file: Option<PathBuf>,
    test_profile_last_mtime: Option<SystemTime>,
    test_profile_last_check: Instant,
    postfx: Option<PostFxParams>,
}

#[derive(Clone)]
struct Profile {
    gain: f32,
    gamma: f32,
    curve_drive: f32,
    attack: f32,
    gravity_step: f32,
    avg_frames: usize,
    smooth_radius: usize,
    bass_boost: f32,
    bass_power: f32,
    low_band_gain: f32,
    mid_band_gain: f32,
    high_band_gain: f32,
    silence_timeout_ms: u64,
    height_scale: f32,
    dune_amount: f32,
    dune_cycles: f32,
    edge_falloff_pow: f32,
    dune_floor: f32,
    dune_softness: f32,
    twin_amount: f32,
    twin_separation: f32,
    twin_width: f32,
    center_dip: f32,
    loud_floor: f32,
    loud_floor_curve: f32,
    center_jump_amount: f32,
    center_jump_sharpness: f32,
    center_jump_threshold: f32,
    center_jump_decay: f32,
    bar_gap: usize,
    side_padding: usize,
    bottom_padding: usize,
    min_bar_height_px: usize,
    ring_x: i32,
    ring_y: i32,
    ring_radius: i32,
    ring_thickness: i32,
    ring_base_thickness: i32,
    ring_bar_thickness: i32,
    ring_min_bar: f32,
    ring_max_bar: f32,
}

#[derive(Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    age: f32,
    size: i32,
    alpha: f32,
    fade_start: f32,
    fade_power: f32,
    flicker_amount: f32,
    flicker_speed: f32,
    flicker_phase: f32,
}

impl Profile {
    fn defaults(cfg: &AppConfig) -> Self {
        Self {
            gain: 2.1,
            gamma: 0.7,
            curve_drive: 0.95,
            attack: 0.74,
            gravity_step: 2.9,
            avg_frames: 4,
            smooth_radius: 1,
            bass_boost: 0.22,
            bass_power: 2.1,
            low_band_gain: 1.0,
            mid_band_gain: 1.0,
            high_band_gain: 1.0,
            silence_timeout_ms: 260,
            height_scale: 0.52,
            dune_amount: 0.45,
            dune_cycles: 1.0,
            edge_falloff_pow: 1.65,
            dune_floor: 0.10,
            dune_softness: 0.95,
            twin_amount: 0.88,
            twin_separation: 0.21,
            twin_width: 0.11,
            center_dip: 0.45,
            loud_floor: 0.22,
            loud_floor_curve: 1.18,
            center_jump_amount: 0.55,
            center_jump_sharpness: 8.0,
            center_jump_threshold: 0.018,
            center_jump_decay: 0.86,
            bar_gap: 1,
            side_padding: 0,
            bottom_padding: 8,
            min_bar_height_px: 0,
            ring_x: (cfg.width / 2) as i32,
            ring_y: (cfg.height / 2) as i32,
            ring_radius: (cfg.width.min(cfg.height) as f32 * 0.16) as i32,
            ring_thickness: 2,
            ring_base_thickness: 20,
            ring_bar_thickness: 3,
            ring_min_bar: 2.0,
            ring_max_bar: 175.0,
        }
    }
}

const MAX_BAND_GAIN: f32 = 2.5;
const MAX_BASS_BOOST: f32 = 1.5;
const MAX_BASS_POWER: f32 = 4.0;

#[derive(Clone)]
struct AudioState {
    latest_bins: Vec<f32>,
    counter: u64,
    last_update: Instant,
}

const MAX_GPU_BARS: usize = 1024;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct GpuParams {
    data: [f32; 64],
}

struct GpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    params_buf: wgpu::Buffer,
    heights_buf: wgpu::Buffer,
    texture: wgpu::Texture,
    output_buf: wgpu::Buffer,
    padded_bpr: u32,
    width: u32,
    height: u32,
}

impl GpuRenderer {
    fn new(width: usize, height: usize, bars: usize) -> io::Result<Self> {
        if bars > MAX_GPU_BARS {
            return Err(io::Error::other(format!(
                "bars {} exceeds GPU max {}",
                bars, MAX_GPU_BARS
            )));
        }

        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .ok_or_else(|| io::Error::other("wgpu adapter not found"))?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("kitsune-gpu-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
        }, None))
        .map_err(|e| io::Error::other(format!("wgpu device: {e}")))?;

        let width = width as u32;
        let height = height as u32;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kitsune-gpu-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let params = GpuParams { data: [0.0; 64] };
        let params_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("kitsune-gpu-params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let heights_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kitsune-gpu-heights"),
            size: (MAX_GPU_BARS * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("kitsune-gpu-shader"),
            source: wgpu::ShaderSource::Wgsl(
                r#"
struct Params {
  data: array<f32, 64>,
};
@group(0) @binding(0) var<storage, read> params: Params;
@group(0) @binding(1) var<storage, read> heights: array<f32>;

const PI: f32 = 3.14159265359;
const TAU: f32 = 6.28318530718;

fn clamp01(v: f32) -> f32 {
  return clamp(v, 0.0, 1.0);
}

fn wrap_angle_diff(a: f32, b: f32) -> f32 {
  var d = abs(a - b);
  if (d > PI) {
    d = TAU - d;
  }
  return d;
}

fn h_at(i: i32, bars: i32) -> f32 {
  let idx = clamp(i, 0, bars - 1);
  return heights[u32(idx)];
}

fn h_angle(phase01: f32, bars: i32) -> f32 {
  let f = phase01 * f32(bars);
  let i0 = i32(floor(f)) % bars;
  let i1 = (i0 + 1) % bars;
  let t = fract(f);
  return mix(h_at(i0, bars), h_at(i1, bars), t);
}

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
  var pos = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -3.0),
    vec2<f32>(3.0, 1.0),
    vec2<f32>(-1.0, 1.0)
  );
  let p = pos[vid];
  return vec4<f32>(p, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
  let width = params.data[0];
  let height = params.data[1];
  let bars = i32(params.data[2]);
  let mode = i32(params.data[3]); // 0 bars, 1 ring
  let style = i32(params.data[4]); // 0 bars(line),1 waves,2 waves_fill,3 dots,4 bars_fill

  let color = vec3<f32>(params.data[5], params.data[6], params.data[7]);
  let alpha_global = params.data[8];

  let bottom_padding = params.data[9];
  let height_scale = params.data[10];
  let side_padding = params.data[11];
  let bar_gap = params.data[12];
  let min_bar_h = params.data[13];
  let bars_wave_thickness = params.data[14];
  let bars_dot_r = params.data[15];

  let ring_x = params.data[16];
  let ring_y = params.data[17];
  let ring_radius = params.data[18];
  let ring_base_thickness = params.data[19];
  let ring_bar_thickness = params.data[20];
  let ring_min_bar = params.data[21];
  let ring_max_bar = params.data[22];
  let ring_thickness = params.data[23];
  let ring_wave_thickness = params.data[24];
  let ring_dot_r = params.data[25];
  let ring_fill_softness = params.data[26];
  let ring_fill_overlap_px = params.data[27];

  let x = frag.x;
  let y = frag.y;

  var a = 0.0;

  if (mode == 0) {
    let y_base = height - bottom_padding;
    let usable_h = max(1.0, (height - bottom_padding) * height_scale);
    let usable_w = max(1.0, width - side_padding * 2.0);
    let gap = bar_gap;
    let bar_w = max(1.0, (usable_w - gap * f32(max(0, bars - 1))) / f32(max(1, bars)));
    let lane = bar_w + gap;
    let t = (x - side_padding) / lane;
    let i0 = i32(floor(t));
    if (i0 >= 0 && i0 < bars) {
      let local_x = (x - side_padding) - f32(i0) * lane;
      if (local_x >= 0.0 && local_x <= bar_w + 0.5) {
        let h = max(min_bar_h, h_at(i0, bars) * usable_h);
        let y_top = y_base - h;

        if (style == 0) {
          let line_t = max(1.0, min(bar_w, bars_wave_thickness));
          let cx = bar_w * 0.5;
          if (abs(local_x - cx) <= line_t * 0.5 && y >= y_top && y <= y_base) {
            a = 1.0;
          }
        } else if (style == 4) {
          if (y >= y_top && y <= y_base) {
            a = 1.0;
          }
        } else if (style == 3) {
          let step = max(2.0, bars_dot_r * 2.0 + 1.0);
          var yy = y_base - bars_dot_r;
          loop {
            let dx = (x - (side_padding + (f32(i0) + 0.5) * lane - gap * 0.5));
            let dy = y - yy;
            if (dx * dx + dy * dy <= bars_dot_r * bars_dot_r) {
              a = 1.0;
              break;
            }
            yy = yy - step;
            if (yy < y_top) { break; }
          }
        } else {
          let i1 = min(i0 + 1, bars - 1);
          let h0 = max(min_bar_h, h_at(i0, bars) * usable_h);
          let h1 = max(min_bar_h, h_at(i1, bars) * usable_h);
          let x0 = side_padding + (f32(i0) + 0.5) * lane - gap * 0.5;
          let x1 = side_padding + (f32(i1) + 0.5) * lane - gap * 0.5;
          let tt = clamp((x - x0) / max(1.0, (x1 - x0)), 0.0, 1.0);
          let y_curve = y_base - mix(h0, h1, tt);
          if (style == 1) {
            if (abs(y - y_curve) <= bars_wave_thickness * 0.5) {
              a = 1.0;
            }
          } else if (style == 2) {
            if (y >= y_curve && y <= y_base) {
              a = 1.0;
            }
          }
        }
      }
    }
  } else {
    let dx = x - ring_x;
    let dy = y - ring_y;
    let r = sqrt(dx * dx + dy * dy);
    let base_inner = max(1.0, ring_radius - ring_base_thickness);
    let base_outer = max(base_inner + 1.0, ring_radius + ring_base_thickness);
    let ang = atan2(dy, dx);
    var phase01 = (ang + PI * 0.5) / TAU;
    if (phase01 < 0.0) { phase01 = phase01 + 1.0; }
    if (phase01 >= 1.0) { phase01 = phase01 - 1.0; }
    let h = h_angle(phase01, bars);
    let len = ring_min_bar + h * max(1.0, ring_max_bar * height_scale);
    let r_target = base_outer + len;

    if (r >= base_inner && r <= base_outer) {
      a = 1.0;
    }

    if (style == 0) {
      let f = phase01 * f32(bars);
      let nearest = i32(round(f)) % bars;
      let bin_ang = (f32(nearest) / f32(bars)) * TAU - PI * 0.5;
      let ad = wrap_angle_diff(ang, bin_ang);
      let arc_d = ad * max(1.0, r);
      let hn = h_at(nearest, bars);
      let len_n = ring_min_bar + hn * max(1.0, ring_max_bar * height_scale);
      if (r >= base_outer && r <= base_outer + len_n && arc_d <= ring_bar_thickness * 0.5) {
        a = 1.0;
      }
    } else if (style == 1) {
      if (abs(r - r_target) <= ring_wave_thickness * 0.5) {
        a = 1.0;
      }
    } else if (style == 2) {
      let r0 = base_outer - ring_fill_overlap_px;
      if (r >= r0 && r <= r_target) {
        var aa = 1.0;
        let soft_px = ring_fill_softness * 8.0;
        if (soft_px > 0.01) {
          let in_t = clamp((r - r0) / soft_px, 0.0, 1.0);
          let out_t = clamp((r_target - r) / soft_px, 0.0, 1.0);
          aa = min(in_t, out_t);
        }
        a = max(a, aa);
      }
      if (abs(r - r_target) <= ring_wave_thickness * 0.5) {
        a = 1.0;
      }
    } else if (style == 3) {
      if (abs(r - r_target) <= ring_dot_r) {
        a = 1.0;
      }
    }

    if (abs(r - base_outer) <= ring_thickness * 0.5) {
      a = 1.0;
    }
  }

  if (a <= 0.001) {
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
  }

  return vec4<f32>(color, clamp01(a * alpha_global));
}
"#
                    .into(),
            ),
        });

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kitsune-gpu-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("kitsune-gpu-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: heights_buf.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kitsune-gpu-pipeline-layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kitsune-gpu-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let bpr = width * 4;
        let padded_bpr = bpr.div_ceil(256) * 256;
        let output_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kitsune-gpu-output"),
            size: padded_bpr as u64 * height as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        // Keep texture view alive by attaching once in pipeline creation path.
        let _ = texture_view;

        Ok(Self {
            device,
            queue,
            pipeline,
            bind_group,
            params_buf,
            heights_buf,
            texture,
            output_buf,
            padded_bpr,
            width,
            height,
        })
    }

    fn render(
        &mut self,
        cfg: &AppConfig,
        profile: &Profile,
        heights: &[f32],
        color: Color,
        alpha: f32,
        out_rgba: &mut [u8],
    ) -> io::Result<()> {
        let mut params = GpuParams { data: [0.0; 64] };
        params.data[0] = cfg.width as f32;
        params.data[1] = cfg.height as f32;
        params.data[2] = cfg.bars as f32;
        params.data[3] = if cfg.mode == Mode::Ring { 1.0 } else { 0.0 };
        let style = if cfg.mode == Mode::Ring {
            cfg.ring_style
        } else {
            cfg.bars_style
        };
        params.data[4] = match style {
            RenderStyle::Bars => 0.0,
            RenderStyle::Waves => 1.0,
            RenderStyle::WavesFill => 2.0,
            RenderStyle::Dots => 3.0,
            RenderStyle::BarsFill => 4.0,
        };
        params.data[5] = color.r as f32 / 255.0;
        params.data[6] = color.g as f32 / 255.0;
        params.data[7] = color.b as f32 / 255.0;
        params.data[8] = alpha.clamp(0.0, 1.0);

        params.data[9] = profile.bottom_padding as f32;
        params.data[10] = profile.height_scale;
        params.data[11] = profile.side_padding as f32;
        params.data[12] = profile.bar_gap as f32;
        params.data[13] = profile.min_bar_height_px as f32;
        params.data[14] = cfg.bars_wave_thickness as f32;
        params.data[15] = cfg.bars_dot_radius as f32;

        params.data[16] = profile.ring_x as f32;
        params.data[17] = profile.ring_y as f32;
        params.data[18] = profile.ring_radius as f32;
        params.data[19] = profile.ring_base_thickness as f32;
        params.data[20] = profile.ring_bar_thickness as f32;
        params.data[21] = profile.ring_min_bar;
        params.data[22] = profile.ring_max_bar;
        params.data[23] = profile.ring_thickness as f32;
        params.data[24] = cfg.ring_wave_thickness as f32;
        params.data[25] = cfg.ring_dot_radius as f32;
        params.data[26] = cfg.ring_fill_softness;
        params.data[27] = cfg.ring_fill_overlap_px;

        self.queue
            .write_buffer(&self.params_buf, 0, bytemuck::bytes_of(&params));

        let mut hb = vec![0.0f32; MAX_GPU_BARS];
        let copy_n = heights.len().min(MAX_GPU_BARS);
        hb[..copy_n].copy_from_slice(&heights[..copy_n]);
        self.queue
            .write_buffer(&self.heights_buf, 0, bytemuck::cast_slice(&hb));

        let view = self.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("kitsune-gpu-encoder"),
            });

        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("kitsune-gpu-rp"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, &self.bind_group, &[]);
            rp.draw(0..3, 0..1);
        }

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &self.output_buf,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.padded_bpr),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let slice = self.output_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        let _ = self.device.poll(wgpu::Maintain::Wait);
        match rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(io::Error::other(format!("gpu map failed: {e}"))),
            Err(e) => return Err(io::Error::other(format!("gpu map channel failed: {e}"))),
        }

        let data = slice.get_mapped_range();
        let row_bytes = self.width as usize * 4;
        for y in 0..self.height as usize {
            let src_off = y * self.padded_bpr as usize;
            let dst_off = y * row_bytes;
            out_rgba[dst_off..(dst_off + row_bytes)]
                .copy_from_slice(&data[src_off..(src_off + row_bytes)]);
        }
        drop(data);
        self.output_buf.unmap();

        Ok(())
    }
}

struct Engine {
    cfg: AppConfig,
    backend: RenderBackend,
    gpu_renderer: Option<GpuRenderer>,
    spectrum_mode: SpectrumMode,
    group_layers: Vec<SpectrumLayer>,
    group_poll_ms: u64,
    group_last_check: Instant,
    group_last_mtime: Option<SystemTime>,
    profile: Profile,
    frame: Vec<u8>,
    composite_frame: Vec<u8>,
    postfx_a: Vec<u8>,
    postfx_b: Vec<u8>,
    particles: Vec<Particle>,
    particle_emit_carry: f32,
    rng_state: u64,
    heights: Vec<f32>,
    history: VecDeque<Vec<f32>>,
    prev_global_energy: f32,
    center_jump_state: f32,
    profile_names: Vec<String>,
    profile_index: usize,
    profile_last_switch: Instant,
    draw_color: Color,
    target_color: Color,
    color_last_poll: Instant,
    ring_visibility: f32,
    draw_alpha_scale: f32,
    runtime_mode: RuntimeMode,
    rotate_profiles: bool,
    test_profile_file: PathBuf,
    test_profile_poll_ms: u64,
    test_profile_last_check: Instant,
    test_profile_last_mtime: Option<SystemTime>,
}

fn clamp01(v: f32) -> f32 {
    if v <= 0.0 {
        0.0
    } else if v >= 1.0 {
        1.0
    } else {
        v
    }
}

fn parse_key_value_file(path: &Path) -> io::Result<HashMap<String, String>> {
    let f = File::open(path)?;
    let r = BufReader::new(f);
    let mut out = HashMap::new();
    for line in r.lines() {
        let l = line?;
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = s.split_once('=') {
            out.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    Ok(out)
}

fn get_string(map: &HashMap<String, String>, key: &str, default: &str) -> String {
    map.get(key).cloned().unwrap_or_else(|| default.to_string())
}

fn get_num<T: std::str::FromStr + Copy>(map: &HashMap<String, String>, key: &str, default: T) -> T {
    map.get(key)
        .and_then(|v| v.parse::<T>().ok())
        .unwrap_or(default)
}

fn get_list(map: &HashMap<String, String>, key: &str, default: &[&str]) -> Vec<String> {
    if let Some(v) = map.get(key) {
        let values: Vec<String> = v
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        if !values.is_empty() {
            return values;
        }
    }
    default.iter().map(|s| s.to_string()).collect()
}

fn load_app_config(path: &Path) -> io::Result<AppConfig> {
    let map = parse_key_value_file(path)?;
    let mode = Mode::from_str(&get_string(&map, "mode", "bars"));
    let runtime_mode = RuntimeMode::from_str(&get_string(&map, "runtime_mode", "standard"));
    let width = get_num(&map, "width", 1920usize);
    let height = get_num(&map, "height", 1080usize);
    let profile_dir_str = get_string(&map, "profile_dir", "./config/profiles");
    Ok(AppConfig {
        backend: RenderBackend::from_str(&get_string(&map, "backend", "cpu")),
        spectrum_mode: SpectrumMode::from_str(&get_string(&map, "spectrum_mode", "single")),
        group_file: PathBuf::from(get_string(&map, "group_file", "./config/groups/default.group")),
        group_poll_ms: get_num(&map, "group_poll_ms", 400u64).max(50),
        mode,
        runtime_mode,
        monitor: get_string(&map, "monitor", "DP-1"),
        width,
        height,
        fps: get_num(&map, "fps", 60u32).max(1),
        bars: get_num(&map, "bars", 240usize).max(8),
        fifo_video: get_string(&map, "fifo_video", "/tmp/kitsune-spectrum.rgba"),
        fifo_cava: get_string(&map, "fifo_cava", "/tmp/cava-rs.raw"),
        color: parse_hex_color(&get_string(&map, "color", "#a60cdb")),
        dynamic_color: get_num::<u8>(&map, "dynamic_color", 0) > 0,
        color_source_file: PathBuf::from(get_string(&map, "color_source_file", "/tmp/kitsune-accent.hex")),
        color_poll_seconds: get_num(&map, "color_poll_seconds", 2u64).max(1),
        color_smooth: get_num(&map, "color_smooth", 0.25f32).clamp(0.01, 1.0),
        color_instant_apply: get_num::<u8>(&map, "color_instant_apply", 0) > 0,
        ring_auto_hide: get_num::<u8>(&map, "ring_auto_hide", 1) > 0,
        ring_show_threshold: get_num(&map, "ring_show_threshold", 0.030f32).clamp(0.001, 1.0),
        ring_hide_threshold: get_num(&map, "ring_hide_threshold", 0.012f32).clamp(0.0, 1.0),
        ring_fade_in_sec: get_num(&map, "ring_fade_in_sec", 0.25f32).clamp(0.01, 5.0),
        ring_fade_out_sec: get_num(&map, "ring_fade_out_sec", 0.85f32).clamp(0.01, 10.0),
        rotate_profiles: get_num::<u8>(&map, "rotate_profiles", 1) > 0,
        rotation_seconds: get_num(&map, "rotation_seconds", 10u64).max(1),
        static_profile: get_string(&map, "static_profile", ""),
        test_profile_file: PathBuf::from(get_string(&map, "test_profile_file", "./config/profiles/test.profile")),
        test_profile_poll_ms: get_num(&map, "test_profile_poll_ms", 300u64).max(50),
        profile_dir: PathBuf::from(profile_dir_str),
        bars_profiles: get_list(&map, "bars_profiles", &["bars_balanced", "bars_punchy", "bars_soft"]),
        ring_profiles: get_list(&map, "ring_profiles", &["ring_video_uno", "ring_video_dos", "ring_energy"]),
        bars_style: RenderStyle::from_str(&get_string(&map, "bars_style", "bars")),
        ring_style: RenderStyle::from_str(&get_string(&map, "ring_style", "bars")),
        bars_wave_thickness: get_num(&map, "bars_wave_thickness", 3i32).max(1),
        bars_dot_radius: get_num(&map, "bars_dot_radius", 2i32).max(1),
        ring_wave_thickness: get_num(&map, "ring_wave_thickness", 2i32).max(1),
        ring_dot_radius: get_num(&map, "ring_dot_radius", 2i32).max(1),
        bars_wave_roundness: get_num(&map, "bars_wave_roundness", 0.70f32).clamp(0.0, 1.0),
        ring_wave_roundness: get_num(&map, "ring_wave_roundness", 0.65f32).clamp(0.0, 1.0),
        ring_fill_softness: get_num(&map, "ring_fill_softness", 0.35f32).clamp(0.0, 1.0),
        ring_fill_overlap_px: get_num(&map, "ring_fill_overlap_px", 1.8f32).clamp(0.0, 8.0),
        postfx_scope: PostFxScope::from_str(&get_string(&map, "postfx_scope", "final")),
        postfx_enabled: get_num::<u8>(&map, "postfx_enabled", 0) > 0,
        postfx_blur_passes: get_num(&map, "postfx_blur_passes", 1usize).min(4),
        postfx_blur_mix: get_num(&map, "postfx_blur_mix", 0.20f32).clamp(0.0, 1.0),
        postfx_glow_strength: get_num(&map, "postfx_glow_strength", 1.2f32).clamp(0.0, 3.0),
        postfx_glow_mix: get_num(&map, "postfx_glow_mix", 0.22f32).clamp(0.0, 1.0),
        postfx_skip_plain_bars: get_num::<u8>(&map, "postfx_skip_plain_bars", 1) > 0,
        particles_enabled: get_num::<u8>(&map, "particles_enabled", 0) > 0,
        particles_max: get_num(&map, "particles_max", 600usize).clamp(32, 8000),
        particles_spawn_rate: get_num(&map, "particles_spawn_rate", 260.0f32).clamp(0.0, 12000.0),
        particles_life_min: get_num(&map, "particles_life_min", 0.12f32).clamp(0.02, 5.0),
        particles_life_max: get_num(&map, "particles_life_max", 0.32f32).clamp(0.03, 6.0),
        particles_speed_min: get_num(&map, "particles_speed_min", 55.0f32).clamp(1.0, 2000.0),
        particles_speed_max: get_num(&map, "particles_speed_max", 170.0f32).clamp(2.0, 3000.0),
        particles_size_min: get_num(&map, "particles_size_min", 1i32).clamp(1, 12),
        particles_size_max: get_num(&map, "particles_size_max", 2i32).clamp(1, 18),
        particles_size_scale: get_num(&map, "particles_size_scale", 1.0f32).clamp(0.2, 6.0),
        particles_alpha: get_num(&map, "particles_alpha", 0.70f32).clamp(0.0, 1.0),
        particles_drift: get_num(&map, "particles_drift", 38.0f32).clamp(0.0, 900.0),
        particles_fade_jitter: get_num(&map, "particles_fade_jitter", 0.35f32).clamp(0.0, 1.0),
        particles_layer: ParticleLayer::from_str(&get_string(&map, "particles_layer", "front")),
        particles_color: parse_hex_color(&get_string(&map, "particles_color", "#FFFFFF")),
        particles_color_mode: ParticleColorMode::from_str(&get_string(
            &map,
            "particles_color_mode",
            "static",
        )),
    })
}

fn load_profile_from_path(cfg: &AppConfig, path: &Path) -> io::Result<Profile> {
    let mut p = Profile::defaults(cfg);
    let map = parse_key_value_file(&path)?;

    p.gain = get_num(&map, "gain", p.gain);
    p.gamma = get_num(&map, "gamma", p.gamma).max(0.0001);
    p.curve_drive = get_num(&map, "curve_drive", p.curve_drive).max(0.1);
    p.attack = clamp01(get_num(&map, "attack", p.attack));
    p.gravity_step = get_num(&map, "gravity_step", p.gravity_step).max(0.0);
    p.avg_frames = get_num(&map, "avg_frames", p.avg_frames).max(1);
    p.smooth_radius = get_num(&map, "smooth_radius", p.smooth_radius);
    p.bass_boost = get_num(&map, "bass_boost", p.bass_boost).clamp(0.0, MAX_BASS_BOOST);
    p.bass_power = get_num(&map, "bass_power", p.bass_power).clamp(1.0, MAX_BASS_POWER);
    p.low_band_gain = get_num(&map, "low_band_gain", p.low_band_gain).clamp(0.0, MAX_BAND_GAIN);
    p.mid_band_gain = get_num(&map, "mid_band_gain", p.mid_band_gain).clamp(0.0, MAX_BAND_GAIN);
    p.high_band_gain = get_num(&map, "high_band_gain", p.high_band_gain).clamp(0.0, MAX_BAND_GAIN);
    p.silence_timeout_ms = get_num(&map, "silence_timeout_ms", p.silence_timeout_ms).max(100);
    p.height_scale = get_num(&map, "height_scale", p.height_scale).clamp(0.05, 1.0);
    p.dune_amount = clamp01(get_num(&map, "dune_amount", p.dune_amount));
    p.dune_cycles = get_num(&map, "dune_cycles", p.dune_cycles).max(0.5);
    p.edge_falloff_pow = get_num(&map, "edge_falloff_pow", p.edge_falloff_pow).max(0.5);
    p.dune_floor = clamp01(get_num(&map, "dune_floor", p.dune_floor));
    p.dune_softness = get_num(&map, "dune_softness", p.dune_softness).max(0.3);
    p.twin_amount = clamp01(get_num(&map, "twin_amount", p.twin_amount));
    p.twin_separation = get_num(&map, "twin_separation", p.twin_separation).clamp(0.05, 0.45);
    p.twin_width = get_num(&map, "twin_width", p.twin_width).clamp(0.03, 0.35);
    p.center_dip = clamp01(get_num(&map, "center_dip", p.center_dip));
    p.loud_floor = clamp01(get_num(&map, "loud_floor", p.loud_floor));
    p.loud_floor_curve = get_num(&map, "loud_floor_curve", p.loud_floor_curve).max(0.5);
    p.center_jump_amount = clamp01(get_num(&map, "center_jump_amount", p.center_jump_amount));
    p.center_jump_sharpness = get_num(&map, "center_jump_sharpness", p.center_jump_sharpness).max(1.0);
    p.center_jump_threshold = get_num(&map, "center_jump_threshold", p.center_jump_threshold).max(0.001);
    p.center_jump_decay = clamp01(get_num(&map, "center_jump_decay", p.center_jump_decay));
    p.bar_gap = get_num(&map, "bar_gap", p.bar_gap);
    p.side_padding = get_num(&map, "side_padding", p.side_padding);
    p.bottom_padding = get_num(&map, "bottom_padding", p.bottom_padding);
    p.min_bar_height_px = get_num(&map, "min_bar_height_px", p.min_bar_height_px);
    p.ring_x = get_num(&map, "ring_x", p.ring_x);
    p.ring_y = get_num(&map, "ring_y", p.ring_y);
    p.ring_radius = get_num(&map, "ring_radius", p.ring_radius).max(8);
    p.ring_thickness = get_num(&map, "ring_thickness", p.ring_thickness).max(1);
    p.ring_base_thickness = get_num(&map, "ring_base_thickness", p.ring_base_thickness).max(1);
    p.ring_bar_thickness = get_num(&map, "ring_bar_thickness", p.ring_bar_thickness).max(1);
    p.ring_min_bar = get_num(&map, "ring_min_bar", p.ring_min_bar).max(0.0);
    p.ring_max_bar = get_num(&map, "ring_max_bar", p.ring_max_bar).max(2.0);

    Ok(p)
}

fn load_profile(cfg: &AppConfig, name: &str) -> io::Result<Profile> {
    let path = cfg.profile_dir.join(format!("{}.profile", name));
    load_profile_from_path(cfg, &path)
}

fn parse_layer_line(line: &str, cfg: &AppConfig) -> io::Result<SpectrumLayer> {
    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
    if parts.len() < 6 {
        return Err(io::Error::other(
            "layer requires: enabled,mode,style,profile,color,alpha",
        ));
    }
    let enabled = parse_boolish(parts[0]).unwrap_or(false);
    let mode = Mode::from_str(parts[1]);
    let style = RenderStyle::from_str(parts[2]);
    let profile_name = parts[3].to_string();
    let color = parse_hex_color(parts[4]);
    let alpha = parts[5].parse::<f32>().unwrap_or(1.0).clamp(0.0, 1.0);

    let runtime_mode = match parts.get(6).map(|v| v.to_ascii_lowercase()) {
        Some(v) if v == "test" => RuntimeMode::Test,
        Some(v) if v == "standard" => RuntimeMode::Standard,
        _ => cfg.runtime_mode,
    };
    let rotate_profiles = parts
        .get(7)
        .and_then(|v| parse_boolish(v))
        .unwrap_or(cfg.rotate_profiles);
    let mut profiles: Vec<String> = parts
        .get(8)
        .map(|v| {
            v.split('|')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if profiles.is_empty() {
        profiles.push(profile_name.clone());
    }
    let test_profile_file = parts
        .get(9)
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);

    let mut layer_postfx = PostFxParams {
        enabled: true,
        blur_passes: cfg.postfx_blur_passes,
        blur_mix: cfg.postfx_blur_mix,
        glow_strength: cfg.postfx_glow_strength,
        glow_mix: cfg.postfx_glow_mix,
    };
    let mut has_layer_postfx = false;
    for token in parts.iter().skip(10) {
        let Some((k_raw, v_raw)) = token.split_once('=') else {
            continue;
        };
        let key = k_raw.trim().to_ascii_lowercase();
        let val = v_raw.trim();
        match key.as_str() {
            "postfx" | "postfx_enabled" => {
                if let Some(b) = parse_boolish(val) {
                    layer_postfx.enabled = b;
                    has_layer_postfx = true;
                }
            }
            "postfx_blur_passes" => {
                if let Ok(n) = val.parse::<usize>() {
                    layer_postfx.blur_passes = n.min(4);
                    has_layer_postfx = true;
                }
            }
            "postfx_blur_mix" => {
                if let Ok(f) = val.parse::<f32>() {
                    layer_postfx.blur_mix = f.clamp(0.0, 1.0);
                    has_layer_postfx = true;
                }
            }
            "postfx_glow_strength" => {
                if let Ok(f) = val.parse::<f32>() {
                    layer_postfx.glow_strength = f.clamp(0.0, 3.0);
                    has_layer_postfx = true;
                }
            }
            "postfx_glow_mix" => {
                if let Ok(f) = val.parse::<f32>() {
                    layer_postfx.glow_mix = f.clamp(0.0, 1.0);
                    has_layer_postfx = true;
                }
            }
            _ => {}
        }
    }

    let mut profile_index = 0usize;
    if let Some(pos) = profiles.iter().position(|p| p == &profile_name) {
        profile_index = pos;
    }
    let profile = if runtime_mode == RuntimeMode::Test {
        if let Some(p) = &test_profile_file {
            if p.exists() {
                load_profile_from_path(cfg, p)?
            } else {
                load_profile(cfg, &profiles[profile_index])?
            }
        } else {
            load_profile(cfg, &profiles[profile_index])?
        }
    } else {
        load_profile(cfg, &profiles[profile_index])?
    };

    Ok(SpectrumLayer {
        enabled,
        mode,
        style,
        profile,
        color,
        alpha,
        runtime_mode,
        rotate_profiles,
        profiles,
        profile_index,
        profile_last_switch: Instant::now(),
        test_profile_file,
        test_profile_last_mtime: None,
        test_profile_last_check: Instant::now(),
        postfx: if has_layer_postfx {
            Some(layer_postfx)
        } else {
            None
        },
    })
}

fn load_spectrum_group(cfg: &AppConfig) -> io::Result<Vec<SpectrumLayer>> {
    let f = File::open(&cfg.group_file)?;
    let r = BufReader::new(f);
    let mut out = Vec::new();
    for line in r.lines() {
        let l = line?;
        let s = l.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = s.split_once('=')
            && k.trim() == "layer"
        {
            match parse_layer_line(v.trim(), cfg) {
                Ok(layer) => out.push(layer),
                Err(e) => eprintln!("[group] invalid layer '{}': {}", s, e),
            }
        }
    }
    if out.is_empty() {
        return Err(io::Error::other("group has no valid layers"));
    }
    Ok(out)
}

fn open_fifo_writer(path: &str) -> io::Result<File> {
    loop {
        match OpenOptions::new().write(true).open(path) {
            Ok(f) => return Ok(f),
            Err(e) => {
                eprintln!("[renderer] waiting fifo writer {}: {}", path, e);
                thread::sleep(Duration::from_millis(300));
            }
        }
    }
}

fn spawn_cava_reader(path: String, bars: usize, state: Arc<Mutex<AudioState>>) {
    thread::spawn(move || {
        let frame_bytes = bars * 2;
        let mut buf = vec![0u8; frame_bytes];

        loop {
            let mut f = match File::open(&path) {
                Ok(file) => file,
                Err(e) => {
                    eprintln!("[audio] waiting fifo {}: {}", path, e);
                    thread::sleep(Duration::from_millis(300));
                    continue;
                }
            };

            loop {
                match f.read_exact(&mut buf) {
                    Ok(()) => {
                        let mut bins = vec![0.0f32; bars];
                        for (i, b) in bins.iter_mut().enumerate() {
                            let o = i * 2;
                            let v = u16::from_le_bytes([buf[o], buf[o + 1]]);
                            *b = v as f32 / 65535.0;
                        }
                        if let Ok(mut s) = state.lock() {
                            s.latest_bins = bins;
                            s.counter = s.counter.wrapping_add(1);
                            s.last_update = Instant::now();
                        }
                    }
                    Err(e) => {
                        eprintln!("[audio] read error, reopening: {}", e);
                        break;
                    }
                }
            }

            thread::sleep(Duration::from_millis(120));
        }
    });
}

impl Engine {
    fn new(cfg: AppConfig, profile_names: Vec<String>, profile: Profile) -> Self {
        let mut backend = cfg.backend;
        let mut gpu_renderer = None;
        if backend == RenderBackend::Gpu {
            match GpuRenderer::new(cfg.width, cfg.height, cfg.bars) {
                Ok(g) => {
                    eprintln!("[gpu] backend initialized");
                    gpu_renderer = Some(g);
                }
                Err(e) => {
                    eprintln!("[gpu] init failed, fallback to cpu: {}", e);
                    backend = RenderBackend::Cpu;
                }
            }
        }

        let mut spectrum_mode = cfg.spectrum_mode;
        let mut group_layers = Vec::new();
        if spectrum_mode == SpectrumMode::Group {
            match load_spectrum_group(&cfg) {
                Ok(layers) => {
                    eprintln!("[group] loaded {} layers from {}", layers.len(), cfg.group_file.display());
                    group_layers = layers;
                }
                Err(e) => {
                    eprintln!(
                        "[group] failed to load {}: {} (fallback single)",
                        cfg.group_file.display(),
                        e
                    );
                    spectrum_mode = SpectrumMode::Single;
                }
            }
        }

        let frame = vec![0u8; cfg.width * cfg.height * 4];
        let composite_frame = vec![0u8; cfg.width * cfg.height * 4];
        let postfx_a = vec![0u8; cfg.width * cfg.height * 4];
        let postfx_b = vec![0u8; cfg.width * cfg.height * 4];
        let rng_seed = 0x9E3779B97F4A7C15u64
            ^ (cfg.width as u64).wrapping_mul(1315423911)
            ^ ((cfg.height as u64) << 16);
        let heights = vec![0.0f32; cfg.bars];
        let initial_color = cfg.color;
        let runtime_mode = cfg.runtime_mode;
        let rotate_profiles = cfg.rotate_profiles;
        let color_poll_seconds = cfg.color_poll_seconds;
        let test_profile_file = cfg.test_profile_file.clone();
        let test_profile_poll_ms = cfg.test_profile_poll_ms;
        let group_poll_ms = cfg.group_poll_ms;
        Self {
            cfg,
            backend,
            gpu_renderer,
            spectrum_mode,
            group_layers,
            group_poll_ms,
            group_last_check: Instant::now(),
            group_last_mtime: None,
            profile,
            frame,
            composite_frame,
            postfx_a,
            postfx_b,
            particles: Vec::new(),
            particle_emit_carry: 0.0,
            rng_state: rng_seed,
            heights,
            history: VecDeque::new(),
            prev_global_energy: 0.0,
            center_jump_state: 0.0,
            profile_names,
            profile_index: 0,
            profile_last_switch: Instant::now(),
            draw_color: initial_color,
            target_color: initial_color,
            color_last_poll: Instant::now() - Duration::from_secs(color_poll_seconds),
            ring_visibility: 1.0,
            draw_alpha_scale: 1.0,
            runtime_mode,
            rotate_profiles,
            test_profile_file,
            test_profile_poll_ms,
            test_profile_last_check: Instant::now(),
            test_profile_last_mtime: None,
        }
    }

    fn maybe_rotate_profile(&mut self) {
        if self.runtime_mode != RuntimeMode::Standard || !self.rotate_profiles {
            return;
        }
        if self.profile_names.len() < 2 {
            return;
        }
        if self.profile_last_switch.elapsed().as_secs() < self.cfg.rotation_seconds {
            return;
        }
        self.profile_last_switch = Instant::now();
        self.profile_index = (self.profile_index + 1) % self.profile_names.len();
        let name = &self.profile_names[self.profile_index];
        match load_profile(&self.cfg, name) {
            Ok(p) => {
                self.profile = p;
                eprintln!("[profile] switched -> {}", name);
            }
            Err(e) => {
                eprintln!("[profile] failed {}: {}", name, e);
            }
        }
    }

    fn maybe_reload_test_profile(&mut self) {
        if self.runtime_mode != RuntimeMode::Test {
            return;
        }
        if self.test_profile_last_check.elapsed() < Duration::from_millis(self.test_profile_poll_ms) {
            return;
        }
        self.test_profile_last_check = Instant::now();

        let meta = match fs::metadata(&self.test_profile_file) {
            Ok(m) => m,
            Err(e) => {
                eprintln!(
                    "[test] no se pudo leer {}: {}",
                    self.test_profile_file.display(),
                    e
                );
                return;
            }
        };
        let mtime = meta.modified().ok();
        if mtime.is_some() && mtime == self.test_profile_last_mtime {
            return;
        }
        self.test_profile_last_mtime = mtime;

        match load_profile_from_path(&self.cfg, &self.test_profile_file) {
            Ok(p) => {
                self.profile = p;
                eprintln!("[test] perfil recargado: {}", self.test_profile_file.display());
            }
            Err(e) => eprintln!(
                "[test] error recargando {}: {}",
                self.test_profile_file.display(),
                e
            ),
        }
    }

    fn clear_frame(&mut self) {
        self.frame.fill(0);
    }

    fn maybe_update_dynamic_color(&mut self) {
        if !self.cfg.dynamic_color {
            return;
        }
        if self.color_last_poll.elapsed().as_secs() < self.cfg.color_poll_seconds {
            return;
        }
        self.color_last_poll = Instant::now();
        match color_from_file(&self.cfg.color_source_file) {
            Ok(c) => {
                self.target_color = c;
                if self.cfg.color_instant_apply {
                    self.draw_color = c;
                }
            }
            Err(e) => eprintln!(
                "[color] poll failed {}: {}",
                self.cfg.color_source_file.display(),
                e
            ),
        }
    }

    fn update_color_fade(&mut self, dt_sec: f32) {
        if !self.cfg.dynamic_color {
            self.draw_color = self.cfg.color;
            return;
        }
        let alpha = (self.cfg.color_smooth * dt_sec * 60.0).clamp(0.01, 1.0);
        let mix = |a: u8, b: u8| -> u8 {
            let av = a as f32;
            let bv = b as f32;
            (av + (bv - av) * alpha).round().clamp(0.0, 255.0) as u8
        };
        self.draw_color = Color {
            r: mix(self.draw_color.r, self.target_color.r),
            g: mix(self.draw_color.g, self.target_color.g),
            b: mix(self.draw_color.b, self.target_color.b),
            a: 255,
        };
    }

    fn put_pixel(&mut self, x: i32, y: i32) {
        if x < 0 || y < 0 || x >= self.cfg.width as i32 || y >= self.cfg.height as i32 {
            return;
        }
        let a = ((self.draw_color.a as f32) * self.draw_alpha_scale)
            .round()
            .clamp(0.0, 255.0) as u8;
        if a == 0 {
            return;
        }
        let idx = ((y as usize) * self.cfg.width + (x as usize)) * 4;
        self.frame[idx] = self.draw_color.r;
        self.frame[idx + 1] = self.draw_color.g;
        self.frame[idx + 2] = self.draw_color.b;
        self.frame[idx + 3] = a;
    }

    fn draw_rect(&mut self, x0: i32, y0: i32, w: i32, h: i32) {
        let x1 = (x0 + w).min(self.cfg.width as i32);
        let y1 = (y0 + h).min(self.cfg.height as i32);
        let xx0 = x0.max(0);
        let yy0 = y0.max(0);
        if x1 <= xx0 || y1 <= yy0 {
            return;
        }
        for y in yy0..y1 {
            let row = y as usize * self.cfg.width * 4;
            for x in xx0..x1 {
                let a = ((self.draw_color.a as f32) * self.draw_alpha_scale)
                    .round()
                    .clamp(0.0, 255.0) as u8;
                if a == 0 {
                    continue;
                }
                let idx = row + x as usize * 4;
                self.frame[idx] = self.draw_color.r;
                self.frame[idx + 1] = self.draw_color.g;
                self.frame[idx + 2] = self.draw_color.b;
                self.frame[idx + 3] = a;
            }
        }
    }

    fn draw_disk(&mut self, cx: i32, cy: i32, radius: i32) {
        let r = radius.max(0);
        let rr = r * r;
        for dy in -r..=r {
            let yy = cy + dy;
            if yy < 0 || yy >= self.cfg.height as i32 {
                continue;
            }
            let xspan = ((rr - dy * dy).max(0) as f32).sqrt() as i32;
            for x in (cx - xspan)..=(cx + xspan) {
                self.put_pixel(x, yy);
            }
        }
    }

    fn draw_annulus(&mut self, cx: i32, cy: i32, inner_r: i32, outer_r: i32) {
        let in_r = inner_r.max(0);
        let out_r = outer_r.max(in_r + 1);
        let in2 = in_r * in_r;
        let out2 = out_r * out_r;
        for dy in -out_r..=out_r {
            let yy = cy + dy;
            if yy < 0 || yy >= self.cfg.height as i32 {
                continue;
            }
            let y2 = dy * dy;
            let out_dx = ((out2 - y2).max(0) as f32).sqrt() as i32;
            let in_dx = if y2 < in2 {
                ((in2 - y2).max(0) as f32).sqrt() as i32
            } else {
                -1
            };
            for x in (cx - out_dx)..=(cx + out_dx) {
                let inside_inner = in_dx >= 0 && x >= cx - in_dx && x <= cx + in_dx;
                if !inside_inner {
                    self.put_pixel(x, yy);
                }
            }
        }
    }

    fn draw_thick_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, thickness: i32) {
        let dx = x1 - x0;
        let dy = y1 - y0;
        let steps = dx.abs().max(dy.abs()).max(1);
        let stamp_r = ((thickness - 1) / 2).max(0);
        for i in 0..=steps {
            let p = i as f32 / steps as f32;
            let x = (x0 as f32 + dx as f32 * p).round() as i32;
            let y = (y0 as f32 + dy as f32 * p).round() as i32;
            if stamp_r > 0 {
                self.draw_disk(x, y, stamp_r);
            } else {
                self.put_pixel(x, y);
            }
        }
    }

    fn draw_radial_bar(&mut self, cx: i32, cy: i32, angle: f32, r0: f32, len: f32, thickness: i32) {
        let ca = angle.cos();
        let sa = angle.sin();
        let x0 = (cx as f32 + r0 * ca).round() as i32;
        let y0 = (cy as f32 + r0 * sa).round() as i32;
        let x1 = (cx as f32 + (r0 + len) * ca).round() as i32;
        let y1 = (cy as f32 + (r0 + len) * sa).round() as i32;
        self.draw_thick_line(x0, y0, x1, y1, thickness);
    }

    fn draw_polyline(&mut self, points: &[(i32, i32)], thickness: i32, closed: bool) {
        if points.len() < 2 {
            return;
        }
        for i in 0..(points.len() - 1) {
            let (x0, y0) = points[i];
            let (x1, y1) = points[i + 1];
            self.draw_thick_line(x0, y0, x1, y1, thickness);
        }
        if closed {
            let (x0, y0) = points[points.len() - 1];
            let (x1, y1) = points[0];
            self.draw_thick_line(x0, y0, x1, y1, thickness);
        }
    }

    fn with_alpha_scale<F: FnOnce(&mut Self)>(&mut self, mul: f32, f: F) {
        let prev = self.draw_alpha_scale;
        self.draw_alpha_scale = (prev * mul).clamp(0.0, 1.0);
        f(self);
        self.draw_alpha_scale = prev;
    }

    fn rounded_wave_points(&self, points: &[(i32, i32)], closed: bool, roundness: f32) -> Vec<(i32, i32)> {
        if points.len() < 3 {
            return points.to_vec();
        }
        let iters = (roundness.clamp(0.0, 1.0) * 3.0).round() as usize;
        if iters == 0 {
            return points.to_vec();
        }

        let mut pts: Vec<(f32, f32)> = points.iter().map(|&(x, y)| (x as f32, y as f32)).collect();

        for _ in 0..iters {
            let n = pts.len();
            if n < 3 {
                break;
            }
            let mut out: Vec<(f32, f32)> = Vec::with_capacity(n * 2);

            if closed {
                for i in 0..n {
                    let (x0, y0) = pts[i];
                    let (x1, y1) = pts[(i + 1) % n];
                    out.push((0.75 * x0 + 0.25 * x1, 0.75 * y0 + 0.25 * y1));
                    out.push((0.25 * x0 + 0.75 * x1, 0.25 * y0 + 0.75 * y1));
                }
            } else {
                out.push(pts[0]);
                for i in 0..(n - 1) {
                    let (x0, y0) = pts[i];
                    let (x1, y1) = pts[i + 1];
                    out.push((0.75 * x0 + 0.25 * x1, 0.75 * y0 + 0.25 * y1));
                    out.push((0.25 * x0 + 0.75 * x1, 0.25 * y0 + 0.75 * y1));
                }
                out.push(pts[n - 1]);
            }

            pts = out;
        }

        pts.iter()
            .map(|&(x, y)| (x.round() as i32, y.round() as i32))
            .collect()
    }

    fn fill_polygon(&mut self, points: &[(i32, i32)]) {
        if points.len() < 3 {
            return;
        }

        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;
        for &(_, y) in points {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }

        min_y = min_y.clamp(0, self.cfg.height as i32 - 1);
        max_y = max_y.clamp(0, self.cfg.height as i32 - 1);
        if min_y > max_y {
            return;
        }

        for y in min_y..=max_y {
            let scan_y = y as f32 + 0.5;
            let mut xs: Vec<f32> = Vec::new();

            for i in 0..points.len() {
                let (x1, y1i) = points[i];
                let (x2, y2i) = points[(i + 1) % points.len()];
                let y1 = y1i as f32;
                let y2 = y2i as f32;

                // Half-open edge test avoids double-counting vertices.
                if (y1 <= scan_y && y2 > scan_y) || (y2 <= scan_y && y1 > scan_y) {
                    let t = (scan_y - y1) / (y2 - y1);
                    let x = x1 as f32 + (x2 - x1) as f32 * t;
                    xs.push(x);
                }
            }

            if xs.len() < 2 {
                continue;
            }
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let mut i = 0usize;
            while i + 1 < xs.len() {
                let x0 = xs[i].ceil() as i32;
                let x1 = xs[i + 1].floor() as i32;
                if x1 >= x0 {
                    self.draw_rect(x0, y, x1 - x0 + 1, 1);
                }
                i += 2;
            }
        }
    }

    fn radial_offset_contour(&self, points: &[(i32, i32)], cx: i32, cy: i32, offset: f32) -> Vec<(i32, i32)> {
        let mut out = Vec::with_capacity(points.len());
        for &(x, y) in points {
            let vx = x as f32 - cx as f32;
            let vy = y as f32 - cy as f32;
            let len = (vx * vx + vy * vy).sqrt().max(1.0);
            let nx = vx / len;
            let ny = vy / len;
            out.push((
                (x as f32 + nx * offset).round() as i32,
                (y as f32 + ny * offset).round() as i32,
            ));
        }
        out
    }

    fn temporal_average(&self, avg_frames: usize) -> Vec<f32> {
        if self.history.is_empty() {
            return vec![0.0; self.cfg.bars];
        }
        let frames = self.history.len().min(avg_frames.max(1));
        let mut out = vec![0.0; self.cfg.bars];
        let mut denom = 0.0f32;
        for age in 0..frames {
            let idx = self.history.len() - 1 - age;
            let w = (frames - age) as f32;
            denom += w;
            let src = &self.history[idx];
            for i in 0..self.cfg.bars {
                out[i] += src[i] * w;
            }
        }
        for v in &mut out {
            *v /= denom.max(1e-6);
        }
        out
    }

    fn spatial_smooth(&self, src: &[f32], radius: usize) -> Vec<f32> {
        if radius == 0 {
            return src.to_vec();
        }
        let mut out = vec![0.0; self.cfg.bars];
        for i in 0..self.cfg.bars {
            let start = i.saturating_sub(radius);
            let end = (i + radius).min(self.cfg.bars - 1);
            let mut sum = 0.0;
            let mut wsum = 0.0;
            for (j, &val) in src.iter().enumerate().take(end + 1).skip(start) {
                let d = (i as i32 - j as i32).abs() as f32;
                let w = (radius as f32 + 1.0) - d;
                sum += val * w;
                wsum += w;
            }
            out[i] = if wsum > 0.0 { sum / wsum } else { src[i] };
        }
        out
    }

    fn compute_weighted(&self, src: &[f32]) -> Vec<f32> {
        let p = &self.profile;
        let denom = (self.cfg.bars - 1).max(1) as f32;
        let tw_l = 0.5 - p.twin_separation;
        let tw_r = 0.5 + p.twin_separation;
        let tw_var = (2.0 * p.twin_width * p.twin_width).max(1e-6);
        let c_var = (2.0 * (p.twin_width * 0.7).powi(2)).max(1e-6);

        let mut out = vec![0.0; self.cfg.bars];

        for i in 0..self.cfg.bars {
            let pos = i as f32 / denom;
            let low = 1.0 - pos;
            let band_gain = if pos < 0.33 {
                p.low_band_gain
            } else if pos < 0.66 {
                p.mid_band_gain
            } else {
                p.high_band_gain
            };
            let boosted = src[i] * band_gain * (1.0 + p.bass_boost * low.powf(p.bass_power));
            let raw = (boosted * p.gain).max(0.0).powf(p.gamma);
            let curved = 1.0 - (-raw * p.curve_drive).exp();

            if self.cfg.mode == Mode::Ring {
                out[i] = clamp01(curved);
                continue;
            }

            let center_dist = (pos - 0.5).abs() * 2.0;
            let edge = (1.0 - center_dist * center_dist).max(0.0).powf(p.edge_falloff_pow);
            let ridge_raw = 0.5 + 0.5 * (2.0 * std::f32::consts::PI * p.dune_cycles * center_dist).cos();
            let ridges = ridge_raw.powf(p.dune_softness);
            let shaped = edge * ((1.0 - p.dune_amount) + p.dune_amount * ridges);
            let envelope_dune = p.dune_floor + (1.0 - p.dune_floor) * shaped;

            let left_peak = (-(pos - tw_l).powi(2) / tw_var).exp();
            let right_peak = (-(pos - tw_r).powi(2) / tw_var).exp();
            let twin_raw = clamp01(left_peak + right_peak);
            let center_mask = (-(pos - 0.5).powi(2) / c_var).exp();
            let twin = clamp01(((1.0 - p.twin_amount) + p.twin_amount * twin_raw) * (1.0 - p.center_dip * center_mask));

            out[i] = clamp01(curved * clamp01(envelope_dune * twin));
        }

        out
    }

    fn update_heights(&mut self, weighted: &[f32], dt_sec: f32, audio_last_update: Instant) {
        let p = &self.profile;
        let silence = audio_last_update.elapsed() >= Duration::from_millis(p.silence_timeout_ms);
        let fall_step = p.gravity_step * dt_sec;

        let mut energy = 0.0;
        for &v in weighted {
            energy += v;
        }
        energy /= self.cfg.bars as f32;

        let transient = (energy - self.prev_global_energy).max(0.0);
        if transient > p.center_jump_threshold {
            self.center_jump_state = self.center_jump_state.max((transient * 10.0).min(1.0));
        }
        self.center_jump_state *= p.center_jump_decay.powf(dt_sec * 60.0).max(0.001);
        self.prev_global_energy = energy;

        let loud_floor = p.loud_floor * energy.powf(p.loud_floor_curve);
        let denom = (self.cfg.bars - 1).max(1) as f32;

        for i in 0..self.cfg.bars {
            let target = if silence {
                0.0
            } else if self.cfg.mode == Mode::Ring {
                weighted[i].max(loud_floor * 0.85)
            } else {
                let pos = i as f32 / denom;
                let center_dist = (pos - 0.5).abs() * 2.0;
                let center_shape = (1.0 - center_dist).max(0.0).powf(p.center_jump_sharpness);
                let center_boost = self.center_jump_state * p.center_jump_amount * center_shape;
                let floor_bar = loud_floor * (0.85 + 0.15 * (1.0 - center_dist));
                (weighted[i] + center_boost).max(floor_bar)
            };

            if target >= self.heights[i] {
                self.heights[i] += (target - self.heights[i]) * p.attack;
            } else {
                self.heights[i] = (self.heights[i] - fall_step).max(target);
            }
            self.heights[i] = clamp01(self.heights[i]);
        }
    }

    fn update_ring_visibility(&mut self, weighted: &[f32], dt_sec: f32, audio_last_update: Instant) {
        let has_ring_in_group = self.spectrum_mode == SpectrumMode::Group
            && self.group_layers.iter().any(|l| l.enabled && l.mode == Mode::Ring);
        let uses_ring = self.cfg.mode == Mode::Ring || has_ring_in_group;
        if !uses_ring || !self.cfg.ring_auto_hide {
            self.ring_visibility = 1.0;
            self.draw_alpha_scale = 1.0;
            return;
        }

        let silent_by_timeout = audio_last_update.elapsed() >= Duration::from_millis(self.profile.silence_timeout_ms);
        let mut energy = 0.0f32;
        for &v in weighted {
            energy += v;
        }
        energy /= self.cfg.bars as f32;

        let target = if silent_by_timeout {
            0.0
        } else if energy >= self.cfg.ring_show_threshold {
            1.0
        } else if energy <= self.cfg.ring_hide_threshold {
            0.0
        } else {
            self.ring_visibility
        };

        if target > self.ring_visibility {
            let step = (dt_sec / self.cfg.ring_fade_in_sec).clamp(0.0, 1.0);
            self.ring_visibility = (self.ring_visibility + step).min(1.0);
        } else if target < self.ring_visibility {
            let step = (dt_sec / self.cfg.ring_fade_out_sec).clamp(0.0, 1.0);
            self.ring_visibility = (self.ring_visibility - step).max(0.0);
        }

        self.draw_alpha_scale = self.ring_visibility.clamp(0.0, 1.0);
    }

    fn build_bar_layout_from(&self, profile: &Profile) -> Vec<(i32, i32, i32)> {
        let bottom_padding = profile.bottom_padding;
        let height_scale = profile.height_scale;
        let side_padding = profile.side_padding;
        let bar_gap = profile.bar_gap;
        let min_bar_height_px = profile.min_bar_height_px;

        let usable_h = ((self.cfg.height.saturating_sub(bottom_padding)) as f32 * height_scale)
            .max(1.0) as i32;
        let usable_w = self.cfg.width.saturating_sub(side_padding * 2).max(1);

        let mut gap = bar_gap;
        if self.cfg.bars > 1 {
            let max_gap = (usable_w.saturating_sub(self.cfg.bars)) / (self.cfg.bars - 1);
            gap = gap.min(max_gap);
        }

        let total_gap = gap * self.cfg.bars.saturating_sub(1);
        let width_for_bars = usable_w.saturating_sub(total_gap).max(self.cfg.bars);
        let base_w = width_for_bars / self.cfg.bars;
        let mut extra = width_for_bars - base_w * self.cfg.bars;

        let mut out = Vec::with_capacity(self.cfg.bars);
        let mut x = side_padding as i32;
        for i in 0..self.cfg.bars {
            let w = (base_w + usize::from(extra > 0)) as i32;
            extra = extra.saturating_sub(1);
            let h = (((self.heights[i] * usable_h as f32) as i32).max(min_bar_height_px as i32)).max(0);
            out.push((x, w.max(1), h));
            x += w + gap as i32;
        }
        out
    }

    fn draw_bars_as_lines(&mut self, layout: &[(i32, i32, i32)]) {
        let y_base = self.cfg.height as i32 - self.profile.bottom_padding as i32;
        let t = self.cfg.bars_wave_thickness.max(1);
        for &(x, w, h) in layout {
            if h <= 0 {
                continue;
            }
            let cx = x + (w / 2);
            let y = y_base - h;
            self.draw_rect(cx - (t / 2), y, t, h);
        }
    }

    fn draw_bars_as_rects(&mut self, layout: &[(i32, i32, i32)]) {
        let y_base = self.cfg.height as i32 - self.profile.bottom_padding as i32;
        let join_neighbors = self.cfg.bars_style == RenderStyle::BarsFill;
        for (i, &(x, w, h)) in layout.iter().enumerate() {
            if h <= 0 {
                continue;
            }
            let y = y_base - h;
            let draw_w = if join_neighbors && i + 1 < layout.len() {
                (layout[i + 1].0 - x).max(w)
            } else {
                w
            };
            self.draw_rect(x, y, draw_w, h);
        }
    }

    fn draw_bars_as_waves(&mut self, layout: &[(i32, i32, i32)]) {
        let y_base = self.cfg.height as i32 - self.profile.bottom_padding as i32;
        let mut points = Vec::with_capacity(layout.len());
        for &(x, w, h) in layout {
            let cx = x + (w / 2);
            let y = y_base - h.max(0);
            points.push((cx, y));
        }
        let rounded = self.rounded_wave_points(&points, false, self.cfg.bars_wave_roundness);
        self.draw_polyline(&rounded, self.cfg.bars_wave_thickness, false);
    }

    fn draw_bars_as_waves_fill(&mut self, layout: &[(i32, i32, i32)]) {
        let y_base = self.cfg.height as i32 - self.profile.bottom_padding as i32;
        let mut points = Vec::with_capacity(layout.len());
        for &(x, w, h) in layout {
            let cx = x + (w / 2);
            let y = y_base - h.max(0);
            points.push((cx, y));
        }
        let rounded = self.rounded_wave_points(&points, false, self.cfg.bars_wave_roundness);
        if rounded.len() < 2 {
            return;
        }

        for i in 0..(rounded.len() - 1) {
            let (x0, y0) = rounded[i];
            let (x1, y1) = rounded[i + 1];
            let x_start = x0.min(x1);
            let x_end = x0.max(x1);
            if x_start == x_end {
                let top = y0.min(y1).min(y_base);
                let h = (y_base - top).max(0);
                if h > 0 {
                    self.draw_rect(x_start, top, 1, h + 1);
                }
                continue;
            }
            for x in x_start..=x_end {
                let t = (x - x0) as f32 / (x1 - x0) as f32;
                let y = (y0 as f32 + (y1 - y0) as f32 * t).round() as i32;
                let top = y.min(y_base);
                let h = (y_base - top).max(0);
                if h > 0 {
                    self.draw_rect(x, top, 1, h + 1);
                }
            }
        }

        self.draw_polyline(&rounded, self.cfg.bars_wave_thickness, false);
    }

    fn draw_bars_as_dots(&mut self, layout: &[(i32, i32, i32)]) {
        let r = self.cfg.bars_dot_radius.max(1);
        let y_base = self.cfg.height as i32 - self.profile.bottom_padding as i32;
        let step = (r * 2 + 1).max(2);

        for &(x, w, h) in layout {
            if h <= 0 {
                continue;
            }
            let cx = x + (w / 2);
            let top = y_base - h;
            let mut y = y_base - r;
            while y >= top {
                self.draw_disk(cx, y, r);
                y -= step;
            }
        }
    }

    fn draw_bars(&mut self) {
        let layout = self.build_bar_layout_from(&self.profile);
        match self.cfg.bars_style {
            RenderStyle::Bars => self.draw_bars_as_lines(&layout),
            RenderStyle::BarsFill => self.draw_bars_as_rects(&layout),
            RenderStyle::Waves => self.draw_bars_as_waves(&layout),
            RenderStyle::WavesFill => self.draw_bars_as_waves_fill(&layout),
            RenderStyle::Dots => self.draw_bars_as_dots(&layout),
        }
    }

    fn draw_ring(&mut self) {
        let cx = self.profile.ring_x;
        let cy = self.profile.ring_y;
        let radius = self.profile.ring_radius as f32;
        let ring_base_thickness = self.profile.ring_base_thickness as f32;
        let ring_max_bar = self.profile.ring_max_bar;
        let height_scale = self.profile.height_scale;
        let ring_min_bar = self.profile.ring_min_bar;
        let ring_bar_thickness = self.profile.ring_bar_thickness;
        let ring_thickness = self.profile.ring_thickness;

        let base_inner = (radius - ring_base_thickness).max(1.0) as i32;
        let base_outer = (radius + ring_base_thickness).max((base_inner + 1) as f32) as i32;

        self.draw_annulus(cx, cy, base_inner, base_outer);

        let usable = (ring_max_bar * height_scale).max(1.0);
        let step = 2.0 * std::f32::consts::PI / self.cfg.bars as f32;
        let mut end_points = Vec::with_capacity(self.cfg.bars);

        for i in 0..self.cfg.bars {
            let len = ring_min_bar + self.heights[i] * usable;
            let angle = i as f32 * step - std::f32::consts::PI / 2.0;
            let r_end = base_outer as f32 + len;
            let x1 = (cx as f32 + r_end * angle.cos()).round() as i32;
            let y1 = (cy as f32 + r_end * angle.sin()).round() as i32;
            end_points.push((x1, y1, angle, len));
        }

        match self.cfg.ring_style {
            RenderStyle::Bars => {
                for &(_, _, angle, len) in &end_points {
                    self.draw_radial_bar(cx, cy, angle, base_outer as f32, len, ring_bar_thickness);
                }
            }
            RenderStyle::BarsFill => {
                for &(_, _, angle, len) in &end_points {
                    self.draw_radial_bar(cx, cy, angle, base_outer as f32, len, ring_bar_thickness);
                }
            }
            RenderStyle::Waves => {
                let points: Vec<(i32, i32)> = end_points.iter().map(|&(x, y, _, _)| (x, y)).collect();
                let rounded = self.rounded_wave_points(&points, true, self.cfg.ring_wave_roundness);
                self.draw_polyline(&rounded, self.cfg.ring_wave_thickness, true);
            }
            RenderStyle::WavesFill => {
                let outer_raw: Vec<(i32, i32)> = end_points.iter().map(|&(x, y, _, _)| (x, y)).collect();
                let outer = self.rounded_wave_points(&outer_raw, true, self.cfg.ring_wave_roundness);

                // Real fill: from ring base edge to outer waveform edge (not just a thin strip).
                let fill_outer = &outer_raw;
                if fill_outer.len() >= 3 {
                    let n = fill_outer.len();
                    let fill_inner_r = (base_outer as f32 - self.cfg.ring_fill_overlap_px).max(1.0);
                    let mut fill_inner = Vec::with_capacity(n);
                    for i in 0..n {
                        let a =
                            i as f32 / n as f32 * 2.0 * std::f32::consts::PI - std::f32::consts::PI / 2.0;
                        let x = (cx as f32 + fill_inner_r * a.cos()).round() as i32;
                        let y = (cy as f32 + fill_inner_r * a.sin()).round() as i32;
                        fill_inner.push((x, y));
                    }
                    for i in 0..n {
                        let j = (i + 1) % n;
                        let quad = [fill_outer[i], fill_outer[j], fill_inner[j], fill_inner[i]];
                        self.fill_polygon(&quad);
                    }
                }

                let feather_steps = (self.cfg.ring_fill_softness * 8.0).round() as i32;
                if feather_steps > 0 {
                    let wave_thickness = self.cfg.ring_wave_thickness.max(1);
                    for k in 1..=feather_steps {
                        let t = 1.0 - (k as f32 / (feather_steps as f32 + 1.0));
                        let a = (0.55 * t).clamp(0.05, 0.55);
                        let out_c = self.radial_offset_contour(&outer, cx, cy, k as f32);
                        let in_c = self.radial_offset_contour(&outer, cx, cy, -(k as f32));
                        self.with_alpha_scale(a, |eng| {
                            eng.draw_polyline(&out_c, wave_thickness, true);
                            eng.draw_polyline(&in_c, wave_thickness, true);
                        });
                    }
                }

                self.draw_polyline(&outer, self.cfg.ring_wave_thickness, true);
            }
            RenderStyle::Dots => {
                let r = self.cfg.ring_dot_radius.max(1);
                for &(x, y, _, _) in &end_points {
                    self.draw_disk(x, y, r);
                }
            }
        }

        let stroke_steps = (2.0 * std::f32::consts::PI * base_outer as f32).max(64.0) as i32;
        let half = (ring_thickness / 2).max(0);
        for i in 0..stroke_steps {
            let a = i as f32 / stroke_steps as f32 * 2.0 * std::f32::consts::PI;
            let ca = a.cos();
            let sa = a.sin();
            for k in -half..=half {
                let x = (cx as f32 + (base_outer + k) as f32 * ca).round() as i32;
                let y = (cy as f32 + (base_outer + k) as f32 * sa).round() as i32;
                self.put_pixel(x, y);
            }
        }
    }

    fn blend_over(dst: &mut [u8], src: &[u8]) {
        let px = dst.len().min(src.len()) / 4;
        for i in 0..px {
            let o = i * 4;
            let sr = src[o] as f32 / 255.0;
            let sg = src[o + 1] as f32 / 255.0;
            let sb = src[o + 2] as f32 / 255.0;
            let sa = src[o + 3] as f32 / 255.0;
            if sa <= 0.0 {
                continue;
            }
            let dr = dst[o] as f32 / 255.0;
            let dg = dst[o + 1] as f32 / 255.0;
            let db = dst[o + 2] as f32 / 255.0;
            let da = dst[o + 3] as f32 / 255.0;

            let out_a = sa + da * (1.0 - sa);
            if out_a <= 1e-6 {
                continue;
            }
            let out_r = (sr * sa + dr * da * (1.0 - sa)) / out_a;
            let out_g = (sg * sa + dg * da * (1.0 - sa)) / out_a;
            let out_b = (sb * sa + db * da * (1.0 - sa)) / out_a;

            dst[o] = (out_r * 255.0).round().clamp(0.0, 255.0) as u8;
            dst[o + 1] = (out_g * 255.0).round().clamp(0.0, 255.0) as u8;
            dst[o + 2] = (out_b * 255.0).round().clamp(0.0, 255.0) as u8;
            dst[o + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
        }
    }

    fn global_postfx_params(&self) -> PostFxParams {
        PostFxParams {
            enabled: self.cfg.postfx_enabled,
            blur_passes: self.cfg.postfx_blur_passes.min(4),
            blur_mix: self.cfg.postfx_blur_mix.clamp(0.0, 1.0),
            glow_strength: self.cfg.postfx_glow_strength.clamp(0.0, 3.0),
            glow_mix: self.cfg.postfx_glow_mix.clamp(0.0, 1.0),
        }
    }

    fn rand01(&mut self) -> f32 {
        // xorshift64* for cheap deterministic randomness.
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        let v = x.wrapping_mul(0x2545F4914F6CDD1D);
        ((v >> 40) as u32) as f32 / ((1u32 << 24) as f32)
    }

    fn rand_range(&mut self, lo: f32, hi: f32) -> f32 {
        if hi <= lo {
            return lo;
        }
        lo + (hi - lo) * self.rand01()
    }

    fn spawn_particle(&mut self, p: Particle) {
        if self.particles.len() >= self.cfg.particles_max {
            return;
        }
        self.particles.push(p);
    }

    fn random_particle_size(&mut self, size_lo: i32, size_hi: i32, amp: f32) -> i32 {
        let base = self
            .rand_range(size_lo as f32, size_hi as f32 + 0.999)
            .floor()
            .max(1.0);
        let amp_mul = 0.85 + 0.50 * amp.clamp(0.0, 1.0);
        (base * self.cfg.particles_size_scale * amp_mul)
            .round()
            .clamp(1.0, 96.0) as i32
    }

    fn maybe_emit_bars_particles(&mut self, profile: &Profile, dt_sec: f32, layer_weight: f32) {
        if dt_sec <= 0.0 || layer_weight <= 0.0 || self.cfg.bars == 0 {
            return;
        }
        let layout = self.build_bar_layout_from(profile);
        if layout.is_empty() {
            return;
        }

        let mut energy = 0.0f32;
        for &h in &self.heights {
            energy += h;
        }
        energy /= self.cfg.bars as f32;

        let target = self.cfg.particles_spawn_rate
            * dt_sec
            * layer_weight
            * (0.15 + 1.35 * energy);
        self.particle_emit_carry += target;
        let mut to_spawn = self.particle_emit_carry.floor() as usize;
        self.particle_emit_carry -= to_spawn as f32;

        let room = self.cfg.particles_max.saturating_sub(self.particles.len());
        to_spawn = to_spawn.min(room);

        let y_base = self.cfg.height as i32 - profile.bottom_padding as i32;
        let size_lo = self.cfg.particles_size_min.min(self.cfg.particles_size_max);
        let size_hi = self.cfg.particles_size_min.max(self.cfg.particles_size_max);
        let life_lo = self.cfg.particles_life_min.min(self.cfg.particles_life_max);
        let life_hi = self.cfg.particles_life_min.max(self.cfg.particles_life_max);
        let speed_lo = self.cfg.particles_speed_min.min(self.cfg.particles_speed_max);
        let speed_hi = self.cfg.particles_speed_min.max(self.cfg.particles_speed_max);

        for _ in 0..to_spawn {
            let idx = ((self.rand01() * self.cfg.bars as f32) as usize).min(self.cfg.bars - 1);
            let amp = self.heights[idx].clamp(0.0, 1.0);
            if amp <= 0.005 {
                continue;
            }
            let (x, w, h_bar) = layout[idx];
            let w_f = w.max(1) as f32;
            let left = x as f32;
            let right = x as f32 + w_f;
            let top = (y_base as f32 - (h_bar.max(1) as f32)).max(0.0);
            let bottom = y_base as f32;
            let speed = self.rand_range(speed_lo, speed_hi) * (0.75 + 1.10 * amp);
            let drift = self.rand_range(-self.cfg.particles_drift, self.cfg.particles_drift) * 0.35;
            let life = self.rand_range(life_lo, life_hi) * (0.95 + 0.65 * amp);
            let size = self.random_particle_size(size_lo, size_hi, amp);
            let alpha = self.cfg.particles_alpha * (0.50 + 0.50 * amp) * layer_weight;
            let x0 = self.rand_range(left, right);
            let y0 = self.rand_range(top, bottom.max(top + 1.0));
            let fade_j = self.cfg.particles_fade_jitter;
            let fade_start = self.rand_range(0.45, 0.90);
            let fade_power = self.rand_range(1.0, 2.8);
            let flicker_amount = fade_j * self.rand_range(0.25, 1.0);
            let flicker_speed = self.rand_range(6.0, 20.0);
            let flicker_phase = self.rand_range(0.0, std::f32::consts::TAU);
            self.spawn_particle(Particle {
                x: x0,
                y: y0,
                vx: drift,
                vy: -speed,
                life,
                age: 0.0,
                size,
                alpha: alpha.clamp(0.0, 1.0),
                fade_start,
                fade_power,
                flicker_amount,
                flicker_speed,
                flicker_phase,
            });
        }
    }

    fn maybe_emit_ring_particles(&mut self, profile: &Profile, dt_sec: f32, layer_weight: f32) {
        if dt_sec <= 0.0 || layer_weight <= 0.0 || self.cfg.bars == 0 {
            return;
        }
        let mut energy = 0.0f32;
        for &h in &self.heights {
            energy += h;
        }
        energy /= self.cfg.bars as f32;

        let target = self.cfg.particles_spawn_rate
            * dt_sec
            * layer_weight
            * (0.10 + 1.25 * energy);
        self.particle_emit_carry += target;
        let mut to_spawn = self.particle_emit_carry.floor() as usize;
        self.particle_emit_carry -= to_spawn as f32;

        let room = self.cfg.particles_max.saturating_sub(self.particles.len());
        to_spawn = to_spawn.min(room);

        let size_lo = self.cfg.particles_size_min.min(self.cfg.particles_size_max);
        let size_hi = self.cfg.particles_size_min.max(self.cfg.particles_size_max);
        let life_lo = self.cfg.particles_life_min.min(self.cfg.particles_life_max);
        let life_hi = self.cfg.particles_life_min.max(self.cfg.particles_life_max);
        let speed_lo = self.cfg.particles_speed_min.min(self.cfg.particles_speed_max);
        let speed_hi = self.cfg.particles_speed_min.max(self.cfg.particles_speed_max);

        let cx = profile.ring_x as f32;
        let cy = profile.ring_y as f32;
        let base_outer = (profile.ring_radius + profile.ring_base_thickness).max(2) as f32;
        let usable = (profile.ring_max_bar * profile.height_scale).max(1.0);
        let step = 2.0 * std::f32::consts::PI / self.cfg.bars as f32;

        for _ in 0..to_spawn {
            let idx = ((self.rand01() * self.cfg.bars as f32) as usize).min(self.cfg.bars - 1);
            let amp = self.heights[idx].clamp(0.0, 1.0);
            if amp <= 0.005 {
                continue;
            }
            let angle = idx as f32 * step - std::f32::consts::PI / 2.0;
            let nx = angle.cos();
            let ny = angle.sin();
            let tx = -ny;
            let ty = nx;

            let speed = self.rand_range(speed_lo, speed_hi) * (0.40 + 0.90 * amp);
            let tangent = self.rand_range(-self.cfg.particles_drift, self.cfg.particles_drift) * 0.18;
            let life = self.rand_range(life_lo, life_hi) * (0.95 + 0.65 * amp);
            let radial_band = (profile.ring_min_bar + amp * usable).max(2.0);
            let radial = base_outer + self.rand_range(0.0, radial_band);
            let tangent_jitter = self.rand_range(-radial_band * 0.18, radial_band * 0.18);
            let size = self.random_particle_size(size_lo, size_hi, amp);
            let alpha = self.cfg.particles_alpha * (0.50 + 0.50 * amp) * layer_weight;
            let fade_j = self.cfg.particles_fade_jitter;
            let fade_start = self.rand_range(0.40, 0.88);
            let fade_power = self.rand_range(0.9, 2.6);
            let flicker_amount = fade_j * self.rand_range(0.25, 1.0);
            let flicker_speed = self.rand_range(5.0, 18.0);
            let flicker_phase = self.rand_range(0.0, std::f32::consts::TAU);
            self.spawn_particle(Particle {
                x: cx + nx * radial + tx * tangent_jitter,
                y: cy + ny * radial + ty * tangent_jitter,
                vx: nx * speed + tx * tangent,
                vy: ny * speed + ty * tangent,
                life,
                age: 0.0,
                size,
                alpha: alpha.clamp(0.0, 1.0),
                fade_start,
                fade_power,
                flicker_amount,
                flicker_speed,
                flicker_phase,
            });
        }
    }

    fn update_particles(&mut self, dt_sec: f32) {
        if dt_sec <= 0.0 || self.particles.is_empty() {
            return;
        }
        let w = self.cfg.width as f32;
        let h = self.cfg.height as f32;
        self.particles.retain_mut(|p| {
            p.age += dt_sec;
            p.x += p.vx * dt_sec;
            p.y += p.vy * dt_sec;
            p.vx *= 0.992f32.powf(dt_sec * 60.0);
            p.vy *= 0.992f32.powf(dt_sec * 60.0);
            p.vy -= 6.0 * dt_sec;

            p.age < p.life
                && p.x >= -16.0
                && p.y >= -16.0
                && p.x <= w + 16.0
                && p.y <= h + 16.0
        });
    }

    fn particle_draw_color(&self) -> Color {
        if self.cfg.particles_color_mode == ParticleColorMode::Spectrum {
            self.draw_color
        } else {
            self.cfg.particles_color
        }
    }

    fn add_particle_pixel_to(buf: &mut [u8], width: usize, height: usize, x: i32, y: i32, alpha: f32, color: Color) {
        if x < 0 || y < 0 || x >= width as i32 || y >= height as i32 {
            return;
        }
        let a = alpha.clamp(0.0, 1.0);
        if a <= 0.0 {
            return;
        }
        let i = ((y as usize) * width + (x as usize)) * 4;

        // Slightly brighter tint than the current spectrum color.
        let tint = |c: u8| -> f32 {
            let c = c as f32;
            (c + (255.0 - c) * 0.30).clamp(0.0, 255.0)
        };
        let sr = tint(color.r);
        let sg = tint(color.g);
        let sb = tint(color.b);

        // Additive blend: never subtract alpha/color from existing spectrum pixels.
        buf[i] = (buf[i] as f32 + sr * a).clamp(0.0, 255.0) as u8;
        buf[i + 1] = (buf[i + 1] as f32 + sg * a).clamp(0.0, 255.0) as u8;
        buf[i + 2] = (buf[i + 2] as f32 + sb * a).clamp(0.0, 255.0) as u8;
        let da = buf[i + 3] as f32 / 255.0;
        let oa = (da + a * (1.0 - da)).clamp(0.0, 1.0);
        buf[i + 3] = (oa * 255.0).round() as u8;
    }

    fn draw_particle_disk_additive_to(
        buf: &mut [u8],
        width: usize,
        height: usize,
        cx: i32,
        cy: i32,
        radius: i32,
        alpha: f32,
        color: Color,
    ) {
        let r = radius.max(1);
        let rr = r * r;
        for yy in (cy - r)..=(cy + r) {
            let dy = yy - cy;
            for x in (cx - r)..=(cx + r) {
                let dx = x - cx;
                let d2 = dx * dx + dy * dy;
                if d2 > rr {
                    continue;
                }
                let edge = 1.0 - (d2 as f32 / rr as f32).clamp(0.0, 1.0);
                Self::add_particle_pixel_to(buf, width, height, x, yy, alpha * (0.35 + 0.65 * edge), color);
            }
        }
    }

    fn render_particles_to(
        out: &mut [u8],
        width: usize,
        height: usize,
        particles: &[Particle],
        enabled: bool,
        color: Color,
    ) {
        if !enabled || particles.is_empty() {
            return;
        }
        for p in particles {
            if p.life <= 0.0 {
                continue;
            }
            let t = (1.0 - p.age / p.life).clamp(0.0, 1.0);
            if t <= 0.0 {
                continue;
            }
            let fade_start = p.fade_start.clamp(0.05, 0.98);
            let fade = if t > fade_start {
                1.0
            } else {
                let u = (t / fade_start).clamp(0.0, 1.0);
                u.powf(p.fade_power.clamp(0.5, 4.0))
            };
            let flicker = 1.0 - p.flicker_amount.clamp(0.0, 1.0)
                * ((p.age * p.flicker_speed + p.flicker_phase).sin().abs());
            let a = p.alpha * fade * flicker.clamp(0.0, 1.0);
            if a <= 0.001 {
                continue;
            }
            Self::draw_particle_disk_additive_to(
                out,
                width,
                height,
                p.x.round() as i32,
                p.y.round() as i32,
                p.size.max(1),
                a,
                color,
            );
        }
    }

    fn maybe_emit_particles(&mut self, dt_sec: f32) {
        if !self.cfg.particles_enabled || self.cfg.particles_spawn_rate <= 0.0 {
            return;
        }

        if self.spectrum_mode == SpectrumMode::Group {
            let layers = self.group_layers.clone();
            for layer in &layers {
                if !layer.enabled || layer.alpha <= 0.001 {
                    continue;
                }
                let mut w = layer.alpha.clamp(0.0, 1.0);
                if layer.mode == Mode::Ring {
                    w *= self.ring_visibility;
                }
                match layer.mode {
                    Mode::Bars => self.maybe_emit_bars_particles(&layer.profile, dt_sec, w),
                    Mode::Ring => self.maybe_emit_ring_particles(&layer.profile, dt_sec, w),
                }
            }
            return;
        }

        let profile = self.profile.clone();
        match self.cfg.mode {
            Mode::Bars => self.maybe_emit_bars_particles(&profile, dt_sec, 1.0),
            Mode::Ring => self.maybe_emit_ring_particles(&profile, dt_sec, self.ring_visibility),
        }
    }

    fn postfx_blur_cross(src: &[u8], dst: &mut [u8], w: usize, h: usize) {
        if src.len() < w * h * 4 || dst.len() < w * h * 4 {
            return;
        }
        for y in 0..h {
            for x in 0..w {
                let mut r = 0u32;
                let mut g = 0u32;
                let mut b = 0u32;
                let mut a = 0u32;
                let mut n = 0u32;
                let mut sample = |sx: usize, sy: usize| {
                    let i = (sy * w + sx) * 4;
                    r += src[i] as u32;
                    g += src[i + 1] as u32;
                    b += src[i + 2] as u32;
                    a += src[i + 3] as u32;
                    n += 1;
                };

                sample(x, y);
                if x > 0 {
                    sample(x - 1, y);
                }
                if x + 1 < w {
                    sample(x + 1, y);
                }
                if y > 0 {
                    sample(x, y - 1);
                }
                if y + 1 < h {
                    sample(x, y + 1);
                }

                let o = (y * w + x) * 4;
                dst[o] = (r / n) as u8;
                dst[o + 1] = (g / n) as u8;
                dst[o + 2] = (b / n) as u8;
                dst[o + 3] = (a / n) as u8;
            }
        }
    }

    fn apply_postfx_with(&mut self, fx: PostFxParams) {
        if !fx.enabled {
            return;
        }

        let w = self.cfg.width;
        let h = self.cfg.height;
        let px = w * h;
        if self.frame.len() < px * 4 {
            return;
        }

        self.postfx_a.copy_from_slice(&self.frame);
        self.postfx_b.fill(0);

        let passes = fx.blur_passes.min(4);
        for _ in 0..passes {
            Self::postfx_blur_cross(&self.postfx_a, &mut self.postfx_b, w, h);
            std::mem::swap(&mut self.postfx_a, &mut self.postfx_b);
        }

        let blur_mix = fx.blur_mix.clamp(0.0, 1.0);
        let glow_mix = fx.glow_mix.clamp(0.0, 1.0);
        let glow_strength = fx.glow_strength.clamp(0.0, 3.0);
        let glow_scale = glow_mix * glow_strength;

        for i in 0..px {
            let o = i * 4;
            let or = self.frame[o] as f32 / 255.0;
            let og = self.frame[o + 1] as f32 / 255.0;
            let ob = self.frame[o + 2] as f32 / 255.0;
            let oa = self.frame[o + 3] as f32 / 255.0;

            let br = self.postfx_a[o] as f32 / 255.0;
            let bg = self.postfx_a[o + 1] as f32 / 255.0;
            let bb = self.postfx_a[o + 2] as f32 / 255.0;
            let ba = self.postfx_a[o + 3] as f32 / 255.0;

            let mut r = or * (1.0 - blur_mix) + br * blur_mix;
            let mut g = og * (1.0 - blur_mix) + bg * blur_mix;
            let mut b = ob * (1.0 - blur_mix) + bb * blur_mix;
            let a = (oa + ba * glow_mix).clamp(0.0, 1.0);

            // Additive halo from blurred contribution.
            r = (r + br * glow_scale).clamp(0.0, 1.0);
            g = (g + bg * glow_scale).clamp(0.0, 1.0);
            b = (b + bb * glow_scale).clamp(0.0, 1.0);

            self.frame[o] = (r * 255.0).round() as u8;
            self.frame[o + 1] = (g * 255.0).round() as u8;
            self.frame[o + 2] = (b * 255.0).round() as u8;
            self.frame[o + 3] = (a * 255.0).round() as u8;
        }
    }

    fn render_layer_cpu(&mut self, layer: &SpectrumLayer) {
        let prev_mode = self.cfg.mode;
        let prev_bars_style = self.cfg.bars_style;
        let prev_ring_style = self.cfg.ring_style;
        let prev_profile = self.profile.clone();
        let prev_color = self.draw_color;
        let prev_alpha = self.draw_alpha_scale;

        self.clear_frame();
        self.cfg.mode = layer.mode;
        match layer.mode {
            Mode::Bars => self.cfg.bars_style = layer.style,
            Mode::Ring => self.cfg.ring_style = layer.style,
        }
        self.profile = layer.profile.clone();
        self.draw_color = layer.color;
        let ring_vis = if layer.mode == Mode::Ring {
            self.ring_visibility
        } else {
            1.0
        };
        self.draw_alpha_scale = (layer.alpha * ring_vis).clamp(0.0, 1.0);

        match layer.mode {
            Mode::Bars => self.draw_bars(),
            Mode::Ring => self.draw_ring(),
        }

        self.cfg.mode = prev_mode;
        self.cfg.bars_style = prev_bars_style;
        self.cfg.ring_style = prev_ring_style;
        self.profile = prev_profile;
        self.draw_color = prev_color;
        self.draw_alpha_scale = prev_alpha;
    }

    fn render_layer_gpu(&mut self, layer: &SpectrumLayer) -> io::Result<()> {
        // bars_fill currently looks inconsistent in the GPU shader path; keep it on CPU.
        if layer.mode == Mode::Bars && layer.style == RenderStyle::BarsFill {
            return Err(io::Error::other("bars_fill layer forced to cpu"));
        }
        let mut cfg_tmp = self.cfg.clone();
        cfg_tmp.mode = layer.mode;
        match layer.mode {
            Mode::Bars => cfg_tmp.bars_style = layer.style,
            Mode::Ring => cfg_tmp.ring_style = layer.style,
        }
        let alpha = if layer.mode == Mode::Ring {
            layer.alpha * self.ring_visibility
        } else {
            layer.alpha
        }
        .clamp(0.0, 1.0);
        self.frame.fill(0);
        let gpu = self
            .gpu_renderer
            .as_mut()
            .ok_or_else(|| io::Error::other("gpu renderer missing"))?;
        gpu.render(
            &cfg_tmp,
            &layer.profile,
            &self.heights,
            layer.color,
            alpha,
            &mut self.frame,
        )
    }

    fn maybe_reload_group_file(&mut self) {
        if self.spectrum_mode != SpectrumMode::Group {
            return;
        }
        if self.group_last_check.elapsed() < Duration::from_millis(self.group_poll_ms) {
            return;
        }
        self.group_last_check = Instant::now();
        let meta = match fs::metadata(&self.cfg.group_file) {
            Ok(m) => m,
            Err(_) => return,
        };
        let mtime = meta.modified().ok();
        if mtime.is_some() && mtime == self.group_last_mtime {
            return;
        }
        self.group_last_mtime = mtime;
        match load_spectrum_group(&self.cfg) {
            Ok(layers) => {
                self.group_layers = layers;
                eprintln!(
                    "[group] reloaded {} layers from {}",
                    self.group_layers.len(),
                    self.cfg.group_file.display()
                );
            }
            Err(e) => eprintln!("[group] reload failed: {}", e),
        }
    }

    fn maybe_update_group_layers_runtime(&mut self) {
        if self.spectrum_mode != SpectrumMode::Group {
            return;
        }
        let now = Instant::now();
        for layer in &mut self.group_layers {
            if !layer.enabled {
                continue;
            }
            match layer.runtime_mode {
                RuntimeMode::Standard => {
                    if layer.rotate_profiles
                        && layer.profiles.len() > 1
                        && layer.profile_last_switch.elapsed().as_secs() >= self.cfg.rotation_seconds
                    {
                        layer.profile_last_switch = now;
                        layer.profile_index = (layer.profile_index + 1) % layer.profiles.len();
                        let name = &layer.profiles[layer.profile_index];
                        match load_profile(&self.cfg, name) {
                            Ok(p) => {
                                layer.profile = p;
                                eprintln!("[group] layer rotate -> {}", name);
                            }
                            Err(e) => eprintln!("[group] layer rotate failed {}: {}", name, e),
                        }
                    }
                }
                RuntimeMode::Test => {
                    let path = layer
                        .test_profile_file
                        .clone()
                        .unwrap_or_else(|| self.cfg.test_profile_file.clone());
                    if layer.test_profile_last_check.elapsed()
                        < Duration::from_millis(self.cfg.test_profile_poll_ms)
                    {
                        continue;
                    }
                    layer.test_profile_last_check = now;
                    let meta = match fs::metadata(&path) {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let mtime = meta.modified().ok();
                    if mtime.is_some() && mtime == layer.test_profile_last_mtime {
                        continue;
                    }
                    layer.test_profile_last_mtime = mtime;
                    match load_profile_from_path(&self.cfg, &path) {
                        Ok(p) => {
                            layer.profile = p;
                            eprintln!("[group] layer test reloaded {}", path.display());
                        }
                        Err(e) => eprintln!("[group] layer test reload failed {}: {}", path.display(), e),
                    }
                }
            }
        }
    }

    fn render_group(&mut self) {
        self.maybe_reload_group_file();
        self.maybe_update_group_layers_runtime();
        self.composite_frame.fill(0);
        let particle_color = self.particle_draw_color();
        let particles_back = self.cfg.particles_enabled && self.cfg.particles_layer == ParticleLayer::Back;
        let particles_front = self.cfg.particles_enabled && self.cfg.particles_layer == ParticleLayer::Front;
        if particles_back {
            Self::render_particles_to(
                &mut self.composite_frame,
                self.cfg.width,
                self.cfg.height,
                &self.particles,
                self.cfg.particles_enabled,
                particle_color,
            );
        }
        let layers = self.group_layers.clone();
        let scope = self.cfg.postfx_scope;
        let apply_layer_fx = matches!(scope, PostFxScope::Layer | PostFxScope::Mixed);
        let apply_final_fx = matches!(scope, PostFxScope::Final | PostFxScope::Mixed);
        for layer in &layers {
            if !layer.enabled {
                continue;
            }
            let ok = if self.backend == RenderBackend::Gpu {
                self.render_layer_gpu(layer).is_ok()
            } else {
                self.render_layer_cpu(layer);
                true
            };
            if !ok {
                eprintln!("[group] gpu layer render failed, fallback cpu");
                self.backend = RenderBackend::Cpu;
                self.gpu_renderer = None;
                self.render_layer_cpu(layer);
            }
            if apply_layer_fx
                && let Some(fx) = layer.postfx
            {
                self.apply_postfx_with(fx);
            }
            Self::blend_over(&mut self.composite_frame, &self.frame);
        }
        if particles_front {
            self.frame.fill(0);
            Self::render_particles_to(
                &mut self.frame,
                self.cfg.width,
                self.cfg.height,
                &self.particles,
                self.cfg.particles_enabled,
                particle_color,
            );
            Self::blend_over(&mut self.composite_frame, &self.frame);
        }
        self.frame.copy_from_slice(&self.composite_frame);
        if apply_final_fx {
            self.apply_postfx_with(self.global_postfx_params());
        }
    }

    fn step(&mut self, bins: &[f32], dt_sec: f32, audio_last_update: Instant) {
        self.maybe_update_dynamic_color();
        self.update_color_fade(dt_sec);

        self.history.push_back(bins.to_vec());
        let keep = self.profile.avg_frames.max(1) + 2;
        while self.history.len() > keep {
            self.history.pop_front();
        }

        let t = self.temporal_average(self.profile.avg_frames);
        let s = self.spatial_smooth(&t, self.profile.smooth_radius);
        let w = self.compute_weighted(&s);
        self.update_heights(&w, dt_sec, audio_last_update);
        self.update_ring_visibility(&w, dt_sec, audio_last_update);
        self.update_particles(dt_sec);
        self.maybe_emit_particles(dt_sec);

        if self.spectrum_mode == SpectrumMode::Group {
            self.render_group();
            return;
        }

        let mut rendered = false;
        let force_cpu_plain = self.cfg.mode == Mode::Bars && self.cfg.bars_style == RenderStyle::BarsFill;
        if self.backend == RenderBackend::Gpu && !force_cpu_plain {
            if let Some(gpu) = self.gpu_renderer.as_mut() {
                if gpu
                    .render(
                        &self.cfg,
                        &self.profile,
                        &self.heights,
                        self.draw_color,
                        self.draw_alpha_scale,
                        &mut self.frame,
                    )
                    .is_ok()
                {
                    rendered = true;
                } else {
                    eprintln!("[gpu] render failed, fallback to cpu");
                    self.backend = RenderBackend::Cpu;
                    self.gpu_renderer = None;
                }
            } else {
                self.backend = RenderBackend::Cpu;
            }
        }

        let particles_back = self.cfg.particles_enabled && self.cfg.particles_layer == ParticleLayer::Back;
        let particles_front = self.cfg.particles_enabled && self.cfg.particles_layer == ParticleLayer::Front;
        let particle_color = self.particle_draw_color();

        if !rendered {
            self.clear_frame();
            if particles_back {
                Self::render_particles_to(
                    &mut self.frame,
                    self.cfg.width,
                    self.cfg.height,
                    &self.particles,
                    self.cfg.particles_enabled,
                    particle_color,
                );
            }
            match self.cfg.mode {
                Mode::Bars => self.draw_bars(),
                Mode::Ring => {
                    if self.ring_visibility > 0.002 {
                        self.draw_ring();
                    }
                }
            }
        } else if particles_back {
            self.postfx_a.fill(0);
            Self::render_particles_to(
                &mut self.postfx_a,
                self.cfg.width,
                self.cfg.height,
                &self.particles,
                self.cfg.particles_enabled,
                particle_color,
            );
            Self::blend_over(&mut self.postfx_a, &self.frame);
            self.frame.copy_from_slice(&self.postfx_a);
        }

        if particles_front {
            Self::render_particles_to(
                &mut self.frame,
                self.cfg.width,
                self.cfg.height,
                &self.particles,
                self.cfg.particles_enabled,
                particle_color,
            );
        }
        let skip_postfx_plain_bars = self.cfg.postfx_skip_plain_bars
            && self.cfg.mode == Mode::Bars
            && matches!(self.cfg.bars_style, RenderStyle::Bars | RenderStyle::BarsFill);
        if !skip_postfx_plain_bars {
            // In single mode always allow global postfx regardless of scope.
            self.apply_postfx_with(self.global_postfx_params());
        }
    }
}

fn arg_config_path() -> PathBuf {
    let args: Vec<String> = env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--config")
        && let Some(v) = args.get(i + 1)
    {
        return PathBuf::from(v);
    }
    PathBuf::from("./config/base.conf")
}

fn main() -> io::Result<()> {
    let cfg_path = arg_config_path();
    let cfg = load_app_config(&cfg_path)?;

    let (profile_names, profile, active_profile_label, standard_first): (
        Vec<String>,
        Profile,
        String,
        Option<String>,
    ) = if cfg.spectrum_mode == SpectrumMode::Group {
        (
            Vec::new(),
            Profile::defaults(&cfg),
            cfg.group_file.to_string_lossy().to_string(),
            None,
        )
    } else {
        let profile_names = if cfg.mode == Mode::Ring {
            cfg.ring_profiles.clone()
        } else {
            cfg.bars_profiles.clone()
        };

        if profile_names.is_empty() && cfg.runtime_mode == RuntimeMode::Standard {
            return Err(io::Error::other("No profiles configured"));
        }

        let standard_first = if !cfg.static_profile.trim().is_empty() {
            cfg.static_profile.trim().to_string()
        } else {
            profile_names
                .first()
                .cloned()
                .unwrap_or_else(|| "bars_balanced".to_string())
        };

        if cfg.runtime_mode == RuntimeMode::Test && !cfg.test_profile_file.exists() {
            let source = cfg.profile_dir.join(format!("{}.profile", standard_first));
            if source.exists() {
                fs::copy(&source, &cfg.test_profile_file)?;
                eprintln!(
                    "[test] creado {} desde {}",
                    cfg.test_profile_file.display(),
                    source.display()
                );
            } else {
                fs::write(&cfg.test_profile_file, b"")?;
                eprintln!(
                    "[test] creado archivo vacio {}",
                    cfg.test_profile_file.display()
                );
            }
        }

        let profile = if cfg.runtime_mode == RuntimeMode::Test {
            load_profile_from_path(&cfg, &cfg.test_profile_file)?
        } else {
            load_profile(&cfg, &standard_first)?
        };
        let active_profile_label = if cfg.runtime_mode == RuntimeMode::Test {
            cfg.test_profile_file.to_string_lossy().to_string()
        } else {
            standard_first.clone()
        };
        (profile_names, profile, active_profile_label, Some(standard_first))
    };

    eprintln!(
        "[boot] backend={:?} spectrum={:?} mode={:?} runtime={:?} {}x{} fps={} bars={} monitor={} profile={} rotate={} dyn_color={} poll={}s postfx={} scope={:?} blur_passes={} blur_mix={:.2} glow={:.2}/{:.2} particles={} layer={:?} color_mode={:?} color=#{:02X}{:02X}{:02X} max={} rate={:.0}/s life={:.2}-{:.2}s",
        cfg.backend,
        cfg.spectrum_mode,
        cfg.mode,
        cfg.runtime_mode,
        cfg.width,
        cfg.height,
        cfg.fps,
        cfg.bars,
        cfg.monitor,
        active_profile_label,
        cfg.rotate_profiles,
        cfg.dynamic_color,
        cfg.color_poll_seconds,
        cfg.postfx_enabled,
        cfg.postfx_scope,
        cfg.postfx_blur_passes,
        cfg.postfx_blur_mix,
        cfg.postfx_glow_strength,
        cfg.postfx_glow_mix,
        cfg.particles_enabled,
        cfg.particles_layer,
        cfg.particles_color_mode,
        cfg.particles_color.r,
        cfg.particles_color.g,
        cfg.particles_color.b,
        cfg.particles_max,
        cfg.particles_spawn_rate,
        cfg.particles_life_min,
        cfg.particles_life_max
    );

    let audio_state = Arc::new(Mutex::new(AudioState {
        latest_bins: vec![0.0; cfg.bars],
        counter: 0,
        last_update: Instant::now(),
    }));

    spawn_cava_reader(cfg.fifo_cava.clone(), cfg.bars, Arc::clone(&audio_state));

    let mut engine = Engine::new(cfg.clone(), profile_names.clone(), profile);
    if let Some(standard_first) = standard_first
        && let Some(idx) = profile_names.iter().position(|n| n == &standard_first)
    {
        engine.profile_index = idx;
    }
    let frame_ms = (1000.0 / cfg.fps as f32).max(1.0);

    let mut out = open_fifo_writer(&cfg.fifo_video)?;
    let mut last_tick = Instant::now();

    loop {
        let now = Instant::now();
        let dt = (now - last_tick).as_secs_f32().clamp(0.001, 0.2);
        last_tick = now;

        if engine.spectrum_mode == SpectrumMode::Single {
            engine.maybe_reload_test_profile();
            engine.maybe_rotate_profile();
        }

        let (bins, last_update) = match audio_state.lock() {
            Ok(s) => (s.latest_bins.clone(), s.last_update),
            Err(poisoned) => {
                eprintln!("[audio] mutex poisoned; continuing with last known state");
                let s = poisoned.into_inner();
                (s.latest_bins.clone(), s.last_update)
            }
        };

        engine.step(&bins, dt, last_update);

        if let Err(e) = out.write_all(&engine.frame) {
            eprintln!("[renderer] write error, reopening fifo: {}", e);
            out = open_fifo_writer(&cfg.fifo_video)?;
        }

        let elapsed_ms = now.elapsed().as_secs_f32() * 1000.0;
        if elapsed_ms < frame_ms {
            thread::sleep(Duration::from_millis((frame_ms - elapsed_ms) as u64));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Mode, RenderBackend, RenderStyle, RuntimeMode, SpectrumMode, parse_boolish,
        parse_hex_color,
    };

    #[test]
    fn parse_hex_color_valid_value() {
        let c = parse_hex_color("#102030");
        assert_eq!(c.r, 0x10);
        assert_eq!(c.g, 0x20);
        assert_eq!(c.b, 0x30);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn parse_hex_color_invalid_uses_default() {
        let c = parse_hex_color("not-a-color");
        assert_eq!(c.r, 0xA6);
        assert_eq!(c.g, 0x0C);
        assert_eq!(c.b, 0xDB);
        assert_eq!(c.a, 255);
    }

    #[test]
    fn parse_boolish_supports_expected_values() {
        assert_eq!(parse_boolish("1"), Some(true));
        assert_eq!(parse_boolish("false"), Some(false));
        assert_eq!(parse_boolish("unknown"), None);
    }

    #[test]
    fn enum_parsers_fallback_to_defaults() {
        assert_eq!(Mode::from_str("ring"), Mode::Ring);
        assert_eq!(Mode::from_str("???"), Mode::Bars);
        assert_eq!(RuntimeMode::from_str("test"), RuntimeMode::Test);
        assert_eq!(RuntimeMode::from_str("???"), RuntimeMode::Standard);
        assert_eq!(RenderBackend::from_str("gpu"), RenderBackend::Gpu);
        assert_eq!(RenderBackend::from_str("???"), RenderBackend::Cpu);
        assert_eq!(SpectrumMode::from_str("group"), SpectrumMode::Group);
        assert_eq!(SpectrumMode::from_str("???"), SpectrumMode::Single);
        assert_eq!(RenderStyle::from_str("waves_fill"), RenderStyle::WavesFill);
        assert_eq!(RenderStyle::from_str("???"), RenderStyle::Bars);
    }
}
