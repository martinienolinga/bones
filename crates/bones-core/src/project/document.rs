//! Structure du fichier : noms des entrées, manifeste et document JSON.
//!
//! Le document JSON contient le `Rig` complet sérialisé par serde, mais dont
//! les données lourdes ont été retirées (géométrie du maillage, poids, poses
//! des clés d'animation, octets des textures) : elles sont écrites à part, en
//! binaire. Tout champ ajouté plus tard au `Rig` est donc enregistré sans rien
//! changer ici.

use serde::{Deserialize, Serialize};

use super::state::EditorState;
use super::{ProjectError, ProjectInfo};
use crate::Rig;

/// Identifiant du format, dans le manifeste.
pub(super) const FORMAT: &str = "paintsharp-bones-project";
/// Version du format écrite ; les fichiers plus récents sont refusés.
pub(super) const VERSION: u32 = 1;

pub(super) const MANIFEST: &str = "manifest.json";
pub(super) const THUMBNAIL: &str = "thumbnail.jpg";
pub(super) const DOCUMENT: &str = "document.json";
pub(super) const POSITIONS: &str = "mesh/positions.bin";
pub(super) const NORMALS: &str = "mesh/normals.bin";
pub(super) const UVS: &str = "mesh/uvs.bin";
pub(super) const INDICES: &str = "mesh/indices.bin";
pub(super) const WEIGHTS: &str = "skin/weights.bin";
pub(super) const POSES: &str = "anim/poses.bin";
pub(super) const TEXTURES_DIR: &str = "textures/";

/// Nombre maximal de sommets d'un projet.
const MAX_VERTICES: u32 = 1 << 23;
/// Nombre maximal d'indices de triangles.
const MAX_INDICES: u32 = 3 << 24;
/// Nombre maximal de transformations de poses-clés (clés × os).
const MAX_POSES: u64 = 1 << 23;
/// Nombre maximal d'os : les poids les désignent sur 16 bits.
const MAX_BONES: usize = u16::MAX as usize;

/// Petit résumé lisible sans rien décompresser d'autre (galerie).
#[derive(Debug, Serialize, Deserialize)]
pub(super) struct Manifest {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub app_version: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub created: u64,
    #[serde(default)]
    pub modified: u64,
    #[serde(default)]
    pub stats: Stats,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Stats {
    pub bones: u32,
    pub vertices: u32,
    pub triangles: u32,
    pub frames: u32,
    pub fps: f32,
    pub textured: bool,
}

impl Manifest {
    pub fn new(info: &ProjectInfo) -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            app_version: env!("CARGO_PKG_VERSION").into(),
            name: info.name.clone(),
            created: info.created,
            modified: info.modified,
            stats: Stats {
                bones: info.bones,
                vertices: info.vertices,
                triangles: info.triangles,
                frames: info.frames,
                fps: info.fps,
                textured: info.textured,
            },
        }
    }

    /// Résumé ; le nom vient du fichier (source de vérité), pas du manifeste.
    pub fn info(&self, name: String) -> ProjectInfo {
        let s = &self.stats;
        ProjectInfo {
            name,
            created: self.created,
            modified: self.modified,
            bones: s.bones,
            vertices: s.vertices,
            triangles: s.triangles,
            frames: s.frames,
            fps: s.fps,
            textured: s.textured,
        }
    }
}

/// Contenu de `document.json`.
#[derive(Serialize, Deserialize)]
pub(super) struct Document {
    /// Document sans ses données lourdes.
    pub rig: Rig,
    /// Nombre d'éléments des tableaux binaires.
    pub counts: Counts,
    /// Entrées des textures (`textures/…`), sans doublon.
    #[serde(default)]
    pub textures: Vec<String>,
    /// Pour chaque matériau, indice de sa texture de couleur dans `textures`.
    #[serde(default)]
    pub material_textures: Vec<Option<u32>>,
    /// Les poids correspondaient au squelette et au maillage enregistrés.
    #[serde(default)]
    pub weights_bound: bool,
    #[serde(default)]
    pub editor: EditorState,
}

/// Nombre d'éléments de chaque tableau binaire (0 : entrée absente).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Counts {
    pub positions: u32,
    pub normals: u32,
    pub uvs: u32,
    pub indices: u32,
    pub weights: u32,
    /// Transformations par pose-clé (toutes les clés en ont autant).
    pub key_bones: u32,
}

impl Counts {
    /// Vérifie les tailles (mêmes limites à l'écriture et à la lecture : un
    /// projet enregistré reste toujours lisible). Retourne le nombre de
    /// transformations des poses-clés.
    pub fn check(&self, keys: usize, bones: usize) -> Result<u32, ProjectError> {
        let vertices = [self.positions, self.normals, self.uvs, self.weights];
        let poses = u64::from(self.key_bones) * keys as u64;
        if vertices.iter().any(|n| *n > MAX_VERTICES)
            || self.indices > MAX_INDICES
            || poses > MAX_POSES
            || bones > MAX_BONES
        {
            return Err(ProjectError::TooLarge(format!(
                "{} vertices, {} triangles, {bones} bones, {poses} key transforms",
                self.positions,
                self.indices / 3
            )));
        }
        Ok(poses as u32)
    }
}
