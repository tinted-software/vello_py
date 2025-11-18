use std::sync::{Arc, Mutex};

use image::{ImageBuffer, Rgba};
use pyo3::{Py, PyRef, Python, pyclass, pymethods};
use vello::{
    AaConfig, Renderer, RendererOptions,
    peniko::color::{AlphaColor, palette},
    wgpu::{
        self, Adapter, Backends, BufferAddress, BufferDescriptor, BufferUsages,
        CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d, Instance, InstanceDescriptor,
        MapMode, Origin3d, PollType, Queue, RequestAdapterOptions, TexelCopyBufferInfo,
        TexelCopyBufferLayout, TexelCopyTextureInfo, TextureAspect, TextureDescriptor,
        TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
    },
};

use crate::scene::PyScene;

#[pyclass(name = "Renderer")]
pub struct PyRenderer {
    scene: Py<PyScene>,
    instance: Instance,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    renderer: Arc<Mutex<Renderer>>,
}

#[pymethods]
impl PyRenderer {
    #[new]
    pub fn new(scene: Py<PyScene>) -> Self {
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
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

        PyRenderer {
            instance,
            adapter,
            device,
            queue,
            renderer,
            scene,
        }
    }

    async fn render_to_png(
        &mut self,
        width: u32,
        height: u32,
        background_color: (f32, f32, f32, f32),
        output_path: String,
    ) {
        let target_texture = self.device.create_texture(&TextureDescriptor {
            label: Some("Render Target"),
            size: Extent3d {
                width: 800,
                height: 600,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::COPY_SRC
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

        let output_buffer_size = (width * height * 4) as u64;

        let output_staging_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("Copy Texture to Buffer"),
            });

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: &target_texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &output_staging_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width * 4),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(Some(encoder.finish()));

        let (sender, receiver) = flume::bounded(1);

        let buffer_slice = output_staging_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, move |r| sender.send(r).unwrap());
        self.device
            .poll(PollType::wait_indefinitely())
            .expect("Failed to poll device");
        receiver.recv_async().await.unwrap().unwrap();

        let view = buffer_slice.get_mapped_range();

        let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, view.to_vec())
            .expect("Failed to create image buffer");
        drop(view);
        output_staging_buffer.unmap();

        println!("Saving rendered image to {}", output_path);
        buffer.save(output_path).expect("Failed to save PNG image");
    }
}
