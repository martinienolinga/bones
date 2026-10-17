//! Génération du squelette humanoïde à partir des marqueurs.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::Layout;
use super::layout::FINGER_NAMES;
use crate::{Limb, Markers, Mesh, Skeleton};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutoRigOptions {
    /// Nombre d'os de colonne (1 à 6).
    pub spine_count: u8,
    /// Nombre d'os de cou (1 à 3).
    pub neck_count: u8,
    pub fingers: bool,
    pub toes: bool,
    /// Recale les articulations sur le volume du maillage ([`generate_fitted`]).
    #[serde(default = "enabled")]
    pub fit_to_mesh: bool,
    /// Ajoute un rig facial (mâchoire, yeux, lèvres…) si la tête a un visage
    /// reconnaissable. Voir [`super::face`].
    #[serde(default = "enabled")]
    pub face: bool,
}

fn enabled() -> bool {
    true
}

impl Default for AutoRigOptions {
    fn default() -> Self {
        Self {
            spine_count: 3,
            neck_count: 1,
            fingers: true,
            toes: true,
            fit_to_mesh: true,
            face: true,
        }
    }
}

/// Génère le squelette humanoïde et ses membres à partir des marqueurs
/// (proportions génériques).
pub fn generate(markers: &Markers, options: &AutoRigOptions) -> (Skeleton, Vec<Limb>) {
    build(&Layout::from_markers(markers, options), options)
}

/// Comme [`generate`], puis, si `options.fit_to_mesh`, ajuste les
/// articulations au volume du maillage (centrage dans les membres, hanches,
/// coudes, genoux, pieds, doigts). Une partie mal reconnue garde sa position
/// générique : le résultat n'est jamais moins bon que celui de [`generate`].
pub fn generate_fitted(markers: &Markers, options: &AutoRigOptions, mesh: &Mesh) -> (Skeleton, Vec<Limb>) {
    let mut layout = Layout::from_markers(markers, options);
    if options.fit_to_mesh {
        super::fit_layout(&mut layout, mesh);
    }
    build(&layout, options)
}

/// Comme [`generate_fitted`], mais avec un garde-fou global : si l'ajustement
/// au maillage produit un squelette dégénéré (articulation non finie ou os de
/// longueur quasi nulle), on revient à la disposition générique des marqueurs
/// ([`generate`]). Un maillage inhabituel n'est ainsi jamais moins bien riggé
/// que par les seuls marqueurs. C'est l'entrée utilisée par l'éditeur.
pub fn generate_safe(markers: &Markers, options: &AutoRigOptions, mesh: &Mesh) -> (Skeleton, Vec<Limb>) {
    let fitted = generate_fitted(markers, options, mesh);
    if !options.fit_to_mesh || skeleton_sound(&fitted.0) {
        fitted
    } else {
        generate(markers, options)
    }
}

/// Un squelette est sain si toutes ses articulations sont finies et si aucun
/// os n'est de longueur quasi nulle.
fn skeleton_sound(skeleton: &Skeleton) -> bool {
    skeleton
        .rest_world()
        .iter()
        .all(|t| t.translation.is_finite() && t.rotation.is_finite())
        && skeleton
            .bones()
            .iter()
            .all(|b| b.length.is_finite() && b.length > 1e-4)
}

/// Crée les os et les membres IK/FK d'une disposition d'articulations.
pub fn build(layout: &Layout, options: &AutoRigOptions) -> (Skeleton, Vec<Limb>) {
    let l = layout;
    let mut s = Skeleton::new();

    // Bassin et colonne.
    let root = s.add_bone_world("root.x", None, l.root, l.spine[0]);
    let mut parent = root;
    for (i, pair) in l.spine.windows(2).enumerate() {
        parent = s.add_bone_world(format!("spine_{:02}.x", i + 1), Some(parent), pair[0], pair[1]);
    }
    let chest = parent;

    // Cou et tête.
    let single_neck = l.neck.len() == 2;
    for (i, pair) in l.neck.windows(2).enumerate() {
        let name = if single_neck {
            "neck.x".to_owned()
        } else {
            format!("neck_{:02}.x", i + 1)
        };
        parent = s.add_bone_world(name, Some(parent), pair[0], pair[1]);
    }
    let chin = *l.neck.last().expect("cou non vide");
    s.add_bone_world("head.x", Some(parent), chin, l.head_top);

    let mut limbs = Vec::new();
    for (suffix, mirror) in [("l", false), ("r", true)] {
        let p = |v: Vec3| if mirror { l.mirror(v) } else { v };

        let shoulder = s.add_bone_world(
            format!("shoulder.{suffix}"),
            Some(chest),
            p(l.clavicle),
            p(l.shoulder),
        );
        let arm = s.add_bone_world(format!("arm.{suffix}"), Some(shoulder), p(l.shoulder), p(l.elbow));
        let forearm = s.add_bone_world(format!("forearm.{suffix}"), Some(arm), p(l.elbow), p(l.wrist));
        let hand = s.add_bone_world(format!("hand.{suffix}"), Some(forearm), p(l.wrist), p(l.hand_tip));
        if options.fingers {
            for (name, joints) in FINGER_NAMES.iter().zip(&l.fingers) {
                let mut parent = hand;
                for (i, pair) in joints.windows(2).enumerate() {
                    let bone = format!("{name}{}.{suffix}", i + 1);
                    parent = s.add_bone_world(bone, Some(parent), p(pair[0]), p(pair[1]));
                }
            }
        }
        limbs.push(Limb::new(
            format!("Arm {}", suffix.to_uppercase()),
            arm,
            forearm,
            Some(hand),
        ));

        let thigh = s.add_bone_world(format!("thigh.{suffix}"), Some(root), p(l.hip), p(l.knee));
        let leg = s.add_bone_world(format!("leg.{suffix}"), Some(thigh), p(l.knee), p(l.ankle));
        let foot = s.add_bone_world(format!("foot.{suffix}"), Some(leg), p(l.ankle), p(l.toe));
        if options.toes {
            s.add_bone_world(format!("toes.{suffix}"), Some(foot), p(l.toe), p(l.toe_tip));
        }
        limbs.push(Limb::new(
            format!("Leg {}", suffix.to_uppercase()),
            thigh,
            leg,
            Some(foot),
        ));
    }

    let world = s.rest_world();
    for limb in &mut limbs {
        limb.snap_to_pose(&s, &world);
    }
    (s, limbs)
}
