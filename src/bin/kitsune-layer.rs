use memmap2::MmapMut;
use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::AsFd;
use wayland_client::globals::{GlobalListContents, registry_queue_init};
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_compositor::WlCompositor;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_shm::{self, WlShm};
use wayland_client::protocol::wl_shm_pool::WlShmPool;
use wayland_client::protocol::wl_surface::WlSurface;
use wayland_client::{Connection, Dispatch, QueueHandle, delegate_noop};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_shell_v1::{
    Layer, ZwlrLayerShellV1,
};
use wayland_protocols_wlr::layer_shell::v1::client::zwlr_layer_surface_v1::{
    Anchor, Event as LayerEvent, KeyboardInteractivity, ZwlrLayerSurfaceV1,
};

#[derive(Debug, Clone)]
struct Args {
    fifo_video: String,
    width: u32,
    height: u32,
    monitor: Option<String>,
}

impl Args {
    fn from_env() -> Self {
        let mut fifo_video = "/tmp/kitsune-spectrum.rgba".to_string();
        let mut width = 1920u32;
        let mut height = 1080u32;
        let mut monitor = None;

        let mut it = env::args().skip(1);
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--fifo" => {
                    if let Some(v) = it.next() {
                        fifo_video = v;
                    }
                }
                "--width" => {
                    if let Some(v) = it.next() {
                        if let Ok(n) = v.parse::<u32>() {
                            width = n;
                        }
                    }
                }
                "--height" => {
                    if let Some(v) = it.next() {
                        if let Ok(n) = v.parse::<u32>() {
                            height = n;
                        }
                    }
                }
                "--monitor" => {
                    monitor = it.next();
                }
                _ => {}
            }
        }

        Self {
            fifo_video,
            width,
            height,
            monitor,
        }
    }
}

#[derive(Debug, Clone)]
struct OutputInfo {
    global_name: u32,
    wl: WlOutput,
    name: Option<String>,
}

struct AppState {
    configured: bool,
    configured_width: u32,
    configured_height: u32,
    outputs: Vec<OutputInfo>,
    monitor_name: Option<String>,
}

impl AppState {
    fn new(monitor_name: Option<String>) -> Self {
        Self {
            configured: false,
            configured_width: 0,
            configured_height: 0,
            outputs: Vec::new(),
            monitor_name,
        }
    }

    fn bind_output(
        &mut self,
        registry: &WlRegistry,
        qh: &QueueHandle<Self>,
        global_name: u32,
        version: u32,
    ) {
        let bind_version = version.min(4);
        let output = registry.bind::<WlOutput, _, _>(global_name, bind_version, qh, ());
        self.outputs.push(OutputInfo {
            global_name,
            wl: output,
            name: None,
        });
    }

    fn remove_output_global(&mut self, global_name: u32) {
        self.outputs.retain(|o| o.global_name != global_name);
    }
}

impl Dispatch<WlOutput, ()> for AppState {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: wayland_client::protocol::wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wayland_client::protocol::wl_output::Event::Name { name } = event {
            if let Some(info) = state.outputs.iter_mut().find(|o| o.wl == *proxy) {
                info.name = Some(name);
            }
        }
    }
}

impl Dispatch<ZwlrLayerSurfaceV1, ()> for AppState {
    fn event(
        state: &mut Self,
        proxy: &ZwlrLayerSurfaceV1,
        event: LayerEvent,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            LayerEvent::Configure {
                serial,
                width,
                height,
            } => {
                proxy.ack_configure(serial);
                state.configured = true;
                if width > 0 {
                    state.configured_width = width;
                }
                if height > 0 {
                    state.configured_height = height;
                }
            }
            LayerEvent::Closed => {
                eprintln!("[layer] compositor closed surface");
                std::process::exit(0);
            }
            _ => {}
        }
    }
}

delegate_noop!(AppState: ignore WlCompositor);
delegate_noop!(AppState: ignore WlShm);
delegate_noop!(AppState: ignore WlShmPool);
delegate_noop!(AppState: ignore WlBuffer);
delegate_noop!(AppState: ignore WlSurface);
delegate_noop!(AppState: ignore ZwlrLayerShellV1);

fn open_fifo_reader(path: &str) -> io::Result<File> {
    File::open(path)
}

struct ShmFrame {
    _file: File,
    _pool: WlShmPool,
    buffer: WlBuffer,
    map: MmapMut,
    width: u32,
    height: u32,
}

fn create_shm_frame(
    shm: &WlShm,
    qh: &QueueHandle<AppState>,
    width: u32,
    height: u32,
) -> io::Result<ShmFrame> {
    let stride = (width as usize) * 4;
    let size = stride * (height as usize);

    let file = tempfile::tempfile()?;
    file.set_len(size as u64)?;
    let map = unsafe { MmapMut::map_mut(&file)? };

    let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());
    let buffer = pool.create_buffer(
        0,
        width as i32,
        height as i32,
        stride as i32,
        wl_shm::Format::Argb8888,
        qh,
        (),
    );

    Ok(ShmFrame {
        _file: file,
        _pool: pool,
        buffer,
        map,
        width,
        height,
    })
}

