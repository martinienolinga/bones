//! Erreurs des préréglages (messages en anglais, affichés tels quels).

use std::fmt;

use crate::IoError;
use crate::project::ProjectError;

#[derive(Debug)]
pub enum PresetError {
    Io(std::io::Error),
    /// Conteneur illisible, trop gros, etc. (fichier `.kb3da`).
    File(ProjectError),
    Json(serde_json::Error),
    /// Ce n'est pas un préréglage, ou son contenu est incohérent.
    Invalid(String),
    /// Préréglage enregistré par une version plus récente de l'application.
    TooNew {
        version: u32,
        supported: u32,
    },
    /// Le squelette ne convient pas (os nécessaires introuvables).
    Incompatible(String),
    /// Import ou export dans un autre format.
    Exchange(IoError),
    /// Rien à enregistrer (pas d'animation, intervalle vide…).
    Empty(String),
}

impl fmt::Display for PresetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PresetError::Io(e) => write!(f, "input/output error: {e}"),
            PresetError::File(e) => write!(f, "{}", e.to_string().replace("project", "preset")),
            PresetError::Json(e) => write!(f, "damaged preset file: {e}"),
            PresetError::Invalid(e) => write!(f, "invalid preset: {e}"),
            PresetError::TooNew { version, supported } => write!(
                f,
                "this preset was saved by a newer version of the app (format {version}, \
                 this version reads up to {supported}); please update the app"
            ),
            PresetError::Incompatible(e) => write!(f, "this preset does not fit the skeleton: {e}"),
            PresetError::Exchange(e) => write!(f, "{e}"),
            PresetError::Empty(e) => write!(f, "nothing to save: {e}"),
        }
    }
}

impl std::error::Error for PresetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PresetError::Io(e) => Some(e),
            PresetError::File(e) => Some(e),
            PresetError::Json(e) => Some(e),
            PresetError::Exchange(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for PresetError {
    fn from(e: std::io::Error) -> Self {
        PresetError::Io(e)
    }
}

impl From<ProjectError> for PresetError {
    fn from(e: ProjectError) -> Self {
        match e {
            ProjectError::Io(e) => PresetError::Io(e),
            ProjectError::Json(e) => PresetError::Json(e),
            e => PresetError::File(e),
        }
    }
}

impl From<serde_json::Error> for PresetError {
    fn from(e: serde_json::Error) -> Self {
        PresetError::Json(e)
    }
}

impl From<IoError> for PresetError {
    fn from(e: IoError) -> Self {
        match e {
            IoError::Io(e) => PresetError::Io(e),
            e => PresetError::Exchange(e),
        }
    }
}

pub(crate) fn incompatible(what: impl Into<String>) -> PresetError {
    PresetError::Incompatible(what.into())
}

pub(crate) fn invalid(what: impl Into<String>) -> PresetError {
    PresetError::Invalid(what.into())
}
