//! Tests des poids du visage : yeux rigides, normalisation, pas de fuite sur
//! le corps, et séparation haut/bas de la bouche (sur le mannequin embarqué).

use glam::Vec3;

use crate::{BindMethod, Mesh, Rig, Skeleton};

/// Tête + deux yeux (pièces séparées) avec un squelette facial minimal.
fn synthetic_head() -> (Mesh, Skeleton) {
    let mut mesh = Mesh::capsule(Vec3::new(0.0, 1.52, 0.0), Vec3::new(0.0, 1.70, 0.0), 0.11, 32, 20);
    let eye = |x: f32| Mesh::capsule(Vec3::new(x, 1.62, 0.088), Vec3::new(x, 1.62, 0.094), 0.018, 12, 8);
    mesh.append(&eye(0.05));
    mesh.append(&eye(-0.05));
    mesh.compute_normals();

    let mut s = Skeleton::new();
    let head = s.add_bone_world(
        "head.x",
        None,
        Vec3::new(0.0, 1.5, 0.0),
        Vec3::new(0.0, 1.72, 0.0),
    );
    s.add_bone_world(
        "jaw.x",
        Some(head),
        Vec3::new(0.0, 1.6, 0.0),
        Vec3::new(0.0, 1.54, 0.08),
    );
    s.add_bone_world(
        "eye.l",
        Some(head),
        Vec3::new(0.05, 1.62, 0.09),
        Vec3::new(0.05, 1.62, 0.108),
    );
    s.add_bone_world(
        "eye.r",
        Some(head),
        Vec3::new(-0.05, 1.62, 0.09),
        Vec3::new(-0.05, 1.62, 0.108),
    );
    (mesh, s)
}

#[test]
fn eyes_are_rigid_and_weights_normalized() {
    let (mesh, skeleton) = synthetic_head();
    let skin = BindMethod::Heat.compute(&mesh, &skeleton);
    assert_eq!(skin.weights.len(), mesh.vertex_count());
    let eye_l = skeleton.find("eye.l").unwrap();
    let eye_r = skeleton.find("eye.r").unwrap();
    let center_l = Vec3::new(0.05, 1.62, 0.091);

    let mut rigid = 0;
    for (p, vw) in mesh.positions.iter().zip(&skin.weights) {
        let sum: u32 = vw.weights.iter().map(|w| *w as u32).sum();
        assert_eq!(sum, u16::MAX as u32, "non normalisé en {p:?}");
        if vw.weight_of(eye_l) > 0.99 {
            rigid += 1;
            // Tout sommet à 100 % sur eye.l est bien sur le globe gauche.
            assert!((*p - center_l).length() < 0.03, "eye.l hors du globe {p:?}");
        }
        // L'arrière de la tête ne reçoit aucun poids des yeux (pas de fuite).
        if p.z < -0.05 {
            assert_eq!(
                vw.weight_of(eye_l) + vw.weight_of(eye_r),
                0.0,
                "fuite sur la tête {p:?}"
            );
        }
    }
    // Tout un globe (le petit sphéroïde gauche) est rigide.
    assert!(rigid > 60, "globe gauche pas rigide ({rigid})");
}

#[test]
fn no_face_detected_falls_back_to_body() {
    // Un squelette sans os de visage repasse par la liaison ordinaire.
    let rig = Rig::demo();
    let skin = BindMethod::Heat.compute(&rig.mesh, &rig.skeleton);
    assert_eq!(skin.weights.len(), rig.mesh.vertex_count());
}

/// Sur le mannequin embarqué (corps + visage), la lèvre inférieure suit la
/// mâchoire, la supérieure la tête, et rien du visage ne déborde sur le cou.
#[test]
fn mannequin_lips_split_and_no_leak_on_neck() {
    let rig = crate::samples::mannequin();
    let s = &rig.skeleton;
    let Some(jaw) = s.find("jaw.x") else {
        return; // mannequin sans visage (asset non régénéré) : test sans objet
    };
    let lip_top_y = s.rest_head_tail(s.find("lip_top.x").unwrap()).1.y;
    let lip_bot_y = s.rest_head_tail(s.find("lip_bot.x").unwrap()).1.y;

    // Ensemble piloté par la mâchoire : jaw.x et tous ses descendants
    // (lip_bot*, chin, teeth_bot…). C'est ce qui ouvre la bouche.
    let in_jaw: Vec<bool> = s
        .ids()
        .map(|id| {
            let mut cur = Some(id);
            while let Some(c) = cur {
                if c == jaw {
                    return true;
                }
                cur = s.bone(c).parent();
            }
            false
        })
        .collect();
    let jaw_w = |lo: f32, hi: f32| {
        let (mut sum, mut n) = (0.0f32, 0);
        for (p, vw) in rig.mesh.positions.iter().zip(&rig.skin.weights) {
            if p.x.abs() < 0.025 && p.z > 0.1 && (lo..hi).contains(&p.y) {
                sum += vw
                    .iter()
                    .filter(|(b, _)| in_jaw[b.index()])
                    .map(|(_, w)| w)
                    .sum::<f32>();
                n += 1;
            }
        }
        if n > 0 { sum / n as f32 } else { 0.0 }
    };
    let lower = jaw_w(lip_bot_y - 0.012, lip_bot_y);
    let upper = jaw_w(lip_top_y, lip_top_y + 0.012);
    // La lèvre inférieure suit la mâchoire, la supérieure la tête.
    assert!(lower > 0.6, "lèvre basse peu pilotée par la mâchoire : {lower}");
    assert!(
        lower > upper + 0.4,
        "séparation haut/bas insuffisante ({upper} / {lower})"
    );

    // Aucun os du visage ne pèse sur le cou / les épaules (y < base du cou).
    let neck_y = s.rest_head_tail(s.find("neck.x").unwrap()).0.y;
    let face_ids: Vec<_> = crate::autorig::face::present(s)
        .iter()
        .map(|(_, id)| *id)
        .collect();
    for (p, vw) in rig.mesh.positions.iter().zip(&rig.skin.weights) {
        if p.y < neck_y - 0.05 {
            let leak: f32 = face_ids
                .iter()
                .filter(|id| **id != jaw)
                .map(|id| vw.weight_of(*id))
                .sum();
            assert!(leak < 1e-3, "fuite du visage sur le corps en {p:?} : {leak}");
        }
    }
}
