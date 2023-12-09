struct Opacity {
    value : f32,
};

@group(0) @binding(0) var<uniform> opacity_1 : Opacity;
@group(0) @binding(1) var<uniform> opacity_2 : Opacity;

@group(1) @binding(0) var input_texture_1 : texture_2d<f32>;
@group(1) @binding(1) var input_texture_2 : texture_2d<f32>;
@group(1) @binding(2) var output_texture : texture_storage_2d<rgba8unorm, write>;

@compute
@workgroup_size(16,16)
fn main(
  @builtin(global_invocation_id) global_id : vec3<u32>,
) {
    let dimensions = vec2<i32>(textureDimensions(input_texture_1));
    var position = vec2<i32>(global_id.xy);
    
    if(position.x >= dimensions.x || position.y >= dimensions.y) {
        return;
    }

    let original_1 = textureLoad(input_texture_1, position, 0);
    let original_2 = textureLoad(input_texture_2, position, 0);
    var color : vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    
    color = (original_1 * opacity_1.value + original_2 * opacity_2.value)/(opacity_1.value+opacity_2.value);

    textureStore(output_texture, position, color);
}