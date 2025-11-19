use std::sync::{Arc, Mutex};

use pyo3::{Py, PyResult, Python, pyclass};
use vello_hybrid::{RenderSize, RenderTargetConfig, Renderer, Scene};
use wgpu::{
    Adapter, Backends, CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d, Instance,
    InstanceDescriptor, Queue, RequestAdapterOptions, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureViewDescriptor, util::TextureBlitter,
};

use crate::{app::PyWindowHandle, scene::PyScene};

#[pyclass(name = "Renderer")]
pub struct PyRenderer {
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    surface: wgpu::Surface<'static>,
    scene: Py<PyScene>,
    renderer: Arc<Mutex<Renderer>>,
    window_handle: Py<PyWindowHandle>,
}

impl PyRenderer {
    pub(crate) fn resize(&self, width: u32, height: u32) {
        *self
            .scene
            .borrow(unsafe { Python::assume_attached() })
            .scene
            .lock()
            .unwrap() = Scene::new(width as u16, height as u16);
        self.surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: self
                    .surface
                    .get_capabilities(&self.adapter)
                    .formats
                    .first()
                    .cloned()
                    .unwrap_or(TextureFormat::Bgra8UnormSrgb),
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );
    }

    pub fn new(scene: Py<PyScene>, window_handle: Py<PyWindowHandle>) -> Self {
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let window = window_handle
            .borrow(unsafe { Python::assume_attached() })
            .window
            .clone();
        let window_size = window.inner_size();
        let surface = instance
            .create_surface(window)
            .expect("Failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: Some(&surface),
            ..Default::default()
        }))
        .expect("Failed to find an appropriate adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&DeviceDescriptor {
            ..Default::default()
        }))
        .expect("Failed to create device");
        let renderer = Arc::new(Mutex::new(Renderer::new(
            &device,
            &RenderTargetConfig {
                width: window_size.width,
                height: window_size.height,
                format: TextureFormat::Rgba8Unorm,
            },
        )));

        surface.configure(
            &device,
            &wgpu::SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: surface
                    .get_capabilities(&adapter)
                    .formats
                    .first()
                    .cloned()
                    .unwrap_or(TextureFormat::Bgra8UnormSrgb),
                width: window_size.width,
                height: window_size.height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );

        PyRenderer {
            instance,
            adapter,
            device,
            queue,
            surface,
            renderer,
            scene,
            window_handle,
        }
    }

    pub(crate) async fn render(&mut self, background_color: (f32, f32, f32, f32)) -> PyResult<()> {
        let window = &self
            .window_handle
            .borrow(unsafe { Python::assume_attached() })
            .window;
        let (width, height) = {
            let size = window.inner_size();
            (size.width, size.height)
        };

        let frame = self
            .surface
            .get_current_texture()
            .expect("Failed to get current frame");
        let frame_view = frame.texture.create_view(&TextureViewDescriptor::default());

        let target_texture = self.device.create_texture(&TextureDescriptor {
            label: Some("Render Target"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_SRC
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });
        let target_view = target_texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                ..Default::default()
            });

        let scene = self.scene.borrow(unsafe { Python::assume_attached() });
        let scene = scene.scene.lock().unwrap();

        self.renderer
            .lock()
            .unwrap()
            .render(
                &scene,
                &self.device,
                &self.queue,
                &mut encoder,
                &RenderSize { width, height },
                &target_view,
            )
            .expect("failed to render to surface");

        let blitter = TextureBlitter::new(
            &self.device,
            self.surface.get_capabilities(&self.adapter).formats[0],
        );

        blitter.copy(&self.device, &mut encoder, &target_view, &frame_view);

        self.queue.submit(Some(encoder.finish()));
        window.pre_present_notify();
        frame.present();

        Ok(())
    }

    // async fn render_to_png(
    //     &mut self,
    //     width: u32,
    //     height: u32,
    //     background_color: (f32, f32, f32, f32),
    //     output_path: String,
    // ) {
    //     let target_texture = self.device.create_texture(&TextureDescriptor {
    //         label: Some("Render Target"),
    //         size: Extent3d {
    //             width: 800,
    //             height: 600,
    //             depth_or_array_layers: 1,
    //         },
    //         mip_level_count: 1,
    //         sample_count: 1,
    //         dimension: TextureDimension::D2,
    //         format: TextureFormat::Rgba8Unorm,
    //         usage: TextureUsages::RENDER_ATTACHMENT
    //             | TextureUsages::COPY_SRC
    //             | TextureUsages::STORAGE_BINDING,
    //         view_formats: &[],
    //     });
    //     let target_view = target_texture.create_view(&TextureViewDescriptor::default());

    //     self.renderer
    //         .lock()
    //         .unwrap()
    //         .render_to_texture(
    //             &self.device,
    //             &self.queue,
    //             &self
    //                 .scene
    //                 .borrow(unsafe { Python::assume_attached() })
    //                 .scene,
    //             &target_view,
    //             &vello::RenderParams {
    //                 base_color: AlphaColor::new([
    //                     background_color.0,
    //                     background_color.1,
    //                     background_color.2,
    //                     background_color.3,
    //                 ]),
    //                 width,
    //                 height,
    //                 antialiasing_method: AaConfig::Msaa16,
    //             },
    //         )
    //         .expect("failed to render to surface");

    //     let output_buffer_size = (width * height * 4) as u64;

    //     let output_staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
    //         label: None,
    //         size: output_buffer_size,
    //         usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    //         mapped_at_creation: false,
    //     });

    //     let mut encoder = self
    //         .device
    //         .create_command_encoder(&CommandEncoderDescriptor {
    //             label: Some("Copy Texture to Buffer"),
    //         });

    //     encoder.copy_texture_to_buffer(
    //         TexelCopyTextureInfo {
    //             texture: &target_texture,
    //             mip_level: 0,
    //             origin: Origin3d::ZERO,
    //             aspect: TextureAspect::All,
    //         },
    //         TexelCopyBufferInfo {
    //             buffer: &output_staging_buffer,
    //             layout: TexelCopyBufferLayout {
    //                 offset: 0,
    //                 bytes_per_row: Some(width * 4),
    //                 rows_per_image: Some(height),
    //             },
    //         },
    //         Extent3d {
    //             width,
    //             height,
    //             depth_or_array_layers: 1,
    //         },
    //     );

    //     self.queue.submit(Some(encoder.finish()));

    //     let (sender, receiver) = flume::bounded(1);

    //     let buffer_slice = output_staging_buffer.slice(..);
    //     buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
    //     self.device
    //         .poll(PollType::wait_indefinitely())
    //         .expect("Failed to poll device");
    //     receiver.recv_async().await.unwrap().unwrap();

    //     let view = buffer_slice.get_mapped_range();

    //     let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, view.to_vec())
    //         .expect("Failed to create image buffer");
    //     drop(view);
    //     output_staging_buffer.unmap();

    //     buffer.save(output_path).expect("Failed to save PNG image");
    // }
}
