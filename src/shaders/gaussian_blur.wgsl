struct Settings {
    filter_size : u32,
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
    let dimensions = vec2<i32>(textureDimensions(input_texture));
    var position = vec2<i32>(global_id.xy);
    if (orientation.vertical == 0u) {
        position = position.yx;
    }
    
    if(position.x >= dimensions.x || position.y >= dimensions.y) {
        return;
    }

    let original = textureLoad(input_texture, position, 0);
    var color : vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    
    for (var i : i32 = 0; i < filter_size; i = i + 1) {
        if (orientation.vertical > 0u) {
            let y = position.y - filter_radius + i;
            color = color + kernel.values[i] * textureLoad(input_texture, vec2<i32>(position.x, y), 0);
        } else {
            let x = position.x - filter_radius + i;
            color = color + kernel.values[i] * textureLoad(input_texture, vec2<i32>(x, position.y), 0);
        }
    }
    color = color / kernel.sum;

    textureStore(output_texture, position, color);
}