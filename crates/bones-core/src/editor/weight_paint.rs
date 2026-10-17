//! Mode « Weight Paint » : traits de pinceau, outils du menu « Weights » et
//! protection des poids peints.

use glam::Vec2;

use super::Editor;
use crate::{BoneId, Viewport, WeightsTool};

impl Editor {
    /// Os dont on peint les poids : l'os sélectionné.
    pub fn paint_bone(&self) -> Option<BoneId> {
        self.selected.filter(|b| self.rig.skeleton.contains(*b))
    }

    /// Pression du crayon (0 à 1 ; 1 au doigt ou à la souris), appliquée à la
    /// force et au rayon selon les réglages du pinceau.
    pub fn set_pointer_pressure(&mut self, pressure: f32) {
        self.paint.pressure = if pressure.is_finite() { pressure.clamp(0.0, 1.0) } else { 1.0 };
    }

    /// Position écran et rayon (pixels) du pinceau pendant un trait.
    pub fn brush_cursor(&self) -> Option<(Vec2, f32)> {
        None
    }

    /// Outil du menu « Weights » (une étape d'annulation).
    pub fn apply_weights_tool(&mut self, tool: WeightsTool) {
        let _ = tool;
    }

    /// Vrai si les poids ont été peints (ou retouchés par un outil) : ils ne
    /// sont plus recalculés automatiquement.
    pub fn weights_painted(&self) -> bool {
        false
    }

    /// Début de geste du mode « Weight Paint ».
    pub(super) fn paint_down(&mut self, pos: Vec2, vp: &Viewport, radius: f32) -> bool {
        if let Some(bone) = self.pick(pos, vp, radius) {
            self.selected = Some(bone);
        }
        false
    }
}
