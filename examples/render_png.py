import asyncio

from vello_py import Renderer, Scene, Shape


async def main():
    scene = Scene()

    scene.fill((0.0, 1.0, 0.0, 1.0), Shape.circle(128, 128, 100))

    renderer = Renderer(scene)

    await renderer.render_to_png(256, 256, (0.0, 0.0, 0.0, 1.0), "output.png")


if __name__ == "__main__":
    asyncio.run(main())
