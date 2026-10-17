//! Biovision Hierarchy (BVH) : squelette et capture de mouvement.
//!
//! À l'import, les articulations deviennent des os (queue vers l'enfant
//! principal ou vers l'« End Site ») et chaque image devient une clé. Les
//! fichiers en centimètres (ou en pouces) sont ramenés à une taille humaine
//! en mètres.

mod export;
mod parse;

use glam::{Mat4, Vec3};

pub(super) use export::export;
use parse::Bvh;

use super::IoError;
use super::rigging::{Joint, assemble, build_skeleton};
use crate::{Mesh, Rig};

/// Échelle ramenant le squelette en mètres (taille d'un humain ≈ 1,75 m).
fn unit_scale(bvh: &Bvh) -> f32 {
    let worlds = joint_worlds(bvh, None, 1.0);
    let points: Vec<Vec3> = worlds
        .iter()
        .zip(&bvh.joints)
        .flat_map(|(w, j)| {
            let head = w.w_axis.truncate();
            [Some(head), j.end_site.map(|o| w.transform_point3(o))]
        })
        .flatten()
        .collect();
    let height = points.iter().map(|p| p.y).fold(f32::MIN, f32::max)
        - points.iter().map(|p| p.y).fold(f32::MAX, f32::min);
    match height {
        h if h > 50.0 => 0.01,
        h if h > 3.0 => 1.75 / h,
        _ => 1.0,
    }
}

/// Matrices monde des articulations (repos si `frame` vaut `None`).
fn joint_worlds(bvh: &Bvh, frame: Option<usize>, scale: f32) -> Vec<Mat4> {
    let mut worlds: Vec<Mat4> = Vec::with_capacity(bvh.joints.len());
    for (i, j) in bvh.joints.iter().enumerate() {
        let local = bvh.local(i, frame, scale);
        let w = match j.parent {
            Some(p) => worlds[p] * local,
            None => local,
        };
        worlds.push(w);
    }
    worlds
}

pub(super) fn import(bytes: &[u8], name: &str) -> Result<Rig, IoError> {
    let text = String::from_utf8_lossy(bytes);
    let bvh = parse::parse(&text)?;
    let scale = unit_scale(&bvh);
    let frames = bvh.frame_count();

    // Pose de repos : rotations nulles ; la racine est posée à sa position
    // de la première image si son décalage est nul (cas le plus courant).
    let mut rest = joint_worlds(&bvh, None, scale);
    if frames > 0 && bvh.joints[0].offset == Vec3::ZERO {
        let first = joint_worlds(&bvh, Some(0), scale);
        let lift = Mat4::from_translation(first[0].w_axis.truncate());
        rest = rest.into_iter().map(|w| lift * w).collect();
    }
    let joints: Vec<Joint> = bvh
        .joints
        .iter()
        .zip(&rest)
        .map(|(j, w)| Joint {
            name: j.name.clone(),
            parent: j.parent,
            world: *w,
            leaf_tail: j.end_site.map(|o| w.transform_point3(o * scale)),
        })
        .collect();
    let built = build_skeleton(&joints);
    let clip = (frames > 0).then(|| {
        built.clip(1.0 / bvh.frame_time, frames as u32, |f| {
            joint_worlds(&bvh, Some(f as usize), scale)
        })
    });
    Ok(assemble(name, Mesh::default(), Some(built), None, clip))
}
