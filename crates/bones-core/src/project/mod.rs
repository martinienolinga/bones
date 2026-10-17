//! Projets `.kbrig` : un fichier compressé par projet, contenant tout ce qui
//! le constitue (modèle 3D et textures, squelette, pose, animation, poids,
//! membres IK/FK, épingles, marqueurs d'auto-rig, caméra, réglages de
//! l'éditeur) et une vignette pour la galerie. L'historique d'annulation
//! n'est pas enregistré.
//!
//! Le fichier est un conteneur ZIP (lecture directe d'une entrée sans
//! décompresser les autres) :
//!
//! | entrée               | contenu                                      | compression |
//! |----------------------|----------------------------------------------|-------------|
//! | `manifest.json`      | format, version, nom, dates, statistiques    | deflate     |
//! | `thumbnail.jpg`      | vignette JPEG                                | aucune      |
//! | `document.json`      | `Rig` sans données lourdes + éditeur         | deflate     |
//! | `mesh/positions.bin` | `[f32; 3]` par sommet                        | deflate     |
//! | `mesh/normals.bin`   | `[f32; 3]` par sommet                        | deflate     |
//! | `mesh/uvs.bin`       | `[f32; 2]` par sommet (si présentes)         | deflate     |
//! | `mesh/indices.bin`   | `u32` : différences zigzag, plans d'octets   | deflate     |
//! | `skin/weights.bin`   | 4 × `u16` os + 4 × `u16` poids par sommet    | deflate     |
//! | `anim/poses.bin`     | 10 × `f32` par os et par clé                 | deflate     |
//! | `textures/<n>.<ext>` | image d'origine (PNG, JPEG…)                 | aucune      |
//!
//! Les tableaux binaires sont en petit-boutiste ; une entrée absente vaut un
//! tableau vide, un champ JSON absent sa valeur par défaut. Les entrées sont
//! compressées et décompressées en parallèle (rayon). L'enregistrement est
//! atomique (fichier temporaire synchronisé puis renommé) et la lecture
//! bornée et vérifiée : les fichiers reçus d'ailleurs ne peuvent ni épuiser
//! la mémoire ni faire paniquer l'application. Un fichier d'une version du
//! format plus récente est refusé avec un message clair.

mod codec;
pub(crate) mod container;
mod document;
mod error;
mod finite;
mod read;
mod snapshot;
mod split;
mod state;
pub mod store;
mod thumbnail;
mod validate;
mod write;

#[cfg(test)]
mod tests;

use std::path::Path;

pub use error::ProjectError;
pub use snapshot::ProjectSnapshot;
pub use thumbnail::{decode_thumbnail, encode_thumbnail};

/// Extension des fichiers de projet.
pub const EXTENSION: &str = "kbrig";

/// Résumé d'un projet, lisible sans le charger (galerie).
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectInfo {
    pub name: String,
    /// Secondes Unix (UTC).
    pub created: u64,
    pub modified: u64,
    pub bones: u32,
    pub vertices: u32,
    pub triangles: u32,
    pub frames: u32,
    pub fps: f32,
    pub textured: bool,
}

/// Projet ouvert depuis le disque.
pub struct LoadedProject {
    pub info: ProjectInfo,
    pub snapshot: ProjectSnapshot,
    /// Vignette JPEG encodée, si le fichier en contient une.
    pub thumbnail: Option<Vec<u8>>,
}

/// Ouvre un projet complet.
pub fn load(path: &Path) -> Result<LoadedProject, ProjectError> {
    read::load(path)
}

/// Lecture rapide : manifeste + vignette encodée, sans décompresser le reste.
pub fn read_info(path: &Path) -> Result<(ProjectInfo, Option<Vec<u8>>), ProjectError> {
    read::read_info(path)
}

/// Heure courante en secondes Unix (UTC), pour les dates des projets.
pub fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
