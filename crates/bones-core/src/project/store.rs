//! Fichiers de projets d'un dossier ; le nom du projet = nom du fichier.
//!
//! Renommer un projet revient donc à renommer son fichier (instantané) ; le
//! nom inscrit dans le manifeste n'est qu'informatif, rafraîchi à chaque
//! enregistrement.

use std::fs;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use zip::ZipWriter;

use super::archive::Archive;
use super::atomic::write_atomically;
use super::document::MANIFEST;
use super::error::invalid;
use super::read::{project_name, read_manifest};
use super::{EXTENSION, ProjectError, unix_now, write};

/// Longueur maximale d'un nom de projet, en caractères.
const MAX_NAME_CHARS: usize = 80;
/// … et en octets UTF-8 (les noms de fichiers sont limités à 255 octets).
const MAX_NAME_BYTES: usize = 200;
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
    let Ok(read) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut entries: Vec<Entry> = read
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let name = project_name(&path);
            if !is_project(&path) || name.starts_with('.') {
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

/// Chemin libre pour un projet nommé `name` (nettoyé) : « Untitled »,
/// « Untitled 2 », « Untitled 3 »…
pub fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let base = sanitize_name(name);
    let base = if base.is_empty() {
        UNTITLED.to_owned()
    } else {
        base
    };
    (1u64..)
        .map(|n| match n {
            1 => dir.join(file_name(&base)),
            n => dir.join(file_name(&format!("{base} {n}"))),
        })
        .find(|p| p.symlink_metadata().is_err())
        .expect("itérateur infini")
}

/// Renomme un projet ; le nom est nettoyé et rendu unique dans le dossier.
/// Retourne le nouveau chemin.
pub fn rename(path: &Path, new_name: &str) -> Result<PathBuf, ProjectError> {
    let name = sanitize_name(new_name);
    if name.is_empty() {
        return Err(invalid("the project name is empty"));
    }
    if name == project_name(path) {
        return Ok(path.to_owned());
    }
    let dir = parent(path);
    let direct = dir.join(file_name(&name));
    // Changement de casse seul : sur un volume insensible à la casse, la
    // « cible » existante est le projet lui-même.
    let target = if direct.symlink_metadata().is_err() || same_file(path, &direct) {
        direct
    } else {
        unique_path(dir, &name)
    };
    fs::rename(path, &target)?;
    Ok(target)
}

/// Copie un projet sous un nom libre (« Nom 2 »…). Les entrées sont copiées
/// telles quelles, sans recompression ; seul le manifeste est réécrit (nom,
/// dates).
pub fn duplicate(path: &Path) -> Result<PathBuf, ProjectError> {
    let mut archive = Archive::open(path)?;
    let mut manifest = read_manifest(&mut archive)?;
    let stem = project_name(path);
    let target = unique_path(parent(path), strip_counter(&stem));
    let now = unix_now();
    manifest.name = project_name(&target);
    manifest.created = now;
    manifest.modified = now;
    let manifest = write::deflate(MANIFEST, &serde_json::to_vec_pretty(&manifest)?)?;
    write_atomically(&target, |file| {
        let mut zip = ZipWriter::new(BufWriter::with_capacity(1 << 16, file));
        zip.add_prepared_file(manifest)?;
        archive.copy_raw(&mut zip, MANIFEST)?;
        let file = zip.finish()?.into_inner().map_err(|e| e.into_error())?;
        file.sync_all()?;
        Ok(())
    })?;
    Ok(target)
}

/// Supprime un projet (déjà absent : rien à faire).
pub fn delete(path: &Path) -> Result<(), ProjectError> {
    match fs::remove_file(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => Ok(result?),
    }
}

/// Nom utilisable comme nom de fichier : sans séparateurs ni caractères de
/// contrôle, sans point initial (fichier caché), espaces superflus retirés,
/// longueur bornée. Les accents et autres caractères Unicode sont gardés.
pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let mut out = String::new();
    for (i, c) in cleaned
        .trim()
        .trim_start_matches('.')
        .trim_start()
        .chars()
        .enumerate()
    {
        if i >= MAX_NAME_CHARS || out.len() + c.len_utf8() > MAX_NAME_BYTES {
            break;
        }
        out.push(c);
    }
    // Fin de nom : ni espace ni point (refusés par certains systèmes).
    out.trim_end_matches([' ', '.']).trim_end().to_owned()
}

pub(super) fn is_project(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

fn file_name(name: &str) -> String {
    format!("{name}.{EXTENSION}")
}

fn parent(path: &Path) -> &Path {
    path.parent().unwrap_or(Path::new("."))
}

/// « Nom 2 » → « Nom » (pour numéroter une copie à partir du nom d'origine).
fn strip_counter(name: &str) -> &str {
    match name.rsplit_once(' ') {
        Some((base, n))
            if !base.trim().is_empty() && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base
        }
        _ => name,
    }
}

/// Même fichier sur le disque (mêmes volume et inode).
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (fs::metadata(a), fs::metadata(b)) {
            (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (a, b);
        false
    }
}
