//! Projets de l'utilisateur, un fichier `.kbrig` compressé par projet
//! (format de `bones_core::project`) dans `Documents/Projects`.
//!
//! - [`current`] : projet ouvert dans l'éditeur, ce qui reste à
//!   enregistrer ;
//! - [`autosave`] : enregistrement automatique après 2 s de repos ;
//! - [`worker`] : fil dédié aux accès disque, travaux exécutés dans l'ordre
//!   et enregistrements regroupés ;
//! - [`tasks`] : travaux (enregistrer, ouvrir, créer, renommer…) et
//!   résultats ;
//! - [`templates`] : nouveaux projets (mannequin, projet vide).
//!
//! L'intégration à l'application (écrans, vignettes GPU) est dans
//! `crate::session`.

pub mod autosave;
pub mod current;
pub mod tasks;
pub mod templates;
pub mod worker;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use current::OpenProject;
pub use tasks::{Dirs, Event, Rgba, Task, ThumbnailSource};
pub use templates::Template;
pub use worker::{Waker, Worker};

/// Sous-dossier des projets dans Documents.
pub const FOLDER: &str = "Projects";

/// Dossiers de travail pour le dossier Documents `docs`.
pub fn dirs(docs: &Path) -> Dirs {
    Dirs {
        projects: projects_dir(docs),
        documents: docs.to_owned(),
    }
}

pub fn projects_dir(docs: &Path) -> PathBuf {
    docs.join(FOLDER)
}
