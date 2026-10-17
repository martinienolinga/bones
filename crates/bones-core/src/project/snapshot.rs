//! Instantané d'un projet ouvert : le document et les réglages de l'éditeur.

use std::path::Path;

use super::state::EditorState;
use super::{ProjectError, ProjectInfo, write};
use crate::{Editor, Revisions, Rig};

/// Instantané de tout ce qui constitue un projet ouvert : peu coûteux à
/// capturer (données lourdes partagées par Arc), `Send`, pour être
/// enregistré en tâche de fond.
#[derive(Clone)]
pub struct ProjectSnapshot {
    pub(crate) rig: Rig,
    pub(crate) state: EditorState,
}

// Garantie du contrat : un instantané part sur un fil de fond.
const _: () = {
    const fn send<T: Send + 'static>() {}
    send::<ProjectSnapshot>()
};

impl ProjectSnapshot {
    pub fn capture(editor: &Editor) -> Self {
        Self {
            rig: editor.rig.clone(),
            state: EditorState::capture(editor),
        }
    }

    /// Document capturé.
    pub fn rig(&self) -> &Rig {
        &self.rig
    }

    pub fn info(&self, name: &str, created: u64, modified: u64) -> ProjectInfo {
        let (rig, mesh) = (&self.rig, &*self.rig.mesh);
        ProjectInfo {
            name: name.to_owned(),
            created,
            modified,
            bones: count(rig.skeleton.len()),
            vertices: count(mesh.vertex_count()),
            triangles: count(mesh.triangle_count()),
            frames: rig.clip.frame_count,
            fps: rig.clip.fps,
            textured: mesh.materials.iter().any(|m| m.base_color_texture.is_some()),
        }
    }

    /// Enregistre de façon atomique ; `thumbnail` = image JPEG déjà encodée.
    pub fn save(
        &self,
        path: &Path,
        name: &str,
        created: u64,
        thumbnail: Option<&[u8]>,
    ) -> Result<ProjectInfo, ProjectError> {
        write::save(self, path, name, created, thumbnail)
    }

    /// Éditeur prêt à l'emploi : révisions neuves (les caches GPU renverront
    /// tout), pose et membres IK recalés, historique vide. Si les poids
    /// n'étaient pas à jour, une nouvelle liaison sera demandée.
    pub fn into_editor(self) -> Editor {
        let Self { mut rig, state } = self;
        let stale = rig.needs_bind();
        rig.revisions = Revisions::default();
        if stale {
            rig.touch_skeleton();
        }
        rig.pose.sync_len(&rig.skeleton);
        let mut editor = Editor::new(rig);
        state.apply(&mut editor);
        editor
    }
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}
