//! Disposition des os du visage : position monde (tête, queue) de chaque os,
//! déduite des marqueurs du visage et de leur échelle. Le côté droit (`.r`)
//! est le miroir du gauche par rapport au plan `x = center_x`.
//!
//! Conventions (voir [`super`]) : les os de surface (sourcils, joues, nez,
//! narines, lèvres, commissures, menton, dents) ont la tête juste sous la peau
//! et pointent vers l'avant (+Z) ; la mâchoire pointe de la charnière vers le
//! menton ; les paupières pivotent au centre du globe vers le bord de l'œil ;
//! la langue suit sa chaîne de la base vers le bout.

use glam::Vec3;

use super::{FaceBone, FaceMarkers};

/// Positions monde des os du visage, calculées à la demande.
#[derive(Clone, Copy, Debug)]
pub struct FaceLayout {
    pub markers: FaceMarkers,
    pub center_x: f32,
}

impl FaceLayout {
    pub fn new(markers: FaceMarkers, center_x: f32) -> Self {
        Self { markers, center_x }
    }

    /// Miroir d'un point par rapport au plan `x = center_x`.
    fn mirror(&self, p: Vec3) -> Vec3 {
        Vec3::new(2.0 * self.center_x - p.x, p.y, p.z)
    }

    /// Longueur des petits os de surface (pointant vers l'avant).
    fn surface_len(&self) -> f32 {
        (0.5 * self.markers.eye_radius).max(4e-3)
    }

    /// Os de surface : tête juste sous la peau, queue sur la peau (vers +Z).
    fn surface(&self, skin: Vec3) -> (Vec3, Vec3) {
        (skin - Vec3::Z * self.surface_len(), skin)
    }

    /// (tête, queue) monde de l'os `bone`, ou `None` s'il est absent du modèle.
    pub fn joint(&self, bone: FaceBone) -> Option<(Vec3, Vec3)> {
        use FaceBone::*;
        let m = &self.markers;
        let cx = self.center_x;
        let ex = m.eye.x - cx; // demi-écart inter-oculaire (> 0)
        let r = m.eye_radius;
        Some(match bone {
            Jaw => (m.jaw, m.chin),
            Chin => self.surface(m.chin),
            Tongue1 | Tongue2 | Tongue3 => {
                let [base, tip] = m.tongue?;
                let seg = |i: f32| base.lerp(tip, i / 3.0);
                match bone {
                    Tongue1 => (seg(0.0), seg(1.0)),
                    Tongue2 => (seg(1.0), seg(2.0)),
                    _ => (seg(2.0), seg(3.0)),
                }
            }
            TeethTop => self.surface(m.teeth_top?),
            TeethBottom => self.surface(m.teeth_bottom?),
            EyeL => (m.eye, m.eye + Vec3::Z * r.max(2e-3)),
            // Paupières : pivot au centre du globe, queue vers le bord de l'œil.
            // La direction est dans le plan YZ : la rotation de repos est donc
            // un roulis autour de X monde, et une rotation locale autour de X
            // ouvre/ferme la paupière.
            EyelidTopL => (m.eye, m.eye + Vec3::new(0.0, 1.0, 1.1).normalize() * r * 1.3),
            EyelidBottomL => (m.eye, m.eye + Vec3::new(0.0, -1.0, 1.1).normalize() * r * 1.3),
            BrowInL => self.surface(Vec3::new(cx + ex * 0.5, m.brow.y + 0.1 * r, m.brow.z)),
            BrowMidL => self.surface(m.brow),
            BrowOutL => self.surface(Vec3::new(cx + ex * 1.5, m.brow.y - 0.2 * r, m.brow.z)),
            CheekL => self.surface(Vec3::new(
                cx + ex * 1.1,
                (m.eye.y + m.lip_top.y) * 0.5,
                m.mouth_corner.z + 0.3 * r,
            )),
            Nose => self.surface(m.nose_tip),
            NostrilL => self.surface(Vec3::new(
                cx + r.max(2e-3) * 0.6,
                m.nose_tip.y - r * 0.4,
                m.nose_tip.z - r * 0.5,
            )),
            LipTop => self.surface(m.lip_top),
            LipTopL => self.surface(self.lip_side(m.lip_top)),
            LipBottom => self.surface(m.lip_bottom),
            LipBottomL => self.surface(self.lip_side(m.lip_bottom)),
            LipCornerL => self.surface(m.mouth_corner),
            EarL => {
                let tip = m.ear?;
                (Vec3::new(cx + (tip.x - cx) * 0.5, tip.y, tip.z), tip)
            }
            // Côtés droits : miroir du gauche.
            EyeR | EyelidTopR | EyelidBottomR | BrowInR | BrowMidR | BrowOutR | CheekR | NostrilR
            | LipTopR | LipBottomR | LipCornerR | EarR => {
                let (h, t) = self.joint(bone.mirrored()?)?;
                (self.mirror(h), self.mirror(t))
            }
        })
    }

    /// Point de lèvre latéral (à mi-chemin du centre à la commissure gauche).
    fn lip_side(&self, center: Vec3) -> Vec3 {
        let mc = self.markers.mouth_corner;
        Vec3::new(
            self.center_x + (mc.x - self.center_x) * 0.5,
            center.y,
            (center.z + mc.z) * 0.5,
        )
    }
}
