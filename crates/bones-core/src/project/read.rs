//! Ouverture d'un projet : manifeste (version), document, tableaux binaires
//! et textures, puis assemblage vérifié.

use glam::Vec3;
use std::path::Path;
use std::sync::Arc;

use bytemuck::Pod;
use rayon::prelude::*;

use super::codec::decode_indices;
use super::container::{Archive, MAX_JSON, MAX_MANIFEST, MAX_TEXTURE, MAX_THUMBNAIL};
use super::document::*;
use super::error::invalid;
use super::objects::ObjectCounts;
use super::validate::{Arrays, ObjectArrays, assemble};
use super::{LoadedProject, ProjectError, ProjectInfo};

/// Lecture rapide : manifeste + vignette encodée, sans décompresser le reste.
pub(super) fn read_info(path: &Path) -> Result<(ProjectInfo, Option<Vec<u8>>), ProjectError> {
    let mut archive = Archive::open(path)?;
    let manifest = read_manifest(&mut archive)?;
    let thumbnail = archive.read(THUMBNAIL, MAX_THUMBNAIL)?;
    Ok((manifest.info(project_name(path)), thumbnail))
}

pub(super) fn load(path: &Path) -> Result<LoadedProject, ProjectError> {
    load_archive(Archive::open(path)?, &project_name(path))
}

/// Projet complet lu en mémoire ; `name` remplace le nom de fichier.
pub(super) fn load_bytes(bytes: &[u8], name: &str) -> Result<LoadedProject, ProjectError> {
    load_archive(Archive::from_bytes(Arc::from(bytes))?, name)
}

fn load_archive(mut archive: Archive, name: &str) -> Result<LoadedProject, ProjectError> {
    let manifest = read_manifest(&mut archive)?;
    let thumbnail = archive.read(THUMBNAIL, MAX_THUMBNAIL)?;
    let json = archive
        .read(DOCUMENT, MAX_JSON)?
        .ok_or_else(|| invalid("missing document"))?;
    let document: Document = serde_json::from_slice(&json)?;
    drop(json);

    let c = document.counts;
    let poses = c.check(document.rig.clip.keys().len(), document.rig.skeleton.len())?;
    super::objects::check_objects(&document.objects)?;
    let arrays = read_arrays(&archive, &c, poses)?;
    let object_arrays = read_object_arrays(&archive, &document.objects)?;
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

    let snapshot = assemble(document, arrays, object_arrays, textures)?;
    let info = snapshot.info(name, manifest.created, manifest.modified);
    Ok(LoadedProject {
        info,
        snapshot,
        thumbnail,
    })
}

/// Tableaux binaires, décompressés en parallèle (un clone de l'archive par
/// tâche).
fn read_arrays(archive: &Archive, c: &Counts, poses: u32) -> Result<Arrays, ProjectError> {
    fn get<T: Pod + Send>(archive: &Archive, name: &str, count: u32) -> Result<Vec<T>, ProjectError> {
        archive.clone().read_array(name, count)
    }
    let indices = || get::<u8>(archive, INDICES, c.indices * 4).map(|b| decode_indices(&b));
    let ((positions, normals), ((uvs, indices), (weights, poses))) = rayon::join(
        || {
            rayon::join(
                || get(archive, POSITIONS, c.positions),
                || get(archive, NORMALS, c.normals),
            )
        },
        || {
            rayon::join(
                || rayon::join(|| get(archive, UVS, c.uvs), indices),
                || rayon::join(|| get(archive, WEIGHTS, c.weights), || get(archive, POSES, poses)),
            )
        },
    );
    Ok(Arrays {
        positions: positions?,
        normals: normals?,
        uvs: uvs?,
        indices: indices?,
        weights: weights?,
        poses: poses?,
        key_bones: c.key_bones as usize,
    })
}

/// Tableaux binaires de chaque objet, décompressés en parallèle.
fn read_object_arrays(archive: &Archive, counts: &[ObjectCounts]) -> Result<Vec<ObjectArrays>, ProjectError> {
    use super::objects::entry;
    counts
        .par_iter()
        .enumerate()
        .map(|(i, c)| {
            let a = |kind: &str, count: u32| -> Result<Vec<Vec3>, ProjectError> {
                archive.clone().read_array(&entry(i, kind), count)
            };
            let bytes: Vec<u8> = archive.clone().read_array(&entry(i, "indices"), c.indices * 4)?;
            Ok(ObjectArrays {
                positions: a("positions", c.positions)?,
                normals: a("normals", c.normals)?,
                uvs: archive.clone().read_array(&entry(i, "uvs"), c.uvs)?,
                indices: decode_indices(&bytes),
                weights: archive.clone().read_array(&entry(i, "weights"), c.weights)?,
            })
        })
        .collect()
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
