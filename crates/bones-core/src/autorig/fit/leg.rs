//! Jambes et pieds : axe de la cuisse sous l'entrejambe (hanche), cheville
//! dans l'axe du tibia, genou au milieu, pied d'après son empreinte.

use glam::Vec3;

use super::surface::{Ring, Surface, nearest};
use crate::autorig::Layout;

/// Hauteur de l'articulation de la hanche au-dessus de l'entrejambe.
const HIP_ABOVE_CROTCH: f32 = 0.045;
/// Hauteur de la cheville au-dessus du sol.
const ANKLE_HEIGHT: f32 = 0.045;
/// Avancée minimale du genou devant la droite hanche–cheville.
const MIN_BEND: f32 = 0.006;

/// Tranche horizontale de la jambe gauche à la hauteur `y`, la plus proche
/// de `near` (les contours trop petits, trop gros ou sur l'axe sont exclus).
fn leg_ring(s: &Surface, l: &Layout, y: f32, near: Vec3) -> Option<Ring> {
    let h = l.height();
    let cx = l.center_x;
    let rings: Vec<Ring> = s
        .section(Vec3::new(cx, y, l.root.z), Vec3::Y)
        .into_iter()
        .filter(|r| {
            let dx = r.center.x - cx;
            dx > 0.015 * h && dx < 0.16 * h && r.radius() > 0.012 * h && r.radius() < 0.075 * h
        })
        .collect();
    nearest(&rings, Vec3::new(near.x, y, near.z))
}

/// Point de la droite (a, b) à la hauteur `y`.
fn at_height(a: Vec3, b: Vec3, y: f32) -> Vec3 {
    let d = b - a;
    if d.y.abs() < 1e-6 {
        return Vec3::new(a.x, y, a.z);
    }
    a + d * ((y - a.y) / d.y)
}

/// Place hanche, genou, cheville et orteils de la jambe gauche ; `floor` est
/// la hauteur du sol (bas du maillage). `None` (rien n'est modifié) si les
/// cuisses ne sont pas reconnues.
pub(super) fn fit_leg(l: &mut Layout, s: &Surface, floor: f32) -> Option<()> {
    let h = l.height();
    let guess = Vec3::new(l.center_x + 0.06 * h, 0.0, l.root.z);

    // Entrejambe : première tranche, en descendant, où la cuisse est isolée.
    // Si elle l'est dès le bassin (pièces séparées qui se chevauchent), la
    // hauteur générique de la hanche est gardée.
    let step = 0.004 * h;
    let crotch = (0..80)
        .map(|i| l.root.y - i as f32 * step)
        .find(|y| leg_ring(s, l, *y, guess).is_some())?;
    let hip_y = if crotch > l.root.y - step * 0.5 {
        l.hip.y
    } else {
        (crotch + HIP_ABOVE_CROTCH * h).min(l.root.y)
    };
    let c1 = leg_ring(s, l, crotch - 0.03 * h, guess)?.center;
    let c2 = leg_ring(s, l, crotch - 0.12 * h, c1)?.center;
    let mut hip = at_height(c1, c2, hip_y);
    hip.x = hip.x.max(l.center_x + 0.03 * h);

    // Cheville dans l'axe du bas du tibia.
    let a1 = leg_ring(s, l, floor + 0.075 * h, c2)?.center;
    let a2 = leg_ring(s, l, floor + 0.15 * h, a1)?.center;
    let ankle = at_height(a1, a2, floor + ANKLE_HEIGHT * h);

    // Genou à mi-hauteur, au centre de la tranche, légèrement en avant.
    let mid = at_height(hip, ankle, (hip.y + ankle.y) * 0.5);
    let knee = leg_ring(s, l, mid.y, mid).map_or(mid, |r| r.center);
    let axis = (ankle - hip).normalize_or(Vec3::NEG_Y);
    let foot_of = hip + axis * (knee - hip).dot(axis);
    let front = (Vec3::Z - axis * axis.dot(Vec3::Z)).normalize_or(Vec3::Z);
    let ahead = (knee - foot_of).dot(front);
    l.hip = hip;
    l.knee = knee + front * (MIN_BEND * h - ahead).max(0.0);
    l.ankle = ankle;
    fit_foot(l, s, floor);
    Some(())
}

/// Base et bout des orteils d'après l'empreinte du pied.
fn fit_foot(l: &mut Layout, s: &Surface, floor: f32) {
    let h = l.height();
    let foot: Vec<Vec3> = s
        .vertices()
        .map(|v| s.positions[v as usize])
        .filter(|p| p.y < floor + ANKLE_HEIGHT * h && (p.x - l.ankle.x).abs() < 0.1 * h)
        .collect();
    let (Some(heel), Some(tip)) = (
        foot.iter().copied().min_by(|a, b| a.z.total_cmp(&b.z)),
        foot.iter().copied().max_by(|a, b| a.z.total_cmp(&b.z)),
    ) else {
        return;
    };
    let mut dir = tip - heel;
    dir.y = 0.0;
    let (dir, length) = (dir.normalize_or(Vec3::Z), dir.length());
    // Articulations métatarso-phalangiennes vers 72 % de la longueur du pied.
    let ball = heel + dir * (0.72 * length);
    let ball = Vec3::new(ball.x, floor + 0.02 * h, ball.z);
    let toe = nearest(&s.section(ball, dir), ball)
        .filter(|r| r.center.distance(ball) < 0.05 * h)
        .map_or(ball, |r| r.center);
    l.toe = toe;
    l.toe_tip = Vec3::new(toe.x, toe.y * 0.8 + floor * 0.2, toe.z) + dir * (tip - toe).dot(dir);
}
