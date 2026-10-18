//! Miniature hors écran (`Renderer::snapshot`) : taille demandée, MSAA
//! résolu, pixels RGBA, et indépendance vis-à-vis de l'image préparée
//! ensuite (la relecture se fait après coup, depuis un autre fil).

mod common;

use bones_core::{OrbitCamera, Rig, Viewport};
use bones_render::{Renderer, Scene};
use common::FORMAT;

const THUMB: [u32; 2] = [320, 240];

fn scene<'a>(rig: &Rig, joints: &'a [glam::Mat4], show_mesh: bool, background: [f64; 3]) -> Scene<'a> {
    let mut camera = OrbitCamera::default();
    let (lo, hi) = rig.mesh.bounds().unwrap();
    camera.frame(lo, hi);
    let viewport = Viewport::new(THUMB[0] as f32, THUMB[1] as f32);
    Scene {
        view_proj: camera.view_proj(&viewport),
        eye: camera.eye(),
        viewport: [0.0, 0.0, viewport.width, viewport.height],
        background,
        joints,
        show_mesh,
        textured: true,
        weights: None,
        lines: &[],
        bones: &[],
        overlay_lines: &[],
        objects: &[],
    }
}

#[test]
fn snapshot_reads_back_the_scene_from_another_thread() {
    let Some((device, queue)) = common::device() else {
        eprintln!("aucun adaptateur GPU : test de rendu ignoré");
        return;
    };
    let rig = Rig::demo();
    let joints = rig.skinning_matrices();
    for samples in [1, 4] {
        let mut renderer = Renderer::with_samples(&device, FORMAT, samples);
        renderer.set_mesh(&device, &queue, &rig.mesh, 1);
        renderer.set_skin(&device, &queue, &rig.skin, 1);
        let readback = renderer
            .snapshot(
                &device,
                &queue,
                &scene(&rig, &joints, true, [0.2, 0.3, 0.4]),
                THUMB,
            )
            .expect("format lisible");
        assert_eq!(readback.size(), THUMB);

        // Image suivante préparée (fond rouge, sans maillage) avant même
        // la relecture : la miniature ne doit pas en être affectée.
        renderer.prepare(&device, &queue, &scene(&rig, &joints, false, [1.0, 0.0, 0.0]));
        let rgba = std::thread::spawn(move || readback.wait())
            .join()
            .unwrap()
            .expect("relecture GPU");
        common::dump(&rgba, THUMB, &format!("snapshot-msaa{samples}"));

        assert_eq!(rgba.len(), (THUMB[0] * THUMB[1] * 4) as usize);
        let pixels = common::pixels(&rgba);
        let background = [51u8, 77, 102, 255];
        assert!(
            pixels[0].iter().zip(background).all(|(a, b)| a.abs_diff(b) <= 1),
            "fond inattendu {:?} (MSAA {samples})",
            pixels[0]
        );
        let model = pixels
            .iter()
            .filter(|p| p.iter().zip(background).any(|(a, b)| a.abs_diff(b) > 12))
            .count();
        let ratio = model as f32 / pixels.len() as f32;
        assert!(ratio > 0.04, "modèle absent ({ratio}, MSAA {samples})");
        if samples == 4 {
            // MSAA résolu : des tons intermédiaires sur les bords du modèle.
            let edges = pixels
                .iter()
                .filter(|p| p[2].abs_diff(102) > 4 && p[2].abs_diff(102) < 40 && p[0] < 120)
                .count();
            assert!(edges > 20, "bords sans anticrénelage ({edges})");
        }
    }
}
