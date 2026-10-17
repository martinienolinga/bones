//! Petit constructeur d'expressions et actions faciales nommées, pour écrire
//! la bibliothèque comme des données lisibles. Amplitudes `k` ~ [0, 1].
//!
//! Repère local des os de surface au repos (contrat [`crate::autorig::face`]) :
//! X local = +X monde, Y local = +Z monde (vers l'avant), Z local = −Y monde
//! (vers le bas). D'où les signes retenus :
//! - rotation autour de X local : `+` abaisse la pointe (peau vers le bas),
//!   `−` la relève ;
//! - rotation autour de Z local : `+` pousse la pointe vers −X monde (vers
//!   l'intérieur pour un os gauche), `−` vers +X monde (vers l'extérieur).
//!
//! Les paires `.l`/`.r` reçoivent le reflet du delta gauche (quaternion
//! `(w, x, −y, −z)`, translation X opposée), sauf les oreilles dont les bases
//! de repos sont déjà symétriques : elles reçoivent le même delta local.

use glam::{EulerRot, Quat, Vec3};

use super::data::{ExpressionData, compose};
use crate::Transform;
use crate::autorig::face::FaceBone;

pub(super) struct Expr {
    bones: Vec<(String, Transform)>,
}

fn is_ear(b: FaceBone) -> bool {
    matches!(b, FaceBone::EarL | FaceBone::EarR)
}

impl Expr {
    pub fn new() -> Self {
        Self { bones: Vec::new() }
    }

    /// Cumule un delta sur un os (compose si l'os est déjà posé).
    fn put(&mut self, b: FaceBone, t: Transform) {
        match self.bones.iter_mut().find(|(n, _)| n == b.name()) {
            Some(e) => e.1 = compose(&e.1, &t),
            None => self.bones.push((b.name().to_string(), t)),
        }
    }

    fn rot(&mut self, b: FaceBone, x: f32, y: f32, z: f32) {
        let q = Quat::from_euler(EulerRot::XYZ, x, y, z);
        self.put(b, Transform::from_rotation_translation(q, Vec3::ZERO));
    }

    fn tr(&mut self, b: FaceBone, t: Vec3) {
        self.put(b, Transform::from_translation(t));
    }

    /// Même rotation des deux côtés (reflet, sauf oreilles : delta identique).
    fn pair_rot(&mut self, left: FaceBone, x: f32, y: f32, z: f32) {
        self.rot(left, x, y, z);
        if let Some(r) = left.mirrored() {
            let q = Quat::from_euler(EulerRot::XYZ, x, y, z);
            let q = if is_ear(left) {
                q
            } else {
                Quat::from_xyzw(q.x, -q.y, -q.z, q.w)
            };
            self.put(r, Transform::from_rotation_translation(q, Vec3::ZERO));
        }
    }

    fn pair_tr(&mut self, left: FaceBone, t: Vec3) {
        self.tr(left, t);
        if let Some(r) = left.mirrored() {
            let t = if is_ear(left) {
                t
            } else {
                Vec3::new(-t.x, t.y, t.z)
            };
            self.put(r, Transform::from_translation(t));
        }
    }

    pub fn finish(self) -> ExpressionData {
        ExpressionData { bones: self.bones }
    }

    // --- Sourcils ---
    pub fn brows_up(&mut self, k: f32) {
        for b in [FaceBone::BrowInL, FaceBone::BrowMidL, FaceBone::BrowOutL] {
            self.pair_rot(b, -0.30 * k, 0.0, 0.0);
        }
    }
    pub fn brows_down(&mut self, k: f32) {
        self.pair_rot(FaceBone::BrowInL, 0.28 * k, 0.0, 0.12 * k);
        self.pair_rot(FaceBone::BrowMidL, 0.20 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::BrowOutL, 0.10 * k, 0.0, 0.0);
    }
    /// Sourcils obliques (intérieur relevé) : tristesse, peur.
    pub fn brows_inner_up(&mut self, k: f32) {
        self.pair_rot(FaceBone::BrowInL, -0.30 * k, 0.0, 0.10 * k);
        self.pair_rot(FaceBone::BrowMidL, -0.08 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::BrowOutL, 0.10 * k, 0.0, 0.0);
    }

    // --- Paupières ---
    pub fn lids_widen(&mut self, k: f32) {
        self.pair_rot(FaceBone::EyelidTopL, -0.35 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::EyelidBottomL, 0.25 * k, 0.0, 0.0);
    }
    pub fn lids_close(&mut self, k: f32) {
        self.pair_rot(FaceBone::EyelidTopL, 0.85 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::EyelidBottomL, -0.25 * k, 0.0, 0.0);
    }
    /// Ferme une seule paupière (clin d'œil) ; `left` choisit le côté.
    pub fn lid_close_one(&mut self, left: bool, k: f32) {
        let (top, bot) = if left {
            (FaceBone::EyelidTopL, FaceBone::EyelidBottomL)
        } else {
            (FaceBone::EyelidTopR, FaceBone::EyelidBottomR)
        };
        self.rot(top, 0.85 * k, 0.0, 0.0);
        self.rot(bot, -0.25 * k, 0.0, 0.0);
    }
    pub fn squint(&mut self, k: f32) {
        self.pair_rot(FaceBone::EyelidBottomL, -0.40 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::EyelidTopL, 0.20 * k, 0.0, 0.0);
    }

