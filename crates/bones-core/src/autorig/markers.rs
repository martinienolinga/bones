//! Marqueurs « Smart » : points clés du corps placés sur le modèle vu de face,
//! avec une estimation automatique à partir de la silhouette.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::Mesh;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarkerKind {
    HeadTop,
    Chin,
    Neck,
    Shoulder,
    Wrist,
    Root,
    Ankle,
}

impl MarkerKind {
    pub const ALL: [MarkerKind; 7] = [
        MarkerKind::HeadTop,
        MarkerKind::Chin,
        MarkerKind::Neck,
        MarkerKind::Shoulder,
        MarkerKind::Wrist,
        MarkerKind::Root,
        MarkerKind::Ankle,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MarkerKind::HeadTop => "Top of head",
            MarkerKind::Chin => "Chin",
            MarkerKind::Neck => "Neck",
            MarkerKind::Shoulder => "Shoulder",
            MarkerKind::Wrist => "Wrist",
            MarkerKind::Root => "Pelvis",
            MarkerKind::Ankle => "Ankle",
        }
    }
}

/// Marqueurs « Smart ». Les membres sont donnés côté gauche (+X).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Markers {
    pub head_top: Vec3,
    pub chin: Vec3,
    pub neck: Vec3,
    pub shoulder: Vec3,
    pub wrist: Vec3,
    pub root: Vec3,
    pub ankle: Vec3,
}

impl Markers {
    pub fn get(&self, kind: MarkerKind) -> Vec3 {
        match kind {
            MarkerKind::HeadTop => self.head_top,
            MarkerKind::Chin => self.chin,
            MarkerKind::Neck => self.neck,
            MarkerKind::Shoulder => self.shoulder,
            MarkerKind::Wrist => self.wrist,
            MarkerKind::Root => self.root,
            MarkerKind::Ankle => self.ankle,
        }
    }

    pub fn set(&mut self, kind: MarkerKind, p: Vec3) {
        let slot = match kind {
            MarkerKind::HeadTop => &mut self.head_top,
            MarkerKind::Chin => &mut self.chin,
            MarkerKind::Neck => &mut self.neck,
            MarkerKind::Shoulder => &mut self.shoulder,
            MarkerKind::Wrist => &mut self.wrist,
            MarkerKind::Root => &mut self.root,
            MarkerKind::Ankle => &mut self.ankle,
        };
        *slot = p;
    }

    /// Estimation automatique à partir de la silhouette du maillage (pose en
    /// T ou en A, de face). L'utilisateur affine ensuite à la main.
    pub fn guess(mesh: &Mesh) -> Option<Markers> {
        let (lo, hi) = mesh.bounds()?;
        let h = (hi.y - lo.y).max(1e-3);
        let cx = (lo.x + hi.x) * 0.5;
        let cz = (lo.z + hi.z) * 0.5;
        let at = |t: f32| lo.y + t * h;

        // Bras : sommet le plus à gauche entre les cuisses et le haut des
        // épaules (bout des doigts, en pose en T comme en A).
        let arm_tip = mesh
            .positions
            .iter()
            .filter(|p| p.y > at(0.3) && p.y < at(0.92))
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .copied()
            .unwrap_or(Vec3::new(hi.x, at(0.8), cz));
        // Pieds : abscisse moyenne des sommets bas du côté gauche.
        let (sum, count) = mesh
            .positions
            .iter()
            .filter(|p| p.y < at(0.08) && p.x > cx)
            .fold((0.0, 0), |(s, c), p| (s + p.x, c + 1));
        let ankle_x = if count > 0 {
            sum / count as f32
        } else {
            cx + 0.06 * h
        };
        let shoulder_y = at(0.81);
        let reach = (arm_tip.x - cx).max(0.1 * h);
        let shoulder = Vec3::new(cx + 0.11 * h, shoulder_y, cz);
        // Pose en T : poignet à l'horizontale de la main. Pose en A (main
        // nettement plus basse que l'épaule) : sur la droite épaule – bout des
        // doigts, la main faisant environ le quart du bras.
        let wrist = if arm_tip.y > shoulder_y - 0.1 * h {
            Vec3::new(cx + reach * 0.88, arm_tip.y.min(shoulder_y), cz)
        } else {
            shoulder + (Vec3::new(arm_tip.x, arm_tip.y, cz) - shoulder) * 0.76
        };

        Some(Markers {
            head_top: Vec3::new(cx, hi.y, cz),
            chin: Vec3::new(cx, at(0.87), cz),
            neck: Vec3::new(cx, at(0.835), cz),
            shoulder,
            wrist,
            root: Vec3::new(cx, at(0.53), cz),
            ankle: Vec3::new(ankle_x, at(0.045), cz),
        })
    }
}
