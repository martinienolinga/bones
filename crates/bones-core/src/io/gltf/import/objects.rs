//! Import glTF en objets séparés : un objet par nœud maillé, hiérarchie
//! des nœuds conservée. Un maillage skinné garde ses poids (placé dans
//! l'espace de liaison) ; un maillage rigide sous une articulation est
//! attaché à l'os correspondant, relativement à l'os dans la pose du fichier.

use std::path::Path;

use glam::Mat4;

use super::{load, main_skin, mesh, scene, skin_joints};
use crate::io::rigging::{build_skeleton, weights_from_joints};
use crate::io::{ImportedObject, ImportedParent, IoError, SceneImport};
use crate::{Mesh, Pose, Transform};

pub(crate) fn import_scene(bytes: &[u8], name: &str, base: Option<&Path>) -> Result<SceneImport, IoError> {
    let (doc, buffers) = load(bytes, base)?;
    let doc = &doc;
    let scene = scene(doc);
    let skin = main_skin(doc, &scene);
    let rig_data = skin.as_ref().map(|s| skin_joints(s, &scene, &buffers));
    let joint_nodes: Vec<usize> = skin.iter().flat_map(|s| s.joints().map(|j| j.index())).collect();
    let bind = rig_data.as_ref().map_or(Mat4::IDENTITY, |d| d.bind);
    let built = rig_data.as_ref().map(|d| build_skeleton(&d.joints));
    // Pose du fichier : celle des nœuds articulations dans la scène.
    let pose = built.as_ref().map_or_else(Pose::default, |b| {
        let worlds: Vec<Mat4> = joint_nodes.iter().map(|n| scene.world[*n]).collect();
        b.pose_from_worlds(&worlds)
    });
    let bone_world = built.as_ref().map_or_else(Vec::new, |b| pose.world(&b.skeleton));
    let materials = mesh::materials(doc, &buffers, base);

    let nodes: Vec<gltf::Node> = doc.nodes().collect();
    let mut object_of: Vec<Option<usize>> = vec![None; nodes.len()];
    let mut objects: Vec<ImportedObject> = Vec::new();
    for &n in &scene.order {
        let node = &nodes[n];
        let Some(gmesh) = node.mesh() else { continue };
        let skinned = skin.is_some() && node.skin().map(|s| s.index()) == skin.as_ref().map(|s| s.index());
        // Objet rigide : transformation rigide du nœud, son échelle (et tout
        // cisaillement) cuite dans le maillage.
        let world = if skinned {
            Transform::IDENTITY
        } else {
            let (_, r, t) = scene.world[n].to_scale_rotation_translation();
            Transform::from_rotation_translation(r.normalize(), t)
        };
        let matrix = if skinned {
            bind
        } else {
            world.to_matrix().inverse() * scene.world[n]
        };
        let mut part = Mesh {
            materials: materials.clone(),
            ..Mesh::default()
        };
        let mut raw = Vec::new();
        mesh::read_node(node, &buffers, skinned, matrix, None, &mut part, &mut raw)?;
        if part.triangle_count() == 0 {
            continue;
        }
        part.compact_materials();
        part.shrink_to_fit();
        let weights = built
            .as_ref()
            .filter(|_| skinned)
            .map(|b| weights_from_joints(b, &raw));

        // Parent : nœud maillé ou articulation le plus proche en remontant.
        let joint_bone = |c: usize| {
            let b = built.as_ref()?;
            joint_nodes.iter().position(|j| *j == c).map(|j| b.bone_of[j])
        };
        let mut parent = (ImportedParent::World, world);
        let mut cur = if joint_bone(n).is_some() {
            Some(n)
        } else {
            scene.parent[n]
        };
        while let Some(c) = cur.filter(|_| !skinned) {
            if let Some(o) = object_of[c].filter(|_| c != n) {
                parent = (ImportedParent::Object(o), objects[o].world.inverse().mul(&world));
                break;
            }
            if let Some(b) = joint_bone(c) {
                let bone = bone_world.get(b.index()).copied().unwrap_or_default();
                parent = (ImportedParent::Joint(b), bone.inverse().mul(&world));
                break;
            }
            cur = scene.parent[c];
        }
        object_of[n] = Some(objects.len());
        let name = node
            .name()
            .or(gmesh.name())
            .map_or_else(|| format!("Object {}", objects.len() + 1), str::to_owned);
        objects.push(ImportedObject {
            name,
            mesh: part,
            parent: parent.0,
            local: parent.1,
            world,
            weights,
        });
    }
    if objects.is_empty() && built.is_none() {
        return Err(super::err("no triangulated mesh"));
    }
    let clip = built
        .as_ref()
        .and_then(|b| super::anim::sample(doc, &buffers, &scene, &joint_nodes, b));
    Ok(SceneImport {
        name: name.to_owned(),
        skeleton: built.map(|b| b.skeleton),
        pose,
        clip,
        objects,
    })
}
