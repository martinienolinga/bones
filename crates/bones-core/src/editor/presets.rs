//! Préréglages d'animation dans l'éditeur : application au projet (une
//! seule étape d'annulation) et création à partir de l'animation.
//!
//! La bibliothèque de l'utilisateur (liste, enregistrement, renommage,
//! duplication, suppression des fichiers `.kb3da`), l'aperçu animé et
//! l'import/export de fichiers sont dans le module [`crate::presets`]
//! (fonctions sans état, testables seules) : voir [`crate::presets::user`],
//! [`crate::presets::Preview`], [`crate::presets::import_file`] et
//! [`crate::presets::export_file`].

use super::Editor;
use crate::presets::apply::{ApplyOptions, ApplyReport, bake_preset, merge};
use crate::presets::create::{CreateOptions, from_animation};
use crate::presets::error::{PresetError, incompatible};
use crate::presets::expression::{self, ExpressionReport};
use crate::presets::model::{Preset, PresetKind};
use crate::presets::roles::{RoleMap, detect};

impl Editor {
    /// Rôles des os du projet (détection automatique), point de départ de
    /// la correspondance réglable dans l'interface.
    pub fn preset_roles(&self) -> RoleMap {
        detect(&self.rig.skeleton)
    }

    /// Applique un préréglage à l'animation du projet. Rien n'est modifié
    /// en cas d'erreur ; sinon une seule étape d'annulation est créée.
    pub fn apply_preset(
        &mut self,
        preset: &Preset,
        options: &ApplyOptions,
    ) -> Result<ApplyReport, PresetError> {
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

    /// Applique une expression faciale au visage du personnage (une seule
    /// étape d'annulation). Ne touche que les os du visage présents ; le reste
    /// de la pose (le corps) est inchangé.
    ///
    /// - `intensity` : 0 = repos, 1 = pleine amplitude, > 1 exagère ;
    /// - `mirror` : échange gauche ↔ droite ;
    /// - `additive` : superpose à la pose actuelle du visage (sinon la
    ///   remplace, les autres os du visage revenant au repos) ;
    /// - `as_key` : écrit une clé à l'image courante (sinon met seulement à
    ///   jour la pose, et la clé déjà présente le cas échéant).
    pub fn apply_expression(
        &mut self,
        preset: &Preset,
        intensity: f32,
        mirror: bool,
        additive: bool,
        as_key: bool,
    ) -> Result<ExpressionReport, PresetError> {
        let PresetKind::Expression(data) = &preset.kind else {
            return Err(incompatible("not a facial expression"));
        };
        let present = expression::present_mapped(&self.rig.skeleton);
        if present.is_empty() {
            return Err(incompatible("the model has no face bones"));
        }
        let (face, report) = expression::face_pose(data, &self.rig.skeleton, intensity, mirror);
        self.checkpoint();
        for (_, id) in &present {
            let i = id.index();
            let delta = face.get(*id);
            self.rig.pose.bones[i] = if additive {
                self.rig.pose.bones[i].mul(&delta)
            } else {
                delta
            };
        }
        if as_key {
            let frame = self.current_frame();
            let pose = self.rig.pose.clone();
            self.rig.clip_mut().set_key(frame, pose);
            self.frame = frame as f32;
        } else {
            self.store_key_if_any();
        }
        self.status = Some(format!(
            "{}: {} face bones",
            preset.info.name,
            report.applied.len()
        ));
        Ok(report)
    }
}
