// Maillage skinné (skinning linéaire calculé sur le GPU), texturé.

struct Material {
    // Couleur de base linéaire.
    base_color: vec4<f32>,
    // x : 1 si une texture est appliquée.
    flags: vec4<f32>,
};

@group(1) @binding(0) var<uniform> material: Material;
@group(1) @binding(1) var base_texture: texture_2d<f32>;
@group(1) @binding(2) var base_sampler: sampler;

struct MeshIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) bones: vec4<u32>,
    @location(3) weights: vec4<f32>,
    @location(4) uv: vec2<f32>,
};

struct MeshOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) normal: vec3<f32>,
    @location(1) world: vec3<f32>,
    @location(2) heat: f32,
    @location(3) uv: vec2<f32>,
    // 1 si l'os de la carte influence le sommet (poids non nul).
    @location(4) referenced: f32,
};

@vertex
fn vs_mesh(v: MeshIn) -> MeshOut {
    let count = u32(globals.params.y);
    let selected = globals.params.x;
    var skin = mat4x4<f32>(vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0));
    var total = 0.0;
    var heat = 0.0;
    var referenced = 0.0;
    for (var i = 0u; i < 4u; i = i + 1u) {
        let w = v.weights[i];
        let b = v.bones[i];
        if (w > 0.0 && b < count) {
            skin = skin + joints[b] * w;
            total = total + w;
            if (selected >= 0.0 && b == u32(selected)) {
                heat = heat + w;
                referenced = 1.0;
            }
        }
    }
    // Placement du modèle avant le skinning (identité pour le corps).
    var world = globals.model * vec4<f32>(v.position, 1.0);
    var normal = (globals.model * vec4<f32>(v.normal, 0.0)).xyz;
    if (total > 1e-5) {
        skin = skin * (1.0 / total);
        world = skin * world;
        normal = (skin * vec4<f32>(normal, 0.0)).xyz;
    }
    var out: MeshOut;
    out.clip = globals.view_proj * world;
    out.normal = normal;
    out.world = world.xyz;
    out.heat = heat;
    out.uv = v.uv;
    out.referenced = referenced;
    return out;
}

// Sommet d'un objet rigide : pas de poids, uniquement la matrice du modèle.
struct ObjectIn {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_object(v: ObjectIn) -> MeshOut {
    let world = globals.model * vec4<f32>(v.position, 1.0);
    var out: MeshOut;
    out.clip = globals.view_proj * world;
    out.normal = (globals.model * vec4<f32>(v.normal, 0.0)).xyz;
    out.world = world.xyz;
    out.heat = 0.0;
    out.uv = v.uv;
    out.referenced = 0.0;
    return out;
}

@fragment
fn fs_mesh(in: MeshOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    var n = normalize(in.normal);
    if (!front) {
        n = -n;
    }
    // Échantillonnage hors de tout branchement (contrôle uniforme) ; la
    // texture blanche des matériaux sans image laisse la couleur intacte.
    let texel = textureSample(base_texture, base_sampler, in.uv).rgb;
    var base = vec3<f32>(0.62, 0.6, 0.58);
    if (globals.params.w > 0.5) {
        base = material.base_color.rgb * mix(vec3<f32>(1.0), texel, material.flags.x);
    }
    if (globals.params.x >= 0.0) {
        base = to_linear(heat_color(in.heat));
        if (globals.paint.x > 0.5) {
            base = base * clamp(in.referenced, 0.0, 1.0);
        }
    }
    var color = to_srgb(shade(base, n, in.world));
    // Sélection : liseré d'accent concentré sur la silhouette (aplat, net).
    if (globals.paint.z > 0.5) {
        let v = normalize(globals.eye.xyz - in.world);
        let edge = pow(1.0 - max(dot(n, v), 0.0), 4.0);
        color = mix(color, globals.tint.rgb, clamp(edge, 0.0, 1.0));
    }
    return vec4<f32>(color, globals.params.z);
}

// Arêtes du maillage (tracé en fil de fer) sur la carte de poids.
@fragment
fn fs_wire() -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 0.32);
}

