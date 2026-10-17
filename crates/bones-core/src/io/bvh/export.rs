//! Écriture BVH : Y vers le haut, rotations en ordre ZXY (degrés), une image
//! par image du clip (durée 1/fps).

use std::fmt::Write as _;

use glam::Quat;
use rayon::prelude::*;

use super::parse::euler_zxy_degrees;
use crate::io::IoError;
use crate::{BoneId, Pose, Rig, Skeleton, head_tail};

pub(in crate::io) fn export(rig: &Rig) -> Result<String, IoError> {
    let skeleton = &rig.skeleton;
    if skeleton.is_empty() {
        return Err(IoError::Unsupported("BVH sans squelette".into()));
    }
    let rest = skeleton.rest_world();
    let roots: Vec<BoneId> = skeleton
        .ids()
        .filter(|b| skeleton.bone(*b).parent().is_none())
        .collect();
    // Plusieurs racines : une articulation englobante est ajoutée.
    let wrapper = roots.len() > 1;

    let mut out = String::from("HIERARCHY\n");
    let mut order: Vec<BoneId> = Vec::with_capacity(skeleton.len());
    if wrapper {
        out.push_str("ROOT Root\n{\n\tOFFSET 0 0 0\n\tCHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation\n");
    }
    for root in &roots {
        write_joint(&mut out, skeleton, &rest, *root, usize::from(wrapper), &mut order);
    }
    if wrapper {
        out.push_str("}\n");
    }

    let clip = &rig.clip;
    let frames = if clip.is_empty() {
        1
    } else {
        clip.frame_count.max(1)
    };
    let rows: Vec<String> = (0..frames)
        .into_par_iter()
        .map_init(
            || Pose::rest(skeleton),
            |pose, f| {
                if clip.is_empty() {
                    pose.bones.clone_from(&rig.pose.bones);
                } else {
                    clip.sample_into(f as f32, pose);
                }
                frame_row(skeleton, &rest, pose, &order, wrapper)
            },
        )
        .collect();

    let _ = write!(
        out,
        "MOTION\nFrames: {frames}\nFrame Time: {:.6}\n",
        1.0 / clip.fps.max(1.0)
    );
    for row in rows {
        out.push_str(&row);
        out.push('\n');
    }
    Ok(out)
}

fn write_joint(
    out: &mut String,
    skeleton: &Skeleton,
    rest: &[crate::Transform],
    bone: BoneId,
    depth: usize,
    order: &mut Vec<BoneId>,
) {
    order.push(bone);
    let tabs = "\t".repeat(depth);
    let b = skeleton.bone(bone);
    let name: String = b
        .name
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect();
    let (head, tail) = head_tail(&rest[bone.index()], b.length);
    let is_root = b.parent().is_none();
    // Racines : décalage = position de repos, canaux de position absolus
    // (convention de Blender).
    let offset = match b.parent() {
        Some(p) => head - rest[p.index()].translation,
        None => head,
    };
    let keyword = if depth == 0 { "ROOT" } else { "JOINT" };
    let _ = writeln!(out, "{tabs}{keyword} {name}\n{tabs}{{");
    let _ = writeln!(
        out,
        "{tabs}\tOFFSET {:.6} {:.6} {:.6}",
        offset.x, offset.y, offset.z
    );
    if is_root {
        let _ = writeln!(
            out,
            "{tabs}\tCHANNELS 6 Xposition Yposition Zposition Zrotation Xrotation Yrotation"
        );
    } else {
        let _ = writeln!(out, "{tabs}\tCHANNELS 3 Zrotation Xrotation Yrotation");
    }
    let children: Vec<BoneId> = skeleton.children(bone).collect();
    if children.is_empty() {
        let end = tail - head;
        let _ = writeln!(
            out,
            "{tabs}\tEnd Site\n{tabs}\t{{\n{tabs}\t\tOFFSET {:.6} {:.6} {:.6}\n{tabs}\t}}",
            end.x, end.y, end.z
        );
    }
    for child in children {
        write_joint(out, skeleton, rest, child, depth + 1, order);
    }
    let _ = writeln!(out, "{tabs}}}");
}

/// Valeurs d'une image : la rotation d'articulation BVH est celle de l'os
/// privée de son orientation de repos (les repères BVH sont alignés sur le
/// monde au repos).
fn frame_row(
    skeleton: &Skeleton,
    rest: &[crate::Transform],
    pose: &Pose,
    order: &[BoneId],
    wrapper: bool,
) -> String {
    let world = pose.world(skeleton);
    let joint_rot: Vec<Quat> = world
        .iter()
        .zip(rest)
        .map(|(w, r)| (w.rotation * r.rotation.inverse()).normalize())
        .collect();
    let mut row = String::with_capacity(order.len() * 30);
    if wrapper {
        row.push_str("0 0 0 0 0 0");
    }
    for &b in order {
        let i = b.index();
        let local = match skeleton.bone(b).parent() {
            Some(p) => joint_rot[p.index()].inverse() * joint_rot[i],
            None => {
                let t = world[i].translation;
                let _ = write!(row, " {:.6} {:.6} {:.6}", t.x, t.y, t.z);
                joint_rot[i]
            }
        };
        let [z, x, y] = euler_zxy_degrees(local);
        let _ = write!(row, " {z:.5} {x:.5} {y:.5}");
    }
    row.trim_start().to_owned()
}
