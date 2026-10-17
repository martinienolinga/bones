//! Réglages de l'application, conservés d'une session à l'autre (fichier
//! JSON dans le dossier des documents).

use std::path::{Path, PathBuf};

use bones_core::{Editor, Mode, PoseTool};
use serde::{Deserialize, Serialize};

use crate::gizmo::GizmoStyle;

const FILE: &str = ".reglages.json";

/// Tâches pour lesquelles seul l'Apple Pencil agit (les doigts naviguent).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PencilPolicy {
    /// IK, rotation et déplacement des os (mode Pose).
    pub pose: bool,
    /// Épingles (mode Pose, outil Épingler).
    pub pins: bool,
    /// Édition du squelette (mode Édition).
    pub skeleton: bool,
    /// Marqueurs d'auto-rig.
    pub markers: bool,
}

impl PencilPolicy {
    /// Libellé et description de chaque tâche, dans l'ordre de [`Self::get_mut`].
    pub const TASKS: [(&'static str, &'static str); 4] = [
        ("Poser", "IK, rotation et déplacement des os"),
        ("Épingler", "poser et retirer les épingles"),
        ("Éditer le squelette", "ajouter, ajuster, supprimer des os"),
        ("Marqueurs d'auto-rig", "placer les marqueurs"),
    ];

    pub fn get_mut(&mut self, index: usize) -> &mut bool {
        match index {
            0 => &mut self.pose,
            1 => &mut self.pins,
            2 => &mut self.skeleton,
            _ => &mut self.markers,
        }
    }

    /// Vrai si l'action en cours de l'éditeur est réservée au crayon.
    pub fn requires_pencil(&self, editor: &Editor) -> bool {
        match editor.mode {
            Mode::Pose if editor.pose_tool == PoseTool::Pin => self.pins,
            Mode::Pose => self.pose,
            Mode::Edit => self.skeleton,
            Mode::AutoRig => self.markers,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub pencil: PencilPolicy,
    /// Rayon de sélection tactile, en points.
    pub pick_radius: f32,
    /// Apparence du guide de rotation 3D.
    pub gizmo: GizmoStyle,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pencil: PencilPolicy::default(),
            pick_radius: 22.0,
            gizmo: GizmoStyle::default(),
        }
    }
}

impl Settings {
    fn path(docs: &Path) -> PathBuf {
        docs.join(FILE)
    }

    /// Réglages enregistrés, ou ceux par défaut.
    pub fn load(docs: &Path) -> Self {
        std::fs::read(Self::path(docs))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, docs: &Path) {
        let result = serde_json::to_vec_pretty(self)
            .map_err(|e| e.to_string())
            .and_then(|bytes| std::fs::write(Self::path(docs), bytes).map_err(|e| e.to_string()));
        if let Err(e) = result {
            log::warn!("réglages non enregistrés : {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bones_core::Rig;

    #[test]
    fn policy_follows_mode_and_tool() {
        let mut editor = Editor::new(Rig::demo());
        let policy = PencilPolicy {
            pins: true,
            ..Default::default()
        };
        assert!(!policy.requires_pencil(&editor));
        editor.pose_tool = PoseTool::Pin;
        assert!(policy.requires_pencil(&editor));
        editor.set_mode(Mode::Edit);
        assert!(!policy.requires_pencil(&editor));
    }

    #[test]
    fn settings_roundtrip() {
        let dir = std::env::temp_dir().join(format!("bones-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut s = Settings::default();
        *s.pencil.get_mut(2) = true;
        s.pick_radius = 30.0;
        s.save(&dir);
        assert_eq!(Settings::load(&dir), s);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
