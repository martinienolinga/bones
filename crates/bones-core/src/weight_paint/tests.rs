//! Tests du moteur de peinture de poids sur de petits maillages construits à
//! la main, avec des valeurs attendues exactes.

use glam::{Vec2, Vec3};

use super::caches::MeshCaches;
use super::influence::{self, Influences};
use super::stroke::Stroke;
use super::weights_tool;
use super::{Blend, Falloff, GradientKind, PaintTool, WeightPaintSettings, WeightsTool};
use crate::{BoneId, Mesh, Skeleton, Skin};

const EPS: f32 = 1.5e-3;

fn b(i: u32) -> BoneId {
    BoneId(i)
}

fn infl(items: &[(u32, f32)]) -> Influences {
    items.iter().map(|(i, w)| (b(*i), *w)).collect()
}

fn skin_of(verts: &[Influences]) -> Skin {
    Skin {
        weights: verts.iter().map(influence::encode).collect(),
    }
}

/// Quad symétrique autour du plan X = 0 : v0/v2 à gauche (+X), v1/v3 à droite.
fn quad() -> Mesh {
    let mut mesh = Mesh {
        positions: vec![
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
            Vec3::new(-1.0, 1.0, 0.0),
        ],
        indices: vec![0, 1, 2, 1, 3, 2],
        ..Default::default()
    };
    mesh.compute_normals();
    mesh
}

fn settings(edit: impl FnOnce(&mut WeightPaintSettings)) -> WeightPaintSettings {
    let mut s = WeightPaintSettings::default();
    s.brush.falloff = Falloff::Constant;
    s.brush.accumulate = true;
    s.auto_normalize = true;
    edit(&mut s);
    s
}

fn new_stroke(
    screen: Vec<Option<Vec2>>,
    skin: &Skin,
    bone: u32,
    mirror: Option<u32>,
    nbones: usize,
) -> Stroke {
    let front = vec![true; screen.len()];
    let deform = vec![true; nbones];
    let mut s = Stroke::new(screen, front, skin, b(bone), mirror.map(b), deform, 50.0, None);
    s.begin_at(Vec2::ZERO);
    s
}

fn w_of(skin: &Skin, v: usize, bone: u32) -> f32 {
    skin.weights[v].weight_of(b(bone))
}

// --- fonctions pures ------------------------------------------------------

#[test]
fn blend_targets_match_blender() {
    let close = |a: f32, b: f32| assert!((a - b).abs() < 1e-6, "{a} ≠ {b}");
    close(Blend::Mix.target(0.3, 0.8), 0.8);
    close(Blend::Add.target(0.3, 0.5), 0.8);
    close(Blend::Subtract.target(0.7, 0.3), 0.4);
    close(Blend::Lighten.target(0.2, 0.5), 0.5);
    close(Blend::Lighten.target(0.7, 0.5), 0.7);
    close(Blend::Darken.target(0.7, 0.5), 0.5);
    close(Blend::Darken.target(0.2, 0.5), 0.2);
    close(Blend::Multiply.target(0.6, 0.5), 0.3);
}

#[test]
fn falloff_curves_have_expected_shape() {
    for f in Falloff::ALL {
        assert!((f.strength(0.0) - 1.0).abs() < 1e-6, "{f:?} au centre");
        assert_eq!(f.strength(1.0), 0.0, "{f:?} au bord");
    }
    assert!((Falloff::Linear.strength(0.5) - 0.5).abs() < 1e-6);
    assert!((Falloff::Sharp.strength(0.5) - 0.25).abs() < 1e-6);
    assert!((Falloff::Smooth.strength(0.5) - 0.5).abs() < 1e-6);
    assert!((Falloff::Root.strength(0.5) - 0.5f32.sqrt()).abs() < 1e-6);
    assert_eq!(Falloff::Constant.strength(0.5), 1.0);
}

