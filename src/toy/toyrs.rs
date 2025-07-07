use std::error::Error;

#[allow(dead_code)] // Demo entry point for toy examples
fn main() -> Result<(), Box<dyn Error>> {
    #[cfg(not(feature = "winit"))]
    return Err("must be compiled with winit feature to run".into());

    #[cfg(all(feature = "winit", not(target_arch = "wasm32")))]
    return winit::main();

    #[cfg(all(feature = "winit", target_arch = "wasm32"))]
    return Err("winit not supported on wasm target".into());
}

#[cfg(all(feature = "winit", not(target_arch = "wasm32")))]
mod winit {
    use crate::toy::{WgpuToyRenderer, context::init_wgpu};
    use http_cache_reqwest::{CACacheManager, Cache, CacheMode, HttpCache, HttpCacheOptions};
    use serde::{Deserialize, Serialize};
    use std::{
        borrow::Cow,
        error::Error,
        path::{Path, PathBuf},
        sync::atomic::{AtomicBool, Ordering},
    };
    use winit::{
        application::ApplicationHandler,
        event::{ElementState, KeyEvent, WindowEvent},
        event_loop::{ActiveEventLoop, EventLoop},
        keyboard::{KeyCode, PhysicalKey},
        window::WindowId,
    };

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    struct ShaderMeta {
        uniforms: Vec<Uniform>,
        textures: Vec<Texture>,
        #[serde(default)]
        float32_enabled: bool,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct Uniform {
        name: String,
        value: f32,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct Texture {
        img: String,
    }

    #[allow(dead_code)] // Demo configuration constants
    const APPLICATION_TITLE: &str = "WgpuToy";
    #[allow(dead_code)] // Demo configuration constants
    const APPLICATION_TITLE_PAUSED: &str = "WgpuToy - Paused";
    #[allow(dead_code)] // Demo configuration constants
    const DEFAULT_SHADER_PATH: &str = "examples/default.wgsl";
    #[allow(dead_code)] // Demo configuration constants
    const COMPUTE_TOYS_BASE_URL: &str = "https://compute.toys/";

    #[inline]
    #[allow(dead_code)] // Demo utility for toy examples
    fn construct_texture_url(img: &str) -> Cow<str> {
        if img.starts_with("http") {
            Cow::Borrowed(img)
        } else {
            let mut url = String::with_capacity(COMPUTE_TOYS_BASE_URL.len() + img.len());
            url.push_str(COMPUTE_TOYS_BASE_URL);
            url.push_str(img);
            Cow::Owned(url)
        }
    }

    #[inline]
    #[allow(dead_code)] // Demo utility for toy examples
    fn construct_metadata_path(filename: &str) -> String {
        let mut path = String::with_capacity(filename.len() + 5);
        path.push_str(filename);
        path.push_str(".json");
        path
    }

    #[allow(dead_code)] // Demo initialization for toy examples
    async fn init(filename: &str) -> Result<WgpuToyRenderer, Box<dyn Error>> {
        let wgpu = init_wgpu(1280, 720, "").await?;
        let mut wgputoy = WgpuToyRenderer::new(wgpu);
        let shader = std::fs::read_to_string(filename)?;

        wgputoy.wgpu.window.set_title(APPLICATION_TITLE);

        let cache_dir = std::env::temp_dir().join("codeskew_cache");
        let client = reqwest_middleware::ClientBuilder::new(reqwest::Client::new())
            .with(Cache(HttpCache {
                mode: CacheMode::Default,
                manager: CACacheManager::new(cache_dir, false),
                options: HttpCacheOptions::default(),
            }))
            .build();

        let metadata_path = construct_metadata_path(filename);
        if let Ok(json) = std::fs::read_to_string(&metadata_path) {
            let metadata: ShaderMeta = serde_json::from_str(&json)?;
            println!("{metadata:?}");

            // Process textures with zero-copy URL construction
            for (i, texture) in metadata.textures.iter().enumerate() {
                let url = construct_texture_url(&texture.img);
                let resp = client.get(url.as_ref()).send().await?;
                let img = resp.bytes().await?.to_vec();

                if texture.img.ends_with(".hdr") {
                    wgputoy.load_channel_hdr(i, &img)?;
                } else {
                    wgputoy.load_channel(i, &img);
                }
            }

            // Process uniforms with pre-allocated capacity
            if !metadata.uniforms.is_empty() {
                let uniform_count = metadata.uniforms.len();
                let mut uniform_names = Vec::with_capacity(uniform_count);
                let mut uniform_values = Vec::with_capacity(uniform_count);

                for uniform in &metadata.uniforms {
                    uniform_names.push(uniform.name.clone());
                    uniform_values.push(uniform.value);
                }

                wgputoy.set_custom_floats(uniform_names, uniform_values);
            }

            wgputoy.set_pass_f32(metadata.float32_enabled);
        }

        if let Some(source) = wgputoy.preprocess_async(&shader).await {
            println!("{}", source.source);
            wgputoy.compile(source);
        }
        Ok(wgputoy)
    }

    #[allow(dead_code)] // Demo state management for live shader reloading
    static NEEDS_REBUILD: AtomicBool = AtomicBool::new(false);

    #[inline]
    #[allow(dead_code)] // Demo utility for command line argument parsing
    fn get_filename_from_args() -> String {
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| DEFAULT_SHADER_PATH.to_string())
    }

    #[inline]
    #[allow(dead_code)] // Demo input handler for toy examples
    fn handle_keyboard_input(
        key_code: KeyCode,
        paused: &mut bool,
        reference_time: &mut f32,
        current_instant: &mut std::time::Instant,
        wgputoy: &mut WgpuToyRenderer,
        close_requested: &mut bool,
    ) {
        match key_code {
            KeyCode::Escape => {
                *close_requested = true;
            }
            KeyCode::Backspace => {
                // Reset time
                *paused = false;
                *reference_time = 0.0;
                *current_instant = std::time::Instant::now();
                // wgputoy.reset(); // Removed due to method not found
                println!("reset time");
            }
            KeyCode::Space => {
                // Toggle pause
                *paused = !*paused;
                if !*paused {
                    *current_instant = std::time::Instant::now();
                    wgputoy.wgpu.window.set_title(APPLICATION_TITLE);
                } else {
                    *reference_time += current_instant.elapsed().as_secs_f32();
                    wgputoy.wgpu.window.set_title(APPLICATION_TITLE_PAUSED);
                }
            }
            _ => {}
        }
    }

    #[inline]
    #[allow(dead_code)] // Demo input handler for toy examples
    fn handle_mouse_move(
        position: winit::dpi::PhysicalPosition<f64>,
        screen_size: winit::dpi::PhysicalSize<u32>,
        wgputoy: &mut WgpuToyRenderer,
    ) {
        let x = position.x as f32 / screen_size.width as f32;
        let y = position.y as f32 / screen_size.height as f32;
        wgputoy.set_mouse_pos(x, y);
    }

    #[inline]
    #[allow(dead_code)] // Demo event handler for toy examples
    fn handle_resize(size: winit::dpi::PhysicalSize<u32>, wgputoy: &mut WgpuToyRenderer) {
        if size.width != 0 && size.height != 0 {
            wgputoy.resize(size.width, size.height, 1.0);
        }
    }

    #[inline]
    #[allow(dead_code)] // Demo shader rebuild handler for live reloading
    fn handle_rebuild(
        filename: &str,
        runtime: &tokio::runtime::Runtime,
        wgputoy: &mut WgpuToyRenderer,
    ) {
        if let Ok(shader) = std::fs::read_to_string(filename) {
            if let Some(source) = runtime.block_on(wgputoy.preprocess_async(&shader)) {
                wgputoy.compile(source);
                wgputoy.wgpu.window.request_redraw();
            }
        }
    }

    #[allow(dead_code)] // Demo application struct for toy examples
    struct ToyApp {
        wgputoy: Option<WgpuToyRenderer>,
        runtime: tokio::runtime::Runtime,
        filename: PathBuf,
        close_requested: bool,
        paused: bool,
        current_instant: std::time::Instant,
        reference_time: f32,
    }

    impl ApplicationHandler for ToyApp {
        fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
            // Window creation is handled in init_wgpu
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            event: WindowEvent,
        ) {
            // Handle file changes
            if NEEDS_REBUILD.swap(false, Ordering::Relaxed) {
                if let Some(wgputoy) = &mut self.wgputoy {
                    handle_rebuild(self.filename.to_str().unwrap_or(""), &self.runtime, wgputoy);
                }
            }

            match event {
                WindowEvent::CloseRequested => {
                    self.close_requested = true;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    if let Some(wgputoy) = &mut self.wgputoy {
                        let window_size = wgputoy.wgpu.window.inner_size();
                        handle_mouse_move(position, window_size, wgputoy);
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    if let (Some(wgputoy), ElementState::Pressed) = (&mut self.wgputoy, state) {
                        wgputoy.set_mouse_click(matches!(button, winit::event::MouseButton::Left));
                    }
                }
                WindowEvent::Resized(size) => {
                    if let Some(wgputoy) = &mut self.wgputoy {
                        handle_resize(size, wgputoy);
                    }
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key,
                            state,
                            ..
                        },
                    ..
                } => match physical_key {
                    PhysicalKey::Code(KeyCode::Space) if state == ElementState::Pressed => {
                        self.paused = !self.paused;
                        if let Some(wgputoy) = &mut self.wgputoy {
                            wgputoy.wgpu.window.set_title(if self.paused {
                                APPLICATION_TITLE_PAUSED
                            } else {
                                APPLICATION_TITLE
                            });
                        }
                        if !self.paused {
                            self.current_instant = std::time::Instant::now();
                        } else {
                            self.reference_time += self.current_instant.elapsed().as_secs_f32();
                        }
                    }
                    PhysicalKey::Code(KeyCode::Backspace) if state == ElementState::Pressed => {
                        self.paused = false;
                        self.reference_time = 0.0;
                        self.current_instant = std::time::Instant::now();
                        if let Some(wgputoy) = &mut self.wgputoy {
                            // wgputoy.reset(); // Removed due to method not found
                            wgputoy.wgpu.window.set_title(APPLICATION_TITLE);
                        }
                        println!("reset time");
                    }
                    PhysicalKey::Code(KeyCode::Escape) if state == ElementState::Pressed => {
                        self.close_requested = true;
                    }
                    _ => {}
                },
                WindowEvent::RedrawRequested => {
                    if let Some(wgputoy) = &mut self.wgputoy {
                        if !self.paused {
                            wgputoy.set_time_elapsed(
                                self.current_instant.elapsed().as_secs_f32() - self.reference_time,
                            );
                        }
                        if let Ok(frame) = wgputoy.wgpu.surface.get_current_texture() {
                            wgputoy.render_to_surface(&frame);
                        }
                        wgputoy.wgpu.window.request_redraw();
                    }
                }
                _ => {}
            }

            if self.close_requested {
                event_loop.exit();
            }
        }
    }

