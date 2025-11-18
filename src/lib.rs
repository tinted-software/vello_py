use pyo3::pymodule;

pub mod render;
pub mod scene;
pub mod window;

#[pymodule(name = "vello_py")]
pub mod py_scene {
    #[pymodule_export]
    use super::scene::{PyScene, PyShape};

    #[pymodule_export]
    use super::window::run_winit;

    #[pymodule_export]
    use super::render::PyRenderer;
}