#[test]
fn auto_normalize_keeps_painted_fills_rest() {
    let deform = [true, true, true];
    let mut i = infl(&[(0, 0.5), (1, 0.25), (2, 0.25)]);
    influence::auto_normalize(&mut i, b(0), &deform);
    // os 0 inchangé, 1 et 2 mis à l'échelle pour compléter à 1.
    assert!((influence::weight_of(&i, b(0)) - 0.5).abs() < 1e-6);
    assert!((influence::weight_of(&i, b(1)) - 0.25).abs() < 1e-6);
    let total: f32 = i.iter().map(|(_, w)| w).sum();
    assert!((total - 1.0).abs() < 1e-6);
}

#[test]
fn limit_keeps_painted_evicts_smallest() {
    let mut i = infl(&[(0, 0.4), (1, 0.05), (2, 0.3), (3, 0.2), (4, 0.05)]);
    // l'os peint 1 (poids faible) doit survivre ; le plus petit autre est évincé.
    influence::limit(&mut i, 4, Some(b(1)));
    assert_eq!(i.len(), 4);
    assert!(i.iter().any(|(bo, _)| *bo == b(1)), "os peint gardé");
    assert!(!i.iter().any(|(bo, _)| *bo == b(4)), "le plus petit autre évincé");
}

// --- touches de pinceau ---------------------------------------------------

