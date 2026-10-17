//! Wavefront OBJ : import avec matériaux MTL et textures (via `tobj`),
//! export du maillage posé.

mod export;
mod mtl;

use std::io::BufReader;
use std::path::Path;

use glam::{Vec2, Vec3};

pub use export::export;

use super::{Format, IoError};
use crate::Mesh;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjError {
    /// Ligne fautive (0 si inconnue).
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for ObjError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "OBJ ligne {} : {}", self.line, self.message)
        } else {
            write!(f, "OBJ : {}", self.message)
        }
    }
}

impl std::error::Error for ObjError {}

/// Lit un OBJ depuis du texte, sans matériaux.
pub fn parse(source: &str) -> Result<Mesh, ObjError> {
    read(source.as_bytes(), None).map_err(|e| ObjError {
        line: 0,
        message: e.to_string(),
    })
}

/// Import complet : les fichiers MTL et les textures sont cherchés dans
/// `base_dir`.
pub(crate) fn import(bytes: &[u8], base_dir: Option<&Path>) -> Result<Mesh, IoError> {
    read(bytes, base_dir)
}

fn read(bytes: &[u8], base_dir: Option<&Path>) -> Result<Mesh, IoError> {
    let mut reader = BufReader::new(bytes);
    let (models, materials) = tobj::load_obj_buf(&mut reader, &tobj::GPU_LOAD_OPTIONS, |p| {
        let Some(dir) = base_dir else {
            return Err(tobj::LoadError::OpenFileFailed);
        };
        let file = std::fs::File::open(dir.join(p)).map_err(|_| tobj::LoadError::OpenFileFailed)?;
        tobj::load_mtl_buf(&mut BufReader::new(file))
    })
    .map_err(|e| IoError::parse(Format::Obj, e))?;

    let mut mesh = Mesh::default();
    if let Ok(materials) = materials {
        let mut textures = mtl::TextureCache::default();
        mesh.materials = materials
            .iter()
            .map(|m| mtl::convert(m, base_dir, &mut textures))
            .collect();
    }
    let has_materials = !mesh.materials.is_empty();

    for model in &models {
        let m = &model.mesh;
        if m.indices.is_empty() {
            continue;
        }
        let mut part = Mesh {
            positions: m
                .positions
                .as_chunks::<3>()
                .0
                .iter()
                .map(|c| Vec3::from_array(*c))
                .collect(),
            normals: m
                .normals
                .as_chunks::<3>()
                .0
                .iter()
                .map(|c| Vec3::from_array(*c))
                .collect(),
            indices: m.indices.clone(),
            // OBJ : origine des UV en bas à gauche.
            uvs: m
                .texcoords
                .as_chunks::<2>()
                .0
                .iter()
                .map(|t| Vec2::new(t[0], 1.0 - t[1]))
                .collect(),
            ..Default::default()
        };
        if part.normals.len() != part.positions.len() {
            part.compute_normals();
        }
        if part.uvs.len() != part.positions.len() {
            part.uvs.clear();
        }
        if has_materials {
            mesh.append_part(part, m.material_id);
        } else {
            mesh.append(&part);
        }
    }
    if mesh.indices.is_empty() {
        return Err(IoError::parse(Format::Obj, "aucune face"));
    }
    mesh.shrink_to_fit();
    Ok(mesh)
}
