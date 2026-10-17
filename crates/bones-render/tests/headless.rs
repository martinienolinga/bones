//! Rendu hors écran du mannequin de démonstration : vérifie à l'exécution
//! que pipelines, MSAA et attachements transitoires passent la validation
//! wgpu, et que le maillage apparaît bien à l'image.

mod common;

use bones_core::{OrbitCamera, Rig, Viewport};
use bones_render::{BoneInstance, LineVertex, Renderer, Scene, bone_matrix};
use common::{FORMAT, SIZE};
use glam::{Vec3, Vec4};

#[test]
fn renders_demo_rig_with_and_without_msaa() {
    let Some((device, queue)) = common::device() else {
        eprintln!("aucun adaptateur GPU : test de rendu ignoré");
        return;
    };
    let rig = Rig::demo();
    let mut camera = OrbitCamera::default();
    let (lo, hi) = rig.mesh.bounds().unwrap();
    camera.frame(lo, hi);
    let viewport = Viewport::new(SIZE[0] as f32, SIZE[1] as f32);
    let joints = rig.skinning_matrices();
    let bones: Vec<BoneInstance> = rig
        .world()
        .iter()
        .zip(rig.skeleton.bones())
        .map(|(w, b)| {
            BoneInstance::new(
                bone_matrix(w.to_matrix(), b.length),
                Vec4::new(1.0, 0.6, 0.1, 1.0),
            )
        })
        .collect();
    let grey = Vec4::new(0.5, 0.5, 0.5, 1.0);
    let lines = [
        LineVertex::new(Vec3::new(-2.0, 0.0, 0.0), grey),
        LineVertex::new(Vec3::new(2.0, 0.0, 0.0), grey),
    ];
    let scene = Scene {
        view_proj: camera.view_proj(&viewport),
        eye: camera.eye(),
        viewport: [0.0, 0.0, viewport.width, viewport.height],
        background: [0.1, 0.1, 0.1],
        joints: &joints,
        show_mesh: true,
        textured: true,
        weights: Some(bones_render::WeightView {
            bone: Some(0),
            ..Default::default()
        }),
        lines: &lines,
        bones: &bones,
        overlay_lines: &lines,
    };

    for samples in [1, 4] {
        let mut renderer = Renderer::with_samples(&device, FORMAT, samples);
        renderer.set_mesh(&device, &queue, &rig.mesh, 1);
        renderer.set_skin(&device, &queue, &rig.skin, 1);
        let data = common::render(&device, &queue, &mut renderer, &scene);
        common::dump(&data, SIZE, &format!("demo-msaa{samples}"));
        let pixels = common::pixels(&data);
        let background = [26u8, 26, 26];
        let corner = &pixels[0][..3];
        assert!(
            corner.iter().zip(background).all(|(a, b)| a.abs_diff(b) <= 1),
            "fond inattendu {corner:?} (MSAA {samples})"
        );
        let drawn = pixels
            .iter()
            .filter(|p| p[..3].iter().zip(background).any(|(a, b)| a.abs_diff(b) > 8))
            .count();
        let ratio = drawn as f32 / (SIZE[0] * SIZE[1]) as f32;
        assert!(
            ratio > 0.05,
            "presque rien n'est dessiné ({ratio}, MSAA {samples})"
        );
    }
}
