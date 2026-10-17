//! Liste des projets affichés, du plus récent au plus ancien. Les
//! résumés (nombre d'os, d'images…) arrivent au fil de la lecture des
//! fichiers ; les opérations (supprimer, renommer, enregistrer) sont
//! reflétées tout de suite, avant même la fin de l'accès disque.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use bones_core::project::{ProjectInfo, store};

#[derive(Clone, Debug, PartialEq)]
pub struct Card {
    pub path: PathBuf,
    pub name: String,
    pub modified: SystemTime,
    pub info: Option<ProjectInfo>,
}

#[derive(Default)]
pub struct Library {
    cards: Vec<Card>,
    /// La liste a été lue au moins une fois (sinon, pas d'état vide).
    listed: bool,
}

impl Library {
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    pub fn is_listed(&self) -> bool {
        self.listed
    }

    pub fn get(&self, path: &Path) -> Option<&Card> {
        self.cards.iter().find(|c| c.path == path)
    }

    /// Nouvelle liste lue sur le disque ; les résumés déjà connus des
    /// fichiers inchangés sont gardés.
    pub fn set_entries(&mut self, entries: Vec<store::Entry>) {
        let old = std::mem::take(&mut self.cards);
        self.cards = entries
            .into_iter()
            .map(|e| {
                let info = old
                    .iter()
                    .find(|c| c.path == e.path && c.modified == e.modified)
                    .and_then(|c| c.info.clone());
                Card {
                    path: e.path,
                    name: e.name,
                    modified: e.modified,
                    info,
                }
            })
            .collect();
        self.listed = true;
    }

    /// Résumé lu pour la version `modified` d'un fichier.
    pub fn set_info(&mut self, path: &Path, modified: SystemTime, info: ProjectInfo) {
        if let Some(card) = self
            .cards
            .iter_mut()
            .find(|c| c.path == path && c.modified == modified)
        {
            card.info = Some(info);
        }
    }

    /// Projet enregistré : résumé à jour, carte remontée en tête.
    pub fn saved(&mut self, path: &Path, info: ProjectInfo, modified: SystemTime) {
        let card = match self.cards.iter().position(|c| c.path == path) {
            Some(i) => self.cards.remove(i),
            None => Card {
                path: path.to_owned(),
                name: crate::files::stem(path),
                modified,
                info: None,
            },
        };
        self.cards.insert(
            0,
            Card {
                modified,
                info: Some(info),
                ..card
            },
        );
    }

    /// Rang d'un projet dans la grille.
    pub fn index(&self, path: &Path) -> Option<usize> {
        self.cards.iter().position(|c| c.path == path)
    }

    /// Projet qu'on vient de quitter (en cours d'enregistrement) : en tête,
    /// comme le plus récent.
    pub fn bring_to_front(&mut self, path: &Path) {
        if let Some(i) = self.index(path) {
            let card = self.cards.remove(i);
            self.cards.insert(0, card);
        }
    }

    pub fn remove(&mut self, paths: &[PathBuf]) {
        self.cards.retain(|c| !paths.contains(&c.path));
    }

    /// Affiche tout de suite le nouveau nom (le fichier suit).
    pub fn rename(&mut self, path: &Path, name: &str) {
        if let Some(card) = self.cards.iter_mut().find(|c| c.path == path) {
            card.name = name.to_owned();
        }
    }

    /// Le fichier a changé de chemin.
    pub fn moved(&mut self, from: &Path, to: &Path) {
        if let Some(card) = self.cards.iter_mut().find(|c| c.path == from) {
            card.path = to.to_owned();
            card.name = crate::files::stem(to);
        }
    }
}

/// Sous-titre court d'une carte : « 52 bones · 60 frames ».
pub fn subtitle(info: &ProjectInfo) -> String {
    let count = |n: u32, one: &str, many: &str| match n {
        1 => format!("1 {one}"),
        n => format!("{n} {many}"),
    };
    let bones = match info.bones {
        0 => "No bones".to_owned(),
        n => count(n, "bone", "bones"),
    };
    let second = if info.frames > 1 {
        count(info.frames, "frame", "frames")
    } else if info.triangles == 0 {
        "No mesh".to_owned()
    } else if info.triangles >= 10_000 {
        format!("{:.0}k triangles", info.triangles as f32 / 1000.0)
    } else if info.triangles >= 1000 {
        format!("{:.1}k triangles", info.triangles as f32 / 1000.0)
    } else {
        count(info.triangles, "triangle", "triangles")
    };
    format!("{bones} · {second}")
}
