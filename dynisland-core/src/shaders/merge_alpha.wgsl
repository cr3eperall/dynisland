struct Opacity {
    value : f32,
};

struct Lod{
    lod: f32,
};

@group(0) @binding(0) var input_texture_1 : texture_2d<f32>;
@group(0) @binding(1) var input_texture_2 : texture_2d<f32>;
@group(0) @binding(2) var output_texture : texture_storage_2d<rgba8unorm, write>;
@group(0) @binding(3) var<uniform> opacity_1 : Opacity;
@group(0) @binding(4) var<uniform> opacity_2 : Opacity;
@group(0) @binding(5) var t_sampler : sampler;

// @group(2) @binding(0) var<uniform> lod : Lod;


@compute
@workgroup_size(16,16)
fn main(
  @builtin(global_invocation_id) global_id : vec3<u32>,
) {
    // let lod=lod.lod;
    let dimensions = vec2<i32>(textureDimensions(output_texture));
    let position = vec2<i32>(global_id.xy);
    let f_position = vec2<f32>(position.xy) / vec2<f32>(dimensions);
    
    if(position.x >= dimensions.x || position.y >= dimensions.y) {
        return;
    }

    let original_1 = textureSampleLevel(input_texture_1, t_sampler, f_position, 0.0); //maybe use lod 0
    let original_2 = textureSampleLevel(input_texture_2, t_sampler, f_position, 0.0);
    var color : vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    
    color = (original_1 * opacity_1.value + original_2 * opacity_2.value)/(opacity_1.value+opacity_2.value);

    textureStore(output_texture, position, color);
}