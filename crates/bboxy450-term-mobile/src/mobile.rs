// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term-mobile — https://github.com/bboxy450/bboxy450-term-mobile
// Credit copied, adapted and translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term-mobile/blob/main/ATTRIBUTION.md

use crate::inbox::{Frame, Inbox};
use bbt_render::{
    chrome::{ChromeRenderer, ChromeTarget, icons::Assets},
    device::DeviceContext,
};
use bbt_ui_model::layout::Rect;
use std::sync::Arc;
use wgpu::CurrentSurfaceTexture;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, Ime, Touch, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
pub extern "C" fn bboxy450_ios_main() {
    run(EventLoop::new().expect("iOS event loop"));
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    use winit::platform::android::EventLoopBuilderExtAndroid;

    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("bboxy450-mobile")
            .with_max_level(log::LevelFilter::Info)
            .with_filter(
                android_logger::FilterBuilder::new()
                    .filter_level(log::LevelFilter::Warn)
                    .filter_module(module_path!(), log::LevelFilter::Info)
                    .build(),
            ),
    );
    run(EventLoop::builder()
        .with_android_app(app)
        .build()
        .expect("Android event loop"));
}

fn report(message: &str) {
    #[cfg(target_os = "ios")]
    println!("{message}");
    #[cfg(target_os = "android")]
    log::info!("{message}");
}

fn run(event_loop: EventLoop<()>) {
    let mut app = App {
        window: None,
        display: None,
        inbox: Inbox::fixture(),
        gesture: None,
    };
    event_loop.run_app(&mut app).expect("mobile event loop");
}

struct Display {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    chrome: ChromeRenderer,
    ready: bool,
}

impl Display {
    async fn new(window: Arc<Window>) -> Result<Self, Box<dyn std::error::Error>> {
        let size = surface_size(&window);
        let backends = if cfg!(target_os = "ios") {
            wgpu::Backends::METAL
        } else {
            wgpu::Backends::VULKAN | wgpu::Backends::GL
        };
        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = backends;
        // Older emulator SPIR-V consumers reject the newer WGSL source-language debug tag.
        if cfg!(target_os = "android") {
            instance_desc.flags.remove(wgpu::InstanceFlags::DEBUG);
        }
        let instance = wgpu::Instance::new(instance_desc);
        let surface = instance.create_surface(window)?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::None,
                compatible_surface: Some(&surface),
                ..Default::default()
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_limits: wgpu::Limits::downlevel_defaults()
                    .using_resolution(adapter.limits())
                    .using_alignment(adapter.limits()),
                ..Default::default()
            })
            .await?;
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("surface has no default configuration")?;
        // The shared chrome shader composites in display space.
        let formats = surface.get_capabilities(&adapter).formats;
        let gamma = config.format.remove_srgb_suffix();
        if formats.contains(&gamma) {
            config.format = gamma;
        } else if config.format.is_srgb() {
            config.view_formats.push(gamma);
        }
        let context = DeviceContext {
            device,
            queue,
            adapter_info: adapter.get_info(),
        };
        surface.configure(&context.device, &config);
        let chrome = ChromeRenderer::with_context(context, Assets::default())?;
        Ok(Self {
            surface,
            config,
            chrome,
            ready: false,
        })
    }

    fn draw(&mut self, window: &Window, frame: &Frame) -> Result<bool, Box<dyn std::error::Error>> {
        let size = surface_size(window);
        if size.width == 0 || size.height == 0 {
            return Ok(true);
        }
        let device = &self.chrome.context.device;
        if self.config.width != size.width || self.config.height != size.height {
            self.config.width = size.width;
            self.config.height = size.height;
            self.surface.configure(device, &self.config);
        }
        let texture = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(texture) => texture,
            CurrentSurfaceTexture::Suboptimal(texture) => {
                drop(texture);
                self.surface.configure(device, &self.config);
                window.request_redraw();
                return Ok(true);
            }
            CurrentSurfaceTexture::Outdated => {
                self.surface.configure(device, &self.config);
                window.request_redraw();
                return Ok(true);
            }
            CurrentSurfaceTexture::Lost => return Ok(false),
            CurrentSurfaceTexture::Timeout => {
                window.request_redraw();
                return Ok(true);
            }
            CurrentSurfaceTexture::Occluded => return Ok(true),
            CurrentSurfaceTexture::Validation => {
                return Err("surface texture validation failed".into());
            }
        };
        let view = texture.texture.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&Default::default());
        self.chrome.begin_frame();
        self.chrome.draw(
            &frame.draw,
            ChromeTarget {
                width: self.config.width,
                height: self.config.height,
                scale: window.scale_factor() as f32,
                view: &view,
                format: self.config.format,
            },
            &mut encoder,
        )?;
        self.chrome.flush();
        let queue = &self.chrome.context.queue;
        queue.submit([encoder.finish()]);
        window.pre_present_notify();
        queue.present(texture);
        if !self.ready {
            report(&format!(
                "BBOXY_INBOX_READY backend={:?}",
                self.chrome.context.adapter_info.backend
            ));
            self.ready = true;
        }
        Ok(true)
    }
}

fn surface_size(window: &Window) -> PhysicalSize<u32> {
    #[cfg(target_os = "ios")]
    {
        window.outer_size()
    }
    #[cfg(target_os = "android")]
    {
        window.inner_size()
    }
}

