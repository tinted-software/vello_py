use pyo3::{pyclass, pymethods};
use vello::{
    Scene,
    kurbo::{Affine, BezPath, Circle, Rect, Stroke},
    peniko::{Color, Fill},
};

enum ShapeInner {
    Path(BezPath),
    Rect(Rect),
    Circle(Circle),
}

#[pyclass(name = "Shape")]
pub struct PyShape {
    inner: ShapeInner,
}

#[pymethods]
impl PyShape {
    #[staticmethod]
    pub fn rect(x: f64, y: f64, w: f64, h: f64) -> Self {
        let rect = Rect::new(x, y, x + w, y + h);
        PyShape {
            inner: ShapeInner::Rect(rect),
        }
    }

    #[staticmethod]
    pub fn bezpath(points: Vec<(f64, f64)>) -> Self {
        let mut path = BezPath::new();

        for (i, (x, y)) in points.iter().enumerate() {
            if i == 0 {
                path.move_to((*x, *y));
            } else {
                path.line_to((*x, *y));
            }
        }

        PyShape {
            inner: ShapeInner::Path(path),
        }
    }

    #[staticmethod]
    pub fn circle(x: f64, y: f64, radius: f64) -> Self {
        let circle = Circle::new((x, y), radius);
        PyShape {
            inner: ShapeInner::Circle(circle),
        }
    }
}

#[pyclass(name = "Scene")]
pub struct PyScene {
    pub scene: Scene,
}

#[pymethods]
impl PyScene {
    #[new]
    fn new() -> Self {
        PyScene {
            scene: Scene::new(),
        }
    }

    fn stroke(&mut self, stroke: f64, color: (f32, f32, f32, f32), shape: &PyShape) {
        let stroke = Stroke::new(stroke);
        let color = Color::new([color.0, color.1, color.2, color.3]);

        match &shape.inner {
            ShapeInner::Path(path) => {
                self.scene
                    .stroke(&stroke, Affine::IDENTITY, color, None, path);
            }
            ShapeInner::Rect(rect) => {
                self.scene
                    .stroke(&stroke, Affine::IDENTITY, color, None, rect);
            }
            ShapeInner::Circle(circle) => {
                self.scene
                    .stroke(&stroke, Affine::IDENTITY, color, None, circle);
            }
        }
    }

    fn fill(&mut self, color: (f32, f32, f32, f32), shape: &PyShape) {
        let color = Color::new([color.0, color.1, color.2, color.3]);

        match &shape.inner {
            ShapeInner::Path(path) => {
                self.scene
                    .fill(Fill::NonZero, Affine::IDENTITY, color, None, path);
            }
            ShapeInner::Rect(rect) => {
                self.scene
                    .fill(Fill::NonZero, Affine::IDENTITY, color, None, rect);
            }
            ShapeInner::Circle(circle) => {
                self.scene
                    .fill(Fill::NonZero, Affine::IDENTITY, color, None, circle);
            }
        }
    }
}
