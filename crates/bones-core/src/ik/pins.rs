//! Points épinglés : maintien d'extrémités pendant la pose.

use glam::Vec3;
use serde::{Deserialize, Serialize};

use super::{IkSettings, solve_fabrik};
use crate::{BoneId, Pose, Skeleton};

/// Point épinglé (« fulcrum » dans Cascadeur) : la queue de l'os doit rester
/// à `target` pendant que l'on pose le reste du corps.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    pub bone: BoneId,
    pub target: Vec3,
}

/// Ramène chaque os épinglé sur sa cible par FABRIK sur `chain_len` os,
/// sans jamais bouger les os de `locked`.
pub fn solve_pins(skeleton: &Skeleton, pose: &mut Pose, pins: &[Pin], chain_len: usize, locked: &[BoneId]) {
    for pin in pins {
        if !skeleton.contains(pin.bone) {
            continue;
        }
        let chain: Vec<BoneId> = skeleton
            .chain(pin.bone, chain_len)
            .into_iter()
            .skip_while(|b| locked.contains(b))
            .collect();
        if chain.iter().any(|b| locked.contains(b)) {
            continue;
        }
        solve_fabrik(skeleton, pose, &chain, pin.target, IkSettings::default());
    }
}
