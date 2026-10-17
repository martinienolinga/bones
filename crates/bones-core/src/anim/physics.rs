//! Outils physiques inspirés de Cascadeur : centre de masse et correction
//! balistique des sauts. Unité : 1 unité = 1 mètre, Y vers le haut.

use glam::Vec3;
use rayon::prelude::*;

use crate::{Clip, Pose, Skeleton, Skin, Transform, head_tail};

pub const GRAVITY: f32 = 9.81;

/// Masse de chaque os. Avec un maillage lié, elle est proportionnelle aux
/// sommets qu'il entraîne ; sinon à sa longueur.
pub fn bone_masses(skeleton: &Skeleton, skin: &Skin) -> Vec<f32> {
    let from_skin = skin.bone_masses(skeleton.len());
    if from_skin.iter().sum::<f32>() > 0.0 {
        from_skin
    } else {
        skeleton.bones().iter().map(|b| b.length.max(1e-3)).collect()
    }
}

/// Centre de masse : moyenne des milieux des os pondérée par leur masse.
pub fn center_of_mass(skeleton: &Skeleton, world: &[Transform], masses: &[f32]) -> Option<Vec3> {
    let mut total = 0.0;
    let mut sum = Vec3::ZERO;
    for ((bone, w), m) in skeleton.bones().iter().zip(world).zip(masses) {
        let (head, tail) = head_tail(w, bone.length);
        sum += (head + tail) * 0.5 * *m;
        total += *m;
    }
    (total > 0.0).then(|| sum / total)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhysicsError {
    /// Il faut une clé au début et à la fin de l'intervalle.
    MissingKeys,
    IntervalTooShort,
}

impl std::fmt::Display for PhysicsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhysicsError::MissingKeys => write!(f, "a key is needed at the start and the end of the jump"),
            PhysicsError::IntervalTooShort => write!(f, "interval too short"),
        }
    }
}

impl std::error::Error for PhysicsError {}

/// Trajectoire balistique : pendant la phase aérienne `[start, end]`, le
/// centre de masse suit une parabole soumise à la gravité, passant par ses
/// positions aux deux clés. Les racines sont décalées image par image et le
/// résultat est cuit en clés. Retourne le nombre de clés écrites.
pub fn bake_ballistic(
    skeleton: &Skeleton,
    clip: &mut Clip,
    masses: &[f32],
    start: u32,
    end: u32,
    gravity: f32,
) -> Result<usize, PhysicsError> {
    if end <= start + 1 {
        return Err(PhysicsError::IntervalTooShort);
    }
    let (Some(k0), Some(k1)) = (clip.key_at(start), clip.key_at(end)) else {
        return Err(PhysicsError::MissingKeys);
    };
    let com_of = |pose: &Pose| center_of_mass(skeleton, &pose.world(skeleton), masses).unwrap_or(Vec3::ZERO);
    let p0 = com_of(&k0.pose);
    let p1 = com_of(&k1.pose);
    let g = Vec3::new(0.0, -gravity, 0.0);
    let duration = (end - start) as f32 / clip.fps;
    let v0 = (p1 - p0 - 0.5 * g * duration * duration) / duration;

    let roots: Vec<_> = skeleton
        .ids()
        .filter(|id| skeleton.bone(*id).parent().is_none())
        .collect();
    let frames: Vec<u32> = (start + 1..end).collect();

    let baked: Vec<(u32, Pose)> = frames
        .par_iter()
        .map(|&f| {
            let mut pose = Pose::rest(skeleton);
            clip.sample_into(f as f32, &mut pose);
            let t = (f - start) as f32 / clip.fps;
            let wanted = p0 + v0 * t + 0.5 * g * t * t;
            let offset = wanted - com_of(&pose);
            for &r in &roots {
                let rest = &skeleton.bone(r).rest;
                let local = rest.rotation.inverse() * offset / rest.scale;
                pose.bones[r.index()].translation += local;
            }
            (f, pose)
        })
        .collect();

    let count = baked.len();
    for (f, pose) in baked {
        clip.set_key(f, pose);
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn com_of_symmetric_pair_is_centered() {
        let mut s = Skeleton::new();
        s.add_bone_world("l", None, Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 2.0, 0.0));
        s.add_bone_world("r", None, Vec3::new(-1.0, 0.0, 0.0), Vec3::new(-1.0, 2.0, 0.0));
        let com = center_of_mass(&s, &s.rest_world(), &[1.0, 1.0]).unwrap();
        assert!(com.abs_diff_eq(Vec3::new(0.0, 1.0, 0.0), 1e-6));
    }

    #[test]
    fn ballistic_follows_parabola() {
        let mut s = Skeleton::new();
        s.add_bone_world("root", None, Vec3::ZERO, Vec3::Y);
        let masses = [1.0];
        let mut clip = Clip::new(10.0, 60);
        clip.set_key(0, Pose::rest(&s));
        let mut end = Pose::rest(&s);
        end.bones[0].translation = Vec3::new(2.0, 0.0, 0.0);
        clip.set_key(10, end);

        let n = bake_ballistic(&s, &mut clip, &masses, 0, 10, GRAVITY).unwrap();
        assert_eq!(n, 9);
        // À mi-parcours (t = 0,5 s), la hauteur vaut g·t·(T−t)/2 = 9,81·0,5·0,5/2.
        let mid = &clip.key_at(5).unwrap().pose;
        let com = center_of_mass(&s, &mid.world(&s), &masses).unwrap();
        let expected = 0.5 + GRAVITY * 0.5 * 0.5 / 2.0;
        assert!((com.y - expected).abs() < 1e-3, "{com:?}");
        assert!((com.x - 1.0).abs() < 1e-3);
    }
}