fn convert_rgba_to_argb8888(src: &[u8], dst: &mut [u8]) {
    let px = src.len() / 4;
    for i in 0..px {
        let s = i * 4;
        let d = i * 4;
        let r = src[s];
        let g = src[s + 1];
        let b = src[s + 2];
        let a = src[s + 3];
        dst[d] = b;
        dst[d + 1] = g;
        dst[d + 2] = r;
        dst[d + 3] = a;
    }
}

fn effective_size(state: &AppState, fallback_width: u32, fallback_height: u32) -> (u32, u32) {
    let width = if state.configured_width > 0 {
        state.configured_width
    } else {
        fallback_width
    };
    let height = if state.configured_height > 0 {
        state.configured_height
    } else {
        fallback_height
    };
    (width, height)
}

fn pick_output(state: &AppState) -> Option<WlOutput> {
    if let Some(ref wanted) = state.monitor_name {
        for out in &state.outputs {
            if out.name.as_deref() == Some(wanted.as_str()) {
                eprintln!("[layer] selected output by monitor name: {}", wanted);
                return Some(out.wl.clone());
            }
        }
        eprintln!(
            "[layer] monitor '{}' not found; using compositor default output",
            wanted
        );
    }
    state.outputs.first().map(|o| o.wl.clone())
}

impl Dispatch<WlRegistry, GlobalListContents> for AppState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == "wl_output" {
                    state.bind_output(registry, qh, name, version);
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                state.remove_output_global(name);
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::from_env();
    eprintln!(
        "[layer] starting fifo={} size={}x{} monitor={}",
        args.fifo_video,
        args.width,
        args.height,
        args.monitor.clone().unwrap_or_else(|| "<auto>".to_string())
    );

    let conn = Connection::connect_to_env()?;
    let (globals, mut event_queue) = registry_queue_init::<AppState>(&conn)?;
    let qh = event_queue.handle();

    let compositor: WlCompositor = globals.bind(&qh, 4..=6, ())?;
    let shm: WlShm = globals.bind(&qh, 1..=1, ())?;
    let layer_shell: ZwlrLayerShellV1 = globals.bind(&qh, 1..=4, ())?;

    let mut state = AppState::new(args.monitor.clone());
    let registry = globals.registry();
    for g in globals.contents().clone_list() {
        if g.interface == "wl_output" {
            state.bind_output(registry, &qh, g.name, g.version);
        }
    }

    let _ = event_queue.roundtrip(&mut state);
    let _ = event_queue.roundtrip(&mut state);

    let wl_surface = compositor.create_surface(&qh, ());
    let wl_output = pick_output(&state);

    let layer_surface = layer_shell.get_layer_surface(
        &wl_surface,
        wl_output.as_ref(),
        Layer::Bottom,
        "kitsune".to_string(),
        &qh,
        (),
    );

    layer_surface.set_anchor(Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right);
    layer_surface.set_exclusive_zone(-1);
    layer_surface.set_size(args.width, args.height);
    layer_surface.set_keyboard_interactivity(KeyboardInteractivity::None);
    wl_surface.commit();

    while !state.configured {
        event_queue.blocking_dispatch(&mut state)?;
    }

    let (width, height) = effective_size(&state, args.width, args.height);

    eprintln!("[layer] configured surface {}x{}", width, height);
    let mut shm_frame = create_shm_frame(&shm, &qh, width, height)?;

    let frame_len = (width as usize) * (height as usize) * 4;
    let mut rgba_frame = vec![0u8; frame_len];

    loop {
        let mut reader = match open_fifo_reader(&args.fifo_video) {
            Ok(f) => f,
            Err(err) => {
                eprintln!("[layer] fifo open error: {err}");
                std::thread::sleep(std::time::Duration::from_millis(300));
                continue;
            }
        };

        loop {
            let _ = event_queue.dispatch_pending(&mut state);
            let (target_width, target_height) = effective_size(&state, args.width, args.height);
            if target_width != shm_frame.width || target_height != shm_frame.height {
                eprintln!(
                    "[layer] resize detected: {}x{} -> {}x{}",
                    shm_frame.width, shm_frame.height, target_width, target_height
                );
                shm_frame = create_shm_frame(&shm, &qh, target_width, target_height)?;
                rgba_frame.resize((target_width as usize) * (target_height as usize) * 4, 0);
            }

            if let Err(err) = reader.read_exact(&mut rgba_frame) {
                eprintln!("[layer] fifo read error, reopening: {err}");
                break;
            }

            convert_rgba_to_argb8888(&rgba_frame, &mut shm_frame.map);
            wl_surface.attach(Some(&shm_frame.buffer), 0, 0);
            wl_surface.damage_buffer(0, 0, shm_frame.width as i32, shm_frame.height as i32);
            wl_surface.commit();

            let _ = event_queue.dispatch_pending(&mut state);
            let _ = conn.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::convert_rgba_to_argb8888;

    #[test]
    fn convert_rgba_to_argb8888_converts_channels() {
        let src = vec![0x11, 0x22, 0x33, 0x44, 0xAA, 0xBB, 0xCC, 0xDD];
        let mut dst = vec![0u8; src.len()];
        convert_rgba_to_argb8888(&src, &mut dst);
        assert_eq!(dst, vec![0x33, 0x22, 0x11, 0x44, 0xCC, 0xBB, 0xAA, 0xDD]);
    }
}
