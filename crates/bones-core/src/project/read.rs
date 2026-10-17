//! Ouverture d'un projet : manifeste (version), document, tableaux binaires
//! et textures, puis assemblage vérifié.

use std::path::Path;
use std::sync::Arc;

use super::codec::decode_indices;
use super::container::{Archive, MAX_JSON, MAX_MANIFEST, MAX_TEXTURE, MAX_THUMBNAIL};
use super::document::*;
use super::error::invalid;
use super::validate::{Arrays, assemble};
use super::{LoadedProject, ProjectError, ProjectInfo};

/// Nombre maximal de sommets d'un projet.
pub(super) const MAX_VERTICES: u32 = 1 << 23;
/// Nombre maximal d'indices de triangles.
pub(super) const MAX_INDICES: u32 = 3 << 24;
/// Nombre maximal de transformations de poses-clés (clés × os).
pub(super) const MAX_POSES: u64 = 1 << 23;

/// Lecture rapide : manifeste + vignette encodée, sans décompresser le reste.
pub(super) fn read_info(path: &Path) -> Result<(ProjectInfo, Option<Vec<u8>>), ProjectError> {
    let mut archive = Archive::open(path)?;
    let manifest = read_manifest(&mut archive)?;
    let thumbnail = archive.read(THUMBNAIL, MAX_THUMBNAIL)?;
    Ok((manifest.info(project_name(path)), thumbnail))
}

pub(super) fn load(path: &Path) -> Result<LoadedProject, ProjectError> {
    let mut archive = Archive::open(path)?;
    let manifest = read_manifest(&mut archive)?;
    let thumbnail = archive.read(THUMBNAIL, MAX_THUMBNAIL)?;
    let json = archive
        .read(DOCUMENT, MAX_JSON)?
        .ok_or_else(|| invalid("missing document"))?;
    let document: Document = serde_json::from_slice(&json)?;
    drop(json);

    let c = document.counts;
    let vertices = [c.positions, c.normals, c.uvs, c.weights];
    let poses = u64::from(c.key_bones) * document.rig.clip.keys().len() as u64;
    if vertices.iter().any(|n| *n > MAX_VERTICES) || c.indices > MAX_INDICES || poses > MAX_POSES {
        return Err(ProjectError::TooLarge(format!(
            "{} vertices, {} indices, {poses} key transforms",
            c.positions, c.indices
        )));
    }
    let arrays = Arrays {
        positions: archive.read_array(POSITIONS, c.positions)?,
        normals: archive.read_array(NORMALS, c.normals)?,
        uvs: archive.read_array(UVS, c.uvs)?,
        indices: decode_indices(&archive.read_array::<u8>(INDICES, c.indices * 4)?),
        weights: archive.read_array(WEIGHTS, c.weights)?,
        poses: archive.read_array(POSES, poses as u32)?,
        key_bones: c.key_bones as usize,
    };
    let textures = document
        .textures
        .iter()
        .map(|name| {
            let bytes = archive
                .read(name, MAX_TEXTURE)?
                .ok_or_else(|| invalid(format!("missing entry {name}")))?;
            Ok(Arc::<[u8]>::from(bytes))
        })
        .collect::<Result<Vec<_>, ProjectError>>()?;

    let snapshot = assemble(document, arrays, textures)?;
    let info = snapshot.info(&project_name(path), manifest.created, manifest.modified);
    Ok(LoadedProject {
        info,
        snapshot,
        thumbnail,
    })
}

pub(super) fn read_manifest(archive: &mut Archive) -> Result<Manifest, ProjectError> {
    let not_project = || invalid("not a PaintSharp Bones project");
    let bytes = archive.read(MANIFEST, MAX_MANIFEST)?.ok_or_else(not_project)?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|_| not_project())?;
    if manifest.format != FORMAT {
        return Err(not_project());
    }
    if manifest.version > VERSION {
        return Err(ProjectError::TooNew {
            version: manifest.version,
            supported: VERSION,
        });
    }
    Ok(manifest)
}

/// Nom du projet : celui du fichier (source de vérité, renommage instantané).
pub(super) fn project_name(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}
