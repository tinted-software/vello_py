use pyo3::prelude::*;
use pyo3::types::PyAny;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::EventLoop;
use winit::window::Window;

pub struct PyWindow {
    py_callback: Py<PyAny>,
    window: Option<Window>,
}

impl ApplicationHandler for PyWindow {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        Python::attach(|py| {
            let _ = self.py_callback.call1(py, (format!("{:?}", event),));
        });
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        Python::attach(|py| {
            let _ = self.py_callback.call1(py, ("frame",));
        });
    }
}

#[pyfunction]
pub fn run_winit(callback: Py<PyAny>) {
    let handler = Box::leak(Box::new(PyWindow {
        py_callback: callback,
        window: None,
    }));

    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(handler).unwrap();
}