    // --- Bouche et mâchoire ---
    pub fn jaw_open(&mut self, k: f32) {
        self.rot(FaceBone::Jaw, 0.35 * k, 0.0, 0.0);
    }
    pub fn smile(&mut self, k: f32) {
        self.pair_rot(FaceBone::LipCornerL, -0.35 * k, 0.0, -0.28 * k);
        self.pair_rot(FaceBone::LipTopL, -0.12 * k, 0.0, 0.0);
        self.cheeks_raise(0.6 * k);
    }
    pub fn frown_mouth(&mut self, k: f32) {
        self.pair_rot(FaceBone::LipCornerL, 0.32 * k, 0.0, 0.12 * k);
        self.pair_rot(FaceBone::LipBottomL, 0.10 * k, 0.0, 0.0);
    }
    /// Un seul coin relevé (sourire en coin, mépris) ; `left` choisit le côté.
    pub fn corner_up(&mut self, left: bool, k: f32) {
        let (c, out) = if left {
            (FaceBone::LipCornerL, -0.25 * k)
        } else {
            (FaceBone::LipCornerR, 0.25 * k)
        };
        self.rot(c, -0.38 * k, 0.0, out);
    }
    pub fn lips_part(&mut self, k: f32) {
        self.rot(FaceBone::LipTop, -0.15 * k, 0.0, 0.0);
        self.rot(FaceBone::LipBottom, 0.15 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::LipTopL, -0.10 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::LipBottomL, 0.10 * k, 0.0, 0.0);
    }
    /// Bouche en avant et resserrée (bisou, « ou »).
    pub fn pucker(&mut self, k: f32) {
        self.pair_rot(FaceBone::LipCornerL, 0.0, 0.0, 0.45 * k);
        self.pair_tr(FaceBone::LipCornerL, Vec3::new(-0.002 * k, 0.003 * k, 0.0));
        self.tr(FaceBone::LipTop, Vec3::new(0.0, 0.004 * k, 0.0));
        self.tr(FaceBone::LipBottom, Vec3::new(0.0, 0.004 * k, 0.0));
        self.rot(FaceBone::LipTop, -0.10 * k, 0.0, 0.0);
        self.rot(FaceBone::LipBottom, 0.10 * k, 0.0, 0.0);
    }
    /// Lèvres étirées sur les côtés (« i », « e »).
    pub fn widen_mouth(&mut self, k: f32) {
        self.pair_rot(FaceBone::LipCornerL, 0.0, 0.0, -0.30 * k);
    }
    /// Lèvre inférieure remontée contre les dents du haut (« f », « v »).
    pub fn lower_lip_up(&mut self, k: f32) {
        self.rot(FaceBone::LipBottom, -0.22 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::LipBottomL, -0.18 * k, 0.0, 0.0);
        self.rot(FaceBone::LipTop, -0.08 * k, 0.0, 0.0);
    }
    /// Lèvres pincées l'une contre l'autre (« m », « b », « p »).
    pub fn lips_press(&mut self, k: f32) {
        self.rot(FaceBone::LipTop, 0.10 * k, 0.0, 0.0);
        self.rot(FaceBone::LipBottom, -0.10 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::LipTopL, 0.06 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::LipBottomL, -0.06 * k, 0.0, 0.0);
    }

    // --- Nez, joues ---
    pub fn nose_wrinkle(&mut self, k: f32) {
        self.rot(FaceBone::Nose, -0.22 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::NostrilL, -0.25 * k, 0.0, 0.0);
    }
    pub fn nostrils_flare(&mut self, k: f32) {
        self.pair_tr(FaceBone::NostrilL, Vec3::new(0.003 * k, 0.0, 0.0));
        self.pair_rot(FaceBone::NostrilL, -0.10 * k, 0.0, 0.0);
    }
    pub fn cheeks_raise(&mut self, k: f32) {
        self.pair_rot(FaceBone::CheekL, -0.30 * k, 0.0, 0.0);
    }

    // --- Langue (ignorée si absente) ---
    pub fn tongue_out(&mut self, k: f32) {
        self.rot(FaceBone::Tongue1, 0.25 * k, 0.0, 0.0);
        self.rot(FaceBone::Tongue2, 0.22 * k, 0.0, 0.0);
    }
    pub fn tongue_up(&mut self, k: f32) {
        self.rot(FaceBone::Tongue1, -0.30 * k, 0.0, 0.0);
        self.rot(FaceBone::Tongue2, -0.18 * k, 0.0, 0.0);
    }

    // --- Oreilles (animaux) ---
    pub fn ears_back(&mut self, k: f32) {
        self.pair_rot(FaceBone::EarL, -0.30 * k, 0.40 * k, 0.0);
    }
    pub fn ears_up(&mut self, k: f32) {
        self.pair_rot(FaceBone::EarL, 0.30 * k, -0.20 * k, 0.0);
    }
}
