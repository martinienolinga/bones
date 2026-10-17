//! Squelettes de référence des végétaux et des objets (cordes, machines).

use glam::Vec3;

use super::Builder;
use crate::{BoneId, Skeleton};

/// Points d'une courbe : `n` segments de `from` dans la direction `dir`,
/// qui s'incurve de `bend` par segment.
fn arc(from: Vec3, dir: Vec3, step: f32, n: usize, bend: Vec3) -> Vec<Vec3> {
    let mut points = vec![from];
    let mut d = dir.normalize();
    let mut p = from;
    for _ in 0..n {
        p += d * step;
        points.push(p);
        d = (d + bend).normalize();
    }
    points
}

/// Arbre de 3 m : tronc de quatre os, deux étages de branches, rameaux.
pub(super) fn tree() -> Skeleton {
    let mut b = Builder::new();
    let trunk = b.chain("trunk_#", None, &arc(Vec3::ZERO, Vec3::Y, 0.75, 4, Vec3::new(0.01, 0.0, 0.0)));
    let mut k = 0;
    let mut branch = |b: &mut Builder, at: BoneId, dir: Vec3, len: f32| {
        k += 1;
        let start = b.0.rest_head_tail(at).1;
        let pts = arc(start, dir, len, 3, Vec3::new(0.0, -0.12, 0.0));
        let bones = b.chain(&format!("branch_{k:02}_#"), Some(at), &pts);
        // Rameau à mi-branche.
        let mid = b.0.rest_head_tail(bones[1]).1;
        let twig = arc(mid, dir + Vec3::Y * 1.2, len * 0.5, 2, Vec3::ZERO);
        b.chain(&format!("branch_{k:02}_twig_#"), Some(bones[1]), &twig);
    };
    branch(&mut b, trunk[1], Vec3::new(1.0, 0.9, 0.2), 0.4);
    branch(&mut b, trunk[1], Vec3::new(-1.0, 0.8, -0.3), 0.4);
    branch(&mut b, trunk[2], Vec3::new(0.2, 0.9, 1.0), 0.33);
    branch(&mut b, trunk[2], Vec3::new(-0.3, 0.9, -1.0), 0.33);
    let top = arc(b.0.rest_head_tail(trunk[3]).1, Vec3::Y, 0.25, 2, Vec3::ZERO);
    b.chain("crown_#", Some(trunk[3]), &top);
    b.0
}

/// Brin d'herbe (ou roseau) de 0,6 m : une chaîne de cinq os.
pub(super) fn grass() -> Skeleton {
    let mut b = Builder::new();
    b.chain("blade_#", None, &arc(Vec3::ZERO, Vec3::Y, 0.12, 5, Vec3::new(0.0, 0.0, 0.03)));
    b.0
}

/// Fleur de 0,5 m : tige, cœur et quatre pétales.
pub(super) fn flower() -> Skeleton {
    let mut b = Builder::new();
    let stem = b.chain("stem_#", None, &arc(Vec3::ZERO, Vec3::Y, 0.11, 4, Vec3::new(0.0, 0.0, 0.04)));
    let top = b.0.rest_head_tail(stem[3]).1;
    let head = b.bone("flower_head", Some(stem[3]), top.to_array(), (top + Vec3::new(0.0, 0.04, 0.03)).to_array());
    let center = top + Vec3::new(0.0, 0.04, 0.03);
    for (i, angle) in [0.0f32, 90.0, 180.0, 270.0].iter().enumerate() {
        let (s, c) = angle.to_radians().sin_cos();
        let dir = Vec3::new(c, 0.3, s * 0.8 + 0.4);
        b.bone(&format!("petal_{:02}", i + 1), Some(head), center.to_array(), (center + dir * 0.06).to_array());
    }
    b.0
}

/// Palmier de 4 m : tronc incurvé de six os, six palmes retombantes.
pub(super) fn palm() -> Skeleton {
    let mut b = Builder::new();
    let trunk = b.chain("trunk_#", None, &arc(Vec3::ZERO, Vec3::new(0.15, 1.0, 0.0), 0.65, 6, Vec3::new(0.02, 0.0, 0.0)));
    let top = b.0.rest_head_tail(trunk[5]).1;
    for i in 0..6 {
        let a = i as f32 / 6.0 * std::f32::consts::TAU;
        let out = Vec3::new(a.cos(), 0.5, a.sin());
        b.chain(&format!("frond_{:02}_#", i + 1), Some(trunk[5]), &arc(top, out, 0.45, 3, Vec3::new(0.0, -0.45, 0.0)));
    }
    b.0
}

/// Corde de 1,6 m suspendue à 2 m.
pub(super) fn rope() -> Skeleton {
    let mut b = Builder::new();
    let points: Vec<Vec3> = (0..=8).map(|i| Vec3::new(0.0, 2.0 - i as f32 * 0.2, 0.0)).collect();
    b.chain("rope_#", None, &points);
    b.0
}

/// Tentacule de 1,2 m dressé depuis le sol, légèrement incurvé.
pub(super) fn tentacle() -> Skeleton {
    let mut b = Builder::new();
    b.chain("tentacle_#", None, &arc(Vec3::ZERO, Vec3::Y, 0.12, 10, Vec3::new(0.0, 0.0, 0.02)));
    b.0
}

/// Pendule : pivot à 2 m, tige de 1,2 m et masse.
pub(super) fn pendulum() -> Skeleton {
    let mut b = Builder::new();
    let rod = b.bone("pendulum_rod", None, [0.0, 2.0, 0.0], [0.0, 0.8, 0.0]);
    b.bone("pendulum_bob", Some(rod), [0.0, 0.8, 0.0], [0.0, 0.65, 0.0]);
    b.0
}

/// Machine : socle, roue à quatre rayons, bielle et piston.
pub(super) fn machine() -> Skeleton {
    let mut b = Builder::new();
    let base = b.bone("base", None, [0.0, 0.0, 0.0], [0.0, 0.5, 0.0]);
    let wheel = b.bone("wheel", Some(base), [0.0, 0.5, 0.0], [0.0, 0.5, 0.1]);
    for i in 0..4 {
        let a = i as f32 * std::f32::consts::FRAC_PI_2;
        let tip = Vec3::new(a.cos() * 0.3, 0.5 + a.sin() * 0.3, 0.1);
        b.bone(&format!("spoke_{:02}", i + 1), Some(wheel), [0.0, 0.5, 0.1], tip.to_array());
    }
    b.bone("piston", Some(base), [0.6, 0.2, 0.0], [0.6, 0.6, 0.0]);
    b.0
}
