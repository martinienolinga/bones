//! Fichiers de projets d'un dossier ; le nom du projet = nom du fichier.
//!
//! Renommer un projet revient donc à renommer son fichier (instantané) ; le
//! nom inscrit dans le manifeste n'est qu'informatif, rafraîchi à chaque
//! enregistrement.
//!
//! Les fonctions `*_ext`, [`rename`], [`delete`] et [`duplicate_file`]
//! valent pour n'importe quelle extension (préréglages…).

mod names;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub use names::sanitize_name;

use super::container::{Archive, deflate, finish, write_atomically, writer};
use super::document::MANIFEST;
use super::error::invalid;
use super::read::read_manifest;
use super::{EXTENSION, ProjectError, unix_now};
use names::{same_file, stem, strip_counter};

/// Nom proposé quand il n'en reste aucun après nettoyage.
const UNTITLED: &str = "Untitled";

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub modified: SystemTime,
}

/// Projets (`*.kbrig`) du dossier, le plus récemment modifié d'abord.
pub fn list(dir: &Path) -> Vec<Entry> {
    list_ext(dir, EXTENSION)
}

/// Chemin libre pour un projet nommé `name` (nettoyé) : « Untitled »,
/// « Untitled 2 », « Untitled 3 »…
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    unique_path_ext(dir, name, EXTENSION)
}

/// Copie un projet sous un nom libre (« Nom 2 »…). Les entrées sont copiées
/// telles quelles, sans recompression ; seul le manifeste est réécrit (nom,
/// dates).
pub fn duplicate(path: &Path) -> Result<PathBuf, ProjectError> {
    let mut archive = Archive::open(path)?;
    let mut manifest = read_manifest(&mut archive)?;
    let target = copy_path(path);
    let now = unix_now();
    manifest.name = stem(&target);
    manifest.created = now;
    manifest.modified = now;
    let manifest = deflate(MANIFEST, &serde_json::to_vec_pretty(&manifest)?)?;
    write_atomically(&target, |file| {
        let mut zip = writer(file);
        zip.add_prepared_file(manifest)?;
        archive.copy_raw(&mut zip, MANIFEST)?;
        finish(zip)
    })?;
    Ok(target)
}

/// Fichiers `*.ext` du dossier (sans les fichiers cachés ni temporaires), le
/// plus récemment modifié d'abord.
pub fn list_ext(dir: &Path, ext: &str) -> Vec<Entry> {
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = read
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let name = stem(&path);
            if !has_extension(&path, ext) || name.starts_with('.') {
                return None;
            }
            let meta = e.metadata().ok().filter(|m| m.is_file())?;
            let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
            Some(Entry { path, name, modified })
        })
        .collect();
    entries.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.name.cmp(&b.name)));
    entries
}

/// Chemin libre `dir/<nom>.<ext>` (nom nettoyé, numéroté si déjà pris).
pub fn unique_path_ext(dir: &Path, name: &str, ext: &str) -> PathBuf {
    let base = sanitize_name(name);
    let base = if base.is_empty() { UNTITLED } else { &base };
    (1u64..)
        .map(|n| match n {
            1 => dir.join(format!("{base}.{ext}")),
            n => dir.join(format!("{base} {n}.{ext}")),
        })
        .find(|p| p.symlink_metadata().is_err())
        .expect("itérateur infini")
}

/// Renomme un fichier (son extension est gardée) ; le nom est nettoyé et
/// rendu unique dans le dossier. Retourne le nouveau chemin.
pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf, ProjectError> {
    let name = sanitize_name(new_name);
    if name.is_empty() {
        return Err(invalid("the name is empty"));
    }
    if name == stem(path) {
        return Ok(path.to_owned());
    }
    let ext = extension(path);
    let dir = parent(path);
    let direct = dir.join(format!("{name}.{ext}"));
    // Changement de casse seul : sur un volume insensible à la casse, la
    // « cible » existante est le fichier lui-même.
    let target = if direct.symlink_metadata().is_err() || same_file(path, &direct) {
        direct
    } else {
        unique_path_ext(dir, &name, &ext)
    };
    fs::rename(path, &target)?;
    Ok(target)
}

/// Copie octet pour octet d'un fichier sous un nom libre (« Nom 2 »…),
/// écrite de façon atomique.
pub fn duplicate_file(path: &Path) -> Result<PathBuf, ProjectError> {
    let target = copy_path(path);
    let mut source = fs::File::open(path)?;
    write_atomically(&target, |mut file| {
        std::io::copy(&mut source, &mut file)?;
        file.sync_all()?;
        Ok(())
    })?;
    Ok(target)
}

/// Supprime un fichier (déjà absent : rien à faire).
pub fn delete(path: &Path) -> Result<(), ProjectError> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => Ok(result?),
    }
}

/// Chemin libre pour une copie de `path`, numérotée à partir du nom d'origine.
fn copy_path(path: &Path) -> PathBuf {
    let name = stem(path);
    unique_path_ext(parent(path), strip_counter(&name), &extension(path))
}

fn has_extension(path: &Path, ext: &str) -> bool {
    path.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext))
}

fn extension(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_else(|| EXTENSION.to_owned())
}

fn parent(path: &Path) -> &Path {
    path.parent().unwrap_or(Path::new("."))
}
