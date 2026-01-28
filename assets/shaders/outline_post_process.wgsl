// Luminance-based edge detection post-process shader
// Uses Sobel operator on luminance to detect edges and draw subtle outlines

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var screen_texture: texture_2d<f32>;
@group(0) @binding(1) var screen_sampler: sampler;

struct OutlineSettings {
    outline_color: vec4<f32>,
    outline_thickness: f32,
    edge_threshold: f32,
    _padding1: f32,
    _padding2: f32,
}

@group(0) @binding(2) var<uniform> settings: OutlineSettings;

// Convert color to luminance
fn luminance(color: vec3<f32>) -> f32 {
    return dot(color, vec3<f32>(0.299, 0.587, 0.114));
}

// Sample luminance at offset position
fn sample_luminance(uv: vec2<f32>, offset: vec2<f32>, texel_size: vec2<f32>) -> f32 {
    let sample_uv = uv + offset * texel_size * settings.outline_thickness;
    let color = textureSample(screen_texture, screen_sampler, sample_uv).rgb;
    return luminance(color);
}

// Roberts cross edge detection (fast, 4 samples)
fn roberts_edge(uv: vec2<f32>, texel_size: vec2<f32>) -> f32 {
    let c  = sample_luminance(uv, vec2<f32>(0.0, 0.0), texel_size);
    let r  = sample_luminance(uv, vec2<f32>(1.0, 0.0), texel_size);
    let b  = sample_luminance(uv, vec2<f32>(0.0, 1.0), texel_size);
    let br = sample_luminance(uv, vec2<f32>(1.0, 1.0), texel_size);

    let gx = c - br;
    let gy = r - b;

    return sqrt(gx * gx + gy * gy);
}

// Sobel edge detection (better quality, 8 samples)
fn sobel_edge(uv: vec2<f32>, texel_size: vec2<f32>) -> f32 {
    let tl = sample_luminance(uv, vec2<f32>(-1.0, -1.0), texel_size);
    let tc = sample_luminance(uv, vec2<f32>( 0.0, -1.0), texel_size);
    let tr = sample_luminance(uv, vec2<f32>( 1.0, -1.0), texel_size);
    let ml = sample_luminance(uv, vec2<f32>(-1.0,  0.0), texel_size);
    let mr = sample_luminance(uv, vec2<f32>( 1.0,  0.0), texel_size);
    let bl = sample_luminance(uv, vec2<f32>(-1.0,  1.0), texel_size);
    let bc = sample_luminance(uv, vec2<f32>( 0.0,  1.0), texel_size);
    let br = sample_luminance(uv, vec2<f32>( 1.0,  1.0), texel_size);

    // Sobel kernels
    let gx = -tl - 2.0 * ml - bl + tr + 2.0 * mr + br;
    let gy = -tl - 2.0 * tc - tr + bl + 2.0 * bc + br;

    return sqrt(gx * gx + gy * gy);
}

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;

    // Get texture dimensions for proper texel offset
    let dims = vec2<f32>(textureDimensions(screen_texture));
    let texel_size = 1.0 / dims;

    // Sample original scene
    let scene_color = textureSample(screen_texture, screen_sampler, uv);

    // Calculate edge strength using Roberts cross (faster)
    let edge = roberts_edge(uv, texel_size);

    // Apply threshold with smooth falloff
    let edge_strength = smoothstep(settings.edge_threshold * 0.5, settings.edge_threshold, edge);

    // Blend outline color with scene
    let outline_factor = edge_strength * settings.outline_color.a;
    let final_color = mix(scene_color.rgb, settings.outline_color.rgb, outline_factor);

    return vec4<f32>(final_color, scene_color.a);
}
