//! Tronc et bras : colonne recentrée dans le volume, bras suivi tranche par
//! tranche de l'épaule au bout des doigts, poignet au plus étroit, coude
//! d'après les proportions humaines.

use glam::Vec3;

use super::surface::{Ring, Surface, nearest};
use crate::autorig::Layout;

/// Part du bras (au coude) dans la longueur épaule–poignet (Drillis & Contini :
/// bras 0,186·H, avant-bras 0,146·H).
const UPPER_ARM_RATIO: f32 = 0.56;
/// Recul minimal du coude derrière la droite épaule–poignet (part de la
/// taille) : fixe le sens de flexion pour l'IK.
const MIN_BEND: f32 = 0.006;

/// Recentre la colonne, le cou et la tête dans l'épaisseur du tronc (Z) ;
/// X reste sur l'axe de symétrie.
pub(super) fn fit_torso(l: &mut Layout, s: &Surface) {
    let h = l.height();
    let cx = l.center_x;
    let centered = |p: Vec3| -> Vec3 {
        let rings: Vec<Ring> = s
            .section(p, Vec3::Y)
            .into_iter()
            .filter(|r| (r.center.x - cx).abs() < 0.08 * h)
            .collect();
        match nearest(&rings, p) {
            Some(r) => Vec3::new(cx, p.y, r.center.z),
            None => p,
        }
    };
    l.root = centered(l.root);
    for p in l.spine.iter_mut().chain(l.neck.iter_mut()) {
        *p = centered(*p);
    }
    let top = centered(l.head_top - Vec3::Y * 0.05 * h);
    l.head_top.z = top.z;
}

/// Axe du bras suivi tranche par tranche.
pub(super) struct ArmTrack {
    /// Centres des tranches, de l'épaule au bout du doigt le plus long.
    pub points: Vec<Vec3>,
    pub radii: Vec<f32>,
}

/// Suit le bras depuis l'épaule le long de `dir`, une tranche
/// perpendiculaire tous les 1 % de la taille.
pub(super) fn track_arm(s: &Surface, shoulder: Vec3, dir: Vec3, h: f32) -> Option<ArmTrack> {
    let step = 0.01 * h;
    // Premier contour isolé du tronc, en descendant le long du bras.
    let (mut c, mut radius) = (5..30).find_map(|i| {
        let q = shoulder + dir * (i as f32 * step);
        let r = nearest(&s.section(q, dir), q)?;
        (r.radius() < 0.045 * h && r.center.distance(q) < 0.04 * h).then_some((r.center, r.radius()))
    })?;
    let mut track = ArmTrack {
        points: vec![shoulder, c],
        radii: vec![radius, radius],
    };
    let mut dir = (c - shoulder).normalize_or(dir);
    for _ in 0..200 {
        let q = c + dir * step;
        let Some(r) = nearest(&s.section(q, dir), q) else {
            break;
        };
        // Contour perdu (bout du doigt) ou fusion avec une autre partie du corps.
        if r.center.distance(q) > (0.6 * radius).max(step) || r.radius() > 1.8 * radius {
            break;
        }
        dir = (dir + (r.center - c).normalize_or(dir)).normalize_or(dir);
        c = r.center;
        radius = r.radius();
        track.points.push(c);
        track.radii.push(radius);
    }
    (track.points.len() > 10).then_some(track)
}

impl ArmTrack {
    /// Longueurs cumulées le long de l'axe.
    fn arc(&self) -> Vec<f32> {
        let mut acc = 0.0;
        let mut out = vec![0.0];
        for w in self.points.windows(2) {
            acc += w[0].distance(w[1]);
            out.push(acc);
        }
        out
    }

    /// Point de l'axe à l'abscisse curviligne `t`.
    fn at(&self, arc: &[f32], t: f32) -> Vec3 {
        let i = arc.partition_point(|a| *a < t).clamp(1, arc.len() - 1);
        let span = (arc[i] - arc[i - 1]).max(1e-6);
        self.points[i - 1].lerp(self.points[i], ((t - arc[i - 1]) / span).clamp(0.0, 1.0))
    }

    /// Indice du poignet : tranche la plus étroite entre 60 et 88 % du bras.
    fn wrist_index(&self, arc: &[f32]) -> Option<usize> {
        let total = *arc.last()?;
        (0..self.points.len())
            .filter(|i| arc[*i] > 0.6 * total && arc[*i] < 0.88 * total)
            .min_by(|a, b| self.radii[*a].total_cmp(&self.radii[*b]))
    }
}

/// Place épaule (profondeur), coude et poignet d'après le suivi du bras.
/// Retourne le bout du doigt le plus long.
pub(super) fn fit_arm(l: &mut Layout, s: &Surface) -> Option<Vec3> {
    let h = l.height();
    let dir = (l.wrist - l.shoulder).normalize_or(Vec3::X);
    let track = track_arm(s, l.shoulder, dir, h)?;
    let arc = track.arc();
    let w = track.wrist_index(&arc)?;

    // L'épaule prend la profondeur du haut du bras.
    let upper = track.points[1];
    l.shoulder.z = upper.z;
    l.clavicle.z = upper.z;
    track_to_layout(l, &track, &arc, w);
    track.points.last().copied()
}

fn track_to_layout(l: &mut Layout, track: &ArmTrack, arc: &[f32], w: usize) {
    let h = l.height();
    l.wrist = track.points[w];
    let elbow = track.at(arc, arc[w] * UPPER_ARM_RATIO);
    // Coude au moins légèrement en arrière de la droite épaule–poignet.
    let axis = (l.wrist - l.shoulder).normalize_or(Vec3::X);
    let foot = l.shoulder + axis * (elbow - l.shoulder).dot(axis);
    let back = (Vec3::NEG_Z - axis * axis.dot(Vec3::NEG_Z)).normalize_or(Vec3::NEG_Z);
    let behind = (elbow - foot).dot(back);
    l.elbow = elbow + back * (MIN_BEND * h - behind).max(0.0);
    let tip = *track.points.last().expect("suivi non vide");
    l.hand_tip = l.wrist + (tip - l.wrist) * 0.45;
}
