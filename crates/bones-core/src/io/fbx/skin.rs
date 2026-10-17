//! Squelette et poids FBX : os (`LimbNode` et cibles des `Cluster`), pose de
//! liaison (`TransformLink`) et influences par sommet de contrôle.

use std::collections::HashMap;

use glam::DMat4;

use super::document::{Document, Object, child_attr};
use super::models::Models;
use crate::io::rigging::Joint;

pub(super) struct Rigging {
    pub joints: Vec<Joint>,
    /// Identifiant FBX du modèle → indice d'articulation.
    pub index: HashMap<i64, usize>,
    /// Identifiants des modèles articulations, dans l'ordre de `joints`.
    pub ids: Vec<i64>,
}

pub(super) fn matrix(node: &fbxcel::tree::v7400::NodeHandle, name: &str) -> Option<DMat4> {
    let v = child_attr(node, name)?.get_arr_f64()?;
    (v.len() >= 16).then(|| DMat4::from_cols_slice(&v[..16]))
}

/// Os ciblé par un cluster.
fn cluster_bone(doc: &Document, cluster: &Object) -> Option<i64> {
    doc.children_of(cluster.id, "Model").first().map(|m| m.id)
}

pub(super) fn rigging(doc: &Document, models: &Models, axis: DMat4) -> Option<Rigging> {
    let mut link: HashMap<i64, DMat4> = HashMap::new();
    for cluster in doc.of_class("Deformer").filter(|d| d.subclass == "Cluster") {
        if let (Some(bone), Some(m)) = (cluster_bone(doc, cluster), matrix(&cluster.node, "TransformLink")) {
            link.insert(bone, m);
        }
    }
    let ids: Vec<i64> = models
        .order
        .iter()
        .copied()
        .filter(|id| link.contains_key(id) || models.info[id].subclass == "LimbNode")
        .collect();
    if ids.is_empty() {
        return None;
    }
    let index: HashMap<i64, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();

    // Pose de liaison : TransformLink, sinon dérivée du parent au repos.
    let mut bind: HashMap<i64, DMat4> = HashMap::new();
    let mut joints = Vec::with_capacity(ids.len());
    for &id in &ids {
        let m = &models.info[&id];
        let parent = models.nearest_in(id, &index);
        let world = link.get(&id).copied().unwrap_or_else(|| match parent {
            Some(p) => {
                let pid = ids[p];
                bind[&pid] * models.info[&pid].rest_global.inverse() * m.rest_global
            }
            None => m.rest_global,
        });
        bind.insert(id, world);
        joints.push(Joint {
            name: m.name.clone(),
            parent,
            world: (axis * world).as_mat4(),
            leaf_tail: None,
        });
    }
    Some(Rigging { joints, index, ids })
}

/// Influences (articulation, poids) de chaque sommet de contrôle.
pub(super) type PointWeights = Vec<Vec<(usize, f32)>>;

/// Influences par sommet de contrôle d'une géométrie, et matrice monde du
/// maillage au moment de la liaison.
pub(super) fn weights(
    doc: &Document,
    geometry: i64,
    rigging: &Rigging,
    control_points: usize,
) -> Option<(PointWeights, Option<DMat4>)> {
    let skin = doc
        .children_of(geometry, "Deformer")
        .into_iter()
        .find(|d| d.subclass == "Skin")?;
    let mut per_point = vec![Vec::new(); control_points];
    let mut mesh_bind = None;
    for cluster in doc.children_of(skin.id, "Deformer") {
        let Some(joint) = cluster_bone(doc, cluster).and_then(|b| rigging.index.get(&b).copied()) else {
            continue;
        };
        // `Transform` est stocké dans l'espace de l'os (l'API officielle le
        // présente en global) : maillage lié = TransformLink · Transform.
        mesh_bind = mesh_bind.or_else(|| {
            let link = matrix(&cluster.node, "TransformLink")?;
            Some(link * matrix(&cluster.node, "Transform").unwrap_or(DMat4::IDENTITY))
        });
        let indexes = child_attr(&cluster.node, "Indexes").and_then(|a| a.get_arr_i32());
        let weights = child_attr(&cluster.node, "Weights").and_then(|a| a.get_arr_f64());
        if let (Some(indexes), Some(weights)) = (indexes, weights) {
            for (cp, w) in indexes.iter().zip(weights) {
                if let Some(slot) = usize::try_from(*cp).ok().and_then(|cp| per_point.get_mut(cp)) {
                    slot.push((joint, *w as f32));
                }
            }
        }
    }
    Some((per_point, mesh_bind))
}
