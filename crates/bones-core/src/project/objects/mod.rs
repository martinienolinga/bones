//! Objets supplémentaires dans le fichier `.kbrig` : leurs données lourdes
//! (géométrie, poids, textures) sont écrites à part, comme celles du corps,
//! et non dans `document.json`. Le pool de textures est partagé avec le
//! corps (une image employée par plusieurs objets n'est écrite qu'une fois).
//!
//! Entrées d'un objet d'indice `i` : `obj/<i>/positions.bin`, `normals.bin`,
//! `uvs.bin`, `indices.bin`, et `weights.bin` s'il est déformé.

mod load;
mod split;

use serde::{Deserialize, Serialize};

pub(super) use load::assemble_objects;
pub(super) use split::{Stripped, TexturePool, strip_objects};

use super::ProjectError;
use super::error::invalid;

/// Nombre maximal d'objets supplémentaires.
pub(super) const MAX_OBJECTS: usize = 4096;

/// Tailles et lien matériau → texture d'un objet (dans `document.json`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct ObjectCounts {
    pub positions: u32,
    pub normals: u32,
    pub uvs: u32,
    pub indices: u32,
    /// 0 : objet rigide (pas d'entrée de poids).
    pub weights: u32,
    /// Indice de texture de couleur de chaque matériau de l'objet.
    pub material_textures: Vec<Option<u32>>,
}

/// Noms des entrées binaires d'un objet.
pub(super) fn entry(index: usize, kind: &str) -> String {
    format!("obj/{index}/{kind}.bin")
}

/// Vérifie les limites des objets (mêmes à l'écriture et à la lecture : un
/// projet enregistré reste toujours lisible).
pub(super) fn check_objects(counts: &[ObjectCounts]) -> Result<(), ProjectError> {
    if counts.len() > MAX_OBJECTS {
        return Err(ProjectError::TooLarge(format!("{} objects", counts.len())));
    }
    for c in counts {
        let vertices = [c.positions, c.normals, c.uvs, c.weights];
        if vertices.iter().any(|n| *n > super::document::MAX_VERTICES)
            || c.indices > super::document::MAX_INDICES
        {
            return Err(ProjectError::TooLarge(format!(
                "object with {} vertices, {} indices",
                c.positions, c.indices
            )));
        }
        // Un objet déformé a un poids par sommet ; un rigide n'en a aucun.
        if c.weights != 0 && c.weights != c.positions {
            return Err(invalid("object weights do not match its vertices"));
        }
    }
    Ok(())
}
