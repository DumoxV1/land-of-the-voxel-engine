struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@location(0) p: vec2<f32>, @location(1) uv: vec2<f32>) -> VOut {
    var o: VOut;
    o.pos = vec4<f32>(p, 0.0, 1.0);
    o.uv = uv;
    return o;
}

@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let c = textureSample(tex, samp, uv);
    if (c.a < 0.5) { discard; }
    return c;
}
