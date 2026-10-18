//! Rendu des objets de scène (accessoires, vêtements, pièces) hors écran :
//! un objet rigide suit l'os posé, un objet déformé se déforme avec la pose,
//! un élément caché n'est pas dessiné et la sélection modifie les pixels.
//!
//! `BONES_RENDER_DUMP=dossier` écrit les images (PPM) pour inspection.

mod common;

use bones_core::mesh::Primitive;
use bones_core::{
    BoneId, Editor, Mode, ObjectRef, OrbitCamera, Parent, Rig, SkinMethod, Transform, Viewport,
};
use bones_render::{Renderer, Scene};
use common::{FORMAT, SIZE};
use glam::Vec3;

/// Caméra de face cadrée largement autour du corps (un objet déplacé reste
/// visible).
fn camera() -> OrbitCamera {
    let rig = Rig::demo();
    let (lo, hi) = rig.mesh.bounds().unwrap();
    let margin = Vec3::new(0.9, 0.3, 0.3);
    let mut camera = OrbitCamera::default();
    camera.frame(lo - margin, hi + margin);
    camera
}

/// Éditeur en mode Objet sur le mannequin (corps déjà lié).
fn editor() -> Editor {
    let mut e = Editor::new(Rig::demo());
    e.set_mode(Mode::Object);
    e
}

/// Exécute toutes les liaisons en attente (corps et objets) sans fil de fond.
fn run_binds(e: &mut Editor) {
    for _ in 0..8 {
        let Some(job) = e.bind_job() else { return };
        let (skin, key) = job.run();
        e.finish_bind(skin, key);
    }
}

/// Rend l'état courant de `e` (corps caché ou non selon `body`) et retourne
/// les pixels RGBA.
fn render(device: &wgpu::Device, queue: &wgpu::Queue, e: &Editor, name: &str) -> Vec<[u8; 4]> {
    let viewport = Viewport::new(SIZE[0] as f32, SIZE[1] as f32);
    let mut r = Renderer::with_samples(device, FORMAT, 1);
    let rev = e.rig.revisions();
    r.set_mesh(device, queue, &e.rig.mesh, rev.mesh);
    r.set_skin(device, queue, &e.rig.skin, rev.skin);
    let joints = e.skinning_matrices();
    let objects = e.object_draws();
    let cam = camera();
    let scene = Scene {
        view_proj: cam.view_proj(&viewport),
        eye: cam.eye(),
        viewport: [0.0, 0.0, viewport.width, viewport.height],
        background: [0.08, 0.08, 0.09],
        joints: &joints,
        show_mesh: e.rig.mesh.triangle_count() > 0 && e.body_visible(),
        textured: false,
        weights: None,
        lines: &[],
        bones: &[],
        overlay_lines: &[],
        objects: &objects,
    };
    let data = common::render(device, queue, &mut r, &scene);
    common::dump(&data, SIZE, name);
    common::pixels(&data).to_vec()
}

/// Un pixel est « éclairé » (objet ou corps) s'il est nettement plus clair que
/// le fond sombre.
fn lit(p: &[u8; 4]) -> bool {
    u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2]) > 180
}

fn lit_count(pixels: &[[u8; 4]]) -> usize {
    pixels.iter().filter(|p| lit(p)).count()
}

/// Abscisse moyenne des pixels éclairés (centre de la silhouette).
fn centroid_x(pixels: &[[u8; 4]]) -> f32 {
    let w = SIZE[0] as usize;
    let (mut sum, mut n) = (0.0f32, 0.0f32);
    for (i, p) in pixels.iter().enumerate() {
        if lit(p) {
            sum += (i % w) as f32;
            n += 1.0;
        }
    }
    if n == 0.0 { 0.0 } else { sum / n }
}

/// Pixels nettement bleus (liseré d'accent de la sélection).
fn accent(pixels: &[[u8; 4]]) -> usize {
    pixels
        .iter()
        .filter(|p| i32::from(p[2]) > i32::from(p[0]) + 40 && i32::from(p[2]) > i32::from(p[1]) + 25)
        .count()
}

/// Pixels qui changent nettement entre deux images.
fn changed(a: &[[u8; 4]], b: &[[u8; 4]]) -> usize {
    a.iter()
        .zip(b)
        .filter(|(p, q)| p.iter().zip(q.iter()).take(3).any(|(x, y)| x.abs_diff(*y) > 30))
        .count()
}

