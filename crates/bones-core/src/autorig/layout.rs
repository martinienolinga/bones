//! Disposition des articulations du squelette humanoïde : position monde de
//! chaque articulation (côté gauche, le droit s'en déduit par symétrie).
//!
//! Elle se calcule d'abord d'après les marqueurs, avec des proportions
//! génériques, puis peut être ajustée au volume du maillage
//! ([`super::fit_layout`]) avant la création des os.

use glam::Vec3;

use super::{AutoRigOptions, Markers};

/// Noms des doigts, dans l'ordre de [`Layout::fingers`].
pub const FINGER_NAMES: [&str; 5] = ["thumb", "index", "middle", "ring", "pinky"];

/// Positions des articulations. Les membres sont donnés côté gauche (+X).
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    /// Plan de symétrie x = `center_x` (axe du corps).
    pub center_x: f32,
    /// Tête de l'os racine (bassin).
    pub root: Vec3,
    /// Articulations de la colonne, du haut du bassin à la base du cou
    /// (un point de plus que d'os).
    pub spine: Vec<Vec3>,
    /// Articulations du cou, de sa base au menton (un point de plus que d'os).
    pub neck: Vec<Vec3>,
    pub head_top: Vec3,
    /// Tête de l'os de l'épaule (clavicule), près de l'axe du corps.
    pub clavicle: Vec3,
    /// Articulation de l'épaule (tête du bras).
    pub shoulder: Vec3,
    pub elbow: Vec3,
    pub wrist: Vec3,
    /// Queue de l'os de la main (base des doigts).
    pub hand_tip: Vec3,
    /// Pouce, index, majeur, annulaire, auriculaire : base puis les trois
    /// articulations suivantes (la dernière est le bout du doigt).
    pub fingers: [[Vec3; 4]; 5],
    pub hip: Vec3,
    pub knee: Vec3,
    pub ankle: Vec3,
    /// Base des orteils (queue du pied).
    pub toe: Vec3,
    pub toe_tip: Vec3,
}

impl Layout {
    /// Disposition générique déduite des marqueurs.
    pub fn from_markers(markers: &Markers, options: &AutoRigOptions) -> Self {
        let m = markers;
        let height = (m.head_top.y - m.ankle.y).abs().max(1e-3);

        // Colonne et cou, répartis régulièrement.
        let spine_count = options.spine_count.clamp(1, 6) as usize;
        let hips_tail = m.root + (m.neck - m.root) * 0.2;
        let mut spine = vec![hips_tail];
        spine.extend(
            (0..spine_count)
                .map(|i| hips_tail + (m.neck - hips_tail) * ((i + 1) as f32 / spine_count as f32)),
        );
        let neck_count = options.neck_count.clamp(1, 3) as usize;
        let mut neck = vec![*spine.last().expect("colonne non vide")];
        neck.extend(
            (0..neck_count).map(|i| m.neck + (m.chin - m.neck) * ((i + 1) as f32 / neck_count as f32)),
        );

        // Bras : le coude est légèrement en arrière pour fixer le sens de flexion.
        let clavicle = Vec3::new(
            m.root.x + (m.shoulder.x - m.root.x) * 0.2,
            m.shoulder.y,
            m.shoulder.z,
        );
        let elbow = (m.shoulder + m.wrist) * 0.5 - Vec3::Z * 0.015 * height;
        let arm_dir = (m.wrist - elbow).normalize_or(Vec3::X);
        let hand_len = (m.wrist - elbow).length() * 0.42;

        // Jambe : le genou est légèrement en avant.
        let hip = Vec3::new(m.ankle.x, m.root.y - 0.02 * height, m.root.z);
        let knee = (hip + m.ankle) * 0.5 + Vec3::Z * 0.02 * height;
        let floor = m.ankle.y - 0.04 * height;
        let toe = Vec3::new(m.ankle.x, floor + 0.015 * height, m.ankle.z + 0.08 * height);

        Self {
            center_x: m.root.x,
            root: m.root,
            spine,
            neck,
            head_top: m.head_top,
            clavicle,
            shoulder: m.shoulder,
            elbow,
            wrist: m.wrist,
            hand_tip: m.wrist + arm_dir * hand_len,
            fingers: generic_fingers(m.wrist, arm_dir, hand_len),
            hip,
            knee,
            ankle: m.ankle,
            toe,
            toe_tip: toe + Vec3::Z * 0.04 * height,
        }
    }

    /// Hauteur approximative du personnage (sommet du crâne – cheville).
    pub fn height(&self) -> f32 {
        (self.head_top.y - self.ankle.y).abs().max(1e-3)
    }

    /// Point symétrique (côté droit) par rapport à l'axe du corps.
    pub fn mirror(&self, p: Vec3) -> Vec3 {
        Vec3::new(2.0 * self.center_x - p.x, p.y, p.z)
    }
}

/// Doigts à trois phalanges, répartis sur la largeur de la paume.
pub(super) fn generic_fingers(wrist: Vec3, dir: Vec3, hand_len: f32) -> [[Vec3; 4]; 5] {
    let palm_end = wrist + dir * hand_len * 0.55;
    // Axe de largeur de la main (vers l'avant du personnage, +Z).
    let side = dir.cross(Vec3::Y).normalize_or(Vec3::Z);
    let side = if side.z < 0.0 { -side } else { side };
    let shape: [(f32, f32); 5] = [
        (0.55, 0.30),
        (0.25, 0.42),
        (0.0, 0.45),
        (-0.22, 0.42),
        (-0.42, 0.35),
    ];
    let width = hand_len * 0.45;
    let mut fingers = [[Vec3::ZERO; 4]; 5];
    for (f, (offset, length)) in shape.into_iter().enumerate() {
        // Le pouce part de la base de la paume, en biais vers l'avant.
        let (base, direction) = if f == 0 {
            let base = wrist + dir * hand_len * 0.2 + side * width * offset;
            (base, (dir + side * 0.9 - Vec3::Y * 0.2).normalize_or(dir))
        } else {
            (palm_end + side * width * offset, dir)
        };
        let seg = hand_len * length / 3.0;
        fingers[f][0] = base;
        for i in 1..4 {
            fingers[f][i] = fingers[f][i - 1] + direction * seg;
        }
    }
    fingers
}
