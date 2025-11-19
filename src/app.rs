use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyAny;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

use crate::render::PyRenderer;
use crate::scene::PyScene;

#[pyclass(name = "MouseButton")]
pub enum PyMouseButton {
    Left(),
    Right(),
    Middle(),
    Back(),
    Forward(),
    Other(u16),
}

#[pyclass(name = "MouseState")]
pub enum PyMouseState {
    Pressed(),
    Released(),
}

#[pyclass(name = "WindowHandle")]
pub struct PyWindowHandle {
    pub window: Arc<Window>,
}

#[pyclass(name = "WindowEvent")]
pub enum PyWindowEvent {
    Created(Py<PyWindowHandle>),
    Resized(u32, u32),
    Closed(),
    Focused(bool),
    Moved(i32, i32),
    KeyboardInput(String),
    MouseInput(Py<PyMouseState>, Py<PyMouseButton>),
    CursorMoved(f64, f64),
    RedrawRequested(),
}

#[pyclass(name = "App")]
pub struct PyApp {
    title: String,
    transparent: bool,
    background_color: (f32, f32, f32, f32),
    app_fn: Py<PyAny>,
    scene: Py<PyScene>,
    window: Option<Arc<Window>>,
    renderer: Option<Py<PyRenderer>>,
}

impl ApplicationHandler for PyApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(&self.title)
                        .with_transparent(self.transparent),
                )
                .unwrap(),
        );
        self.window = Some(window);

        self.renderer = Some(
            Python::attach(|py| {
                let py_window_handle = Py::new(
                    py,
                    PyWindowHandle {
                        window: self.window.as_ref().unwrap().clone(),
                    },
                )
                .unwrap();

                Py::new(
                    py,
                    PyRenderer::new(self.scene.clone_ref(py), Some(py_window_handle)),
                )
            })
            .expect("Failed to create renderer"),
        );
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let renderer = match &self.renderer {
            Some(renderer) => renderer,
            None => return,
        };

        Python::attach(|py| match event {
            WindowEvent::Resized(physical_size) => {
                renderer
                    .borrow(py)
                    .resize(physical_size.width, physical_size.height);
                self.window.as_ref().expect("No window").request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::RedrawRequested => {
                let widget = self.app_fn.call0(py).expect("Failed to call app function");

                let scene = self.scene.borrow_mut(py);

                widget.call1(py, (scene,)).expect("Failed to build scene");

                pollster::block_on(renderer.borrow_mut(py).render(self.background_color))
                    .expect("Failed to render frame");
            }
            _ => {}
        });
    }
}

#[pyfunction(signature =(app_fn, title = "Vello App", transparent = false, background_color = (1.0, 1.0, 1.0, 1.0)))]
pub fn run_app(
    app_fn: Py<PyAny>,
    title: &str,
    transparent: bool,
    background_color: (f32, f32, f32, f32),
) {
    let handler = Box::leak(Box::new(PyApp {
        app_fn: app_fn,
        scene: Py::new(unsafe { Python::assume_attached() }, PyScene::new())
            .expect("Failed to create scene"),
        window: None,
        renderer: None,
        title: title.to_string(),
        transparent,
        background_color,
    }));

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(handler).unwrap();
}