#[test]
fn rigid_object_appears_and_follows_its_bone() {
    let Some((device, queue)) = common::device() else {
        eprintln!("aucun adaptateur GPU : test ignoré");
        return;
    };
    let mut e = editor();
    e.set_object_visible(ObjectRef::Body, false).unwrap();
    let id = e.add_primitive(Primitive::Cube);
    // Cube attaché à la racine, replacé bien en vue.
    e.set_object_transform(
        id,
        Transform {
            scale: Vec3::splat(0.25),
            ..Transform::IDENTITY
        },
    )
    .unwrap();
    e.set_object_parent(id, Parent::Bone(BoneId(0)), true).unwrap();

    let rest = render(&device, &queue, &e, "object-rigid-rest");
    assert!(
        lit_count(&rest) > 60,
        "cube rigide visible ({})",
        lit_count(&rest)
    );
    let cx_rest = centroid_x(&rest);

    // La racine se décale fortement : l'objet rigide suit l'os posé.
    e.rig
        .pose
        .set(BoneId(0), Transform::from_translation(Vec3::new(0.6, 0.0, 0.0)));
    let posed = render(&device, &queue, &e, "object-rigid-posed");
    let cx_posed = centroid_x(&posed);
    assert!(lit_count(&posed) > 60, "cube toujours visible après la pose");
    assert!(
        cx_posed - cx_rest > 20.0,
        "le cube rigide doit suivre l'os ({cx_rest} → {cx_posed})"
    );
}

#[test]
fn skinned_object_deforms_with_the_pose() {
    let Some((device, queue)) = common::device() else {
        return;
    };
    // Sphère déformée (poids transférés depuis le corps), attachée au monde :
    // sans skinning elle ne bougerait pas, seul le squelette la déplace.
    let mut skinned = editor();
    skinned.set_object_visible(ObjectRef::Body, false).unwrap();
    let id = skinned.add_primitive(Primitive::Sphere);
    skinned
        .set_object_transform(
            id,
            Transform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                scale: Vec3::splat(0.5),
                ..Transform::IDENTITY
            },
        )
        .unwrap();
    skinned
        .set_object_binding(id, Some(SkinMethod::Transfer))
        .unwrap();
    run_binds(&mut skinned);
    let rest = render(&device, &queue, &skinned, "object-skinned-rest");
    // La racine se décale : le skinning entraîne les sommets pesés.
    skinned
        .rig
        .pose
        .set(BoneId(0), Transform::from_translation(Vec3::new(0.6, 0.0, 0.0)));
    let posed = render(&device, &queue, &skinned, "object-skinned-posed");
    let moved = changed(&rest, &posed);
    assert!(
        moved > 150,
        "objet déformé doit changer avec la pose ({moved} px)"
    );

    // Contrôle : le même objet rigide, attaché au monde, ne bouge pas.
    let mut rigid = editor();
    rigid.set_object_visible(ObjectRef::Body, false).unwrap();
    let rid = rigid.add_primitive(Primitive::Sphere);
    rigid
        .set_object_transform(
            rid,
            Transform {
                translation: Vec3::new(0.0, 1.0, 0.0),
                scale: Vec3::splat(0.5),
                ..Transform::IDENTITY
            },
        )
        .unwrap();
    let rest_r = render(&device, &queue, &rigid, "object-rigid-control-rest");
    rigid
        .rig
        .pose
        .set(BoneId(0), Transform::from_translation(Vec3::new(0.6, 0.0, 0.0)));
    let posed_r = render(&device, &queue, &rigid, "object-rigid-control-posed");
    assert!(
        changed(&rest_r, &posed_r) < moved / 4,
        "objet rigide au monde ne doit pas se déformer"
    );
}

#[test]
fn hidden_body_and_hidden_object_are_not_drawn() {
    let Some((device, queue)) = common::device() else {
        return;
    };
    let mut e = editor();
    let id = e.add_primitive(Primitive::Cube);
    e.set_object_transform(
        id,
        Transform {
            scale: Vec3::splat(0.25),
            ..Transform::IDENTITY
        },
    )
    .unwrap();
    // Corps caché : seul l'objet reste.
    e.set_object_visible(ObjectRef::Body, false).unwrap();
    let with_object = render(&device, &queue, &e, "object-only");
    assert!(lit_count(&with_object) > 60, "objet seul visible");

    // Objet caché aussi : plus rien (liste de dessin vide).
    e.set_object_visible(ObjectRef::Extra(id), false).unwrap();
    assert!(
        e.object_draws().is_empty(),
        "objet caché exclu des appels de dessin"
    );
    let empty = render(&device, &queue, &e, "object-none");
    assert!(lit_count(&empty) < 10, "rien dessiné ({})", lit_count(&empty));
}

#[test]
fn selection_highlight_changes_pixels() {
    let Some((device, queue)) = common::device() else {
        return;
    };
    let mut e = editor();
    e.set_object_visible(ObjectRef::Body, false).unwrap();
    let id = e.add_primitive(Primitive::Sphere);
    e.set_object_transform(
        id,
        Transform {
            scale: Vec3::splat(0.5),
            ..Transform::IDENTITY
        },
    )
    .unwrap();

    e.select_object(None);
    let plain = render(&device, &queue, &e, "object-plain");
    e.select_object(Some(ObjectRef::Extra(id)));
    let selected = render(&device, &queue, &e, "object-selected");

    let (before, after) = (accent(&plain), accent(&selected));
    assert!(
        after > before + 12,
        "le liseré d'accent doit apparaître à la sélection ({before} → {after})"
    );
}