fn viewport(window: &Window) -> (Rect, Rect, f32) {
    let scale = window.scale_factor() as f32;
    let size = surface_size(window);
    let screen = Rect::new(
        0.,
        0.,
        size.width as f32 / scale,
        size.height as f32 / scale,
    );
    #[cfg(target_os = "ios")]
    let safe = {
        let origin = window.outer_position().unwrap_or_default();
        let inset = window.inner_position().unwrap_or(origin);
        let inner = window.inner_size();
        Rect::new(
            (inset.x - origin.x) as f32 / scale,
            (inset.y - origin.y) as f32 / scale,
            inner.width as f32 / scale,
            inner.height as f32 / scale,
        )
    };
    #[cfg(target_os = "android")]
    let safe = {
        use winit::platform::android::WindowExtAndroid;
        let r = window.content_rect();
        if r.right <= r.left || r.bottom <= r.top {
            screen
        } else {
            let left = (r.left as f32 / scale).clamp(0., screen.w);
            let top = (r.top as f32 / scale).clamp(0., screen.h);
            let right = (r.right as f32 / scale).clamp(left, screen.w);
            let bottom = (r.bottom as f32 / scale).clamp(top, screen.h);
            Rect::new(left, top, right - left, bottom - top)
        }
    };
    (screen, safe, scale)
}

struct Gesture {
    id: u64,
    origin: [f32; 2],
    scroll: f32,
    dragging: bool,
}

struct App {
    window: Option<Arc<Window>>,
    display: Option<Display>,
    inbox: Inbox,
    gesture: Option<Gesture>,
}

impl App {
    fn open_display(&mut self, event_loop: &ActiveEventLoop, window: Arc<Window>) {
        match pollster::block_on(Display::new(window.clone())) {
            Ok(display) => {
                self.display = Some(display);
                window.request_redraw();
            }
            Err(error) => {
                report(&format!("BBOXY_INBOX_ERROR {error}"));
                event_loop.exit();
            }
        }
    }

    fn touch(&mut self, window: &Window, touch: Touch) {
        let (screen, safe, scale) = viewport(window);
        let point = [
            touch.location.x as f32 / scale,
            touch.location.y as f32 / scale,
        ];
        if touch.phase == TouchPhase::Started && self.gesture.is_none() {
            let frame = self.inbox.layout(screen, safe, scale);
            self.gesture = Some(Gesture {
                id: touch.id,
                origin: point,
                scroll: self.inbox.scroll.min(frame.max_scroll),
                dragging: false,
            });
        }
        let Some(gesture) = &mut self.gesture else {
            return;
        };
        if gesture.id != touch.id {
            return;
        }
        let dx = point[0] - gesture.origin[0];
        let dy = point[1] - gesture.origin[1];
        gesture.dragging |= dx.hypot(dy) >= 8.;
        match touch.phase {
            TouchPhase::Moved | TouchPhase::Ended if gesture.dragging => {
                let frame = self.inbox.layout(screen, safe, scale);
                self.inbox.scroll = (gesture.scroll - dy).clamp(0., frame.max_scroll);
            }
            TouchPhase::Ended => {
                let frame = self.inbox.layout(screen, safe, scale);
                if let Some(action) = frame.hit(point) {
                    self.inbox.act(action);
                    window.set_ime_allowed(self.inbox.editing());
                }
            }
            _ => {}
        }
        if matches!(touch.phase, TouchPhase::Ended | TouchPhase::Cancelled) {
            self.gesture = None;
        }
        window.request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            match event_loop.create_window(Window::default_attributes().with_title("Inbox")) {
                Ok(window) => self.window = Some(Arc::new(window)),
                Err(error) => {
                    report(&format!("BBOXY_INBOX_ERROR {error}"));
                    event_loop.exit();
                    return;
                }
            }
        }
        let window = self.window.as_ref().unwrap().clone();
        if self.display.is_none() {
            self.open_display(event_loop, window);
        } else {
            window.request_redraw();
        }
    }

    fn suspended(&mut self, _: &ActiveEventLoop) {
        self.display = None;
        self.gesture = None;
        report("BBOXY_INBOX_SUSPENDED");
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone().filter(|w| w.id() == id) else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                window.request_redraw()
            }
            WindowEvent::Touch(touch) if self.display.is_some() => self.touch(&window, touch),
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.inbox.type_text(&text);
                window.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && self.inbox.editing() =>
            {
                match event.logical_key {
                    Key::Named(NamedKey::Backspace) => self.inbox.backspace(),
                    Key::Named(NamedKey::Enter) => self.inbox.submit(),
                    _ => {
                        if let Some(text) = event.text {
                            self.inbox.type_text(&text);
                        }
                    }
                }
                if !self.inbox.editing() {
                    window.set_ime_allowed(false);
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let Some(display) = &mut self.display else {
                    return;
                };
                let (screen, safe, scale) = viewport(&window);
                let frame = self.inbox.layout(screen, safe, scale);
                match display.draw(&window, &frame) {
                    Ok(true) => {}
                    Ok(false) => {
                        self.display = None;
                        self.open_display(event_loop, window);
                    }
                    Err(error) => {
                        report(&format!("BBOXY_INBOX_ERROR {error}"));
                        event_loop.exit();
                    }
                }
            }
            _ => {}
        }
    }
}