    #[allow(dead_code)] // Public demo entry point for toy examples
    pub fn main() -> Result<(), Box<dyn Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;

        let filename = get_filename_from_args();
        let wgputoy = runtime.block_on(init(&filename))?;
        let event_loop = EventLoop::new()?;

        // Setup file watcher
        let _watcher = {
            use notify::{RecursiveMode, Result, Watcher};

            let event_loop_proxy = event_loop.create_proxy();
            let watcher_result =
                notify::recommended_watcher(move |event: Result<notify::Event>| match event {
                    Ok(_) => {
                        NEEDS_REBUILD.store(true, Ordering::Relaxed);
                        let _ = event_loop_proxy.send_event(());
                    }
                    Err(err) => log::error!("Error watching file: {err}"),
                });

            match watcher_result {
                Ok(mut watcher) => {
                    let path = Path::new(&filename);
                    match watcher.watch(path, RecursiveMode::NonRecursive) {
                        Ok(()) => {
                            log::info!("Watching file: {path:?}");
                            Some(watcher)
                        }
                        Err(err) => {
                            log::error!("Error watching file: {err:?}");
                            None
                        }
                    }
                }
                Err(err) => {
                    log::error!("Error creating watcher: {err:?}");
                    None
                }
            }
        };

        let mut app = ToyApp {
            wgputoy: Some(wgputoy),
            runtime,
            filename: PathBuf::from(filename),
            close_requested: false,
            paused: false,
            current_instant: std::time::Instant::now(),
            reference_time: 0.0,
        };

        event_loop.run_app(&mut app)?;
        Ok(())
    }
}
