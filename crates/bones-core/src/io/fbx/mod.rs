//! FBX binaire 7.x (Maya, 3ds Max, Blender, Mixamo…) via `fbxcel` :
//! maillages, matériaux et textures (intégrées ou externes), squelette,
//! poids de peau et animation. Le système d'axes et l'unité du fichier sont
//! convertis (Y en haut, mètres).
//!
//! Non pris en charge : FBX ASCII, formes de mélange (blend shapes),
//! tangentes de courbes (interpolation linéaire entre clés).

mod anim;
mod clips;
mod document;
mod geometry;
mod material;
mod models;
mod skin;
mod transform;

use std::io::Cursor;
use std::path::Path;

use fbxcel::tree::any::AnyTree;
use glam::{DMat3, DMat4, DVec3};

// Réexporté pour l'import de préréglages (crate::io::read_animations).
pub(crate) use clips::animations;
use document::Document;
use models::Models;

use crate::io::rigging::{assemble, build_skeleton, weights_from_joints};
use crate::io::{Format, IoError};
use crate::mesh::Submesh;
use crate::{Mesh, Rig};

fn err(message: impl std::fmt::Display) -> IoError {
    IoError::parse(Format::Fbx, message)
}

pub(crate) fn import(bytes: &[u8], name: &str, base: Option<&Path>) -> Result<Rig, IoError> {
    if bytes.starts_with(b"; FBX") {
        return Err(IoError::Unsupported("ASCII FBX (re-export it as binary)".into()));
    }
    let tree = match AnyTree::from_seekable_reader(Cursor::new(bytes)).map_err(err)? {
        AnyTree::V7400(_, tree, _) => tree,
        _ => return Err(IoError::Unsupported("unknown FBX version".into())),
    };
    let doc = Document::new(&tree);
    let (axis, fps) = transform::axis_system(&doc);
    let models = Models::read(&doc);
    let materials = material::read(&doc, base);
    let rigging = skin::rigging(&doc, &models, axis);

    let mut mesh = Mesh::default();
    let mut raw: Vec<Vec<(usize, f32)>> = Vec::new();
    let mut geometries: Vec<_> = doc
        .of_class("Geometry")
        .filter(|g| g.subclass == "Mesh")
        .collect();
    geometries.sort_by_key(|g| g.id);
    for geo in geometries {
        let Some(model) = doc.parents_of(geo.id, "Model").first().map(|m| m.id) else {
            continue;
        };
        let Some(info) = models.info.get(&model) else {
            continue;
        };
        let Some(mut part) = geometry::read(geo) else {
            continue;
        };

        let skinning = rigging
            .as_ref()
            .and_then(|r| skin::weights(&doc, geo.id, r, part.control_points));
        let placement = match &skinning {
            Some((_, Some(bind))) => *bind,
            _ => info.rest_global,
        };
        transform_part(&mut part.mesh, axis * placement * info.geometric);

        // Poids : peau, sinon liaison rigide à l'os parent le plus proche.
        let rigid = rigging.as_ref().and_then(|r| models.nearest_in(model, &r.index));
        raw.extend(part.control_point.iter().map(|cp| match &skinning {
            Some((per_point, _)) => per_point[*cp as usize].clone(),
            None => rigid.map(|j| vec![(j, 1.0)]).unwrap_or_default(),
        }));

        // Emplacements de matériaux du modèle → matériaux du document.
        let slots: Vec<Option<usize>> = doc
            .children_of(model, "Material")
            .iter()
            .map(|m| materials.index.get(&m.id).copied())
            .collect();
        part.mesh.submeshes = part
            .slots
            .iter()
            .map(|(range, slot)| Submesh {
                indices: range.clone(),
                material: slots.get(*slot).copied().flatten(),
            })
            .collect();
        mesh.append(&part.mesh);
    }
    mesh.materials = materials.list;
    if mesh.indices.is_empty() && rigging.is_none() {
        return Err(err("no mesh and no skeleton"));
    }
    mesh.shrink_to_fit();

    let Some(rigging) = rigging else {
        return Ok(assemble(name, mesh, None, None, None));
    };
    let built = build_skeleton(&rigging.joints);
    let weights = weights_from_joints(&built, &raw);
    let clip = anim::sample(&doc, &models, &rigging, axis, fps, &built);
    Ok(assemble(name, mesh, Some(built), Some(weights), clip))
}

/// Place la géométrie en monde ; une matrice miroir inverse les triangles.
fn transform_part(mesh: &mut Mesh, m: DMat4) {
    let m32 = m.as_mat4();
    for p in &mut mesh.positions {
        *p = m32.transform_point3(*p);
    }
    if m.determinant() < 0.0 {
        for tri in mesh.indices.as_chunks_mut::<3>().0 {
            tri.swap(1, 2);
        }
    }
    if mesh.normals.len() == mesh.positions.len() {
        let nm = DMat3::from_mat4(m).inverse().transpose();
        for n in &mut mesh.normals {
            *n = (nm * n.as_dvec3()).normalize_or(DVec3::Y).as_vec3();
        }
    } else {
        mesh.compute_normals();
    }
}
