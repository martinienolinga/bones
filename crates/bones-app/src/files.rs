//! Emplacement des documents et liste des fichiers.
//!
//! Sur iPad, le dossier Documents de l'app est visible dans l'app Fichiers
//! (UIFileSharingEnabled) : l'utilisateur y dépose ses modèles (glTF, FBX,
//! OBJ…) et retrouve ses documents `.bones` et ses exports.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bones_core::io::Format;

/// Sous-dossier des exports, pour garder la galerie lisible.
pub const EXPORTS: &str = "Exports";

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub modified: SystemTime,
    pub format: Format,
}

/// Dossier des documents (créé au besoin).
pub fn documents_dir() -> PathBuf {
    let dir = if let Some(custom) = std::env::var_os("BONES_DOCUMENTS") {
        PathBuf::from(custom)
    } else if cfg!(target_os = "ios") {
        home().join("Documents")
    } else {
        home().join("Documents").join("PaintSharp Bones")
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("impossible de créer {}: {e}", dir.display());
    }
    dir
}

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Fichiers dont le format satisfait `keep`, du plus récent au plus ancien.
pub fn list(dir: &Path, keep: impl Fn(Format) -> bool) -> Vec<FileEntry> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<FileEntry> = read
        .filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            let format = Format::from_path(&path).filter(|f| keep(*f))?;
            let meta = e.metadata().ok()?;
            Some(FileEntry {
                name: path.file_stem()?.to_string_lossy().into_owned(),
                modified: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                path,
                format,
            })
        })
        .collect();
    files.sort_by_key(|f| std::cmp::Reverse(f.modified));
    files
}

/// Chemin libre `nom.ext` (ou `nom 2.ext`, `nom 3.ext`…) dans `dir`.
pub fn free_path(dir: &Path, name: &str, ext: &str) -> PathBuf {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || " -_".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = if clean.trim().is_empty() {
        "Sans titre".to_owned()
    } else {
        clean
    };
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|i| dir.join(format!("{stem} {i}.{ext}")))
        .find(|p| !p.exists())
        .expect("itérateur infini")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_path_sanitizes_and_avoids_collisions() {
        let dir = std::env::temp_dir().join(format!("bones-files-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = free_path(&dir, "a/b:c", "bones");
        assert_eq!(p.file_name().unwrap(), "a_b_c.bones");
        std::fs::write(&p, b"{}").unwrap();
        assert_eq!(
            free_path(&dir, "a/b:c", "bones").file_name().unwrap(),
            "a_b_c 2.bones"
        );
        assert_eq!(list(&dir, |f| f == Format::Bones).len(), 1);
        assert!(list(&dir, |f| f == Format::Obj).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
