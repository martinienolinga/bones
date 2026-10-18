//! Assemblage et vérification des objets lus : géométrie et poids réinsérés,
//! matériaux retexturés, parents et identifiants assainis (un fichier venu
//! d'ailleurs ne doit jamais faire paniquer l'éditeur ni le rendu).

use std::sync::Arc;

use super::super::ProjectError;
use super::super::validate::{ObjectArrays, check_mesh};
use super::ObjectCounts;
use crate::scene::{Binding, ObjectId, Parent};
use crate::{Mesh, Rig, Skin, VertexWeights};

/// Réinsère les données lourdes dans les objets du document, puis assainit
/// la hiérarchie (identifiants uniques, parents valides, pas de cycle).
pub(crate) fn assemble_objects(
    rig: &mut Rig,
    arrays: Vec<ObjectArrays>,
    counts: &[ObjectCounts],
    textures: &[Arc<[u8]>],
) -> Result<(), ProjectError> {
    for (index, data) in arrays.into_iter().enumerate() {
        let Some(object) = rig.objects.get_mut(index) else {
            break;
        };
        let empty = Mesh::default();
        let lite = Arc::try_unwrap(std::mem::replace(&mut object.mesh, Arc::new(empty)))
            .unwrap_or_else(|shared| (*shared).clone());
        let mut material_textures = counts
            .get(index)
            .map(|c| c.material_textures.clone())
            .unwrap_or_default();
        material_textures.resize(lite.materials.len(), None);
        let mut mesh = Mesh {
            positions: data.positions,
            normals: data.normals,
            uvs: data.uvs,
            indices: data.indices,
            materials: lite.materials,
            submeshes: lite.submeshes,
        };
        apply_textures(&mut mesh, &material_textures, textures);
        check_mesh(&mut mesh)?;
        let weights_ok = object.binding().is_skinned() && data.weights.len() == mesh.vertex_count();
        let vertices = mesh.vertex_count();
        object.mesh = Arc::new(mesh);
        if let Binding::Skinned(s) = &mut object.binding {
            s.skin = Arc::new(if weights_ok {
                Skin {
                    weights: data.weights,
                }
            } else {
                Skin {
                    weights: vec![VertexWeights::default(); vertices],
                }
            });
        }
    }
    sanitize_hierarchy(rig);
    Ok(())
}

/// Replace les octets des textures de couleur dans les matériaux de l'objet.
fn apply_textures(mesh: &mut Mesh, material_textures: &[Option<u32>], textures: &[Arc<[u8]>]) {
    for (i, material) in mesh.materials.iter_mut().enumerate() {
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
}

/// Identifiants uniques et non nuls, parents existants et sans cycle (sinon
/// l'objet passe au monde), poids d'os existants.
fn sanitize_hierarchy(rig: &mut Rig) {
    let bones = rig.skeleton.len();
    let mut seen: Vec<ObjectId> = Vec::with_capacity(rig.objects.len());
    let mut next = rig
        .objects
        .iter()
        .map(|o| o.id().0)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    for i in 0..rig.objects.len() {
        let id = rig.objects[i].id();
        if id.0 == 0 || seen.contains(&id) {
            let fresh = ObjectId(next);
            next = next.saturating_add(1);
            rig.objects[i].id = fresh;
            seen.push(fresh);
        } else {
            seen.push(id);
        }
    }
    // Poids des os hors plage (fichier incohérent) : influence ignorée.
    for o in &mut rig.objects {
        if let Binding::Skinned(s) = &mut o.binding {
            let skin = Arc::make_mut(&mut s.skin);
            for w in &mut skin.weights {
                if w.bones
                    .iter()
                    .zip(&w.weights)
                    .any(|(b, w)| *w != 0 && *b as usize >= bones)
                {
                    *w = VertexWeights::default();
                }
            }
        }
    }
    // Parents : objet existant qui n'est pas un descendant (vérifié dans
    // l'ordre, ce qui empêche tout cycle), ou os en plage, sinon monde.
    let ids: Vec<ObjectId> = rig.objects.iter().map(|o| o.id()).collect();
    for i in 0..rig.objects.len() {
        let ok = match rig.objects[i].parent() {
            Parent::World => true,
            Parent::Bone(b) => b.index() < bones,
            Parent::Object(p) => ids.contains(&p) && !rig.is_object_ancestor(ids[i], p),
        };
        if !ok {
            rig.objects[i].parent = Parent::World;
        }
    }
}
