//! Génération du squelette humanoïde à partir des marqueurs.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::{BoneId, Limb, Markers, Skeleton};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct AutoRigOptions {
    /// Nombre d'os de colonne (1 à 6).
    pub spine_count: u8,
    /// Nombre d'os de cou (1 à 3).
    pub neck_count: u8,
    pub fingers: bool,
    pub toes: bool,
}

impl Default for AutoRigOptions {
    fn default() -> Self {
        Self {
            spine_count: 3,
            neck_count: 1,
            fingers: true,
            toes: true,
        }
    }
}

fn mirror_about(p: Vec3, center_x: f32) -> Vec3 {
    Vec3::new(2.0 * center_x - p.x, p.y, p.z)
}

/// Génère le squelette humanoïde et ses membres à partir des marqueurs.
pub fn generate(markers: &Markers, options: &AutoRigOptions) -> (Skeleton, Vec<Limb>) {
    let m = markers;
    let mut s = Skeleton::new();
    let height = (m.head_top.y - m.ankle.y).abs().max(1e-3);

    // Bassin et colonne.
    let spine_count = options.spine_count.clamp(1, 6) as usize;
    let torso = m.neck - m.root;
    let hips_tail = m.root + torso * 0.2;
    let root = s.add_bone_world("root.x", None, m.root, hips_tail);
    let mut parent = root;
    let mut head = hips_tail;
    for i in 0..spine_count {
        let tail = hips_tail + (m.neck - hips_tail) * ((i + 1) as f32 / spine_count as f32);
        parent = s.add_bone_world(format!("spine_{:02}.x", i + 1), Some(parent), head, tail);
        head = tail;
    }
    let chest = parent;

    // Cou et tête.
    let neck_count = options.neck_count.clamp(1, 3) as usize;
    for i in 0..neck_count {
        let tail = m.neck + (m.chin - m.neck) * ((i + 1) as f32 / neck_count as f32);
        let name = if neck_count == 1 {
            "neck.x".to_owned()
        } else {
            format!("neck_{:02}.x", i + 1)
        };
        parent = s.add_bone_world(name, Some(parent), head, tail);
        head = tail;
    }
    s.add_bone_world("head.x", Some(parent), m.chin, m.head_top);

    let mut limbs = Vec::new();
    for (suffix, mirror) in [("l", false), ("r", true)] {
        let p = |v: Vec3| if mirror { mirror_about(v, m.root.x) } else { v };

        // Bras : le coude est légèrement en arrière pour fixer le sens de flexion.
        let shoulder_head = p(Vec3::new(
            m.root.x + (m.shoulder.x - m.root.x) * 0.2,
            m.shoulder.y,
            m.shoulder.z,
        ));
        let shoulder = s.add_bone_world(
            format!("shoulder.{suffix}"),
            Some(chest),
            shoulder_head,
            p(m.shoulder),
        );
        let elbow = p((m.shoulder + m.wrist) * 0.5 - Vec3::Z * 0.015 * height);
        let arm = s.add_bone_world(format!("arm.{suffix}"), Some(shoulder), p(m.shoulder), elbow);
        let forearm = s.add_bone_world(format!("forearm.{suffix}"), Some(arm), elbow, p(m.wrist));
        let arm_dir = (p(m.wrist) - elbow).normalize_or(Vec3::X);
        let hand_len = (p(m.wrist) - elbow).length() * 0.42;
        let hand_tail = p(m.wrist) + arm_dir * hand_len;
        let hand = s.add_bone_world(format!("hand.{suffix}"), Some(forearm), p(m.wrist), hand_tail);
        if options.fingers {
            add_fingers(&mut s, hand, p(m.wrist), arm_dir, hand_len, suffix);
        }
        limbs.push(Limb::new(
            format!("Bras {}", suffix.to_uppercase()),
            arm,
            forearm,
            Some(hand),
        ));

        // Jambe : le genou est légèrement en avant.
        let hip = p(Vec3::new(m.ankle.x, m.root.y - 0.02 * height, m.root.z));
        let knee = (hip + p(m.ankle)) * 0.5 + Vec3::Z * 0.02 * height;
        let thigh = s.add_bone_world(format!("thigh.{suffix}"), Some(root), hip, knee);
        let leg = s.add_bone_world(format!("leg.{suffix}"), Some(thigh), knee, p(m.ankle));
        let floor = m.ankle.y - 0.04 * height;
        let toe_base = p(Vec3::new(
            m.ankle.x,
            floor + 0.015 * height,
            m.ankle.z + 0.08 * height,
        ));
        let foot = s.add_bone_world(format!("foot.{suffix}"), Some(leg), p(m.ankle), toe_base);
        if options.toes {
            let toe_tip = toe_base + Vec3::Z * 0.04 * height;
            s.add_bone_world(format!("toes.{suffix}"), Some(foot), toe_base, toe_tip);
        }
        limbs.push(Limb::new(
            format!("Jambe {}", suffix.to_uppercase()),
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

/// Doigts à trois phalanges, répartis sur la largeur de la paume.
fn add_fingers(s: &mut Skeleton, hand: BoneId, wrist: Vec3, dir: Vec3, hand_len: f32, suffix: &str) {
    let palm_end = wrist + dir * hand_len * 0.55;
    // Axe de largeur de la main (vers l'avant du personnage, +Z).
    let side = dir.cross(Vec3::Y).normalize_or(Vec3::Z);
    let side = if side.z < 0.0 { -side } else { side };
    let fingers: [(&str, f32, f32); 5] = [
        ("thumb", 0.55, 0.30),
        ("index", 0.25, 0.42),
        ("middle", 0.0, 0.45),
        ("ring", -0.22, 0.42),
        ("pinky", -0.42, 0.35),
    ];
    for (name, offset, length) in fingers {
        let width = hand_len * 0.45;
        // Le pouce part de la base de la paume, en biais vers l'avant.
        let (base, direction) = if name == "thumb" {
            let base = wrist + dir * hand_len * 0.2 + side * width * offset;
            (base, (dir + side * 0.9 - Vec3::Y * 0.2).normalize_or(dir))
        } else {
            (palm_end + side * width * offset, dir)
        };
        let seg = hand_len * length / 3.0;
        let mut parent = hand;
        let mut head = base;
        for i in 1..=3 {
            let tail = head + direction * seg;
            parent = s.add_bone_world(format!("{name}{i}.{suffix}"), Some(parent), head, tail);
            head = tail;
        }
    }
}
