//! Profil du visage : coupe de la tête par le plan de symétrie, dont on garde
//! le contour avant z(y) (la peau ; l'intérieur de la bouche ou des narines
//! est derrière). Ses bosses et ses creux donnent le bout du nez, la racine
//! du nez, l'arcade sourcilière, les lèvres et la fente de la bouche.

use glam::Vec3;

use crate::autorig::fit::surface::Surface;

/// Point du profil : hauteur et profondeur.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Point {
    pub y: f32,
    pub z: f32,
}

/// Contour avant échantillonné à pas régulier en hauteur.
pub(super) struct Profile {
    y0: f32,
    step: f32,
    /// `NaN` là où la coupe ne passe pas.
    z: Vec<f32>,
}

impl Profile {
    /// Profil entre `y_lo` et `y_hi` (plan x = `center_x`), pas de `step`.
    pub fn new(s: &Surface, center_x: f32, y_lo: f32, y_hi: f32, step: f32) -> Self {
        let n = (((y_hi - y_lo) / step).ceil() as usize).max(1) + 1;
        let mut z = vec![f32::NAN; n];
        for (a, b) in s.slice(Vec3::new(center_x, 0.0, 0.0), Vec3::X) {
            let (a, b) = if a.y <= b.y { (a, b) } else { (b, a) };
            let first = ((a.y - y_lo) / step).ceil().max(0.0) as usize;
            let mut i = first;
            while i < n {
                let y = y_lo + i as f32 * step;
                if y > b.y {
                    break;
                }
                let t = if b.y - a.y > 1e-9 {
                    (y - a.y) / (b.y - a.y)
                } else {
                    0.0
                };
                let zi = a.z + (b.z - a.z) * t;
                if z[i].is_nan() || zi > z[i] {
                    z[i] = zi;
                }
                i += 1;
            }
        }
        Self { y0: y_lo, step, z }
    }

    fn y(&self, i: usize) -> f32 {
        self.y0 + i as f32 * self.step
    }

    fn range(&self, lo: f32, hi: f32) -> impl Iterator<Item = usize> + '_ {
        let a = ((lo - self.y0) / self.step).ceil().max(0.0) as usize;
        let b = (((hi - self.y0) / self.step).floor().max(-1.0) + 1.0) as usize;
        (a..b.min(self.z.len())).filter(|i| !self.z[*i].is_nan())
    }

    /// Profondeur du contour à la hauteur `y` (échantillon le plus proche).
    pub fn at(&self, y: f32) -> Option<f32> {
        let i = ((y - self.y0) / self.step).round();
        (i >= 0.0)
            .then(|| self.z.get(i as usize).copied())
            .flatten()
            .filter(|z| !z.is_nan())
    }

    /// Point le plus en avant entre `lo` et `hi` : (y, z).
    pub fn max(&self, lo: f32, hi: f32) -> Option<Point> {
        self.range(lo, hi)
            .max_by(|a, b| self.z[*a].total_cmp(&self.z[*b]))
            .map(|i| Point {
                y: self.y(i),
                z: self.z[i],
            })
    }

    /// Point le plus en retrait entre `lo` et `hi`.
    pub fn min(&self, lo: f32, hi: f32) -> Option<Point> {
        self.range(lo, hi)
            .min_by(|a, b| self.z[*a].total_cmp(&self.z[*b]))
            .map(|i| Point {
                y: self.y(i),
                z: self.z[i],
            })
    }

    /// Creux le plus marqué entre `lo` et `hi` : minimum local dont le
    /// contour remonte des deux côtés dans un rayon `window` (profondeur =
    /// la plus faible des deux remontées).
    pub fn deepest_notch(&self, lo: f32, hi: f32, window: f32) -> Option<Point> {
        let mut best: Option<(f32, usize)> = None;
        for i in self.range(lo, hi) {
            let z = self.z[i];
            let y = self.y(i);
            let above = self.max(y, y + window).map_or(z, |p| p.z);
            let below = self.max(y - window, y).map_or(z, |p| p.z);
            let depth = (above - z).min(below - z);
            if depth > best.map_or(0.0, |b| b.0) {
                best = Some((depth, i));
            }
        }
        best.map(|(_, i)| Point {
            y: self.y(i),
            z: self.z[i],
        })
    }
}

/// Repères du profil.
#[derive(Clone, Copy, Debug)]
pub(super) struct Landmarks {
    pub nose: Point,
    pub nasion: Point,
    pub brow: Point,
    pub slit: Point,
    pub lip_top: Point,
    pub lip_bottom: Point,
    pub chin: Point,
}

impl Profile {
    /// Repères d'un visage humain entre la base du cou (`neck`) et le sommet
    /// du crâne (`top`). `None` si le profil n'a pas de nez saillant (tête
    /// sans détails : pas de visage à rigger).
    pub fn landmarks(&self, neck: f32, top: f32) -> Option<Landmarks> {
        let hh = top - neck;
        let nose = self.max(neck + 0.3 * hh, neck + 0.75 * hh)?;
        let around = |dy: f32| self.at(nose.y + dy).unwrap_or(nose.z);
        let protrusion = nose.z - around(0.05 * hh).max(around(-0.05 * hh));
        if protrusion < 0.012 * hh {
            return None;
        }
        let nasion = self.min(nose.y + 0.05 * hh, nose.y + 0.25 * hh)?;
        let brow = self.max(nasion.y, nasion.y + 0.15 * hh)?;
        let slit = self
            .deepest_notch(nose.y - 0.2 * hh, nose.y - 0.08 * hh, 0.03 * hh)
            .unwrap_or_else(|| {
                let y = nose.y - 0.13 * hh;
                Point {
                    y,
                    z: self.at(y).unwrap_or(nose.z),
                }
            });
        let lip_top = self.max(slit.y, slit.y + 0.06 * hh)?;
        let lip_bottom = self.max(slit.y - 0.06 * hh, slit.y)?;
        let chin_y = slit.y - 0.75 * (nose.y - slit.y);
        let chin = Point {
            y: chin_y,
            z: self.at(chin_y)?,
        };
        Some(Landmarks {
            nose,
            nasion,
            brow,
            slit,
            lip_top,
            lip_bottom,
            chin,
        })
    }
}
