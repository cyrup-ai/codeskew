#![allow(deprecated)]
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use winit::window::WindowAttributes;

#[cfg(target_arch = "wasm32")]
use raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawWindowHandle,
    WebDisplayHandle, WebWindowHandle, WindowHandle,
};

pub struct WgpuContext {
    #[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
    pub event_loop: Option<winit::event_loop::EventLoop<()>>,
    #[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
    pub window: Arc<winit::window::Window>,
    pub device: Arc<wgpu::Device>,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
}

#[cfg(target_arch = "wasm32")]
struct CanvasWindow {
    id: u32,
}

#[cfg(target_arch = "wasm32")]
impl HasWindowHandle for CanvasWindow {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        unsafe {
            Ok(WindowHandle::borrow_raw(RawWindowHandle::Web(
                WebWindowHandle::new(self.id),
            )))
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl HasDisplayHandle for CanvasWindow {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        // FIXME: Use raw_window_handle::DisplayHandle::<'static>::web() once a new version of raw_window_handle is released
        unsafe { Ok(DisplayHandle::borrow_raw(WebDisplayHandle::new().into())) }
    }
}

#[cfg(target_arch = "wasm32")]
fn init_window(bind_id: &str) -> Result<CanvasWindow, Box<dyn std::error::Error>> {
    use crate::utils::set_panic_hook;
    console_log::init(); // FIXME only do this once
    set_panic_hook();
    let win = web_sys::window().ok_or("window is None")?;
    let doc = win.document().ok_or("document is None")?;
    let element = doc
        .get_element_by_id(bind_id)
        .ok_or(format!("cannot find element {bind_id}"))?;
    use wasm_bindgen::JsCast;
    let canvas = element
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .or(Err("cannot cast to canvas"))?;
    canvas
        .get_context("webgpu")
        .or(Err("no webgpu"))?
        .ok_or("no webgpu")?;
    canvas
        .set_attribute("data-raw-handle", "42")
        .or(Err("cannot set attribute"))?;
    Ok(CanvasWindow { id: 42 })
}

#[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
#[allow(deprecated)]
#[allow(dead_code)] // Demo function for WASM toy examples
fn init_window(
    size: winit::dpi::Size,
    event_loop: &winit::event_loop::ActiveEventLoop,
) -> Result<Arc<winit::window::Window>, Box<dyn std::error::Error>> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );
    let window_attributes = winit::window::WindowAttributes::default()
        .with_inner_size(size)
        .with_transparent(true)
        .with_decorations(false);
    let window = event_loop.create_window(window_attributes)?;
    Ok(Arc::new(window))
}

#[cfg(feature = "winit")]
#[allow(deprecated)]
pub async fn init_wgpu(width: u32, height: u32, _bind_id: &str) -> Result<WgpuContext, String> {
    #[cfg(not(target_arch = "wasm32"))]
    let event_loop = winit::event_loop::EventLoop::new().map_err(|e| e.to_string())?;
    #[cfg(not(target_arch = "wasm32"))]
    let window =
        Arc::new(
            event_loop
                .create_window(WindowAttributes::default()
                    .with_inner_size(winit::dpi::Size::Physical(winit::dpi::PhysicalSize::new(width, height)))
                    .with_transparent(true)
                    .with_decorations(false))
                .map_err(|e| e.to_string())?,
        );

    #[cfg(target_arch = "wasm32")]
    let window = init_window(bind_id).map_err(|e| e.to_string())?;

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        flags: wgpu::InstanceFlags::default(),
        backend_options: Default::default(),
    });

    let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap())
    }
    .map_err(|e| e.to_string())?;

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .map_err(|e| format!("unable to create adapter: {e:?}"))?;

    log::info!("adapter.features = {:#?}", adapter.features());
    log::info!("adapter.limits = {:#?}", adapter.limits());

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some("GPU Device"),
            required_features: adapter.features(),
            ..Default::default()
        })
        .await
        .map_err(|e| e.to_string())?;

    let surface_format = preferred_framebuffer_format(&surface.get_capabilities(&adapter).formats);
    let surface_config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width,
        height,
        present_mode: wgpu::PresentMode::Fifo, // vsync
        alpha_mode: wgpu::CompositeAlphaMode::PostMultiplied,
        view_formats: vec![
            surface_format.add_srgb_suffix(),
            surface_format.remove_srgb_suffix(),
        ],
        desired_maximum_frame_latency: 1,
    };
    surface.configure(&device, &surface_config);

    Ok(WgpuContext {
        #[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
        event_loop: Some(event_loop),
        #[cfg(all(not(target_arch = "wasm32"), feature = "winit"))]
        window,
        device: Arc::new(device),
        queue,
        surface,
        surface_config,
    })
}

fn preferred_framebuffer_format(formats: &[wgpu::TextureFormat]) -> wgpu::TextureFormat {
    // Prioritize Rgba8UnormSrgb for compatibility with render targets
    for &format in formats {
        if format == wgpu::TextureFormat::Rgba8UnormSrgb {
            return format;
        }
    }
    // Fallback to Rgba8Unorm if Srgb not available
    for &format in formats {
        if format == wgpu::TextureFormat::Rgba8Unorm {
            return format;
        }
    }
    // Last resort fallbacks
    for &format in formats {
        if matches!(
            format,
            wgpu::TextureFormat::Bgra8UnormSrgb | wgpu::TextureFormat::Bgra8Unorm
        ) {
            return format;
        }
    }
    formats[0]
}
