//! Assemblage et vérification d'un projet lu sur le disque. Le fichier peut
//! venir de n'importe où : indices, longueurs et valeurs sont contrôlés pour
//! que rien ne puisse ensuite faire paniquer l'éditeur ou le rendu.

use std::sync::Arc;

use glam::{Vec2, Vec3};
use rayon::prelude::*;

use super::ProjectError;
use super::codec::{PackedTransform, unpack};
use super::document::Document;
use super::error::invalid;
use super::finite::make_finite;
use super::snapshot::ProjectSnapshot;
use crate::{Clip, Mesh, Rig, Skin, VertexWeights};

/// Nombre maximal d'images d'animation.
pub(super) const MAX_FRAMES: u32 = 1 << 20;

/// Tableaux binaires lus à part.
pub(super) struct Arrays {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub indices: Vec<u32>,
    pub weights: Vec<VertexWeights>,
    /// Poses des clés, `key_bones` transformations par clé.
    pub poses: Vec<PackedTransform>,
    pub key_bones: usize,
}

/// Tableaux binaires d'un objet supplémentaire.
pub(super) struct ObjectArrays {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub indices: Vec<u32>,
    pub weights: Vec<VertexWeights>,
}

/// Reconstitue le projet : géométrie, poids et textures réinsérés dans le
/// document, puis vérifiés.
pub(super) fn assemble(
    document: Document,
    arrays: Arrays,
    object_arrays: Vec<ObjectArrays>,
    textures: Vec<Arc<[u8]>>,
) -> Result<ProjectSnapshot, ProjectError> {
    let Document {
        mut rig,
        counts: _,
        textures: _,
        material_textures,
        weights_bound,
        objects: object_counts,
        editor: mut state,
    } = document;

    make_finite(&mut rig);

    // Déstructuration exhaustive (voir l'écriture) : géométrie lue à part,
    // matériaux et parties venant du document.
    let lite = std::mem::take(&mut rig.mesh);
    let Mesh {
        positions: _,
        normals: _,
        indices: _,
        uvs: _,
        mut materials,
        submeshes,
    } = Arc::try_unwrap(lite).unwrap_or_else(|shared| (*shared).clone());
    for (i, material) in materials.iter_mut().enumerate() {
        let bytes = material_textures
            .get(i)
            .copied()
            .flatten()
            .and_then(|t| textures.get(t as usize));
        match (&mut material.base_color_texture, bytes) {
            (Some(texture), Some(bytes)) => texture.bytes = bytes.clone(),
            (texture, _) => *texture = None,
        }
    }
    let Arrays {
        positions,
        normals,
        uvs,
        indices,
        weights,
        poses,
        key_bones,
    } = arrays;
    if !rig.clip.is_empty() {
        let mut chunks = poses.chunks_exact(key_bones.max(1));
        for pose in rig.clip_mut().poses_mut() {
            pose.bones = match chunks.next() {
                Some(chunk) if key_bones > 0 => chunk.iter().map(unpack).collect(),
                _ => Vec::new(),
            };
        }
    }
    let mut mesh = Mesh {
        positions,
        normals,
        indices,
        uvs,
        materials,
        submeshes,
    };
    check_mesh(&mut mesh)?;
    rig.mesh = Arc::new(mesh);

    check_skeleton(&mut rig);
    let weights_ok = check_weights(&weights, rig.mesh.vertex_count(), rig.skeleton.len());
    rig.skin = Arc::new(if weights_ok {
        Skin { weights }
    } else {
        Skin {
            weights: vec![VertexWeights::default(); rig.mesh.vertex_count()],
        }
    });
    // Révisions neuves (non sérialisées) : la liaison n'est redemandée que
    // si les poids n'étaient pas à jour, ou sont inutilisables.
    if !weights_bound || !weights_ok {
        rig.touch_skeleton();
    }
    check_clip(&mut rig);
    // Objets supplémentaires : géométrie, poids et hiérarchie vérifiés.
    super::objects::assemble_objects(&mut rig, object_arrays, &object_counts, &textures)?;
    state.sanitize(&rig);
    Ok(ProjectSnapshot { rig, state })
}

