// Shaders de PaintSharp Bones.
// Les calculs d'éclairage se font en linéaire ; la sortie est convertie en
// sRGB à la main car la surface est en Bgra8Unorm (format préféré d'egui).

struct Globals {
    view_proj: mat4x4<f32>,
    eye: vec4<f32>,
    // x : os affiché en carte de poids (-1 : pas de carte), y : nombre de
    // joints, z : opacité du maillage, w : 1 pour afficher matériaux et
    // textures.
    params: vec4<f32>,
    // x : 1 pour afficher en noir les sommets sans poids (« Zero Weights »).
    paint: vec4<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var<storage, read> joints: array<mat4x4<f32>>;

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    return pow(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), vec3<f32>(1.0 / 2.2));
}

// Rampe des poids de Blender, en couleurs d'affichage (sRGB) : bleu (0) →
// cyan (0,25) → vert (0,5) → jaune (0,75) → rouge (1). Même rampe que
// `bones_ui::theme::heat` pour l'interface.
fn heat_color(t: f32) -> vec3<f32> {
    let x = clamp(t, 0.0, 1.0);
    let r = clamp(4.0 * x - 2.0, 0.0, 1.0);
    let g = clamp(4.0 * x, 0.0, 1.0) * clamp(4.0 - 4.0 * x, 0.0, 1.0);
    let b = clamp(2.0 - 4.0 * x, 0.0, 1.0);
    return vec3<f32>(r, g, b);
}

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    return pow(c, vec3<f32>(2.2));
}

fn shade(base: vec3<f32>, n: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let v = normalize(globals.eye.xyz - world);
    // Lumière frontale (« headlight ») + ambiance hémisphérique, façon argile.
    let key = max(dot(n, normalize(v + vec3<f32>(0.25, 0.45, 0.0))), 0.0);
    let hemi = mix(0.18, 0.32, n.y * 0.5 + 0.5);
    let rim = pow(1.0 - max(dot(n, v), 0.0), 3.0) * 0.18;
    return base * (hemi + 0.8 * key) + vec3<f32>(rim);
}

