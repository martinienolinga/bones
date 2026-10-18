//! Entrées/sorties : formats 3D d'échange. Les projets de l'app sont des
//! fichiers `.kbrig` (voir [`crate::project`]).
//!
//! Point d'entrée unique : [`import`] (détection par l'extension) et
//! [`export`]. Chaque format vit dans son propre module. Conventions
//! internes : Y vers le haut, mètres, personnage tourné vers +Z, coordonnées
//! de texture avec l'origine en haut à gauche ; les formats qui en diffèrent
//! sont convertis à la lecture et à l'écriture.
//!
//! | Format | Import | Export |
//! |---|---|---|
//! | glTF / GLB | maillage, matériaux, textures, squelette, poids, animation | idem |
//! | FBX binaire | maillage, matériaux, textures, squelette, poids, animation | — |
//! | OBJ + MTL | maillage, matériaux, textures | maillage posé, matériaux, textures |
//! | STL | maillage | maillage posé |
//! | PLY | maillage, coordonnées de texture | maillage posé |
//! | BVH | squelette, animation | squelette, animation |

mod bvh;
mod error;
mod fbx;
mod format;
pub mod gltf;
pub mod obj;
mod ply;
pub(crate) mod rigging;
mod scene;
mod stl;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

pub use error::IoError;
pub use format::Format;
pub use scene::{ImportedObject, ImportedParent, SceneImport, import_scene, import_scene_bytes};

use crate::{Mesh, Rig};

/// Résultat d'un import.
pub enum Imported {
    /// Document complet (squelette, poids, animation…).
    Rig(Rig),
    /// Géométrie seule, à rigger dans l'app.
    Mesh(Mesh),
}

/// Importe un fichier d'après son extension. Les textures externes sont
/// cherchées à côté du fichier.
pub fn import(path: &Path) -> Result<Imported, IoError> {
    let format = Format::from_path(path).ok_or_else(|| unsupported(path))?;
    let bytes = std::fs::read(path)?;
    let base = path.parent().unwrap_or(Path::new("."));
    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Import".into());
    import_bytes(format, &bytes, &name, Some(base))
}

/// Importe depuis la mémoire. `base_dir` sert à résoudre les fichiers
/// annexes (MTL, textures, `.bin`) ; `None` les ignore.
pub fn import_bytes(
    format: Format,
    bytes: &[u8],
    name: &str,
    base_dir: Option<&Path>,
) -> Result<Imported, IoError> {
    match format {
        Format::Glb | Format::Gltf => gltf::import(bytes, name, base_dir).map(Imported::Rig),
        Format::Fbx => fbx::import(bytes, name, base_dir).map(Imported::Rig),
        Format::Obj => obj::import(bytes, base_dir).map(Imported::Mesh),
        Format::Stl => stl::import(bytes).map(Imported::Mesh),
        Format::Ply => ply::import(bytes).map(Imported::Mesh),
        Format::Bvh => bvh::import(bytes, name).map(Imported::Rig),
    }
}

/// Lit toutes les animations d'un fichier d'échange sur son squelette, pour
/// l'import de préréglages (un préréglage par animation). Un fichier glTF ou
/// FBX peut contenir plusieurs animations nommées ; un BVH en contient une.
pub(crate) fn read_animations(
    format: Format,
    bytes: &[u8],
    base: Option<&Path>,
) -> Result<(crate::Skeleton, Vec<(String, crate::Clip)>), IoError> {
    match format {
        Format::Glb | Format::Gltf => gltf::animations(bytes, base),
        Format::Fbx => fbx::animations(bytes),
        Format::Bvh => bvh::animations(bytes),
        _ => Err(IoError::Unsupported(format!(
            "{} animation import",
            format.extension().to_uppercase()
        ))),
    }
}

/// Exporte le document. Retourne la liste des fichiers écrits (le fichier
/// principal en premier, puis les annexes : MTL, textures, `.bin`).
pub fn export(rig: &Rig, format: Format, path: &Path) -> Result<Vec<PathBuf>, IoError> {
    if matches!(format, Format::Obj | Format::Stl | Format::Ply) && rig.mesh.indices.is_empty() {
        return Err(IoError::Unsupported(format!(
            "{} without a mesh",
            format.extension().to_uppercase()
        )));
    }
    let written = match format {
        Format::Glb => {
            std::fs::write(path, gltf::export_glb(rig))?;
            vec![path.to_path_buf()]
        }
        Format::Gltf => gltf::export_gltf(rig, path)?,
        Format::Obj => obj::export(rig, path)?,
        Format::Stl => {
            std::fs::write(path, stl::export(rig))?;
            vec![path.to_path_buf()]
        }
        Format::Ply => {
            std::fs::write(path, ply::export(rig))?;
            vec![path.to_path_buf()]
        }
        Format::Bvh => {
            std::fs::write(path, bvh::export(rig)?)?;
            vec![path.to_path_buf()]
        }
        Format::Fbx => return Err(IoError::Unsupported("FBX export".into())),
    };
    Ok(written)
}

fn unsupported(path: &Path) -> IoError {
    IoError::Unsupported(format!("extension of \"{}\"", path.display()))
}

/// Nom de fichier sûr pour une annexe (texture, MTL).
pub(crate) fn safe_file_name(name: &str, fallback: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || "-_.".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    if clean.trim_matches(['_', '.']).is_empty() {
        fallback.to_owned()
    } else {
        clean
    }
}
