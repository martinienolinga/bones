//! Carte de poids du mode Weights rendue hors écran : rampe de Blender (bleu
//! à 0, rouge à 1), poids nuls en noir et fil de fer.

mod common;

use bones_core::{OrbitCamera, Rig, Viewport};
use bones_render::{Renderer, Scene, WeightView};
use common::{FORMAT, SIZE};

/// Périphérique avec le tracé en lignes s'il est disponible.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let features = adapter.features() & wgpu::Features::POLYGON_MODE_LINE;
    let desc = wgpu::DeviceDescriptor {
        required_features: features,
        ..Default::default()
    };
    pollster::block_on(adapter.request_device(&desc)).ok()
}

/// Rend le mannequin de démonstration avec la carte de poids `weights`.
fn render(device: &wgpu::Device, queue: &wgpu::Queue, weights: WeightView, name: &str) -> Vec<[u8; 4]> {
    let rig = Rig::demo();
    let mut camera = OrbitCamera::default();
    let (lo, hi) = rig.mesh.bounds().unwrap();
    camera.frame(lo, hi);
    let viewport = Viewport::new(SIZE[0] as f32, SIZE[1] as f32);
    let joints = rig.skinning_matrices();
    let scene = Scene {
        view_proj: camera.view_proj(&viewport),
        eye: camera.eye(),
        viewport: [0.0, 0.0, viewport.width, viewport.height],
        background: [0.1, 0.1, 0.1],
        joints: &joints,
        show_mesh: true,
        textured: false,
        weights: Some(weights),
        lines: &[],
        bones: &[],
        overlay_lines: &[],
        objects: &[],
    };
    let mut renderer = Renderer::with_samples(device, FORMAT, 1);
    renderer.set_mesh(device, queue, &rig.mesh, 1);
    renderer.set_skin(device, queue, &rig.skin, 1);
    let data = common::render(device, queue, &mut renderer, &scene);
    common::dump(&data, SIZE, name);
    common::pixels(&data).to_vec()
}

fn thigh() -> Option<u32> {
    Rig::demo().skeleton.find("thigh.l").map(|b| b.0)
}

fn count(pixels: &[[u8; 4]], keep: impl Fn(i32, i32, i32) -> bool) -> usize {
    pixels
        .iter()
        .filter(|p| keep(i32::from(p[0]), i32::from(p[1]), i32::from(p[2])))
        .count()
}

fn blue(r: i32, g: i32, b: i32) -> bool {
    b > 120 && b > r + 80 && b > g + 60
}

fn red(r: i32, g: i32, b: i32) -> bool {
    r > 120 && r > g + 60 && r > b + 80
}

#[test]
fn heat_map_follows_blenders_ramp() {
    let Some((device, queue)) = device() else {
        eprintln!("aucun adaptateur GPU : test de la carte de poids ignoré");
        return;
    };
    let view = WeightView {
        bone: thigh(),
        ..Default::default()
    };
    let pixels = render(&device, &queue, view, "weights-thigh");
    let (reds, blues) = (count(&pixels, red), count(&pixels, blue));
    assert!(reds > 40, "cuisse peinte à 1 attendue en rouge ({reds} px)");
    assert!(blues > 400, "reste du corps attendu en bleu ({blues} px)");
}

#[test]
fn zero_weights_can_be_shown_black() {
    let Some((device, queue)) = device() else {
        return;
    };
    let none = WeightView::default();
    let blue_body = count(&render(&device, &queue, none, "weights-none"), blue);
    let black = WeightView {
        zero_black: true,
        ..none
    };
    let blue_left = count(&render(&device, &queue, black, "weights-black"), blue);
    assert!(
        blue_body > 400,
        "sans os, le corps entier est bleu ({blue_body} px)"
    );
    assert!(
        blue_left < blue_body / 20,
        "poids nuls en noir ({blue_left} px bleus)"
    );
}

#[test]
fn wireframe_is_drawn_over_the_heat_map() {
    let Some((device, queue)) = device() else {
        return;
    };
    if !device.features().contains(wgpu::Features::POLYGON_MODE_LINE) {
        eprintln!("tracé en lignes indisponible : fil de fer ignoré");
        return;
    }
    let view = WeightView {
        bone: thigh(),
        ..Default::default()
    };
    let plain = render(&device, &queue, view, "weights-plain");
    let wired = render(&device, &queue, WeightView { wire: true, ..view }, "weights-wire");
    let darker = plain
        .iter()
        .zip(&wired)
        .filter(|(a, b)| i32::from(a[2]) - i32::from(b[2]) > 20 || i32::from(a[0]) - i32::from(b[0]) > 20)
        .count();
    assert!(darker > 300, "arêtes peu visibles ({darker} px assombris)");
}
