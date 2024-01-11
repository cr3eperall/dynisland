struct Settings {
    filter_size : u32,
};

struct Lod{
    lod: f32,
};

struct Orientation {
    vertical : u32,
};

struct Kernel {
  sum: f32,
  values : array<f32>,
};

@group(0) @binding(0) var<uniform> settings : Settings;
@group(0) @binding(1) var<storage, read> kernel : Kernel;
@group(0) @binding(2) var<uniform> lod : Lod;
@group(0) @binding(3) var t_sampler : sampler;
@group(1) @binding(0) var input_texture : texture_2d<f32>;
@group(1) @binding(1) var output_texture : texture_storage_2d<rgba8unorm, write>;
@group(1) @binding(2) var<uniform> orientation: Orientation;

@compute
@workgroup_size(128)
fn main(
  @builtin(global_invocation_id) global_id : vec3<u32>,
) {
    let filter_radius = i32((settings.filter_size - 1u) / 2u);
    let filter_size = i32(settings.filter_size);
    let dimensions = vec2<i32>(textureDimensions(output_texture));
    var position = vec2<i32>(global_id.xy);
    if (orientation.vertical == 0u) {
        position = position.yx;
    }
    let f_position = vec2<f32>(position.xy) / vec2<f32>(dimensions);
    
    if(position.x >= dimensions.x || position.y >= dimensions.y) {
        return;
    }

    // let original = textureLoad(input_texture, position, 0);
    var color : vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    
    for (var i : i32 = 0; i < filter_size; i = i + 1) {
        if (orientation.vertical > 0u) {
            let y = f32(position.y - filter_radius + i)/f32(dimensions.y);
            color = color + kernel.values[i] * textureSampleLevel(input_texture, t_sampler, vec2<f32>(f_position.x, y), lod.lod);
        } else {
            let x = f32(position.x - filter_radius + i)/f32(dimensions.x);
            color = color + kernel.values[i] * textureSampleLevel(input_texture, t_sampler, vec2<f32>(x, f_position.y), lod.lod);
        }
    }
    color = color / kernel.sum;

    textureStore(output_texture, position, color);
}