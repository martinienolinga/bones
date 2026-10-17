//! Cadrage de la caméra : placer une boîte englobante dans une zone utile de
//! l'écran (entre les panneaux), et interpoler entre deux cadrages pour les
//! transitions animées.
//!
//! Le cadrage est exact pour une orientation donnée : chaque bord de la zone
//! utile définit un demi-espace que tous les coins de la boîte doivent
//! respecter, ce qui donne des contraintes linéaires sur la position de
//! l'œil. On retient l'œil le plus proche qui les satisfait toutes (la boîte
//! touche deux bords opposés), puis on répartit le jeu restant de part et
//! d'autre sur l'autre axe : la boîte est centrée dans la zone.

use std::f32::consts::{PI, TAU};

use glam::Vec3;

use super::camera::{OrbitCamera, Viewport};

impl OrbitCamera {
    /// Caméra de même orientation (lacet, tangage, champ, projection) qui
    /// cadre au plus près la boîte `[lo, hi]` dans la zone `safe` de l'écran
    /// (pixels physiques, comme `viewport`), centrée dans cette zone.
    /// `None` si la zone ou la boîte sont dégénérées.
    pub fn framing_box(
        &self,
        lo: Vec3,
        hi: Vec3,
        viewport: &Viewport,
        safe: &Viewport,
    ) -> Option<OrbitCamera> {
        if !(lo.is_finite() && hi.is_finite()) || viewport.width < 1.0 || viewport.height < 1.0 {
            return None;
        }
        let (x0, x1, y0, y1) = ndc_window(viewport, safe)?;
        let (right, up, forward) = (self.right(), self.up(), self.forward());
        let corners = corners(lo, hi);
        let center = (lo + hi) * 0.5;
        let ty = (self.fov_y * 0.5).tan();
        let tx = ty * viewport.aspect();
        let (er, eu, ef, distance) = if self.orthographic {
            let (amin, amax) = extent(&corners, |p| p.dot(right));
            let (bmin, bmax) = extent(&corners, |p| p.dot(up));
            // Échelle : demi-hauteur visible = distance · tan(fov / 2).
            let d = ((amax - amin) / ((x1 - x0) * tx))
                .max((bmax - bmin) / ((y1 - y0) * ty))
                .max(1e-4);
            let er = middle(amax - x1 * d * tx, amin - x0 * d * tx);
            let eu = middle(bmax - y1 * d * ty, bmin - y0 * d * ty);
            let ef = center.dot(forward) - d;
            (er, eu, ef, d)
        } else {
            // Bords droit/gauche : a − x·tx·c, avec a et c les coordonnées du
            // coin le long de « droite » et de la visée.
            let (_, mr) = extent(&corners, |p| p.dot(right) - x1 * tx * p.dot(forward));
            let (ml, _) = extent(&corners, |p| p.dot(right) - x0 * tx * p.dot(forward));
            let (_, mt) = extent(&corners, |p| p.dot(up) - y1 * ty * p.dot(forward));
            let (mb, _) = extent(&corners, |p| p.dot(up) - y0 * ty * p.dot(forward));
            let ef = ((ml - mr) / ((x1 - x0) * tx)).min((mb - mt) / ((y1 - y0) * ty));
            let er = middle(mr + x1 * tx * ef, ml + x0 * tx * ef);
            let eu = middle(mt + y1 * ty * ef, mb + y0 * ty * ef);
            (er, eu, ef, center.dot(forward) - ef)
        };
        if !(distance.is_finite() && distance > 0.0) {
            return None;
        }
        let eye = right * er + up * eu + forward * ef;
        Some(OrbitCamera {
            target: eye + forward * distance,
            distance,
            ..*self
        })
    }

    /// Cadrage intermédiaire entre `self` (t = 0) et `to` (t = 1) : cible
    /// linéaire, distance géométrique (le zoom paraît régulier), angles par
    /// le plus court chemin. Les réglages (verrou, projection, champ) restent
    /// ceux de `self`.
    pub fn blend(&self, to: &OrbitCamera, t: f32) -> OrbitCamera {
        let t = t.clamp(0.0, 1.0);
        if t >= 1.0 {
            // Arrivée exacte (sans erreur d'arrondi sur les angles).
            return OrbitCamera {
                target: to.target,
                distance: to.distance,
                yaw: to.yaw,
                pitch: to.pitch,
                ..*self
            };
        }
        let turn = (to.yaw - self.yaw + PI).rem_euclid(TAU) - PI;
        let distance = if self.distance > 0.0 && to.distance > 0.0 {
            self.distance * (to.distance / self.distance).powf(t)
        } else {
            self.distance + (to.distance - self.distance) * t
        };
        OrbitCamera {
            target: self.target.lerp(to.target, t),
            distance,
            yaw: self.yaw + turn * t,
            pitch: self.pitch + (to.pitch - self.pitch) * t,
            ..*self
        }
    }

    /// Même point de vue (cible, distance, angles), aux réglages près.
    pub fn same_view(&self, other: &OrbitCamera) -> bool {
        self.target == other.target
            && self.distance == other.distance
            && self.yaw == other.yaw
            && self.pitch == other.pitch
    }
}

/// Zone utile en coordonnées normalisées (x0 < x1 de gauche à droite, y0 < y1
/// de bas en haut), bornée à l'écran.
fn ndc_window(viewport: &Viewport, safe: &Viewport) -> Option<(f32, f32, f32, f32)> {
    let nx = |px: f32| ((px - viewport.x) / viewport.width * 2.0 - 1.0).clamp(-1.0, 1.0);
    let ny = |py: f32| (1.0 - (py - viewport.y) / viewport.height * 2.0).clamp(-1.0, 1.0);
    let (x0, x1) = (nx(safe.x), nx(safe.x + safe.width));
    let (y0, y1) = (ny(safe.y + safe.height), ny(safe.y));
    (x1 - x0 > 1e-3 && y1 - y0 > 1e-3).then_some((x0, x1, y0, y1))
}

fn corners(lo: Vec3, hi: Vec3) -> [Vec3; 8] {
    std::array::from_fn(|i| {
        Vec3::new(
            if i & 1 == 0 { lo.x } else { hi.x },
            if i & 2 == 0 { lo.y } else { hi.y },
            if i & 4 == 0 { lo.z } else { hi.z },
        )
    })
}

/// Minimum et maximum d'une fonction sur les coins.
fn extent(corners: &[Vec3; 8], f: impl Fn(Vec3) -> f32) -> (f32, f32) {
    corners
        .iter()
        .map(|p| f(*p))
        .fold((f32::MAX, f32::MIN), |(lo, hi), v| (lo.min(v), hi.max(v)))
}

fn middle(a: f32, b: f32) -> f32 {
    (a + b) * 0.5
}

#[cfg(test)]
mod tests;
