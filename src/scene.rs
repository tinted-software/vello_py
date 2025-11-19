use std::sync::{Arc, Mutex};

use pyo3::{pyclass, pymethods};
use vello_common::{
    kurbo::{Affine, BezPath, Circle, Rect, Shape, Stroke},
    peniko::{Color, Fill},
};
use vello_hybrid::Scene;

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
    pub fn path(points: Vec<(f64, f64)>) -> Self {
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
    pub scene: Arc<Mutex<Scene>>,
}

#[pymethods]
impl PyScene {
    #[new]
    pub fn new(width: u16, height: u16) -> Self {
        PyScene {
            scene: Arc::new(Mutex::new(Scene::new(width, height))),
        }
    }

    fn stroke(&mut self, stroke: f64, color: (f32, f32, f32, f32), shape: &PyShape) {
        let mut scene = self.scene.lock().expect("Failed to lock scene");
        let stroke = Stroke::new(stroke);
        let color = Color::new([color.0, color.1, color.2, color.3]);
        let affine = Affine::IDENTITY;

        match &shape.inner {
            ShapeInner::Path(path) => {
                scene.set_transform(affine);
                scene.set_paint(color);
                scene.set_stroke(stroke);
                scene.stroke_path(path);
            }
            ShapeInner::Rect(rect) => {
                scene.set_transform(affine);
                scene.set_paint(color);
                scene.set_stroke(stroke);
                scene.stroke_rect(rect);
            }
            ShapeInner::Circle(circle) => {
                scene.set_transform(affine);
                scene.set_paint(color);
                scene.set_stroke(stroke);
                scene.stroke_path(&circle.to_path(1e-9));
            }
        }
    }

    fn fill(&mut self, color: (f32, f32, f32, f32), shape: &PyShape) {
        let mut scene = self.scene.lock().expect("Failed to lock scene");
        let fill = Fill::NonZero;
        let color = Color::new([color.0, color.1, color.2, color.3]);
        let affine = Affine::IDENTITY;

        match &shape.inner {
            ShapeInner::Path(path) => {
                scene.set_transform(affine);
                scene.set_paint(color);
                scene.set_fill_rule(fill);
                scene.fill_path(path);
            }
            ShapeInner::Rect(rect) => {
                scene.set_transform(affine);
                scene.set_paint(color);
                scene.set_fill_rule(fill);
                scene.fill_rect(rect);
            }
            ShapeInner::Circle(circle) => {
                scene.set_transform(affine);
                scene.set_paint(color);
                scene.set_fill_rule(fill);
                scene.fill_path(&circle.to_path(1e-9));
            }
        }
    }
}
