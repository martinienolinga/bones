//! Conversion des squelettes « à articulations » des autres logiciels
//! (glTF, FBX, BVH) vers notre modèle tête/queue, et assemblage du `Rig`.
//!
//! Chaque os est réorienté pour que son axe +Y vise son enfant principal
//! (indispensable à l'IK et à l'affichage). Cette correction est constante
//! dans le temps : les matrices de skinning W(t)·W(repos)⁻¹ n'en dépendent
//! pas, et les animations sont converties en passant par les matrices monde.

use std::sync::Arc;

use glam::{Mat4, Quat, Vec3};
use rayon::prelude::*;

use crate::math::rotation_arc;
use crate::{BoneId, Clip, Interpolation, Mesh, Pose, Rig, Skeleton, Skin, Transform, VertexWeights};

/// Articulation telle que lue dans un fichier.
pub(crate) struct Joint {
    pub name: String,
    pub parent: Option<usize>,
    /// Matrice monde de liaison (déjà convertie dans notre repère).
    pub world: Mat4,
    /// Bout de chaîne explicite (BVH « End Site »), en monde.
    pub leaf_tail: Option<Vec3>,
}

pub(crate) struct BuiltSkeleton {
    pub skeleton: Skeleton,
    /// Os correspondant à chaque articulation.
    pub bone_of: Vec<BoneId>,
    /// Rotation appliquée à droite du repère de l'articulation.
    correction: Vec<Quat>,
}

fn rigid(m: &Mat4) -> (Vec3, Quat) {
    let (_, r, t) = m.to_scale_rotation_translation();
    (t, r.normalize())
}

/// Construit le squelette (ordre parent avant enfant) depuis les articulations.
pub(crate) fn build_skeleton(joints: &[Joint]) -> BuiltSkeleton {
    let n = joints.len();
    let mut children = vec![Vec::new(); n];
    for (i, j) in joints.iter().enumerate() {
        if let Some(p) = j.parent.filter(|p| *p < n && *p != i) {
            children[p].push(i);
        }
    }
    let heads: Vec<Vec3> = joints.iter().map(|j| rigid(&j.world).0).collect();
    let extent = heads
        .iter()
        .fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| {
            (lo.min(*p), hi.max(*p))
        });
    let default_len = ((extent.1 - extent.0).length() * 0.04).max(1e-3);

    // Ordre topologique (parcours en profondeur depuis les racines).
    let mut order = Vec::with_capacity(n);
    let mut stack: Vec<usize> = (0..n)
        .rev()
        .filter(|i| joints[*i].parent.is_none_or(|p| p >= n || p == *i))
        .collect();
    while let Some(j) = stack.pop() {
        order.push(j);
        stack.extend(children[j].iter().rev());
    }

    let mut skeleton = Skeleton::new();
    let mut bone_of = vec![BoneId(0); n];
    let mut correction = vec![Quat::IDENTITY; n];
    let mut worlds = Vec::with_capacity(n);
    for &j in &order {
        let (head, rotation) = rigid(&joints[j].world);
        let y = rotation * Vec3::Y;
        let target = tail_target(j, &heads, &children[j], joints[j].leaf_tail, y);
        let (fix, length) = match target {
            Some(t) if (t - head).length() > 1e-6 => {
                let fix = rotation.inverse() * rotation_arc(y, t - head) * rotation;
                (fix, (t - head).length())
            }
            _ => {
                let parent_len = joints[j]
                    .parent
                    .filter(|p| *p < n)
                    .map(|p| skeleton.bone(bone_of[p]).length * 0.6);
                (Quat::IDENTITY, parent_len.unwrap_or(default_len).max(1e-3))
            }
        };
        correction[j] = fix.normalize();
        let parent = joints[j].parent.filter(|p| *p < n && *p != j).map(|p| bone_of[p]);
        bone_of[j] = skeleton.add_bone(joints[j].name.clone(), parent, Transform::IDENTITY, length);
        worlds.push(Transform::from_rotation_translation(
            (rotation * fix).normalize(),
            head,
        ));
    }
    skeleton.set_rest_world(&worlds);
    BuiltSkeleton {
        skeleton,
        bone_of,
        correction,
    }
}

/// Point visé par l'os : enfant dans l'axe, sinon moyenne des enfants.
fn tail_target(j: usize, heads: &[Vec3], children: &[usize], leaf: Option<Vec3>, y: Vec3) -> Option<Vec3> {
    let head = heads[j];
    match children {
        [] => leaf,
        [only] => Some(heads[*only]),
        many => many
            .iter()
            .map(|c| heads[*c])
            .find(|p| (*p - head).normalize_or_zero().dot(y) > 0.866)
            .or_else(|| Some(many.iter().map(|c| heads[*c]).sum::<Vec3>() / many.len() as f32)),
    }
}

