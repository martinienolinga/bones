//! Actions faciales nommées (unités d'action à la FACS, simplifiées), qui
//! composent les expressions de la bibliothèque. Amplitudes `k` ~ [0, 1].
//!
//! Décalages en millimètres sur une tête de référence (voir
//! [`Expr::shift`]) ; rotations en radians (conventions dans `builder`).

use super::builder::Expr;
use crate::autorig::face::FaceBone;

impl Expr {
    // --- Sourcils ---
    pub fn brows_up(&mut self, k: f32) {
        for (b, up) in [
            (FaceBone::BrowInL, 16.0),
            (FaceBone::BrowMidL, 18.0),
            (FaceBone::BrowOutL, 13.0),
        ] {
            self.pair_rot(b, -0.30 * k, 0.0, 0.0);
            self.pair_shift(b, 0.0, 1.0 * k, up * k);
        }
    }
    pub fn brows_down(&mut self, k: f32) {
        self.pair_rot(FaceBone::BrowInL, 0.28 * k, 0.0, 0.12 * k);
        self.pair_rot(FaceBone::BrowMidL, 0.20 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::BrowOutL, 0.10 * k, 0.0, 0.0);
        self.pair_shift(FaceBone::BrowInL, -7.0 * k, 3.0 * k, -13.0 * k);
        self.pair_shift(FaceBone::BrowMidL, -3.0 * k, 1.5 * k, -9.0 * k);
        self.pair_shift(FaceBone::BrowOutL, 0.0, 0.0, -4.0 * k);
    }
    /// Sourcils obliques (intérieur relevé) : tristesse, peur.
    pub fn brows_inner_up(&mut self, k: f32) {
        self.pair_rot(FaceBone::BrowInL, -0.30 * k, 0.0, 0.10 * k);
        self.pair_rot(FaceBone::BrowMidL, -0.08 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::BrowOutL, 0.10 * k, 0.0, 0.0);
        self.pair_shift(FaceBone::BrowInL, -4.0 * k, 1.5 * k, 16.0 * k);
        self.pair_shift(FaceBone::BrowMidL, 0.0, 0.0, 5.0 * k);
        self.pair_shift(FaceBone::BrowOutL, 0.0, 0.0, -4.0 * k);
    }

