#import bevy_pbr::forward_io::VertexOutput

struct GradientSkyMaterial {
    top_color: vec4<f32>,
    bottom_color: vec4<f32>,
};

@group(2) @binding(0)
var<uniform> material: GradientSkyMaterial;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Use world position Y to create vertical gradient
    // Normalize based on expected sky dome height
    let normalized_y = clamp((in.world_position.y + 50.0) / 150.0, 0.0, 1.0);

    // Smooth interpolation with a slight curve for more natural sky look
    let t = smoothstep(0.0, 1.0, normalized_y);

    // Mix bottom (horizon) to top (zenith)
    let color = mix(material.bottom_color, material.top_color, t);

    return color;
}