impl BuiltSkeleton {
    /// Pose (deltas) correspondant aux matrices monde des articulations.
    pub fn pose_from_worlds(&self, joint_worlds: &[Mat4]) -> Pose {
        let n = self.skeleton.len();
        let mut bone_world = vec![Transform::IDENTITY; n];
        for (j, m) in joint_worlds.iter().enumerate().take(self.bone_of.len()) {
            let (t, r) = rigid(m);
            bone_world[self.bone_of[j].index()] =
                Transform::from_rotation_translation((r * self.correction[j]).normalize(), t);
        }
        let mut pose = Pose::rest(&self.skeleton);
        for (i, bone) in self.skeleton.bones().iter().enumerate() {
            let local = match bone.parent() {
                Some(p) => bone_world[p.index()].inverse().mul(&bone_world[i]),
                None => bone_world[i],
            };
            pose.bones[i] = bone.rest.inverse().mul(&local);
        }
        pose
    }

    /// Animation échantillonnée : `sample(image)` donne les matrices monde
    /// des articulations. Calcul parallèle, une image par tâche.
    pub fn clip(&self, fps: f32, frames: u32, sample: impl Fn(u32) -> Vec<Mat4> + Sync) -> Clip {
        let poses: Vec<Pose> = (0..frames)
            .into_par_iter()
            .map(|f| self.pose_from_worlds(&sample(f)))
            .collect();
        clip_from_poses(fps, poses)
    }
}

/// Une clé par image, interpolation linéaire (fidèle à l'échantillonnage).
pub(crate) fn clip_from_poses(fps: f32, poses: Vec<Pose>) -> Clip {
    let mut clip = Clip::new(fps.max(1.0), (poses.len() as u32).max(2));
    for (f, pose) in poses.into_iter().enumerate() {
        clip.set_key(f as u32, pose);
        if let Some(k) = clip.key_at_mut(f as u32) {
            k.interpolation = Interpolation::Linear;
        }
    }
    clip
}

/// Influences brutes (articulation, poids) → poids compacts par os.
pub(crate) fn weights_from_joints(built: &BuiltSkeleton, raw: &[Vec<(usize, f32)>]) -> Vec<VertexWeights> {
    raw.par_iter()
        .map(|inf| {
            VertexWeights::from_influences(
                inf.iter()
                    .filter(|(j, _)| *j < built.bone_of.len())
                    .map(|(j, w)| (built.bone_of[*j], *w))
                    .collect(),
            )
        })
        .collect()
}

/// Assemble un document à partir des éléments importés.
pub(crate) fn assemble(
    name: &str,
    mesh: Mesh,
    built: Option<BuiltSkeleton>,
    weights: Option<Vec<VertexWeights>>,
    clip: Option<Clip>,
) -> Rig {
    let mut rig = Rig::new(name, mesh);
    if let Some(built) = built {
        rig.set_skeleton(built.skeleton, Vec::new());
        let revs = rig.revisions();
        let usable =
            weights.filter(|w| w.len() == rig.mesh.vertex_count() && w.iter().any(|v| v.weights[0] > 0));
        match usable {
            Some(weights) => {
                rig.apply_skin(Skin { weights }, (revs.skeleton, revs.mesh));
            }
            // Maillage vide (BVH) : liaison immédiate et triviale.
            None if rig.mesh.vertex_count() == 0 => rig.bind(crate::BindMethod::Envelope),
            None => {}
        }
        if let Some(clip) = clip.filter(|c| !c.is_empty()) {
            let mut pose = Pose::rest(&rig.skeleton);
            clip.sample_into(0.0, &mut pose);
            rig.pose = pose;
            rig.clip = Arc::new(clip);
            rig.touch_clip();
        }
    }
    rig
}

/// Copie du maillage déformée par la pose courante (exports statiques).
pub(crate) fn posed_mesh(rig: &Rig) -> Mesh {
    let mut mesh = (*rig.mesh).clone();
    if rig.skeleton.is_empty() || rig.skin.weights.len() != mesh.vertex_count() {
        return mesh;
    }
    let matrices = rig.skinning_matrices();
    rig.skin
        .deform_into(&rig.mesh, &matrices, &mut mesh.positions, &mut mesh.normals);
    mesh
}
