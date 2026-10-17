//! Squelettes de référence des animaux.

use glam::Vec3;

use super::Builder;
use crate::Skeleton;

/// Chien de 0,6 m au garrot : membres avant (bras) et arrière (jambes)
/// digitigrades, queue de trois os.
pub(super) fn quadruped() -> Skeleton {
    let mut b = Builder::new();
    let root = b.bone("root.x", None, [0.0, 0.55, -0.30], [0.0, 0.57, -0.15]);
    let s1 = b.bone("spine_01.x", Some(root), [0.0, 0.57, -0.15], [0.0, 0.58, 0.05]);
    let chest = b.bone("spine_02.x", Some(s1), [0.0, 0.58, 0.05], [0.0, 0.58, 0.22]);
    let neck = b.bone("neck.x", Some(chest), [0.0, 0.58, 0.22], [0.0, 0.72, 0.36]);
    b.bone("head.x", Some(neck), [0.0, 0.72, 0.36], [0.0, 0.70, 0.54]);
    let tail = [
        [0.0, 0.56, -0.31],
        [0.0, 0.52, -0.45],
        [0.0, 0.45, -0.56],
        [0.0, 0.38, -0.65],
    ];
    b.chain("tail_#.x", Some(root), &tail.map(Vec3::from_array));
    b.pair(|b, s, x| {
        let p = |v: [f32; 3]| Vec3::new(v[0] * x, v[1], v[2]);
        let front = [
            [0.09, 0.50, 0.20],
            [0.09, 0.30, 0.15],
            [0.09, 0.10, 0.18],
            [0.09, 0.03, 0.20],
            [0.09, 0.0, 0.26],
        ];
        let names = ["arm", "forearm", "hand", "toes_front"];
        let mut parent = Some(chest);
        for (i, name) in names.iter().enumerate() {
            parent = Some(b.0.add_bone_world(format!("{name}.{s}"), parent, p(front[i]), p(front[i + 1])));
        }
        let hind = [
            [0.09, 0.50, -0.28],
            [0.09, 0.30, -0.20],
            [0.09, 0.13, -0.32],
            [0.09, 0.03, -0.30],
            [0.09, 0.0, -0.24],
        ];
        let names = ["thigh", "leg", "foot", "toes"];
        let mut parent = Some(root);
        for (i, name) in names.iter().enumerate() {
            parent = Some(b.0.add_bone_world(format!("{name}.{s}"), parent, p(hind[i]), p(hind[i + 1])));
        }
    });
    b.0
}

/// Oiseau de 0,37 m (pigeon) : corps incliné, long cou, ailes déployées.
pub(super) fn bird() -> Skeleton {
    let mut b = Builder::new();
    let root = b.bone("root.x", None, [0.0, 0.20, -0.02], [0.0, 0.22, 0.04]);
    let chest = b.bone("spine_01.x", Some(root), [0.0, 0.22, 0.04], [0.0, 0.25, 0.10]);
    let neck = [
        [0.0, 0.25, 0.10],
        [0.0, 0.29, 0.13],
        [0.0, 0.33, 0.15],
        [0.0, 0.36, 0.16],
    ];
    let neck = b.chain("neck_#.x", Some(chest), &neck.map(Vec3::from_array));
    b.bone(
        "head.x",
        neck.last().copied(),
        [0.0, 0.36, 0.16],
        [0.0, 0.37, 0.22],
    );
    let tail = [[0.0, 0.20, -0.03], [0.0, 0.18, -0.11], [0.0, 0.16, -0.19]];
    b.chain("tail_#.x", Some(root), &tail.map(Vec3::from_array));
    b.pair(|b, s, x| {
        let p = |v: [f32; 3]| Vec3::new(v[0] * x, v[1], v[2]);
        let wing = [
            [0.03, 0.24, 0.08],
            [0.12, 0.24, 0.06],
            [0.25, 0.24, 0.05],
            [0.38, 0.24, 0.03],
        ];
        b.chain(&format!("wing_#.{s}"), Some(chest), &wing.map(p));
        let leg = [
            [0.03, 0.19, 0.01],
            [0.04, 0.14, 0.05],
            [0.04, 0.08, 0.0],
            [0.04, 0.015, 0.01],
            [0.04, 0.0, 0.05],
        ];
        let names = ["thigh", "leg", "foot", "toes"];
        let mut parent = Some(root);
        for (i, name) in names.iter().enumerate() {
            parent = Some(b.0.add_bone_world(format!("{name}.{s}"), parent, p(leg[i]), p(leg[i + 1])));
        }
    });
    b.0
}