    // --- Paupières (pivot au centre du globe : la rotation suffit) ---
    pub fn lids_widen(&mut self, k: f32) {
        self.pair_rot(FaceBone::EyelidTopL, -0.40 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::EyelidBottomL, 0.30 * k, 0.0, 0.0);
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
        self.pair_shift(FaceBone::LipCornerL, 10.0 * k, -4.0 * k, 12.0 * k);
        self.pair_shift(FaceBone::LipTopL, 1.5 * k, 0.0, 2.5 * k);
        self.pair_shift(FaceBone::LipBottomL, 1.5 * k, 0.0, 2.0 * k);
        self.cheeks_raise(0.6 * k);
    }
    pub fn frown_mouth(&mut self, k: f32) {
        self.pair_rot(FaceBone::LipCornerL, 0.32 * k, 0.0, 0.12 * k);
        self.pair_rot(FaceBone::LipBottomL, 0.10 * k, 0.0, 0.0);
        self.pair_shift(FaceBone::LipCornerL, 2.0 * k, -k, -12.0 * k);
        self.pair_shift(FaceBone::LipBottomL, 0.0, 1.5 * k, -2.5 * k);
        self.shift(FaceBone::LipBottom, 0.0, 2.5 * k, 1.5 * k);
    }
    /// Un seul coin relevé (sourire en coin, mépris) ; `left` choisit le côté.
    pub fn corner_up(&mut self, left: bool, k: f32) {
        let (c, out, side) = if left {
            (FaceBone::LipCornerL, -0.25 * k, 1.0)
        } else {
            (FaceBone::LipCornerR, 0.25 * k, -1.0)
        };
        self.rot(c, -0.38 * k, 0.0, out);
        self.shift(c, side * 8.0 * k, -4.0 * k, 13.0 * k);
    }
    pub fn lips_part(&mut self, k: f32) {
        self.rot(FaceBone::LipTop, -0.15 * k, 0.0, 0.0);
        self.rot(FaceBone::LipBottom, 0.15 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::LipTopL, -0.10 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::LipBottomL, 0.10 * k, 0.0, 0.0);
        self.shift(FaceBone::LipTop, 0.0, 0.5 * k, 5.0 * k);
        self.shift(FaceBone::LipBottom, 0.0, 0.5 * k, -5.0 * k);
        self.pair_shift(FaceBone::LipTopL, 0.0, 0.0, 3.5 * k);
        self.pair_shift(FaceBone::LipBottomL, 0.0, 0.0, -3.5 * k);
    }
    /// Bouche en avant et resserrée (bisou, « ou »).
    pub fn pucker(&mut self, k: f32) {
        self.pair_rot(FaceBone::LipCornerL, 0.0, 0.0, 0.45 * k);
        self.rot(FaceBone::LipTop, -0.10 * k, 0.0, 0.0);
        self.rot(FaceBone::LipBottom, 0.10 * k, 0.0, 0.0);
        self.pair_shift(FaceBone::LipCornerL, -13.0 * k, 6.0 * k, 0.0);
        self.shift(FaceBone::LipTop, 0.0, 10.0 * k, -0.5 * k);
        self.shift(FaceBone::LipBottom, 0.0, 10.0 * k, 0.5 * k);
        self.pair_shift(FaceBone::LipTopL, -4.0 * k, 7.0 * k, 0.0);
        self.pair_shift(FaceBone::LipBottomL, -4.0 * k, 7.0 * k, 0.0);
    }
    /// Lèvres étirées sur les côtés (« i », « e »).
    pub fn widen_mouth(&mut self, k: f32) {
        self.pair_rot(FaceBone::LipCornerL, 0.0, 0.0, -0.30 * k);
        self.pair_shift(FaceBone::LipCornerL, 12.0 * k, -4.0 * k, 0.0);
        self.pair_shift(FaceBone::LipTopL, 3.0 * k, -1.5 * k, 0.0);
        self.pair_shift(FaceBone::LipBottomL, 3.0 * k, -1.5 * k, 0.0);
    }
    /// Lèvre inférieure remontée contre les dents du haut (« f », « v »).
    pub fn lower_lip_up(&mut self, k: f32) {
        self.rot(FaceBone::LipBottom, -0.22 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::LipBottomL, -0.18 * k, 0.0, 0.0);
        self.rot(FaceBone::LipTop, -0.08 * k, 0.0, 0.0);
        self.shift(FaceBone::LipBottom, 0.0, -3.5 * k, 7.0 * k);
        self.pair_shift(FaceBone::LipBottomL, 0.0, -3.0 * k, 5.5 * k);
        self.shift(FaceBone::LipTop, 0.0, 0.5 * k, 2.0 * k);
    }
    /// Lèvres pincées l'une contre l'autre (« m », « b », « p »).
    pub fn lips_press(&mut self, k: f32) {
        self.rot(FaceBone::LipTop, 0.10 * k, 0.0, 0.0);
        self.rot(FaceBone::LipBottom, -0.10 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::LipTopL, 0.06 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::LipBottomL, -0.06 * k, 0.0, 0.0);
        self.shift(FaceBone::LipTop, 0.0, -2.5 * k, -2.5 * k);
        self.shift(FaceBone::LipBottom, 0.0, -2.5 * k, 3.0 * k);
        self.pair_shift(FaceBone::LipTopL, 0.0, -1.5 * k, -1.5 * k);
        self.pair_shift(FaceBone::LipBottomL, 0.0, -1.5 * k, 2.5 * k);
        self.pair_shift(FaceBone::LipCornerL, -2.5 * k, 0.0, 0.0);
    }

    // --- Nez, joues ---
    pub fn nose_wrinkle(&mut self, k: f32) {
        self.rot(FaceBone::Nose, -0.22 * k, 0.0, 0.0);
        self.pair_rot(FaceBone::NostrilL, -0.25 * k, 0.0, 0.0);
        self.shift(FaceBone::Nose, 0.0, 0.0, 3.0 * k);
        self.pair_shift(FaceBone::NostrilL, 2.0 * k, 0.0, 7.0 * k);
        // Releveur de la lèvre supérieure (AU 9-10).
        self.shift(FaceBone::LipTop, 0.0, 0.5 * k, 4.0 * k);
        self.pair_shift(FaceBone::LipTopL, 0.0, 0.5 * k, 4.5 * k);
    }
    pub fn nostrils_flare(&mut self, k: f32) {
        self.pair_rot(FaceBone::NostrilL, -0.10 * k, 0.0, 0.0);
        self.pair_shift(FaceBone::NostrilL, 6.0 * k, 0.0, 1.5 * k);
    }
    pub fn cheeks_raise(&mut self, k: f32) {
        self.pair_rot(FaceBone::CheekL, -0.30 * k, 0.0, 0.0);
        self.pair_shift(FaceBone::CheekL, 1.5 * k, 3.0 * k, 9.0 * k);
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
