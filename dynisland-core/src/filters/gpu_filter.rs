use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use wgpu::util::DeviceExt;
use wgpu::{
    util::BufferInitDescriptor, BindGroupDescriptor, BindGroupEntry, BindingResource,
    CommandEncoderDescriptor, ComputePassDescriptor, ComputePipelineDescriptor,
    ShaderModuleDescriptor, ShaderSource, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages,
};
use wgpu::{
    Buffer, BufferUsages, ComputePipeline, Device, Extent3d, InstanceFlags, Queue, Texture,
    TextureViewDescriptor,
};

use super::filter::kernel_size_for_sigma;

const GAUSSIAN_BLUR_SHADER: &str = include_str!("../shaders/gaussian_blur.wgsl");
const MERGE_ALPHA_SHADER: &str = include_str!("../shaders/merge_alpha.wgsl");
const LOD_FACTOR: f32 = 1.0 / 5.0;
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
    pub vertical: Buffer,
    pub horizontal: Buffer,
    pub blur_pipeline: ComputePipeline,
    pub merge_pipeline: ComputePipeline,
}

pub static WGPU_INSTANCE: Lazy<Mutex<GpuContext>> = Lazy::new(|| Mutex::new(GpuContext::new()));
impl GpuContext {
    pub fn new() -> Self {
        let name = "gaussian blur";
        let name_merge = "merge alpha";
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
        let blur_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(format!("{} shader", name).as_str()),
            source: ShaderSource::Wgsl(GAUSSIAN_BLUR_SHADER.into()),
        });

        let merge_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some(format!("{} shader", name_merge).as_str()),
            source: ShaderSource::Wgsl(MERGE_ALPHA_SHADER.into()),
        });

        let blur_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some(format!("{} pipeline", name).as_str()),
            layout: None,
            module: &blur_shader,
            entry_point: "main",
        });

        let merge_pipeline = device.create_compute_pipeline(&ComputePipelineDescriptor {
            label: Some(format!("{} pipeline", name_merge).as_str()),
            layout: None,
            module: &merge_shader,
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
            blur_pipeline,
            merge_pipeline,
        }
    }

    pub fn gaussian_blur(&self, data: &mut [u8], width: usize, height: usize, sigma: f32) {
        let name = "gaussian blur";

        //Blur

        let texture =
            self.texture_from_data(data, width.try_into().unwrap(), height.try_into().unwrap());
        let texture_size = texture.size();

        // let lod_1 = sigma * LOD_FACTOR;
        let lod = 0.0;
        let final_size = Extent3d {
            width: (texture_size.width as f32 / f32::exp2(lod)).ceil() as u32,
            height: (texture_size.height as f32 / f32::exp2(lod)).ceil() as u32,
            depth_or_array_layers: 1,
        };

        let lod_data_1 = self.get_lod_settings(lod);

        let settings_binding = self.get_settings_bind_group(sigma, &lod_data_1);

        let (vertical_pass_texture, vertical_bind_group) =
            self.get_vertical_bind_group(&texture, final_size.height);

        let (horizontal_pass_texture, horizontal_bind_group) = self.get_horizontal_bind_group(
            vertical_pass_texture,
            final_size.width,
            final_size.height,
        );

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(format!("{} pass", name).as_str()),
                timestamp_writes: None,
            });
            //texture_1 blur
            compute_pass.set_pipeline(&self.blur_pipeline);
            compute_pass.set_bind_group(0, &settings_binding, &[]);
            compute_pass.set_bind_group(1, &vertical_bind_group, &[]);
            // compute_pass.set_bind_group(2, &lod_data_1, &[]);
            let (dispatch_with, dispatch_height) =
                compute_work_group_count((texture_size.width, final_size.height), (128, 1));
            compute_pass.dispatch_workgroups(dispatch_with, dispatch_height, 1); //TODO maybe dispatch texture_1 and texture_2 at the same time with 2 layers
            compute_pass.set_bind_group(1, &horizontal_bind_group, &[]);
            let (dispatch_height, dispatch_with) =
                compute_work_group_count((final_size.width, final_size.height), (1, 128));
            compute_pass.dispatch_workgroups(dispatch_with, dispatch_height, 1);
        }
        let output_buffer =
            self.get_output_buffer(&mut encoder, texture_size, horizontal_pass_texture);

        //execute
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});

        self.device.poll(wgpu::Maintain::Wait);

        copy_buffer_to_slice(buffer_slice, texture_size, data);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_blur_and_merge_opacity_dual(
        &self,
        // target: &mut [u8],
        data_1: &mut [u8],
        data_2: &mut [u8],
        size: (usize, usize),
        sigma_1: f32,
        sigma_2: f32,
        opacity_1: f32,
        opacity_2: f32,
    ) {
        let name = "double blur + merge opacity";
        let width = size.0;
        let height = size.1;

        //Blur

        let texture_1 = self.texture_from_data(
            data_1,
            width.try_into().unwrap(),
            height.try_into().unwrap(),
        );
        let texture_2 = self.texture_from_data(
            data_2,
            width.try_into().unwrap(),
            height.try_into().unwrap(),
        );
        let texture_size = texture_1.size();

        let lod_1 = sigma_1 * LOD_FACTOR;
        let final_size_1 = Extent3d {
            width: (texture_size.width as f32 / f32::exp2(lod_1)).ceil() as u32,
            height: (texture_size.height as f32 / f32::exp2(lod_1)).ceil() as u32,
            depth_or_array_layers: 1,
        };

        let lod_2 = sigma_2 * LOD_FACTOR;
        let final_size_2 = Extent3d {
            width: (texture_size.width as f32 / f32::exp2(lod_2)).ceil() as u32,
            height: (texture_size.height as f32 / f32::exp2(lod_2)).ceil() as u32,
            depth_or_array_layers: 1,
        };

        let lod_data_1 = self.get_lod_settings(lod_1);
        let lod_data_2 = self.get_lod_settings(lod_2);

        let settings_binding_1 = self.get_settings_bind_group(sigma_1, &lod_data_1);

        let settings_binding_2 = self.get_settings_bind_group(sigma_2, &lod_data_2);

        let (vertical_pass_texture_1, vertical_bind_group_1) =
            self.get_vertical_bind_group(&texture_1, final_size_1.height);

        let (horizontal_pass_texture_1, horizontal_bind_group_1) = self.get_horizontal_bind_group(
            vertical_pass_texture_1,
            final_size_1.width,
            final_size_1.height,
        );

        let (vertical_pass_texture_2, vertical_bind_group_2) =
            self.get_vertical_bind_group(&texture_2, final_size_2.height);

        let (horizontal_pass_texture_2, horizontal_bind_group_2) = self.get_horizontal_bind_group(
            vertical_pass_texture_2,
            final_size_2.width,
            final_size_2.height,
        );

        // Merge

        let merge_opacity_data = self.get_merge_opacity_data(opacity_1, opacity_2);

        let merge_output_texture = self.device.create_texture(&TextureDescriptor {
            label: None,
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::STORAGE_BINDING,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });

        let merge_textures_bind = self.get_merge_bind_group(
            horizontal_pass_texture_1,
            horizontal_pass_texture_2,
            &merge_output_texture,
            &merge_opacity_data.0,
            &merge_opacity_data.1,
            &lod_data_1.1,
        );

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: None });
        {
            let mut compute_pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some(format!("{} pass", name).as_str()),
                timestamp_writes: None,
            });
            //texture_1 blur
            compute_pass.set_pipeline(&self.blur_pipeline);
            compute_pass.set_bind_group(0, &settings_binding_1, &[]);
            compute_pass.set_bind_group(1, &vertical_bind_group_1, &[]);
            // compute_pass.set_bind_group(2, &lod_data_1, &[]);
            let (dispatch_with, dispatch_height) =
                compute_work_group_count((texture_size.width, final_size_1.height), (128, 1));
            compute_pass.dispatch_workgroups(dispatch_with, dispatch_height, 1); //TODO maybe dispatch texture_1 and texture_2 at the same time with 2 layers
            compute_pass.set_bind_group(1, &horizontal_bind_group_1, &[]);
            let (dispatch_height, dispatch_with) =
                compute_work_group_count((final_size_1.width, final_size_1.height), (1, 128));
            compute_pass.dispatch_workgroups(dispatch_with, dispatch_height, 1);

            //texture_2 blur
            compute_pass.set_bind_group(0, &settings_binding_2, &[]);
            compute_pass.set_bind_group(1, &vertical_bind_group_2, &[]);
            // compute_pass.set_bind_group(2, &lod_data_2, &[]);
            let (dispatch_with, dispatch_height) =
                compute_work_group_count((texture_size.width, final_size_2.height), (128, 1));
            compute_pass.dispatch_workgroups(dispatch_with, dispatch_height, 1);
            compute_pass.set_bind_group(1, &horizontal_bind_group_2, &[]);
            let (dispatch_height, dispatch_with) =
                compute_work_group_count((final_size_2.width, final_size_2.height), (1, 128));
            compute_pass.dispatch_workgroups(dispatch_with, dispatch_height, 1);

            //texture merge
            compute_pass.set_pipeline(&self.merge_pipeline);
            compute_pass.set_bind_group(0, &merge_textures_bind, &[]);
            // compute_pass.set_bind_group(2, &lod_data_1, &[]);
            let (dispatch_with, dispatch_height) =
                compute_work_group_count((texture_size.width, texture_size.height), (16, 16));
            compute_pass.dispatch_workgroups(dispatch_with, dispatch_height, 1);
        }
        let output_buffer =
            self.get_output_buffer(&mut encoder, texture_size, merge_output_texture);

        //execute
        self.queue.submit(Some(encoder.finish()));

        let buffer_slice = output_buffer.slice(..);
        buffer_slice.map_async(wgpu::MapMode::Read, |_| {});

        self.device.poll(wgpu::Maintain::Wait);

        copy_buffer_to_slice(buffer_slice, texture_size, data_1);
    }

    fn get_output_buffer(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture_size: wgpu::Extent3d,
        merge_output_texture: Texture,
    ) -> Buffer {
        let padded_bytes_per_row = padded_bytes_per_row(texture_size.width);
        let output_buffer_size = padded_bytes_per_row as u64
            * texture_size.height as u64
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
                texture: &merge_output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row as u32),
                    rows_per_image: Some(texture_size.height),
                },
            },
            texture_size,
        );
        output_buffer
    }

    fn get_merge_bind_group(
        &self,
        horizontal_pass_texture_1: Texture,
        horizontal_pass_texture_2: Texture,
        merge_output_texture: &Texture,
        merge_opacity_1: &Buffer,
        merge_opacity_2: &Buffer,
        lod_sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        let merge_textures_bind = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Texture bind group"),
            layout: &self.merge_pipeline.get_bind_group_layout(0),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &horizontal_pass_texture_1.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &horizontal_pass_texture_2.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &merge_output_texture.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: merge_opacity_1.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: merge_opacity_2.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(lod_sampler),
                },
            ],
        });
        merge_textures_bind
    }

    fn get_merge_opacity_data(
        &self,
        opacity_1: f32,
        opacity_2: f32,
    ) -> (wgpu::Buffer, wgpu::Buffer) {
        let merge_opacity_1 = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("texture_1 opacity"),
            contents: bytemuck::cast_slice(&[opacity_1]),
            usage: BufferUsages::UNIFORM,
        });
        let merge_opacity_2 = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("texture_2 opacity"),
            contents: bytemuck::cast_slice(&[opacity_2]),
            usage: BufferUsages::UNIFORM,
        });
        (merge_opacity_1, merge_opacity_2)
        // let merge_opacity_bind = self.device.create_bind_group(&BindGroupDescriptor {
        //     label: Some("Images alpha value"),
        //     layout: &self.merge_pipeline.get_bind_group_layout(0),
        //     entries: &[
        //         BindGroupEntry {
        //             binding: 0,
        //             resource: merge_opacity_1.as_entire_binding(),
        //         },
        //         BindGroupEntry {
        //             binding: 1,
        //             resource: merge_opacity_2.as_entire_binding(),
        //         },
        //     ],
        // });
        // merge_opacity_bind
    }

    fn get_horizontal_bind_group(
        &self,
        vertical_pass_texture_1: Texture,
        final_w: u32,
        final_h: u32,
    ) -> (Texture, wgpu::BindGroup) {
        let horizontal_pass_texture_1 = self.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: final_w,
                height: final_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::STORAGE_BINDING,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });
        let horizontal_bind_group_1 = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Texture bind group"),
            layout: &self.blur_pipeline.get_bind_group_layout(1),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &vertical_pass_texture_1.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &horizontal_pass_texture_1.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.horizontal.as_entire_binding(),
                },
            ],
        });
        (horizontal_pass_texture_1, horizontal_bind_group_1)
    }

    fn get_vertical_bind_group(
        &self,
        texture_1: &Texture,
        final_h: u32,
    ) -> (Texture, wgpu::BindGroup) {
        let vertical_pass_texture_1 = self.device.create_texture(&TextureDescriptor {
            label: None,
            size: Extent3d {
                width: texture_1.size().width,
                height: final_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::STORAGE_BINDING,
            view_formats: &[TextureFormat::Rgba8Unorm],
        });
        let vertical_bind_group_1 = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Texture bind group"),
            layout: &self.blur_pipeline.get_bind_group_layout(1),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &texture_1.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &vertical_pass_texture_1.create_view(&TextureViewDescriptor::default()),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: self.vertical.as_entire_binding(),
                },
            ],
        });
        (vertical_pass_texture_1, vertical_bind_group_1)
    }

    fn get_settings_bind_group(
        &self,
        sigma: f32,
        lod_data: &(wgpu::Buffer, wgpu::Sampler),
    ) -> wgpu::BindGroup {
        let kernel = kernel(sigma);
        let kernel_size = kernel.size() as i32;
        let kernel_size_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Image info"),
            contents: bytemuck::cast_slice(&[kernel_size]),
            usage: BufferUsages::UNIFORM,
        });
        let kernel_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&kernel.packed_data()[..]),
            usage: BufferUsages::STORAGE | BufferUsages::COPY_DST,
        });
        let compute_constants_1 = self.device.create_bind_group(&BindGroupDescriptor {
            label: Some("Compute constants"),
            layout: &self.blur_pipeline.get_bind_group_layout(0),
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: kernel_size_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: kernel_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: lod_data.0.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&lod_data.1),
                },
            ],
        });
        compute_constants_1
    }

    fn get_lod_settings(&self, lod: f32) -> (wgpu::Buffer, wgpu::Sampler) {
        let lod_buffer = self.device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Image info"),
            contents: bytemuck::cast_slice(&[lod]),
            usage: BufferUsages::UNIFORM,
        });
        let sampler_buffer = self.device.create_sampler(&wgpu::SamplerDescriptor {
            //TODO look into this
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        (lod_buffer, sampler_buffer)
        // self.device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     label: None,
        //     layout: &self.blur_pipeline.get_bind_group_layout(2),
        //     entries: &[
        //         wgpu::BindGroupEntry {
        //             binding: 0,
        //             resource: lod_buffer.as_entire_binding(),
        //         },
        //         wgpu::BindGroupEntry {
        //             binding: 1,
        //             resource: wgpu::BindingResource::Sampler(&sampler_buffer),
        //         },
        //     ],
        // })
    }

    pub fn texture_from_data(&self, data: &[u8], width: u32, height: u32) -> Texture {
        let texture_size = wgpu::Extent3d {
            width,
            height,
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

fn copy_buffer_to_slice(
    buffer_slice: wgpu::BufferSlice<'_>,
    texture_size: wgpu::Extent3d,
    output: &mut [u8],
) {
    let padded_bytes_per_row = padded_bytes_per_row(texture_size.width);
    let unpadded_bytes_per_row = texture_size.width as usize * 4;

    let padded_data = buffer_slice.get_mapped_range();

    for (padded, pixels) in padded_data
        .chunks_exact(padded_bytes_per_row)
        .zip(output.chunks_exact_mut(unpadded_bytes_per_row))
    {
        pixels.copy_from_slice(&padded[..unpadded_bytes_per_row]);
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
