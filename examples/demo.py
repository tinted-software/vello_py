from vello_py import Scene, Shape, run_app

# api: scene.fill(color: tuple, shape: Shape)
# Shape.path(points: tuple of (x, y))
# Shape.circle(cx: float, cy: float, radius: float)
# Shape.rect(x: float, y: float, width: float, height: float)


class CustomWidget:
    def __init__(self):
        pass

    def __call__(self, scene: Scene):
        # Make a happy face
        scene.fill((1.0, 1.0, 0.0, 1.0), Shape.circle(128, 128, 100))  # Face
        scene.fill((0.0, 0.0, 0.0, 1.0), Shape.circle(88, 108, 15))  # Left eye
        scene.fill((0.0, 0.0, 0.0, 1.0), Shape.circle(168, 108, 15))  # Right eye
        scene.fill(
            (1.0, 0.0, 0.0, 1.0),
            Shape.path(
                (  # Smile
                    (78, 158),
                    (98, 188),
                    (128, 198),
                    (158, 188),
                    (178, 158),
                    (168, 148),
                    (138, 158),
                    (108, 148),
                    (98, 158),
                    (78, 158),
                )
            ),
        )


def App():
    return CustomWidget()


if __name__ == "__main__":
    run_app(App, show_decorations=False)
