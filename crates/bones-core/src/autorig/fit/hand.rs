//! Main et doigts : la main est la partie de la surface au-delà du poignet ;
//! ses cinq branches les plus nettes sont les doigts. Le pouce est celui dont
//! la commissure est la plus proche du poignet ; les autres sont rangés en
//! s'éloignant du pouce. Les articulations suivent les proportions des
//! phalanges, à partir d'un point de départ prolongé dans la paume.

use glam::Vec3;

use super::branches::{Branch, branches, geodesic};
use super::surface::Surface;
use crate::autorig::Layout;

/// Parts des phalanges (proximale, moyenne, distale) dans la longueur d'un
/// doigt, de l'articulation métacarpo-phalangienne au bout.
const PHALANGES: [f32; 3] = [0.47, 0.28, 0.25];
/// Recul de l'articulation métacarpo-phalangienne derrière la commissure,
/// en part de la longueur visible du doigt.
const KNUCKLE_DEPTH: f32 = 0.33;

/// Ajuste la main et les doigts ; `axis` va du coude vers le poignet.
/// Retourne `false` (rien n'est modifié) si les cinq doigts ne sont pas
/// clairement séparés.
pub(super) fn fit_hand(l: &mut Layout, s: &Surface, axis: Vec3) -> bool {
    let h = l.height();
    let n = s.positions.len();
    let beyond = |v: u32| (s.positions[v as usize] - l.wrist).dot(axis) >= 0.0;

    // Région de la main : sommets au-delà du plan du poignet, reliés à lui.
    let Some(seed) = s.vertices().filter(|v| beyond(*v)).min_by(|a, b| {
        let da = s.positions[*a as usize].distance_squared(l.wrist);
        let db = s.positions[*b as usize].distance_squared(l.wrist);
        da.total_cmp(&db)
    }) else {
        return false;
    };
    let mut inside = vec![false; n];
    let mut stack = vec![seed];
    inside[seed as usize] = true;
    while let Some(v) = stack.pop() {
        for &u in s.neighbors(v) {
            if !inside[u as usize] && beyond(u) && s.positions[u as usize].distance(l.wrist) < 0.2 * h {
                inside[u as usize] = true;
                stack.push(u);
            }
        }
    }
    // Frontière : sommets de la région voisins du plan du poignet.
    let sources: Vec<u32> = (0..n as u32)
        .filter(|v| inside[*v as usize] && s.neighbors(*v).iter().any(|u| !beyond(*u)))
        .collect();
    if sources.is_empty() {
        return false;
    }
    let dist = geodesic(s, &sources, &inside);
    let mut found = branches(s, &dist, 0.012 * h, 0.004 * h);
    if found.len() < 5 {
        return false;
    }
    // Les cinq doigts les plus longs, le pouce d'abord.
    found.sort_by(|a, b| (b.length()).total_cmp(&a.length()));
    found.truncate(5);
    found.sort_by(|a, b| a.web.total_cmp(&b.web));
    let thumb = found.remove(0);
    let thumb_base = thumb.base();
    found.sort_by(|a, b| {
        a.base()
            .distance(thumb_base)
            .total_cmp(&b.base().distance(thumb_base))
    });

    for (f, branch) in found.iter().enumerate() {
        l.fingers[f + 1] = finger_joints(branch);
    }
    l.fingers[0] = thumb_joints(&thumb, l.wrist);
    l.hand_tip = l.fingers[2][0];
    true
}

impl Branch {
    /// Longueur visible : de la commissure au bout, le long de l'axe.
    fn length(&self) -> f32 {
        polyline(self).windows(2).map(|w| w[0].distance(w[1])).sum()
    }

    fn base(&self) -> Vec3 {
        self.axis.first().copied().unwrap_or(self.tip)
    }
}

/// Axe du doigt, bout compris.
fn polyline(b: &Branch) -> Vec<Vec3> {
    let mut points = b.axis.clone();
    points.push(b.tip);
    points
}

/// Point de la ligne brisée à l'abscisse curviligne `t` (prolongée avant
/// son début si `t < 0`).
fn along(points: &[Vec3], t: f32) -> Vec3 {
    if t <= 0.0 || points.len() < 2 {
        let dir = start_direction(points);
        return points[0] + dir * t;
    }
    let mut acc = 0.0;
    for w in points.windows(2) {
        let len = w[0].distance(w[1]);
        if acc + len >= t {
            return w[0].lerp(w[1], (t - acc) / len.max(1e-6));
        }
        acc += len;
    }
    *points.last().expect("ligne non vide")
}

/// Direction du doigt à sa base (moyennée sur son premier tiers).
fn start_direction(points: &[Vec3]) -> Vec3 {
    if points.len() < 2 {
        return Vec3::NEG_Y;
    }
    let k = (points.len() / 3).clamp(1, points.len() - 1);
    (points[k] - points[0]).normalize_or(Vec3::NEG_Y)
}

fn finger_joints(b: &Branch) -> [Vec3; 4] {
    let points = polyline(b);
    let visible = b.length();
    let depth = KNUCKLE_DEPTH * visible;
    let total = visible + depth;
    let mut t = -depth;
    let mut joints = [along(&points, t); 4];
    for (i, part) in PHALANGES.iter().enumerate() {
        t += part * total;
        joints[i + 1] = along(&points, t);
    }
    joints
}

/// Pouce : métacarpien depuis la base de l'éminence thénar (entre le
/// poignet et la commissure), puis deux phalanges.
fn thumb_joints(b: &Branch, wrist: Vec3) -> [Vec3; 4] {
    let points = polyline(b);
    let visible = b.length();
    let base = wrist.lerp(b.base(), 0.4);
    let knuckle = along(&points, -0.25 * visible);
    let joint = along(&points, 0.45 * visible);
    [base, knuckle, joint, b.tip]
}
