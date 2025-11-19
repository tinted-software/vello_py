use std::sync::{Arc, Mutex};

use image::{ImageBuffer, Rgba};
use pyo3::{Py, PyResult, Python, exceptions::PyValueError, pyclass, pymethods};
use vello::{
    AaConfig, Renderer, RendererOptions,
    peniko::color::AlphaColor,
    wgpu::{
        self, Backends, CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d, Instance,
        InstanceDescriptor, Origin3d, PollType, Queue, RequestAdapterOptions, TexelCopyBufferInfo,
        TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor,
        TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
        util::TextureBlitter,
    },
};

use crate::{app::PyWindowHandle, scene::PyScene};

#[pyclass(name = "Renderer")]
pub struct PyRenderer {
    instance: Instance,
    adapter: wgpu::Adapter,
    device: Device,
    queue: Queue,
    pub surface: Option<wgpu::Surface<'static>>,
    pub scene: Py<PyScene>,
    renderer: Arc<Mutex<Renderer>>,
    window: Option<Py<PyWindowHandle>>,
}

impl PyRenderer {
    pub(crate) fn resize(&self, width: u32, height: u32) {
        if let Some(surface) = &self.surface {
            surface.configure(
                &self.device,
                &wgpu::SurfaceConfiguration {
                    usage: TextureUsages::RENDER_ATTACHMENT,
                    format: surface
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
    }

    pub fn new(scene: Py<PyScene>, window: Option<Py<PyWindowHandle>>) -> Self {
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });

        let surface = if let Some(window_handle) = &window {
            let window = window_handle
                .borrow(unsafe { Python::assume_attached() })
                .window
                .clone();
            Some(
                instance
                    .create_surface(window)
                    .expect("Failed to create surface"),
            )
        } else {
            None
        };

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            compatible_surface: surface.as_ref(),
            ..Default::default()
        }))
        .expect("Failed to find an appropriate adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(&DeviceDescriptor {
            ..Default::default()
        }))
        .expect("Failed to create device");
        let renderer = Arc::new(Mutex::new(
            Renderer::new(&device, RendererOptions::default()).expect("Couldn't create renderer"),
        ));

        if let Some(surface) = &surface {
            let window = window.as_ref().unwrap();
            let window_ref = window
                .borrow(unsafe { Python::assume_attached() })
                .window
                .clone();
            let size = window_ref.inner_size();
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
                    width: size.width,
                    height: size.height,
                    present_mode: wgpu::PresentMode::Fifo,
                    alpha_mode: wgpu::CompositeAlphaMode::Auto,
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2,
                },
            );
        }

        PyRenderer {
            instance,
            adapter,
            device,
            queue,
            surface,
            renderer,
            scene,
            window,
        }
    }

    pub(crate) async fn render(&mut self, background_color: (f32, f32, f32, f32)) -> PyResult<()> {
        let surface = self.surface.as_ref().expect("No surface available");
        let window = match self.window.as_ref() {
            Some(win) => win
                .borrow(unsafe { Python::assume_attached() })
                .window
                .clone(),
            None => {
                return Err(PyValueError::new_err(
                    "No window associated with this renderer",
                ));
            }
        };
        let (width, height) = {
            let size = window.inner_size();
            (size.width, size.height)
        };

        let frame = surface
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

        self.renderer
            .lock()
            .unwrap()
            .render_to_texture(
                &self.device,
                &self.queue,
                &self
                    .scene
                    .borrow(unsafe { Python::assume_attached() })
                    .scene,
                &target_view,
                &vello::RenderParams {
                    base_color: AlphaColor::new([
                        background_color.0,
                        background_color.1,
                        background_color.2,
                        background_color.3,
                    ]),
                    width,
                    height,
                    antialiasing_method: AaConfig::Msaa16,
                },
            )
            .expect("failed to render to surface");

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Copy Texture to Buffer"),
            });

        let blitter = TextureBlitter::new(
            &self.device,
            surface.get_capabilities(&self.adapter).formats[0],
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
