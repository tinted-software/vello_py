use pyo3::pymodule;

pub mod app;
pub mod render;
pub mod scene;

#[pymodule(name = "vello_py")]
pub mod vello {
    #[pymodule_export]
    use super::scene::{PyScene, PyShape};

    #[pymodule_export]
    use super::render::PyRenderer;

    #[pymodule_export]
    use super::app::{PyApp, PyMouseButton, PyMouseState, PyWindowEvent, PyWindowHandle, run_app};
}
