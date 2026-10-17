//! Erreurs d'import et d'export.

use super::Format;

#[derive(Debug)]
pub enum IoError {
    Io(std::io::Error),
    /// Fichier mal formé.
    Parse {
        format: Format,
        message: String,
    },
    /// Extension inconnue ou opération non prise en charge.
    Unsupported(String),
}

impl IoError {
    pub(crate) fn parse(format: Format, message: impl std::fmt::Display) -> Self {
        IoError::Parse {
            format,
            message: message.to_string(),
        }
    }
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::Io(e) => write!(f, "erreur d'entrée/sortie : {e}"),
            IoError::Parse { format, message } => {
                write!(
                    f,
                    "fichier {} invalide : {message}",
                    format.extension().to_uppercase()
                )
            }
            IoError::Unsupported(what) => write!(f, "non pris en charge : {what}"),
        }
    }
}

impl std::error::Error for IoError {}

impl From<std::io::Error> for IoError {
    fn from(e: std::io::Error) -> Self {
        IoError::Io(e)
    }
}
