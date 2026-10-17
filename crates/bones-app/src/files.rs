//! Emplacement des documents et liste des fichiers.
//!
//! Sur iPad, le dossier Documents de l'app est visible dans l'app Fichiers
//! (UIFileSharingEnabled) : l'utilisateur y dépose ses modèles (glTF, FBX,
//! OBJ…) ou des projets `.kbrig`, et retrouve ses exports. Les projets
//! eux-mêmes sont rangés dans `Projects` (voir `crate::projects`) ; les
//! fichiers ouverts depuis une autre app arrivent dans `Inbox`.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bones_core::io::Format;

/// Sous-dossier des exports, pour garder la galerie lisible.
pub const EXPORTS: &str = "Exports";
/// Boîte de réception d'iOS (fichiers ouverts dans l'app depuis ailleurs).
pub const INBOX: &str = "Inbox";

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

/// Fichier proposé à l'import dans la galerie.
#[derive(Clone, Debug, PartialEq)]
pub struct ImportFile {
    pub name: String,
    pub path: PathBuf,
    /// Format 3D, ou `None` pour un projet `.kbrig`.
    pub format: Option<Format>,
}

/// Modèles 3D et projets `.kbrig` du dossier Documents et de sa boîte de
/// réception, du plus récent au plus ancien.
pub fn importable(docs: &Path) -> Vec<ImportFile> {
    let mut files: Vec<(SystemTime, ImportFile)> = Vec::new();
    for dir in [docs.to_owned(), docs.join(INBOX)] {
        // Les anciens documents `.bones` s'importent aussi : ils deviennent
        // des projets.
        let models = list(&dir, Format::can_import);
        files.extend(models.into_iter().map(|f| {
            let file = ImportFile {
                name: f.name,
                path: f.path,
                format: Some(f.format),
            };
            (f.modified, file)
        }));
        files.extend(projects(&dir));
    }
    files.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    files.into_iter().map(|(_, f)| f).collect()
}

fn projects(dir: &Path) -> Vec<(SystemTime, ImportFile)> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.filter_map(Result::ok)
        .filter_map(|e| {
            let path = e.path();
            let ext = path.extension()?.to_str()?;
            if !ext.eq_ignore_ascii_case(bones_core::project::EXTENSION) {
                return None;
            }
            let meta = e.metadata().ok().filter(|m| m.is_file())?;
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            let file = ImportFile {
                name: stem(&path),
                path,
                format: None,
            };
            Some((modified, file))
        })
        .collect()
}

/// Nom d'un fichier sans son extension.
pub fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
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
        "Untitled".to_owned()
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
        let p = free_path(&dir, "a/b:c", "obj");
        assert_eq!(p.file_name().unwrap(), "a_b_c.obj");
        std::fs::write(&p, b"o").unwrap();
        assert_eq!(
            free_path(&dir, "a/b:c", "obj").file_name().unwrap(),
            "a_b_c 2.obj"
        );
        assert_eq!(list(&dir, |f| f == Format::Obj).len(), 1);
        assert!(list(&dir, |f| f == Format::Stl).is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn importable_lists_models_and_projects_from_documents_and_inbox() {
        let dir = std::env::temp_dir().join(format!("bones-import-{}", std::process::id()));
        let inbox = dir.join(INBOX);
        std::fs::create_dir_all(&inbox).unwrap();
        for name in ["Fox.glb", "Old.bones", "notes.txt", "Hero.kbrig"] {
            std::fs::write(dir.join(name), b"x").unwrap();
        }
        std::fs::write(inbox.join("Scan.ply"), b"x").unwrap();
        let mut found: Vec<(String, Option<Format>)> =
            importable(&dir).into_iter().map(|f| (f.name, f.format)).collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            found,
            [
                ("Fox".to_owned(), Some(Format::Glb)),
                ("Hero".to_owned(), None),
                ("Old".to_owned(), Some(Format::Bones)),
                ("Scan".to_owned(), Some(Format::Ply)),
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