#[test]
fn draw_mix_reaches_brush_weight_with_auto_normalize() {
    let mut skin = skin_of(&[infl(&[(0, 0.3), (1, 0.7)])]);
    let mut stroke = new_stroke(vec![Some(Vec2::ZERO)], &skin, 0, None, 2);
    let s = settings(|s| {
        s.brush.tool = PaintTool::Draw;
        s.brush.blend = Blend::Mix;
        s.brush.weight = 0.6;
    });
    stroke.dab(
        Vec2::ZERO,
        50.0,
        1.0,
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    assert!((w_of(&skin, 0, 0) - 0.6).abs() < EPS);
    assert!((w_of(&skin, 0, 1) - 0.4).abs() < EPS);
}

#[test]
fn draw_add_blend_adds_toward_target() {
    let mut skin = skin_of(&[infl(&[(0, 0.2), (1, 0.8)])]);
    let mut stroke = new_stroke(vec![Some(Vec2::ZERO)], &skin, 0, None, 2);
    let s = settings(|s| {
        s.brush.blend = Blend::Add;
        s.brush.weight = 0.5;
    });
    stroke.dab(
        Vec2::ZERO,
        50.0,
        1.0,
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    assert!((w_of(&skin, 0, 0) - 0.7).abs() < EPS);
}

#[test]
fn falloff_and_pressure_scale_strength() {
    // Sommet à mi-rayon, courbe linéaire : atténuation 0,5.
    let mut skin = skin_of(&[infl(&[(0, 0.0), (1, 1.0)])]);
    let mut stroke = new_stroke(vec![Some(Vec2::new(25.0, 0.0))], &skin, 0, None, 2);
    let s = settings(|s| {
        s.brush.falloff = Falloff::Linear;
        s.brush.weight = 1.0;
    });
    stroke.dab(
        Vec2::ZERO,
        50.0,
        1.0,
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    assert!((w_of(&skin, 0, 0) - 0.5).abs() < EPS, "{}", w_of(&skin, 0, 0));

    // Même geste avec pression 0,5 sur la force : atténuation 0,25.
    let mut skin = skin_of(&[infl(&[(0, 0.0), (1, 1.0)])]);
    let mut stroke = new_stroke(vec![Some(Vec2::new(25.0, 0.0))], &skin, 0, None, 2);
    stroke.dab(
        Vec2::ZERO,
        50.0,
        0.5,
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    assert!((w_of(&skin, 0, 0) - 0.25).abs() < EPS, "{}", w_of(&skin, 0, 0));
}

#[test]
fn front_faces_only_skips_back_faces() {
    let mut skin = skin_of(&[infl(&[(0, 0.0), (1, 1.0)])]);
    let mut stroke = new_stroke(vec![Some(Vec2::ZERO)], &skin, 0, None, 2);
    stroke.front[0] = false;
    let s = settings(|s| {
        s.brush.front_faces_only = true;
        s.brush.weight = 1.0;
    });
    stroke.dab(
        Vec2::ZERO,
        50.0,
        1.0,
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    assert_eq!(w_of(&skin, 0, 0), 0.0, "face arrière non peinte");
    assert!(!stroke.changed);
}

#[test]
fn non_accumulate_has_no_drift_across_many_dabs() {
    let s = settings(|s| {
        s.brush.accumulate = false;
        s.brush.strength = 0.5;
        s.brush.weight = 1.0;
    });
    let mut skin = skin_of(&[infl(&[(0, 0.0), (1, 1.0)])]);
    let mut stroke = new_stroke(vec![Some(Vec2::ZERO)], &skin, 0, None, 2);
    stroke.dab(
        Vec2::ZERO,
        50.0,
        1.0,
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    let after_one = skin.weights[0];
    for _ in 0..50 {
        stroke.dab(
            Vec2::ZERO,
            50.0,
            1.0,
            &s,
            &MeshCaches::build(&quad(), 1),
            &mut skin,
        );
    }
    // Hors accumulation : repasser ne dépasse jamais, et aucune dérive.
    assert_eq!(skin.weights[0], after_one);
}

#[test]
fn accumulate_builds_up_across_dabs() {
    let s = settings(|s| {
        s.brush.accumulate = true;
        s.brush.strength = 0.5;
        s.brush.weight = 1.0;
    });
    let mut skin = skin_of(&[infl(&[(0, 0.0), (1, 1.0)])]);
    let mut stroke = new_stroke(vec![Some(Vec2::ZERO)], &skin, 0, None, 2);
    stroke.dab(
        Vec2::ZERO,
        50.0,
        1.0,
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    let one = w_of(&skin, 0, 0);
    stroke.dab(
        Vec2::ZERO,
        50.0,
        1.0,
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    assert!(w_of(&skin, 0, 0) > one, "l'aérographe accumule");
    assert!((one - 0.5).abs() < EPS);
}

#[test]
fn mirror_paints_symmetric_vertex_with_mirror_bone() {
    // Deux os (0 ↔ 1) ; peindre le sommet gauche peint aussi le droit.
    let mut skin = skin_of(&vec![Influences::new(); 4]);
    let caches = MeshCaches::build(&quad(), 1);
    // v0 à gauche, son miroir est v1.
    assert_eq!(caches.mirror[0], 1);
    let screen = vec![Some(Vec2::ZERO), Some(Vec2::new(500.0, 0.0)), None, None];
    let mut stroke = new_stroke(screen, &skin, 0, Some(1), 2);
    let s = settings(|s| s.brush.weight = 1.0);
    stroke.dab(Vec2::ZERO, 50.0, 1.0, &s, &caches, &mut skin);
    assert!((w_of(&skin, 0, 0) - 1.0).abs() < EPS, "sommet direct");
    assert!((w_of(&skin, 1, 1) - 1.0).abs() < EPS, "sommet miroir, os miroir");
}

#[test]
fn average_pulls_toward_mean() {
    let mut skin = skin_of(&[infl(&[(0, 0.8), (1, 0.2)]), infl(&[(0, 0.2), (1, 0.8)])]);
    let screen = vec![Some(Vec2::ZERO), Some(Vec2::new(10.0, 0.0))];
    let mut stroke = new_stroke(screen, &skin, 0, None, 2);
    let s = settings(|s| s.brush.tool = PaintTool::Average);
    stroke.dab(
        Vec2::ZERO,
        50.0,
        1.0,
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    assert!((w_of(&skin, 0, 0) - 0.5).abs() < EPS);
    assert!((w_of(&skin, 1, 0) - 0.5).abs() < EPS);
}

#[test]
fn blur_smooths_toward_neighbors() {
    // v0=1, voisins de v1 = {v0, v2, v3} → moyenne 1/3.
    let mut skin = skin_of(&[
        infl(&[(0, 1.0)]),
        infl(&[(0, 0.0), (1, 1.0)]),
        infl(&[(0, 0.0), (1, 1.0)]),
        infl(&[(0, 0.0), (1, 1.0)]),
    ]);
    let screen = vec![None, Some(Vec2::ZERO), None, None];
    let mut stroke = new_stroke(screen, &skin, 0, None, 2);
    let s = settings(|s| s.brush.tool = PaintTool::Blur);
    stroke.dab(
        Vec2::ZERO,
        50.0,
        1.0,
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    assert!(
        (w_of(&skin, 1, 0) - 1.0 / 3.0).abs() < EPS,
        "{}",
        w_of(&skin, 1, 0)
    );
}

#[test]
fn gradient_linear_ramps_from_start_to_end() {
    let mut skin = skin_of(&vec![infl(&[(0, 0.0), (1, 1.0)]); 3]);
    // trois sommets alignés à t = 0, 0.5, 1 le long du dégradé.
    let screen = vec![
        Some(Vec2::ZERO),
        Some(Vec2::new(50.0, 0.0)),
        Some(Vec2::new(100.0, 0.0)),
    ];
    let mut stroke = new_stroke(screen, &skin, 0, None, 2);
    let s = settings(|s| {
        s.brush.tool = PaintTool::Gradient;
        s.brush.gradient = GradientKind::Linear;
        s.brush.falloff = Falloff::Linear;
        s.brush.weight = 1.0;
    });
    stroke.gradient(
        Vec2::new(100.0, 0.0),
        &s,
        &MeshCaches::build(&quad(), 1),
        &mut skin,
    );
    assert!(w_of(&skin, 0, 0) < EPS, "début à 0");
    assert!((w_of(&skin, 1, 0) - 0.5).abs() < EPS, "milieu à 0,5");
    assert!((w_of(&skin, 2, 0) - 1.0).abs() < EPS, "fin à 1");
}

// --- outils du menu « Weights » ------------------------------------------

fn two_bone_skeleton() -> Skeleton {
    let mut s = Skeleton::new();
    let a = s.add_bone_world("a.l", None, Vec3::ZERO, Vec3::Y);
    s.add_bone_world("a.r", Some(a), Vec3::Y, Vec3::new(0.0, 2.0, 0.0));
    s
}

#[test]
fn weights_tool_invert_and_levels_and_quantize() {
    let skel = two_bone_skeleton();
    let mesh = quad();
    let caches = MeshCaches::build(&mesh, 1);
    let set = settings(|_| {});

    let mut skin = skin_of(&[infl(&[(0, 0.3), (1, 0.7)])]);
    weights_tool::run(
        &mut skin,
        &skel,
        &mesh,
        &caches,
        Some(b(0)),
        WeightsTool::Invert,
        &set,
    );
    assert!((w_of(&skin, 0, 0) - 0.7).abs() < EPS, "1 − 0,3 = 0,7");

    let mut skin = skin_of(&[infl(&[(0, 0.4), (1, 0.6)])]);
    let tool = WeightsTool::Levels {
        offset: 0.1,
        gain: 1.0,
    };
    weights_tool::run(&mut skin, &skel, &mesh, &caches, Some(b(0)), tool, &set);
    assert!((w_of(&skin, 0, 0) - 0.5).abs() < EPS);

    let mut skin = skin_of(&[infl(&[(0, 0.46), (1, 0.54)])]);
    let tool = WeightsTool::Quantize { steps: 4 };
    weights_tool::run(&mut skin, &skel, &mesh, &caches, Some(b(0)), tool, &set);
    assert!((w_of(&skin, 0, 0) - 0.5).abs() < EPS, "0,46 → 0,5");
}

#[test]
fn weights_tool_clean_and_limit_total() {
    let skel = two_bone_skeleton();
    let mesh = quad();
    let caches = MeshCaches::build(&mesh, 1);
    let set = settings(|_| {});

    // Clean : l'influence faible de l'os peint est retirée (puis renormalisée).
    let mut skin = skin_of(&[infl(&[(0, 0.02), (1, 0.98)])]);
    let tool = WeightsTool::Clean { threshold: 0.1 };
    weights_tool::run(&mut skin, &skel, &mesh, &caches, Some(b(0)), tool, &set);
    assert_eq!(w_of(&skin, 0, 0), 0.0);
    assert!((w_of(&skin, 0, 1) - 1.0).abs() < EPS);

    // Limit Total : au plus deux influences, les plus faibles retirées.
    let mut skin = skin_of(&[infl(&[(0, 0.4), (1, 0.3), (2, 0.2), (3, 0.1)])]);
    let tool = WeightsTool::LimitTotal { max: 2 };
    weights_tool::run(&mut skin, &skel, &mesh, &caches, None, tool, &set);
    assert_eq!(skin.weights[0].iter().count(), 2);
    assert!(w_of(&skin, 0, 0) > 0.0 && w_of(&skin, 0, 1) > 0.0);
}

#[test]
fn weights_tool_mirror_copies_left_onto_right() {
    // Deux sommets symétriques, deux os a.l / a.r.
    let mut mesh = Mesh {
        positions: vec![Vec3::new(1.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)],
        indices: vec![],
        ..Default::default()
    };
    mesh.normals = vec![Vec3::Z; 2];
    let skel = two_bone_skeleton();
    let caches = MeshCaches::build(&mesh, 1);
    assert_eq!(caches.mirror[0], 1);
    // v0 (gauche, +X) porte a.l ; v1 (droite) est vide.
    let mut skin = skin_of(&[infl(&[(0, 1.0)]), Influences::new()]);
    let set = settings(|_| {});
    let changed = weights_tool::run(&mut skin, &skel, &mesh, &caches, None, WeightsTool::Mirror, &set);
    assert!(changed);
    // v1 reçoit l'os miroir a.r (indice 1).
    assert!((w_of(&skin, 1, 1) - 1.0).abs() < EPS);
}

#[test]
fn weights_tool_smooth_averages_over_edges() {
    let skel = two_bone_skeleton();
    let mesh = quad();
    let caches = MeshCaches::build(&mesh, 1);
    let set = settings(|_| {});
    // v0=1, les autres 0 ; après lissage l'os 0 diffuse vers les voisins.
    let mut skin = skin_of(&[
        infl(&[(0, 1.0)]),
        infl(&[(0, 0.0), (1, 1.0)]),
        infl(&[(0, 0.0), (1, 1.0)]),
        infl(&[(0, 0.0), (1, 1.0)]),
    ]);
    let tool = WeightsTool::Smooth {
        iterations: 1,
        factor: 1.0,
    };
    let changed = weights_tool::run(&mut skin, &skel, &mesh, &caches, Some(b(0)), tool, &set);
    assert!(changed);
    assert!(w_of(&skin, 1, 0) > 0.0, "l'os 0 a diffusé vers v1");
}

#[test]
fn normalize_all_rescales_to_unit_sum() {
    let deform = [true, true];
    let mut i = infl(&[(0, 0.2), (1, 0.2)]);
    influence::normalize_deform(&mut i, &deform);
    let total: f32 = i.iter().map(|(_, w)| w).sum();
    assert!((total - 1.0).abs() < 1e-6);
    assert!((influence::weight_of(&i, b(0)) - 0.5).abs() < 1e-6);
}
