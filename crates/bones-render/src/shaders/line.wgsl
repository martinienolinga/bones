// Lignes (grille, trajectoires, repères)

struct LineIn {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct LineOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_line(v: LineIn) -> LineOut {
    var out: LineOut;
    out.clip = globals.view_proj * vec4<f32>(v.position, 1.0);
    out.color = v.color;
    return out;
}

@fragment
fn fs_line(in: LineOut) -> @location(0) vec4<f32> {
    return vec4<f32>(to_srgb(in.color.rgb), in.color.a);
}
