// Flou « dual Kawase » (Bjørge, SIGGRAPH 2015) pour le verre dépoli de
// l'interface : des réductions successives (5 lectures) puis des
// agrandissements (8 lectures), chacun sur une texture deux fois plus
// petite ou plus grande. Le filtrage bilinéaire fait l'essentiel du travail.

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var smp: sampler;

struct Out {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Triangle couvrant toute la cible.
@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> Out {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: Out;
    o.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}

fn tap(uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(src, smp, uv, 0.0);
}

@fragment
fn fs_down(in: Out) -> @location(0) vec4<f32> {
    let h = 1.0 / vec2<f32>(textureDimensions(src));
    var sum = tap(in.uv) * 4.0;
    sum += tap(in.uv - h);
    sum += tap(in.uv + h);
    sum += tap(in.uv + vec2<f32>(h.x, -h.y));
    sum += tap(in.uv - vec2<f32>(h.x, -h.y));
    return sum / 8.0;
}

@fragment
fn fs_up(in: Out) -> @location(0) vec4<f32> {
    let h = 0.5 / vec2<f32>(textureDimensions(src));
    var sum = tap(in.uv + vec2<f32>(-2.0 * h.x, 0.0));
    sum += tap(in.uv + vec2<f32>(-h.x, h.y)) * 2.0;
    sum += tap(in.uv + vec2<f32>(0.0, 2.0 * h.y));
    sum += tap(in.uv + vec2<f32>(h.x, h.y)) * 2.0;
    sum += tap(in.uv + vec2<f32>(2.0 * h.x, 0.0));
    sum += tap(in.uv + vec2<f32>(h.x, -h.y)) * 2.0;
    sum += tap(in.uv + vec2<f32>(0.0, -2.0 * h.y));
    sum += tap(in.uv + vec2<f32>(-h.x, -h.y)) * 2.0;
    return sum / 12.0;
}
