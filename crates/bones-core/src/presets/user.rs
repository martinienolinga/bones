//! Bibliothèque de préréglages de l'utilisateur : fichiers `.kb3da` d'un
//! dossier (par exemple `Documents/Presets`). Réutilise les helpers de
//! [`crate::project::store`] (liste, nom unique, renommage, duplication,
//! suppression) communs à tous les formats de l'application.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::error::PresetError;
use super::file::{self, EXTENSION};
use super::model::{Preset, PresetInfo};
use crate::project::store::{self, Entry};

/// Fiche d'un préréglage utilisateur et son fichier (ligne de la galerie).
#[derive(Clone, Debug)]
pub struct Listed {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub info: PresetInfo,
}

/// Préréglages `.kb3da` d'un dossier, le plus récemment modifié d'abord. Les
/// fichiers illisibles sont ignorés (la galerie reste robuste).
pub fn list(dir: &Path) -> Vec<Listed> {
    store::list_ext(dir, EXTENSION)
        .into_iter()
        .filter_map(|Entry { path, modified, .. }| {
            let info = file::read_info(&path).ok()?;
            Some(Listed { path, modified, info })
        })
        .collect()
}

/// Enregistre un préréglage sous un nom de fichier libre dans `dir`
/// (« Nom », « Nom 2 »…). Retourne son chemin.
pub fn save(preset: &Preset, dir: &Path) -> Result<PathBuf, PresetError> {
    let path = store::unique_path_ext(dir, &preset.info.name, EXTENSION);
    file::save(preset, &path)?;
    Ok(path)
}

/// Ouvre un préréglage utilisateur.
pub fn load(path: &Path) -> Result<Preset, PresetError> {
    file::load(path)
}

/// Renomme un fichier de préréglage (nom nettoyé, rendu unique dans le
/// dossier). Retourne le nouveau chemin.
pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf, PresetError> {
    Ok(store::rename(path, new_name)?)
}

/// Duplique un fichier de préréglage sous un nom libre.
pub fn duplicate(path: &Path) -> Result<PathBuf, PresetError> {
    Ok(store::duplicate_file(path)?)
}

/// Supprime un fichier de préréglage (déjà absent : rien à faire).
pub fn delete(path: &Path) -> Result<(), PresetError> {
    Ok(store::delete(path)?)
}
