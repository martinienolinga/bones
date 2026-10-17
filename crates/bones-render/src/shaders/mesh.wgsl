// Maillage skinné (skinning linéaire calculé sur le GPU)

struct MeshIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) bones: vec4<u32>,
    @location(3) weights: vec4<f32>,
};

struct MeshOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
    @location(2) heat: f32,
};

@vertex
fn vs_mesh(v: MeshIn) -> MeshOut {
    let count = u32(globals.params.y);
    let selected = globals.params.x;
    var skin = mat4x4<f32>(vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0));
    var total = 0.0;
    var heat = 0.0;
    for (var i = 0u; i < 4u; i = i + 1u) {
        let w = v.weights[i];
        let b = v.bones[i];
        if (w > 0.0 && b < count) {
            skin = skin + joints[b] * w;
            total = total + w;
            if (selected >= 0.0 && b == u32(selected)) {
                heat = heat + w;
            }
        }
    }
    var world = vec4<f32>(v.position, 1.0);
    var normal = v.normal;
    if (total > 1e-5) {
        skin = skin * (1.0 / total);
        world = skin * world;
        normal = (skin * vec4<f32>(v.normal, 0.0)).xyz;
    }
    var out: MeshOut;
    out.clip = globals.view_proj * world;
    out.normal = normal;
    out.world = world.xyz;
    out.heat = heat;
    return out;
}

@fragment
fn fs_mesh(in: MeshOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    var n = normalize(in.normal);
    if (!front) {
        n = -n;
    }
    var base = vec3<f32>(0.62, 0.6, 0.58);
    if (globals.params.x >= 0.0) {
        base = mix(vec3<f32>(0.05, 0.08, 0.35), heat_color(in.heat), 0.92);
    }
    return vec4<f32>(to_srgb(shade(base, n, in.world)), globals.params.z);
}