pub(super) fn check_mesh(mesh: &mut Mesh) -> Result<(), ProjectError> {
    let n = mesh.vertex_count();
    if !mesh.positions.par_iter().all(|p| p.is_finite()) {
        mesh.positions
            .par_iter_mut()
            .filter(|p| !p.is_finite())
            .for_each(|p| *p = Vec3::ZERO);
    }
    if !mesh.indices.len().is_multiple_of(3) || !mesh.indices.par_iter().all(|&i| (i as usize) < n) {
        return Err(invalid("mesh has out-of-range triangle indices"));
    }
    let len = mesh.indices.len() as u32;
    for part in &mut mesh.submeshes {
        let r = &part.indices;
        if r.start > r.end || r.end > len || !r.start.is_multiple_of(3) || !r.end.is_multiple_of(3) {
            return Err(invalid("mesh has invalid material parts"));
        }
        part.material = part.material.filter(|m| *m < mesh.materials.len());
    }
    // Données dérivées : réparées plutôt que refusées.
    if mesh.normals.len() != n || !mesh.normals.par_iter().all(|v| v.is_finite()) {
        mesh.compute_normals();
    }
    if !mesh.uvs.is_empty() && (mesh.uvs.len() != n || !mesh.uvs.par_iter().all(|v| v.is_finite())) {
        mesh.uvs = Vec::new();
    }
    Ok(())
}

fn check_skeleton(rig: &mut Rig) {
    // L'ordre parent < enfant est déjà vérifié à la désérialisation, le
    // nombre d'os avec les autres tailles, et les valeurs non finies
    // remplacées (`make_finite`).
    rig.pose.sync_len(&rig.skeleton);
    // Cibles et pôles gardés tels quels : l'éditeur les recale sur la pose en
    // passant en mode Pose.
    rig.limbs.retain(|l| l.is_valid(&rig.skeleton));
    rig.pins.retain(|p| rig.skeleton.contains(p.bone));
}

/// Poids utilisables tels quels : un par sommet, os existants.
fn check_weights(weights: &[VertexWeights], vertices: usize, bones: usize) -> bool {
    weights.len() == vertices
        && weights.par_iter().all(|w| {
            w.bones
                .iter()
                .zip(&w.weights)
                .all(|(b, w)| *w == 0 || (*b as usize) < bones)
        })
}

/// Animation : réglages bornés, clés triées sans doublon, une pose complète
/// par clé.
fn check_clip(rig: &mut Rig) {
    let clip = &rig.clip;
    let fps_ok = clip.fps.is_finite() && clip.fps > 0.0 && clip.fps <= 1000.0;
    let keys = clip.keys();
    let ordered = keys.windows(2).all(|w| w[0].frame < w[1].frame);
    let in_range = keys.last().is_none_or(|k| k.frame < MAX_FRAMES);
    if !fps_ok || !ordered || !in_range || clip.frame_count == 0 || clip.frame_count > MAX_FRAMES {
        let fps = if fps_ok { clip.fps } else { Clip::default().fps };
        let mut clean = Clip::new(fps, clip.frame_count.clamp(1, MAX_FRAMES));
        for key in keys.iter().filter(|k| k.frame < MAX_FRAMES) {
            clean.set_key(key.frame, key.pose.clone());
            if let Some(k) = clean.key_at_mut(key.frame) {
                k.interpolation = key.interpolation;
            }
        }
        rig.clip = Arc::new(clean);
    }
    if rig
        .clip
        .keys()
        .iter()
        .any(|k| k.pose.bones.len() != rig.skeleton.len())
    {
        let skeleton = rig.skeleton.clone();
        rig.clip_mut().sync_len(&skeleton);
    }
}
