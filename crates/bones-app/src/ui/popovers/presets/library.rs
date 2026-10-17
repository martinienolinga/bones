//! Bibliothèque de préréglages chargée pour la galerie : les préréglages
//! intégrés (en mémoire, `&'static`) et ceux de l'utilisateur chargés depuis
//! le dossier `Presets` (créations et imports). Filtrage par section et par
//! texte, et liste des fichiers importables.

use std::path::{Path, PathBuf};

use bones_core::presets::{self, Category, Preset, PresetInfo, Section};

use crate::files;

/// Sous-dossier des préréglages de l'utilisateur, dans les Documents.
pub const DIR: &str = "Presets";

/// Référence à un préréglage de la bibliothèque (intégré ou utilisateur).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Builtin(usize),
    User(usize),
}

#[derive(Default)]
pub struct Library {
    /// Préréglages de l'utilisateur chargés : chemin du fichier + contenu.
    user: Vec<(PathBuf, Preset)>,
    /// Dossier déjà chargé (pour ne relire qu'au besoin).
    loaded: Option<PathBuf>,
}

impl Library {
    /// (Re)charge les préréglages de l'utilisateur depuis `dir`. Les fichiers
    /// illisibles sont ignorés (la galerie reste robuste).
    pub fn refresh(&mut self, dir: &Path) {
        self.user = presets::user::list(dir)
            .into_iter()
            .filter_map(|l| presets::user::load(&l.path).ok().map(|p| (l.path, p)))
            .collect();
        self.loaded = Some(dir.to_owned());
    }

    /// Charge la bibliothèque la première fois (ouverture de la galerie).
    pub fn ensure(&mut self, dir: &Path) {
        if self.loaded.as_deref() != Some(dir) {
            self.refresh(dir);
        }
    }

    pub fn preset(&self, entry: Entry) -> &Preset {
        match entry {
            Entry::Builtin(i) => &presets::builtin()[i],
            Entry::User(i) => &self.user[i].1,
        }
    }

    pub fn info(&self, entry: Entry) -> &PresetInfo {
        &self.preset(entry).info
    }

    /// Chemin du fichier d'un préréglage utilisateur (pour renommer, exporter…).
    pub fn path(&self, entry: Entry) -> Option<&Path> {
        match entry {
            Entry::User(i) => self.user.get(i).map(|(p, _)| p.as_path()),
            Entry::Builtin(_) => None,
        }
    }

    /// Entrées d'une section, filtrées par le texte de recherche.
    pub fn section(&self, section: Section, query: &str) -> Vec<Entry> {
        self.filter(query, |info| section.contains(info))
    }

    /// Entrées d'expression faciale, filtrées par le texte de recherche.
    pub fn expressions(&self, query: &str) -> Vec<Entry> {
        self.filter(query, is_expression)
    }

    /// Nombre de préréglages utilisateur chargés (tests).
    #[cfg(test)]
    pub(super) fn user_len(&self) -> usize {
        self.user.len()
    }

    fn filter(&self, query: &str, keep: impl Fn(&PresetInfo) -> bool) -> Vec<Entry> {
        let builtin = presets::builtin()
            .iter()
            .enumerate()
            .filter(|(_, p)| keep(&p.info) && p.info.matches(query))
            .map(|(i, _)| Entry::Builtin(i));
        let user = self
            .user
            .iter()
            .enumerate()
            .filter(|(_, (_, p))| keep(&p.info) && p.info.matches(query))
            .map(|(i, _)| Entry::User(i));
        builtin.chain(user).collect()
    }
}

/// Un préréglage est une expression faciale.
pub fn is_expression(info: &PresetInfo) -> bool {
    info.category == Category::FacialExpression
}

/// Fichier d'animation importable comme préréglage.
pub struct Importable {
    pub name: String,
    pub path: PathBuf,
    pub ext: String,
}

/// Fichiers d'animation des Documents et de la boîte de réception qui peuvent
/// devenir des préréglages (`.kb3da`, BVH, glTF/GLB, FBX), du plus récent au
/// plus ancien.
pub fn importable(docs: &Path) -> Vec<Importable> {
    const EXTS: [&str; 5] = ["kb3da", "bvh", "gltf", "glb", "fbx"];
    let mut files: Vec<(std::time::SystemTime, Importable)> = Vec::new();
    for dir in [docs.to_owned(), docs.join(files::INBOX)] {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            let Some(ext) = path.extension().and_then(|e| e.to_str()).map(str::to_lowercase) else {
                continue;
            };
            if !EXTS.contains(&ext.as_str()) {
                continue;
            }
            let Ok(meta) = entry.metadata() else { continue };
            let modified = meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            files.push((
                modified,
                Importable {
                    name: files::stem(&path),
                    path,
                    ext,
                },
            ));
        }
    }
    files.sort_by_key(|(m, _)| std::cmp::Reverse(*m));
    files.into_iter().map(|(_, f)| f).collect()
}
