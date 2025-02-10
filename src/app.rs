use glium::{glutin::surface::WindowSurface, winit::{application::ApplicationHandler, event_loop::EventLoop, window::Window}, Display, DrawParameters};

use std::num::NonZeroU32;
use glium::Display;
use glutin::prelude::*;
use glutin::display::GetGlDisplay;
use glutin::surface::WindowSurface;
use raw_window_handle::HasWindowHandle;
use winit::event_loop::ActiveEventLoop;
use winit::event::WindowEvent;
use winit::window::WindowId;
use winit::application::ApplicationHandler;
use crate::camera::CameraState;

pub trait ApplicationContext {
    fn draw_frame(&mut self, _display: &Display<WindowSurface>) { }
    fn new(display: &Display<WindowSurface>) -> Self;
    fn update(&mut self) { }
    fn handle_window_event(&mut self, _event: &glium::winit::event::WindowEvent, _window: &glium::winit::window::Window) { }
    const WINDOW_TITLE:&'static str;
}

pub struct State<T> {
    pub display: glium::Display<WindowSurface>,
    pub window: glium::winit::window::Window,
    pub context: T,
}

struct App<T> {
    state: Option<State<T>>,
    visible: bool,
    close_promptly: bool,
}

impl<T: ApplicationContext + 'static> ApplicationHandler<()> for App<T> {
    // The resumed/suspended handlers are mostly for Android compatiblity since the context can get lost there at any point.
    // For convenience's sake, the resumed handler is also called on other platforms on program startup.
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state = Some(State::new(event_loop, self.visible));
        if !self.visible && self.close_promptly {
            event_loop.exit();
        }
    }
    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        self.state = None;
    }

    fn window_event(&mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            glium::winit::event::WindowEvent::Resized(new_size) => {
                if let Some(state) = &self.state {
                    state.display.resize(new_size.into());
                }
            },
            glium::winit::event::WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    state.context.update();
                    state.context.draw_frame(&state.display);
                    if self.close_promptly {
                        event_loop.exit();
                    }
                }
            },
            // Exit the event loop when requested (by closing the window for example) or when
            // pressing the Esc key.
            glium::winit::event::WindowEvent::CloseRequested
            | glium::winit::event::WindowEvent::KeyboardInput { event: glium::winit::event::KeyEvent {
                state: glium::winit::event::ElementState::Pressed,
                logical_key: glium::winit::keyboard::Key::Named(glium::winit::keyboard::NamedKey::Escape),
                ..
            }, ..} => {
                event_loop.exit()
            },
            // Every other event
            ev => {
                if let Some(state) = &mut self.state {
                    state.context.handle_window_event(&ev, &state.window);
                }
            },
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

impl<T: ApplicationContext + 'static> State<T> {
    pub fn new(
        event_loop: &glium::winit::event_loop::ActiveEventLoop,
        visible: bool,
    ) -> Self {
        let window_attributes = winit::window::Window::default_attributes()
            .with_title(T::WINDOW_TITLE).with_visible(visible);
        let config_template_builder = glutin::config::ConfigTemplateBuilder::new();
        let display_builder = glutin_winit::DisplayBuilder::new().with_window_attributes(Some(window_attributes));

        // First we create a window
        let (window, gl_config) = display_builder
            .build(event_loop, config_template_builder, |mut configs| {
                // Just use the first configuration since we don't have any special preferences here
                configs.next().unwrap()
            })
            .unwrap();
        let window = window.unwrap();

        // Then the configuration which decides which OpenGL version we'll end up using, here we just use the default which is currently 3.3 core
        // When this fails we'll try and create an ES context, this is mainly used on mobile devices or various ARM SBC's
        // If you depend on features available in modern OpenGL Versions you need to request a specific, modern, version. Otherwise things will very likely fail.
        let window_handle = window.window_handle().expect("couldn't obtain window handle");
        let context_attributes = glutin::context::ContextAttributesBuilder::new().build(Some(window_handle.into()));
        let fallback_context_attributes = glutin::context::ContextAttributesBuilder::new()
            .with_context_api(glutin::context::ContextApi::Gles(None))
            .build(Some(window_handle.into()));

        let not_current_gl_context = Some(unsafe {
            gl_config.display().create_context(&gl_config, &context_attributes).unwrap_or_else(|_| {
                gl_config.display()
                    .create_context(&gl_config, &fallback_context_attributes)
                    .expect("failed to create context")
            })
        });

        // Determine our framebuffer size based on the window size, or default to 800x600 if it's invisible
        let (width, height): (u32, u32) = if visible { window.inner_size().into() } else { (800, 600) };
        let attrs = glutin::surface::SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle.into(),
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
        );
        // Now we can create our surface, use it to make our context current and finally create our display
        let surface = unsafe { gl_config.display().create_window_surface(&gl_config, &attrs).unwrap() };
        let current_context = not_current_gl_context.unwrap().make_current(&surface).unwrap();
        let display = glium::Display::from_context_surface(current_context, surface).unwrap();

        Self::from_display_window(display, window)
    }

    pub fn from_display_window(
        display: glium::Display<WindowSurface>,
        window: glium::winit::window::Window,
    ) -> Self {
        let context = T::new(&display);
        Self {
            display,
            window,
            context,
        }
    }

    /// Start the event_loop and keep rendering frames until the program is closed
    pub fn run_loop() {
        let event_loop = glium::winit::event_loop::EventLoop::builder()
            .build()
            .expect("event loop building");
        let mut app = App::<T> {
            state: None,
            visible: true,
            close_promptly: false,
        };
        let result = event_loop.run_app(&mut app);
        result.unwrap();
    }

    /// Create a context and draw a single frame
    pub fn run_once(visible: bool) {
        let event_loop = glium::winit::event_loop::EventLoop::builder()
            .build()
            .expect("event loop building");
        let mut app = App::<T> {
            state: None,
            visible,
            close_promptly: true,
        };
        let result = event_loop.run_app(&mut app);
        result.unwrap();
    }
}
