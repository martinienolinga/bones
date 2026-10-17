//! Garde-fous de l'ajustement : chaque partie recalée (tronc, bras, main,
//! jambe) doit rester plausible par rapport à la disposition générique
//! d'avant l'ajustement ; sinon elle reprend sa position générique. Un
//! maillage stylisé ou inhabituel n'est ainsi jamais moins bien riggé
//! qu'avec les seuls marqueurs.

use glam::Vec3;

use crate::autorig::Layout;

fn finite(points: &[Vec3]) -> bool {
    points.iter().all(|p| p.is_finite())
}

/// Rapport `a / b` dans `[lo, hi]` (et `b` non nul).
fn ratio_in(a: f32, b: f32, lo: f32, hi: f32) -> bool {
    b > 1e-6 && (lo..=hi).contains(&(a / b))
}

impl Layout {
    pub(super) fn restore_torso(&mut self, from: &Layout) {
        self.root = from.root;
        self.spine.clone_from(&from.spine);
        self.neck.clone_from(&from.neck);
        self.head_top = from.head_top;
    }

    pub(super) fn restore_arm(&mut self, from: &Layout) {
        self.clavicle = from.clavicle;
        self.shoulder = from.shoulder;
        self.elbow = from.elbow;
        self.wrist = from.wrist;
        self.hand_tip = from.hand_tip;
        self.fingers = from.fingers;
    }

    /// Pied générique (même décalage qu'avant l'ajustement) sous la cheville.
    pub(super) fn restore_foot(&mut self, from: &Layout) {
        self.toe = self.ankle + (from.toe - from.ankle);
        self.toe_tip = self.toe + (from.toe_tip - from.toe);
    }

    pub(super) fn restore_leg(&mut self, from: &Layout) {
        self.hip = from.hip;
        self.knee = from.knee;
        self.ankle = from.ankle;
        self.toe = from.toe;
        self.toe_tip = from.toe_tip;
    }
}

/// Tronc : seule la profondeur change, de moins de 15 % de la taille.
pub(super) fn torso_ok(f: &Layout, g: &Layout) -> bool {
    let h = g.height();
    let points = || f.spine.iter().chain(&f.neck).chain([&f.root, &f.head_top]);
    let generic = g.spine.iter().chain(&g.neck).chain([&g.root, &g.head_top]);
    finite(&points().copied().collect::<Vec<_>>())
        && points().zip(generic).all(|(a, b)| a.distance(*b) < 0.15 * h)
}

/// Bras : du bon côté, longueur proche de la générique, segments équilibrés.
pub(super) fn arm_ok(f: &Layout, g: &Layout) -> bool {
    let h = g.height();
    let reach = g.wrist.distance(g.shoulder);
    let (upper, lower) = (f.elbow.distance(f.shoulder), f.wrist.distance(f.elbow));
    finite(&[f.clavicle, f.shoulder, f.elbow, f.wrist])
        && f.wrist.x - f.center_x > 0.02 * h
        && f.elbow.x - f.center_x > 0.02 * h
        && ratio_in(upper + lower, reach, 0.5, 1.6)
        && ratio_in(upper, lower, 0.6, 2.0)
        && f.wrist.distance(g.wrist) < 0.6 * reach
}

/// Main : doigts dans la main, phalanges non dégénérées.
pub(super) fn hand_ok(f: &Layout) -> bool {
    let h = f.height();
    let palm = f.hand_tip.distance(f.wrist);
    finite(&[f.hand_tip])
        && (0.02 * h..0.12 * h).contains(&palm)
        && f.fingers.iter().all(|joints| {
            finite(joints)
                && joints.iter().all(|p| p.distance(f.wrist) < 0.15 * h)
                && joints.windows(2).all(|w| w[0].distance(w[1]) > 0.002 * h)
        })
}

/// Jambe : du bon côté, hanche > genou > cheville, segments équilibrés.
pub(super) fn leg_ok(f: &Layout, g: &Layout) -> bool {
    let h = g.height();
    let (thigh, shank) = (f.knee.distance(f.hip), f.ankle.distance(f.knee));
    finite(&[f.hip, f.knee, f.ankle])
        && f.hip.x - f.center_x > 0.01 * h
        && f.hip.y < g.root.y + 0.05 * h
        && f.hip.y > f.knee.y + 0.05 * h
        && f.knee.y > f.ankle.y + 0.05 * h
        && ratio_in(thigh, shank, 0.6, 1.7)
        && ratio_in(
            thigh + shank,
            g.knee.distance(g.hip) + g.ankle.distance(g.knee),
            0.6,
            1.5,
        )
        && f.ankle.distance(g.ankle) < 0.15 * h
}

/// Pied : longueur plausible, orteils vers l'avant.
pub(super) fn foot_ok(f: &Layout) -> bool {
    let h = f.height();
    let foot = f.toe.distance(f.ankle);
    finite(&[f.toe, f.toe_tip])
        && (0.03 * h..0.2 * h).contains(&foot)
        && f.toe.z > f.ankle.z
        && f.toe_tip.z > f.toe.z
}
