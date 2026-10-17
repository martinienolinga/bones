//! Squelette humanoïde vu par les générateurs : os par rôle, géométrie de
//! repos (hanches, chevilles, talons, plantes de pied) et gestes de base
//! (bassin, colonne, regard, pieds en IK, bras, mains).

use glam::Vec3;

use crate::BoneId;
use crate::presets::error::{PresetError, incompatible};
use crate::presets::procedural::ctx::{Body, Rigging};
use crate::presets::roles::{Chain, Lr};

pub(crate) struct Leg {
    pub side: Lr,
    pub thigh: BoneId,
    pub shin: BoneId,
    pub foot: Option<BoneId>,
    pub toe: Option<BoneId>,
    /// Points de repos (monde) : cheville, plante (base des orteils) et
    /// talon, ces deux derniers posés au sol.
    pub ankle: Vec3,
    pub ball: Vec3,
    pub heel: Vec3,
    /// Cuisse + jambe.
    pub length: f32,
    /// Sens de flexion du genou au repos (monde).
    pub bend: Vec3,
}

pub(crate) struct Arm {
    pub side: Lr,
    pub clavicle: Option<BoneId>,
    pub upper: BoneId,
    pub fore: BoneId,
    pub hand: Option<BoneId>,
    pub fingers: Vec<Vec<BoneId>>,
    pub length: f32,
    /// Direction de flexion du coude au repos (monde), ⟂ au bras.
    pub flex: Vec3,
    /// Normale de la paume au repos (monde).
    pub palm: Vec3,
}

pub(crate) struct Human<'r, 'a> {
    pub rig: &'r Rigging<'a>,
    pub root: BoneId,
    pub spine: Vec<BoneId>,
    pub neck: Vec<BoneId>,
    pub head: Option<BoneId>,
    pub legs: [Leg; 2],
    pub arms: [Option<Arm>; 2],
    /// Hauteur du bassin au-dessus du sol au repos.
    pub hip_height: f32,
}

impl<'r, 'a> Human<'r, 'a> {
    pub fn new(rig: &'r Rigging<'a>) -> Result<Self, PresetError> {
        let root = rig.bone(Chain::Root, 0).ok_or_else(|| incompatible("no hips bone"))?;
        let leg = |side| -> Result<Leg, PresetError> {
            let chain = rig.chain(Chain::Leg(side));
            if chain.len() < 2 {
                return Err(incompatible(format!("{} leg not found", side.label().to_lowercase())));
            }
            Ok(Self::leg(rig, side, &chain))
        };
        let legs = [leg(Lr::Left)?, leg(Lr::Right)?];
        let arm = |side| Self::arm(rig, side);
        let hip_height = rig.height(rig.head(root)).max(0.05);
        Ok(Self {
            rig,
            root,
            spine: rig.chain(Chain::Spine),
            neck: rig.chain(Chain::Neck),
            head: rig.bone(Chain::Head, 0),
            legs,
            arms: [arm(Lr::Left), arm(Lr::Right)],
            hip_height,
        })
    }

    fn leg(rig: &Rigging, side: Lr, chain: &[BoneId]) -> Leg {
        let (thigh, shin) = (chain[0], chain[1]);
        let foot = chain.get(2).copied();
        let toe = chain.get(3).copied();
        let ankle = rig.tail(shin);
        let forward = rig.w(Vec3::Z);
        let ball = match foot {
            Some(f) => rig.on_ground(rig.tail(f)),
            None => rig.on_ground(ankle) + forward * 0.12 * rig.len(shin),
        };
        let foot_len = (ball - rig.on_ground(ankle)).length().max(0.05 * rig.len(shin));
        let heel = rig.on_ground(ankle) - forward * 0.25 * foot_len;
        let (hip, knee) = (rig.head(thigh), rig.head(shin));
        let axis = (ankle - hip).normalize_or(-rig.frame.up);
        let off = knee - hip;
        let bend = (off - axis * off.dot(axis))
            .try_normalize()
            .filter(|b| b.dot(forward) > -0.5)
            .unwrap_or(forward);
        Leg {
            side,
            thigh,
            shin,
            foot,
            toe,
            ankle,
            ball,
            heel,
            length: rig.len(thigh) + rig.len(shin),
            bend,
        }
    }

    fn arm(rig: &Rigging, side: Lr) -> Option<Arm> {
        let chain = rig.chain(Chain::Arm(side));
        let (&upper, &fore) = (chain.first()?, chain.get(1)?);
        let dir = rig.dir(upper);
        let forward = rig.w(Vec3::Z);
        let flex = (forward - dir * forward.dot(dir)).normalize_or(forward);
        let fingers: Vec<Vec<BoneId>> = (0..5)
            .map(|f| rig.chain(Chain::Finger(side, f)))
            .filter(|c| !c.is_empty())
            .collect();
        let hand = chain.get(2).copied();
        // Paume : du côté opposé au dos de la main (index → auriculaire × main).
        let index = rig.bone(Chain::Finger(side, 1), 0);
        let pinky = rig.bone(Chain::Finger(side, 4), 0).or(rig.bone(Chain::Finger(side, 3), 0));
        let palm = match (hand, index, pinky) {
            (Some(h), Some(i), Some(p)) => {
                let across = rig.head(i) - rig.head(p);
                (-side.sign() * across.cross(rig.dir(h))).normalize_or(-rig.frame.up)
            }
            _ => -rig.frame.up,
        };
        Some(Arm {
            side,
            clavicle: rig.bone(Chain::Clavicle(side), 0),
            upper,
            fore,
            hand,
            fingers,
            length: rig.len(upper) + rig.len(fore),
            flex,
            palm,
        })
    }

    pub fn leg_of(&self, side: Lr) -> &Leg {
        &self.legs[(side == Lr::Right) as usize]
    }

    pub fn arm_of(&self, side: Lr) -> Option<&Arm> {
        self.arms[(side == Lr::Right) as usize].as_ref()
    }

    /// Bassin : déplacement canonique et rotations (lacet autour de Y,
    /// tangage autour de X — positif penche vers l'avant —, roulis autour de Z).
    pub fn pelvis(&self, body: &mut Body, offset: Vec3, yaw: f32, pitch: f32, roll: f32) {
        body.pelvis(self.root, offset, yaw, pitch, roll);
    }

    /// Courbure de la colonne, répartie sur ses os.
    pub fn bend_spine(&self, body: &mut Body, pitch: f32, yaw: f32, roll: f32) {
        body.spread(&self.spine, pitch, yaw, roll);
    }

    /// Orientation monde de la tête (relative au repère du corps) après la
    /// cinématique directe : le cou fait la moitié du chemin.
    pub fn look(&self, body: &mut Body, pitch: f32, yaw: f32, roll: f32) {
        body.look(&self.neck, self.head, pitch, yaw, roll);
    }
}
