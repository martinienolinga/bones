use std::sync::OnceLock;

use glam::{Quat, Vec3};

use super::*;
use crate::autorig::{self, AutoRigOptions};
use crate::{Markers, Rig};

/// Mannequin embarqué, lu une fois pour tous les tests.
fn rig() -> &'static Rig {
    static RIG: OnceLock<Rig> = OnceLock::new();
    RIG.get_or_init(mannequin)
}

fn head_of(rig: &Rig, name: &str) -> Vec3 {
    let id = rig
        .skeleton
        .find(name)
        .unwrap_or_else(|| panic!("os {name} manquant"));
    rig.skeleton.rest_head_tail(id).0
}

#[test]
fn embedded_asset_is_small_and_well_placed() {
    assert!(MANNEQUIN_GLB.len() < 2 * 1024 * 1024);
    let mesh = mannequin_mesh();
    assert!(
        (5_000..30_000).contains(&mesh.triangle_count()),
        "{}",
        mesh.triangle_count()
    );
    assert_eq!(mesh.normals.len(), mesh.positions.len());
    let (lo, hi) = mesh.bounds().unwrap();
    assert!((hi.y - lo.y - 1.75).abs() < 0.01, "taille {}", hi.y - lo.y);
    assert!(lo.y.abs() < 1e-3, "pieds en {}", lo.y);
    assert!((lo.x + hi.x).abs() < 0.01 && (lo.z + hi.z).abs() < 0.01);
    assert!(hi.z < 0.2 && hi.x < 0.6);
    // Le visage regarde vers +Z : le nez dépasse plus que l'arrière du crâne.
    let head = mesh.positions.iter().filter(|p| p.y > 1.5);
    let (back, front) = head.fold((0.0f32, 0.0f32), |(b, f), p| (b.min(p.z), f.max(p.z)));
    assert!(front > -back, "avant {front}, arrière {back}");
}

#[test]
fn rig_has_the_humanoid_bones_and_limbs() {
    let rig = rig();
    assert_eq!(rig.name, "Mannequin");
    // Même convention de noms que le mannequin de démonstration.
    let demo = Rig::demo();
    assert_eq!(rig.skeleton.len(), demo.skeleton.len());
    for bone in demo.skeleton.bones() {
        assert!(rig.skeleton.find(&bone.name).is_some(), "{} manquant", bone.name);
    }
    let names: Vec<&str> = rig.limbs.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, ["Arm L", "Leg L", "Arm R", "Leg R"]);
    assert!(rig.limbs.iter().all(|l| l.is_valid(&rig.skeleton)));
    assert!(!rig.needs_bind());
}

#[test]
fn joints_sit_where_the_body_bends() {
    let rig = rig();
    let (shoulder, elbow, wrist) = (
        head_of(rig, "arm.l"),
        head_of(rig, "forearm.l"),
        head_of(rig, "hand.l"),
    );
    let (hip, knee, ankle) = (
        head_of(rig, "thigh.l"),
        head_of(rig, "leg.l"),
        head_of(rig, "foot.l"),
    );
    // Hauteurs humaines plausibles pour 1,75 m.
    assert!((1.35..1.48).contains(&shoulder.y), "épaule {shoulder}");
    assert!((1.05..1.2).contains(&elbow.y), "coude {elbow}");
    // Poignet au plus étroit de l'avant-bras (le générique serait vers 0,945).
    assert!((0.85..0.935).contains(&wrist.y), "poignet {wrist}");
    assert!(
        (0.82..0.95).contains(&hip.y) && (0.06..0.13).contains(&hip.x),
        "hanche {hip}"
    );
    assert!((0.42..0.53).contains(&knee.y), "genou {knee}");
    assert!((0.05..0.11).contains(&ankle.y), "cheville {ankle}");
    // Sens de flexion : coude en arrière, genou en avant.
    assert!(elbow.z < shoulder.lerp(wrist, 0.5).z, "coude {elbow}");
    assert!(knee.z > hip.lerp(ankle, 0.5).z, "genou {knee}");
    // Symétrie.
    assert!(head_of(rig, "hand.r").abs_diff_eq(wrist * Vec3::new(-1.0, 1.0, 1.0), 1e-3));
    // Cinq doigts distincts, dans la main.
    let tips: Vec<Vec3> = ["thumb3.l", "index3.l", "middle3.l", "ring3.l", "pinky3.l"]
        .iter()
        .map(|b| rig.skeleton.rest_head_tail(rig.skeleton.find(b).unwrap()).1)
        .collect();
    for (i, a) in tips.iter().enumerate() {
        assert!(a.distance(wrist) < 0.25, "doigt {i} hors de la main : {a}");
        for b in &tips[i + 1..] {
            assert!(a.distance(*b) > 0.01, "bouts confondus {a} {b}");
        }
    }
}

