//! Erreurs de lecture et d'écriture des projets (messages en anglais, affichés
//! tels quels par l'application).

use std::fmt;

#[derive(Debug)]
pub enum ProjectError {
    Io(std::io::Error),
    /// Conteneur ZIP illisible (fichier tronqué, corrompu…).
    Archive(zip::result::ZipError),
    Json(serde_json::Error),
    /// Ce n'est pas un projet, ou son contenu est incohérent.
    Invalid(String),
    /// Projet enregistré par une version plus récente de l'application.
    TooNew {
        version: u32,
        supported: u32,
    },
    /// Taille décompressée excessive (protection contre les « bombes »).
    TooLarge(String),
    /// Vignette impossible à encoder ou à décoder.
    Image(String),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::Io(e) => write!(f, "input/output error: {e}"),
            ProjectError::Archive(e) => write!(f, "damaged project file: {e}"),
            ProjectError::Json(e) => write!(f, "damaged project file: {e}"),
            ProjectError::Invalid(e) => write!(f, "invalid project file: {e}"),
            ProjectError::TooNew { version, supported } => write!(
                f,
                "this project was saved by a newer version of the app (format {version}, \
                 this version reads up to {supported}); please update the app"
            ),
            ProjectError::TooLarge(e) => write!(f, "project file too large: {e}"),
            ProjectError::Image(e) => write!(f, "thumbnail error: {e}"),
        }
    }
}

impl std::error::Error for ProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProjectError::Io(e) => Some(e),
            ProjectError::Archive(e) => Some(e),
            ProjectError::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ProjectError {
    fn from(e: std::io::Error) -> Self {
        ProjectError::Io(e)
    }
}

impl From<zip::result::ZipError> for ProjectError {
    fn from(e: zip::result::ZipError) -> Self {
        match e {
            zip::result::ZipError::Io(e) => ProjectError::Io(e),
            e => ProjectError::Archive(e),
        }
    }
}

impl From<serde_json::Error> for ProjectError {
    fn from(e: serde_json::Error) -> Self {
        ProjectError::Json(e)
    }
}

/// Raccourci pour une erreur de contenu.
pub(super) fn invalid(message: impl Into<String>) -> ProjectError {
    ProjectError::Invalid(message.into())
}
