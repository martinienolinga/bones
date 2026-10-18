//! Vignettes des préréglages joués sur le mannequin (`Tiles`) : fond
//! transparent, modèle cadré, et expressions faciales nettement distinctes
//! les unes des autres (critère de lisibilité des cartes).
//!
//! `BONES_RENDER_DUMP=dossier` écrit les vignettes (PPM) pour inspection.

mod common;

use bones_core::presets::{Category, Shot, StageModel, Staged, builtin, find_builtin};
use bones_core::{Rig, Viewport};
use bones_render::{Readback, Renderer, Scene, Tiles};
use common::FORMAT;
use glam::Mat4;

const TILE: [u32; 2] = [200, 220];

struct Bench {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: Renderer,
    tiles: Tiles,
    model: StageModel,
}

impl Bench {
    fn new() -> Option<Self> {
        let (device, queue) = common::device()?;
        let rig: Rig = bones_core::samples::mannequin();
        let mut renderer = Renderer::new(&device, FORMAT);
        renderer.set_mesh(&device, &queue, &rig.mesh, 1);
        renderer.set_skin(&device, &queue, &rig.skin, 1);
        Some(Self {
            device,
            queue,
            renderer,
            tiles: Tiles::new(),
            model: StageModel::new(&rig),
        })
    }

    /// Rend l'image `frame` de `staged` et retourne ses pixels RGBA.
    fn render(&mut self, staged: &Staged, frame: usize) -> Vec<u8> {
        let (mut world, mut joints): (_, Vec<Mat4>) = (Vec::new(), Vec::new());
        staged.joints(&self.model, frame, &mut world, &mut joints);
        let viewport = Viewport::new(TILE[0] as f32, TILE[1] as f32);
        let camera = staged.camera(viewport.aspect(), 0.08);
        let scene = Scene {
            view_proj: camera.view_proj(&viewport),
            eye: camera.eye(),
            viewport: [0.0, 0.0, viewport.width, viewport.height],
            background: [0.5, 0.5, 0.5],
            joints: &joints,
            show_mesh: true,
            textured: true,
            weights: None,
            lines: &[],
            bones: &[],
            overlay_lines: &[],
            objects: &[],
        };
        let (device, queue) = (&self.device, &self.queue);
        self.tiles
            .render(&mut self.renderer, device, queue, &scene, 0, TILE);
        let mut encoder = device.create_command_encoder(&Default::default());
        let readback = Readback::copy(device, &mut encoder, self.tiles.texture(0).unwrap()).unwrap();
        queue.submit([encoder.finish()]);
        readback.wait().expect("relecture GPU")
    }
}

/// Part des pixels qui diffèrent nettement entre deux images.
fn difference(a: &[u8], b: &[u8]) -> f32 {
    let (a, b) = (common::pixels(a), common::pixels(b));
    let changed = a
        .iter()
        .zip(b)
        .filter(|(p, q)| p.iter().zip(q.iter()).take(3).any(|(x, y)| x.abs_diff(*y) > 24))
        .count();
    changed as f32 / a.len() as f32
}

#[test]
fn tiles_are_transparent_around_the_model() {
    let Some(mut bench) = Bench::new() else {
        eprintln!("aucun adaptateur GPU : test de rendu ignoré");
        return;
    };
    let walk = Staged::new(find_builtin("builtin.human.walk").unwrap(), &bench.model).unwrap();
    let rgba = bench.render(&walk, 0);
    common::dump(&rgba, TILE, "stage-walk");
    let pixels = common::pixels(&rgba);
    assert_eq!(pixels[0], [0, 0, 0, 0], "fond transparent");
    let opaque = pixels.iter().filter(|p| p[3] == 255).count() as f32 / pixels.len() as f32;
    assert!(opaque > 0.08 && opaque < 0.6, "silhouette cadrée ({opaque})");
    // Bords prémultipliés : jamais de couleur plus forte que l'alpha.
    assert!(pixels.iter().all(|p| p[..3].iter().all(|c| *c <= p[3])));
    assert_eq!(bench.tiles.size(0), Some(TILE));
}

#[test]
fn expressions_read_differently_on_the_mannequin() {
    let Some(mut bench) = Bench::new() else {
        eprintln!("aucun adaptateur GPU : test de rendu ignoré");
        return;
    };
    let mut emotions: Vec<(String, Vec<u8>)> = Vec::new();
    let expressions = builtin()
        .iter()
        .filter(|p| p.info.category == Category::FacialExpression);
    for preset in expressions {
        let staged = Staged::new(preset, &bench.model).unwrap();
        assert_eq!(staged.shot(), Shot::Face);
        let rgba = bench.render(&staged, 0);
        let name = format!("face-{}-{}", preset.info.group, preset.info.name).to_lowercase();
        let name: String = name
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        common::dump(&rgba, TILE, &name);
        if preset.info.group == "Emotions" {
            emotions.push((preset.info.name.clone(), rgba));
        }
    }
    // Deux émotions quelconques se distinguent nettement sur la carte : une
    // part sensible des pixels change (mesurée sur le mannequin).
    for (i, (a, pa)) in emotions.iter().enumerate() {
        for (b, pb) in &emotions[i + 1..] {
            let d = difference(pa, pb);
            eprintln!("{a} / {b} : {:.2} %", d * 100.0);
            assert!(d > MIN_DIFFERENCE, "{a} et {b} se ressemblent trop ({d})");
        }
    }
}

/// Part minimale de pixels qui changent entre deux émotions.
const MIN_DIFFERENCE: f32 = 0.002;
