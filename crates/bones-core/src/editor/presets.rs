//! Préréglages d'animation dans l'éditeur : application au projet (une
//! seule étape d'annulation) et création à partir de l'animation.

use super::Editor;
use crate::presets::apply::{ApplyOptions, ApplyReport, bake_preset, merge};
use crate::presets::create::{CreateOptions, from_animation};
use crate::presets::error::PresetError;
use crate::presets::model::Preset;
use crate::presets::roles::{RoleMap, detect};

impl Editor {
    /// Rôles des os du projet (détection automatique), point de départ de
    /// la correspondance réglable dans l'interface.
    pub fn preset_roles(&self) -> RoleMap {
        detect(&self.rig.skeleton)
    }

    /// Applique un préréglage à l'animation du projet. Rien n'est modifié
    /// en cas d'erreur ; sinon une seule étape d'annulation est créée.
    pub fn apply_preset(&mut self, preset: &Preset, options: &ApplyOptions) -> Result<ApplyReport, PresetError> {
        let skeleton = &self.rig.skeleton;
        if skeleton.is_empty() {
            return Err(PresetError::Incompatible("the model has no skeleton".into()));
        }
        let roles = options
            .roles
            .clone()
            .filter(|r| r.fits(skeleton))
            .unwrap_or_else(|| detect(skeleton));
        let base = self.rig.pose.clone();
        let fps = self.rig.clip.fps;
        let (poses, report) = bake_preset(preset, skeleton, &roles, &base, options, fps)?;
        let mut clip = (*self.rig.clip).clone();
        let skeleton = skeleton.clone();
        let (start, frames) = merge(&mut clip, &skeleton, &roles, poses, options, &base);
        self.checkpoint();
        *self.rig.clip_mut() = clip;
        self.playing = false;
        self.set_frame(start as f32);
        self.status = Some(format!("{}: {frames} frames", preset.info.name));
        Ok(ApplyReport {
            mapped: report.mapped,
            unmapped: report.unmapped,
            warnings: report.warnings,
            start,
            frames,
        })
    }

    /// Crée un préréglage à partir de l'animation du projet (le projet
    /// n'est pas modifié). L'intervalle par défaut couvre tout le clip.
    pub fn create_preset(&self, options: &CreateOptions) -> Result<Preset, PresetError> {
        from_animation(&self.rig.skeleton, &self.rig.clip, options)
    }
}
