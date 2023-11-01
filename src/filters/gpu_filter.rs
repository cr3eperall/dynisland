use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use wgpu::util::DeviceExt;
use wgpu::{
    util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BindingResource,
    CommandEncoderDescriptor, ComputePassDescriptor, ComputePipelineDescriptor,
    ShaderModuleDescriptor, ShaderSource, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureViewDescriptor,
};
use wgpu::{
    Buffer, BufferUsages, ComputePipeline, Device, InstanceFlags, Queue,
    Texture,
};

const GAUSSIAN_BLUR_SHADER: &str = include_str!("../shaders/gaussian_blur.wgsl");

struct Kernel {
    sum: f32,
    values: Vec<f32>,
}

impl Kernel {
    fn new(values: Vec<f32>) -> Self {
        let sum = values.iter().sum();
        Self { sum, values }
    }

    fn packed_data(&self) -> Vec<f32> {
        let mut data = vec![0.0; self.values.len() + 1];
        data[0] = self.sum;
        data[1..].copy_from_slice(&self.values);
        data
    }

    fn size(&self) -> isize {
        self.values.len() as isize
    }
}

pub struct GpuContext {
    pub(crate) device: Device,
    pub(crate) queue: Queue,
    // pub(crate) texture: Texture,
    // pub(crate) texture_size: Extent3d,
    pub vertical: Buffer,
    pub horizontal: Buffer,
    pub pipeline: ComputePipeline,
}

pub static GPU_INSTANCE: Lazy<Mutex<GpuContext>> = Lazy::new(|| Mutex::new(GpuContext::new()));
impl GpuContext {
    pub fn new() -> Self {
        let name = "gaussian blur";

        // setup instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
            flags: InstanceFlags::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        });
        let adapter = futures::executor::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            },
        ))
        .unwrap();

        let (device, queue) =
            futures::executor::block_on(adapter.request_device(&Default::default(), None)).unwrap();

        // setup compute
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(format!("{} shader", name).as_str()),
            source: ShaderSource::Wgsl(GAUSSIAN_BLUR_SHADER.into()),
        });

        let pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some(format!("{} pipeline", name).as_str()),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        let vertical = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Orientation"),
            contents: bytemuck::cast_slice::<u32, u8>(&[1]),
            usage: BufferUsages::UNIFORM,
        });
        let horizontal = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Orientation"),
            contents: bytemuck::cast_slice::<u32, u8>(&[0]),
            usage: BufferUsages::UNIFORM,
        });
        Self {
            device,
            queue,
            vertical,
            horizontal,
            pipeline,
        }
    }

    pub fn gaussian_blur(&self, data: &mut [u8], width: usize, height: usize, sigma: f32) {
        let name = "gaussian blur";
        let kernel = kernel(sigma);
        let kernel_size = kernel.size() as i32;

        let texture =
            self.texture_from_data(data, width.try_into().unwrap(), height.try_into().unwrap());

        let settings = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Image info"),
            contents: bytemuck::cast_slice(&[kernel_size]),
            usage: BufferUsages::UNIFORM,
        });

        let kernel = self.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&kernel.packed_data()[..]),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });

        let compute_constants = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Compute constants"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: settings.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: kernel.as_entire_binding(),
                },
            ],
        });

        let vertical_pass_texture = self.device.create_texture(&TextureDescriptor {
            label: None,
            size: texture.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::STORAGE_BINDING,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });
        let horizontal_pass_texture = self.device.create_texture(&TextureDescriptor {
            label: None,
            size: texture.size(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::STORAGE_BINDING,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });

        let vertical_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Texture bind group"),
            layout: &self.pipeline.get_bind_group_layout(1),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &texture.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &vertical_pass_texture.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.vertical.as_entire_binding(),
                },
            ],
        });
        let horizontal_bind_group = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Texture bind group"),
            layout: &self.pipeline.get_bind_group_layout(1),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &vertical_pass_texture.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &horizontal_pass_texture.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.horizontal.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(format!("{} pass", name).as_str()),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &compute_constants, &[]);
            compute_pass.set_bind_group(1, &vertical_bind_group, &[]);
            let (dispatch_with, dispatch_height) =
                compute_work_group_count((texture.size().width, texture.size().height), (128, 1));
            compute_pass.dispatch_workgroups(dispatch_with, dispatch_height, 1);
            compute_pass.set_bind_group(1, &horizontal_bind_group, &[]);
            let (dispatch_height, dispatch_with) =
                compute_work_group_count((texture.size().width, texture.size().height), (1, 128));
            compute_pass.dispatch_workgroups(dispatch_with, dispatch_height, 1);
        }

        // Get the result.

        let padded_bytes_per_row = padded_bytes_per_row(texture.size().width);
        let unpadded_bytes_per_row = texture.size().width as usize * 4;

        let output_buffer_size = padded_bytes_per_row as u64
            * texture.size().height as u64
            * std::mem::size_of::<u8>() as u64;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &horizontal_pass_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row as u32),
                    rows_per_image: Some(texture.size().height),
                },
            },
            texture.size(),
        );

        //execute
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});

        self.device.poll(wgpu::Maintain::Wait);

        let padded_data = buffer_slice.get_mapped_range();

        for (padded, pixels) in padded_data
            .chunks_exact(padded_bytes_per_row)
            .zip(data.chunks_exact_mut(unpadded_bytes_per_row))
        {
            pixels.copy_from_slice(&padded[..unpadded_bytes_per_row]);
        }
    }

    pub fn texture_from_data(&self, data: &[u8], width: u32, height: u32) -> Texture {
        let texture_size = wgpu::Extent3d {
            width: width,
            height: height,
            depth_or_array_layers: 1,
        };

        let texture: Texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("input texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });

        self.queue.write_texture(
            texture.as_image_copy(),
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: None, // Doesn't need to be specified as we are writing a single image.
            },
            texture_size,
        );
        texture
    }
}

pub fn compute_work_group_count(
    (width, height): (u32, u32),
    (workgroup_width, workgroup_height): (u32, u32),
) -> (u32, u32) {
    let width = (width + workgroup_width - 1) / workgroup_width;
    let height = (height + workgroup_height - 1) / workgroup_height;

    (width, height)
}

fn kernel_size_for_sigma(sigma: f32) -> u32 {
    2 * (sigma * 3.0).ceil() as u32 + 1
}

fn kernel(sigma: f32) -> Kernel {
    let kernel_size = kernel_size_for_sigma(sigma);
    let mut values = vec![0.0; kernel_size as usize];
    let kernel_radius = (kernel_size as usize - 1) / 2;
    for index in 0..=kernel_radius {
        let normpdf = normalized_probablility_density_function(index as f32, sigma);
        values[kernel_radius + index] = normpdf;
        values[kernel_radius - index] = normpdf;
    }

    Kernel::new(values)
}

fn normalized_probablility_density_function(x: f32, sigma: f32) -> f32 {
    0.39894 * (-0.5 * x * x / (sigma * sigma)).exp() / sigma
}

fn padded_bytes_per_row(width: u32) -> usize {
    let bytes_per_row = width as usize * 4;
    let padding = (256 - bytes_per_row % 256) % 256;
    bytes_per_row + padding
}
