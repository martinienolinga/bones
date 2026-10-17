//! Projet ouvert dans l'éditeur : son fichier, son nom, ce qui reste à
//! enregistrer.

use std::path::PathBuf;
use std::time::Instant;

use bones_core::project::ProjectSnapshot;
use bones_core::{BoneId, Editor, Mode, OrbitCamera, ViewOptions};

use super::autosave::Autosave;
use super::tasks::{SaveTask, ThumbnailSource};

/// Point de vue enregistré avec le projet (caméra, mode, affichage, image,
/// sélection). En changer ne compte pas comme une modification
/// (`Editor::edit_count`) et ne déclenche donc pas l'enregistrement
/// automatique, mais il est enregistré en quittant le projet (et à la mise
/// en arrière-plan), avec une vignette à jour.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ViewKey {
    camera: OrbitCamera,
    mode: Mode,
    view: ViewOptions,
    frame: f32,
    selected: Option<BoneId>,
}

impl ViewKey {
    fn of(editor: &Editor) -> Self {
        Self {
            camera: editor.camera,
            mode: editor.mode,
            view: editor.view,
            frame: editor.frame,
            selected: editor.selected,
        }
    }
}

#[derive(Clone, Debug)]
pub struct OpenProject {
    pub path: PathBuf,
    pub name: String,
    /// Date de création (secondes Unix).
    pub created: u64,
    pub autosave: Autosave,
    /// Le fichier n'a pas encore de vignette (projet tout juste créé).
    pub needs_thumbnail: bool,
    /// Renommage en cours : le chemin va changer, rien n'est enregistré
    /// d'ici là.
    pub renaming: bool,
    view: ViewKey,
}

impl OpenProject {
    pub fn new(path: PathBuf, name: String, created: u64, editor: &Editor, has_thumbnail: bool) -> Self {
        Self {
            path,
            name,
            created,
            autosave: Autosave::new(editor.edit_count()),
            needs_thumbnail: !has_thumbnail,
            renaming: false,
            view: ViewKey::of(editor),
        }
    }

    /// Enregistrement automatique à lancer maintenant (modifications au
    /// repos depuis 2 s, ou vignette manquante).
    pub fn autosave_due(&mut self, editor: &Editor, pointer_down: bool, now: Instant) -> bool {
        if self.renaming {
            return false;
        }
        let due = self.autosave.due(editor.edit_count(), pointer_down, now);
        due || (self.needs_thumbnail && !pointer_down)
    }

    /// Quelque chose reste à enregistrer en quittant le projet.
    pub fn needs_save(&self, editor: &Editor) -> bool {
        self.autosave.is_dirty(editor.edit_count())
            || self.needs_thumbnail
            || self.view != ViewKey::of(editor)
    }

    /// Prépare l'enregistrement de l'état courant et le considère comme fait
    /// (les échecs sont signalés à part).
    pub fn save_task(&mut self, editor: &Editor, thumbnail: Option<ThumbnailSource>) -> SaveTask {
        let edits = editor.edit_count();
        self.autosave.saved(edits);
        self.view = ViewKey::of(editor);
        if thumbnail.is_some() {
            self.needs_thumbnail = false;
        }
        SaveTask {
            path: self.path.clone(),
            name: self.name.clone(),
            created: self.created,
            snapshot: ProjectSnapshot::capture(editor),
            thumbnail,
        }
    }
}