#[test]
fn weights_are_normalized_and_cover_every_vertex() {
    let rig = rig();
    assert_eq!(rig.skin.weights.len(), rig.mesh.vertex_count());
    for (v, vw) in rig.skin.weights.iter().enumerate() {
        let sum: u32 = vw.weights.iter().map(|w| *w as u32).sum();
        assert_eq!(sum, u16::MAX as u32, "sommet {v}");
        assert!(vw.iter().all(|(b, _)| b.index() < rig.skeleton.len()));
    }
}

#[test]
fn posed_arm_moves_the_arm_but_not_the_legs() {
    let mut rig = rig().clone();
    let arm = rig.skeleton.find("arm.l").unwrap();
    let parent = rig.world()[rig.skeleton.bone(arm).parent().unwrap().index()];
    let raise = Quat::from_rotation_z(60f32.to_radians());
    rig.pose.rotate_world(&rig.skeleton, arm, &parent, raise);

    let (mut posed, mut normals) = (Vec::new(), Vec::new());
    rig.skin
        .deform_into(&rig.mesh, &rig.skinning_matrices(), &mut posed, &mut normals);
    let moved = |keep: &dyn Fn(Vec3) -> bool| -> Vec<f32> {
        rig.mesh
            .positions
            .iter()
            .zip(&posed)
            .filter(|(p, _)| keep(**p))
            .map(|(p, q)| p.distance(*q))
            .collect()
    };
    let arm_moves = moved(&|p| p.x > 0.25 && (0.95..1.3).contains(&p.y));
    let leg_moves = moved(&|p| p.y < 0.6);
    let other_arm = moved(&|p| p.x < -0.25);
    assert!(!arm_moves.is_empty() && !leg_moves.is_empty() && !other_arm.is_empty());
    assert!(arm_moves.iter().all(|d| *d > 0.05), "bras immobile");
    assert!(leg_moves.iter().all(|d| *d < 1e-4), "jambes déplacées");
    assert!(other_arm.iter().all(|d| *d < 1e-4), "bras droit déplacé");
}

#[test]
fn embedded_project_is_small_and_complete() {
    assert!(
        MANNEQUIN_KBRIG.len() < 2 * 1024 * 1024,
        "{} octets",
        MANNEQUIN_KBRIG.len()
    );
    let rig = load_mannequin(MANNEQUIN_KBRIG).unwrap();
    assert_eq!(rig.mesh.vertex_count(), mannequin_mesh().vertex_count());
    assert_eq!(rig.mesh.triangle_count(), mannequin_mesh().triangle_count());
    assert_eq!(rig.skin.weights.len(), rig.mesh.vertex_count());
    assert_eq!((rig.skeleton.len(), rig.limbs.len()), (52, 4));
    assert!(!rig.needs_bind());
    assert!(load_mannequin(&MANNEQUIN_KBRIG[..100]).is_err());
}

/// Le projet embarqué correspond à l'auto-rig actuel (squelette recalculé
/// sans les poids, rapide). En cas d'échec, le régénérer : voir
/// [`rebuild_embedded_mannequin`].
#[test]
fn embedded_project_matches_the_current_autorig() {
    let mesh = mannequin_mesh();
    let markers = Markers::guess(&mesh).unwrap();
    let (fresh, limbs) = autorig::generate_fitted(&markers, &AutoRigOptions::default(), &mesh);
    let baked = &rig().skeleton;
    let stale = "mannequin.kbrig périmé : cargo test -p bones-core --lib \
                 samples::tests::rebuild_embedded_mannequin -- --ignored";
    assert_eq!(fresh.len(), baked.len(), "{stale}");
    for (id, bone) in fresh.ids().zip(fresh.bones()) {
        let other = baked.find(&bone.name).expect(stale);
        let (a, b) = (fresh.rest_head_tail(id), baked.rest_head_tail(other));
        assert!(
            a.0.abs_diff_eq(b.0, 1e-4) && a.1.abs_diff_eq(b.1, 1e-4),
            "{} : {stale}",
            bone.name
        );
    }
    let names = |l: &[crate::Limb]| l.iter().map(|l| l.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&limbs), names(&rig().limbs), "{stale}");
}

/// Régénère `assets/mannequin/mannequin.kbrig` (maillage, squelette, poids)
/// après une modification du maillage, de l'auto-rig ou des poids :
/// `cargo test -p bones-core --lib samples::tests::rebuild_embedded_mannequin -- --ignored`
#[test]
#[ignore]
fn rebuild_embedded_mannequin() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/mannequin/mannequin.kbrig");
    let info = bake_mannequin(&path).unwrap();
    let size = std::fs::metadata(&path).unwrap().len();
    println!(
        "{} : {size} octets, {} os, {} sommets",
        path.display(),
        info.bones,
        info.vertices
    );
}
