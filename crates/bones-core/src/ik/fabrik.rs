//! Cinématique inverse par FABRIK (Forward And Backward Reaching Inverse
//! Kinematics). Le solveur travaille sur les positions des articulations,
//! puis convertit le résultat en rotations de pose, de la racine vers le bout.

use glam::Vec3;

use crate::math::rotation_arc;
use crate::{BoneId, Pose, Skeleton, Transform, head_tail};

#[derive(Clone, Copy, Debug)]
pub struct IkSettings {
    pub iterations: usize,
    /// Distance en dessous de laquelle la cible est considérée atteinte.
    pub tolerance: f32,
}

impl Default for IkSettings {
    fn default() -> Self {
        Self {
            iterations: 24,
            tolerance: 1e-4,
        }
    }
}

/// Oriente la chaîne `chain` (de la racine vers le bout, chaque os étant le
/// parent du suivant) pour que la queue du dernier os atteigne `target`.
///
/// La tête du premier os reste fixe. Retourne `true` si la cible est atteinte.
pub fn solve_fabrik(
    skeleton: &Skeleton,
    pose: &mut Pose,
    chain: &[BoneId],
    target: Vec3,
    settings: IkSettings,
) -> bool {
    if chain.is_empty() {
        return false;
    }
    debug_assert!(
        chain
            .windows(2)
            .all(|w| skeleton.bone(w[1]).parent() == Some(w[0])),
        "la chaîne IK doit suivre la hiérarchie"
    );

    let world = pose.world(skeleton);
    let mut points: Vec<Vec3> = chain.iter().map(|b| world[b.index()].translation).collect();
    let last = *chain.last().unwrap();
    points.push(head_tail(&world[last.index()], skeleton.bone(last).length).1);

    let lengths: Vec<f32> = points.windows(2).map(|w| (w[1] - w[0]).length()).collect();
    let total: f32 = lengths.iter().sum();
    let root = points[0];
    let n = lengths.len();

    let reached = if (target - root).length() >= total {
        // Hors de portée : la chaîne s'étire en ligne droite vers la cible.
        let dir = (target - root).try_normalize().unwrap_or(Vec3::Y);
        for i in 0..n {
            points[i + 1] = points[i] + dir * lengths[i];
        }
        (target - root).length() - total <= settings.tolerance
    } else {
        let mut reached = false;
        for _ in 0..settings.iterations {
            // Passe arrière : le bout est placé sur la cible.
            points[n] = target;
            for i in (0..n).rev() {
                let dir = direction(points[i] - points[i + 1]);
                points[i] = points[i + 1] + dir * lengths[i];
            }
            // Passe avant : la racine revient à sa place.
            points[0] = root;
            for i in 0..n {
                let dir = direction(points[i + 1] - points[i]);
                points[i + 1] = points[i] + dir * lengths[i];
            }
            if (points[n] - target).length() <= settings.tolerance {
                reached = true;
                break;
            }
        }
        reached
    };

    // Conversion des positions en rotations, os par os.
    for (i, &bone) in chain.iter().enumerate() {
        let world = pose.world(skeleton);
        let head = world[bone.index()].translation;
        let current_next = match chain.get(i + 1) {
            Some(next) => world[next.index()].translation,
            None => head_tail(&world[bone.index()], skeleton.bone(bone).length).1,
        };
        let parent_world = skeleton
            .bone(bone)
            .parent()
            .map(|p| world[p.index()])
            .unwrap_or(Transform::IDENTITY);
        let rotation = rotation_arc(current_next - head, points[i + 1] - head);
        pose.rotate_world(skeleton, bone, &parent_world, rotation);
    }

    reached
}

fn direction(v: Vec3) -> Vec3 {
    v.try_normalize().unwrap_or(Vec3::Y)
}