/// Poisson de 0,5 m nageant à 0,5 m du sol : racine au milieu du corps,
/// tête vers l'avant, queue vers l'arrière, nageoires.
pub(super) fn fish() -> Skeleton {
    let mut b = Builder::new();
    let root = b.bone("root.x", None, [0.0, 0.5, 0.0], [0.0, 0.5, 0.1]);
    let front = b.bone("spine_01.x", Some(root), [0.0, 0.5, 0.1], [0.0, 0.5, 0.18]);
    b.bone("head.x", Some(front), [0.0, 0.5, 0.18], [0.0, 0.5, 0.26]);
    let tail = [
        [0.0, 0.5, 0.0],
        [0.0, 0.5, -0.07],
        [0.0, 0.5, -0.13],
        [0.0, 0.5, -0.18],
        [0.0, 0.5, -0.24],
    ];
    b.chain("tail_#.x", Some(root), &tail.map(Vec3::from_array));
    b.bone("fin_dorsal.x", Some(root), [0.0, 0.54, 0.03], [0.0, 0.58, -0.03]);
    b.pair(|b, s, x| {
        b.0.add_bone_world(
            format!("fin.{s}"),
            Some(front),
            Vec3::new(0.03 * x, 0.48, 0.13),
            Vec3::new(0.08 * x, 0.46, 0.09),
        );
    });
    b.0
}

/// Serpent de 1,8 m posé au sol : racine derrière la tête, corps vers l'arrière.
pub(super) fn serpent() -> Skeleton {
    let mut b = Builder::new();
    let root = b.bone("root.x", None, [0.0, 0.03, 0.9], [0.0, 0.03, 0.78]);
    b.bone("head.x", Some(root), [0.0, 0.03, 0.9], [0.0, 0.03, 1.0]);
    let points: Vec<Vec3> = (0..=14)
        .map(|i| Vec3::new(0.0, 0.03, 0.78 - i as f32 * 0.12))
        .collect();
    b.chain("spine_#.x", Some(root), &points);
    b.0
}

/// Arthropode à `pairs` paires de pattes (4 : araignée, 3 : insecte) :
/// corps central, abdomen, pattes à quatre segments (hanche, fémur, tibia, tarse).
pub(super) fn many_legs(pairs: usize) -> Skeleton {
    let mut b = Builder::new();
    let body = b.bone("root.x", None, [0.0, 0.12, -0.02], [0.0, 0.12, 0.05]);
    b.bone("head.x", Some(body), [0.0, 0.12, 0.05], [0.0, 0.11, 0.08]);
    b.bone("spine_01.x", Some(body), [0.0, 0.13, -0.03], [0.0, 0.15, -0.14]);
    let spread = if pairs > 3 { 0.022 } else { 0.03 };
    for i in 0..pairs {
        let z = 0.035 - i as f32 * spread;
        // Les pattes avant visent l'avant, les arrière l'arrière.
        let fan = (pairs as f32 - 1.0) * 0.5 - i as f32;
        b.pair(|b, s, x| {
            let p = |v: Vec3| Vec3::new(v.x * x, v.y, v.z);
            let reach = Vec3::new(1.0, 0.0, fan * 0.45).normalize();
            let base = Vec3::new(0.02, 0.12, z);
            let points = [
                base,
                base + reach * 0.02 + Vec3::Y * 0.005,
                base + reach * 0.07 + Vec3::Y * 0.05,
                base + reach * 0.12 - Vec3::Y * 0.05,
                base + reach * 0.14 - Vec3::Y * 0.12,
            ];
            b.chain(&format!("leg{}_#.{s}", i + 1), Some(body), &points.map(p));
        });
    }
    b.0
}
