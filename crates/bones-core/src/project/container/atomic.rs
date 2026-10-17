//! Écriture atomique : le fichier est écrit à côté de la cible sous un nom
//! temporaire, synchronisé sur le disque, puis renommé par-dessus la cible.
//! Un échec (disque plein, application tuée…) ne laisse jamais un fichier
//! existant à moitié écrit.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::project::ProjectError;
use crate::project::error::invalid;

/// Suffixe des fichiers temporaires (ignorés par les listes de fichiers).
const TEMP_SUFFIX: &str = ".tmp";

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

/// Écrit `path` via `write`, qui reçoit le fichier temporaire et doit le
/// synchroniser (`sync_all`) avant de rendre la main.
pub(crate) fn write_atomically(
    path: &Path,
    write: impl FnOnce(File) -> Result<(), ProjectError>,
) -> Result<(), ProjectError> {
    let tmp = temp_path(path)?;
    let file = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
    let result = write(file).and_then(|()| fs::rename(&tmp, path).map_err(ProjectError::from));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
        return result;
    }
    sync_dir(path);
    Ok(())
}

/// Nom temporaire caché, unique, dans le dossier de la cible (le renommage
/// final reste ainsi sur le même volume, donc atomique).
fn temp_path(path: &Path) -> Result<PathBuf, ProjectError> {
    let name = path
        .file_name()
        .ok_or_else(|| invalid("missing file name"))?
        .to_string_lossy();
    let n = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    let tmp = format!(".{name}.{}-{n}{TEMP_SUFFIX}", std::process::id());
    Ok(path.with_file_name(tmp))
}

/// Rend le renommage durable (entrée du dossier écrite sur le disque).
/// Sans effet là où l'on ne peut pas ouvrir un dossier.
fn sync_dir(path: &Path) {
    #[cfg(unix)]
    if let Some(dir) = path.parent() {
        let dir = if dir.as_os_str().is_empty() {
            Path::new(".")
        } else {
            dir
        };
        if let Ok(d) = File::open(dir) {
            let _ = d.sync_all();
        }
    }
    #[cfg(not(unix))]
    let _ = path;
}
