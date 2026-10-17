//! Marqueurs du visage (étape « Face » d'Auto-Rig Pro) : quelques points
//! clés de la tête, estimés sur le maillage ([`FaceMarkers::guess`]) puis
//! ajustables à la main. Les points pairs sont donnés côté gauche (+X).

use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FaceMarkers {
    /// Centre du globe oculaire gauche.
    pub eye: Vec3,
    pub eye_radius: f32,
    /// Milieu du sourcil gauche (sur la peau).
    pub brow: Vec3,
    pub nose_tip: Vec3,
    /// Milieu des lèvres supérieure et inférieure (sur la peau).
    pub lip_top: Vec3,
    pub lip_bottom: Vec3,
    /// Commissure gauche des lèvres.
    pub mouth_corner: Vec3,
    /// Pointe du menton (sur la peau).
    pub chin: Vec3,
    /// Charnière de la mâchoire, sur l'axe du corps.
    pub jaw: Vec3,
    /// Oreille gauche (pointe), si le modèle en a.
    #[serde(default)]
    pub ear: Option<Vec3>,
    /// Dents du haut et du bas (pièces séparées dans la bouche), si présentes.
    #[serde(default)]
    pub teeth_top: Option<Vec3>,
    #[serde(default)]
    pub teeth_bottom: Option<Vec3>,
    /// Langue (pièce séparée) : base et bout, si présente.
    #[serde(default)]
    pub tongue: Option<[Vec3; 2]>,
}

/// Marqueurs du visage plaçables à la main.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FaceMarkerKind {
    Eye,
    Brow,
    NoseTip,
    LipTop,
    LipBottom,
    MouthCorner,
    Chin,
    Jaw,
    Ear,
}

impl FaceMarkerKind {
    pub const ALL: [FaceMarkerKind; 9] = [
        FaceMarkerKind::Eye,
        FaceMarkerKind::Brow,
        FaceMarkerKind::NoseTip,
        FaceMarkerKind::LipTop,
        FaceMarkerKind::LipBottom,
        FaceMarkerKind::MouthCorner,
        FaceMarkerKind::Chin,
        FaceMarkerKind::Jaw,
        FaceMarkerKind::Ear,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FaceMarkerKind::Eye => "Eye",
            FaceMarkerKind::Brow => "Eyebrow",
            FaceMarkerKind::NoseTip => "Nose tip",
            FaceMarkerKind::LipTop => "Upper lip",
            FaceMarkerKind::LipBottom => "Lower lip",
            FaceMarkerKind::MouthCorner => "Mouth corner",
            FaceMarkerKind::Chin => "Chin",
            FaceMarkerKind::Jaw => "Jaw hinge",
            FaceMarkerKind::Ear => "Ear",
        }
    }

    /// Marqueur sur l'axe du corps (pas de symétrique).
    pub fn is_central(self) -> bool {
        matches!(
            self,
            FaceMarkerKind::NoseTip
                | FaceMarkerKind::LipTop
                | FaceMarkerKind::LipBottom
                | FaceMarkerKind::Chin
                | FaceMarkerKind::Jaw
        )
    }
}

impl FaceMarkers {
    /// Position d'un marqueur ; `None` pour une oreille absente.
    pub fn get(&self, kind: FaceMarkerKind) -> Option<Vec3> {
        Some(match kind {
            FaceMarkerKind::Eye => self.eye,
            FaceMarkerKind::Brow => self.brow,
            FaceMarkerKind::NoseTip => self.nose_tip,
            FaceMarkerKind::LipTop => self.lip_top,
            FaceMarkerKind::LipBottom => self.lip_bottom,
            FaceMarkerKind::MouthCorner => self.mouth_corner,
            FaceMarkerKind::Chin => self.chin,
            FaceMarkerKind::Jaw => self.jaw,
            FaceMarkerKind::Ear => return self.ear,
        })
    }

    pub fn set(&mut self, kind: FaceMarkerKind, p: Vec3) {
        match kind {
            FaceMarkerKind::Eye => self.eye = p,
            FaceMarkerKind::Brow => self.brow = p,
            FaceMarkerKind::NoseTip => self.nose_tip = p,
            FaceMarkerKind::LipTop => self.lip_top = p,
            FaceMarkerKind::LipBottom => self.lip_bottom = p,
            FaceMarkerKind::MouthCorner => self.mouth_corner = p,
            FaceMarkerKind::Chin => self.chin = p,
            FaceMarkerKind::Jaw => self.jaw = p,
            FaceMarkerKind::Ear => self.ear = Some(p),
        }
    }

    /// Toutes les valeurs sont finies et le rayon de l'œil positif.
    pub fn is_valid(&self) -> bool {
        let optional = self
            .ear
            .into_iter()
            .chain(self.teeth_top)
            .chain(self.teeth_bottom);
        let tongue = self.tongue.into_iter().flatten();
        FaceMarkerKind::ALL
            .iter()
            .filter_map(|k| self.get(*k))
            .chain(optional)
            .chain(tongue)
            .all(|p| p.is_finite())
            && self.eye_radius.is_finite()
            && self.eye_radius > 0.0
    }

    /// Écart entre les centres des yeux : échelle du visage.
    pub fn scale(&self, center_x: f32) -> f32 {
        (2.0 * (self.eye.x - center_x).abs()).max(1e-3)
    }
}
