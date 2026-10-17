// Os octaédriques et poignées (instanciés)

struct BoneIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) m0: vec4<f32>,
    @location(3) m1: vec4<f32>,
    @location(4) m2: vec4<f32>,
    @location(5) m3: vec4<f32>,
    @location(6) color: vec4<f32>,
};

struct BoneOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_bone(v: BoneIn) -> BoneOut {
    let model = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let world = model * vec4<f32>(v.position, 1.0);
    var out: BoneOut;
    out.clip = globals.view_proj * world;
    out.normal = (model * vec4<f32>(v.normal, 0.0)).xyz;
    out.world = world.xyz;
    out.color = v.color;
    return out;
}

@fragment
fn fs_bone(in: BoneOut) -> @location(0) vec4<f32> {
    let n = normalize(in.normal);
    let v = normalize(globals.eye.xyz - in.world);
    let light = 0.55 + 0.45 * abs(dot(n, v));
    return vec4<f32>(to_srgb(in.color.rgb * light), in.color.a);
}

