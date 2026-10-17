//! Rôles des os : à quoi sert chaque os d'un squelette quelconque (bassin,
//! colonne, bras gauche…), pour appliquer une animation conçue sur un autre
//! squelette. Les rôles sont organisés en chaînes ([`Chain`]) numérotées de
//! la racine vers le bout ([`Role::index`]).
//!
//! [`detect`] devine le type de rig et les rôles : noms d'abord (conventions
//! Auto-Rig Pro, Rigify, Mixamo, Unreal, Unity, 3ds Max Biped, BVH CMU…),
//! puis topologie et géométrie de la pose de repos. L'interface peut ensuite
//! corriger chaque os à la main ([`RoleMap::set`]).

mod classify;
mod detect;
mod frame;
mod names;
mod topology;

#[cfg(test)]
mod tests;

use serde::{Deserialize, Serialize};

pub use detect::detect;
pub use frame::BodyFrame;

use crate::{BoneId, Skeleton};

/// Côté d'un membre.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Lr {
    Left,
    Right,
}

impl Lr {
    pub const BOTH: [Lr; 2] = [Lr::Left, Lr::Right];

    pub fn mirror(self) -> Lr {
        match self {
            Lr::Left => Lr::Right,
            Lr::Right => Lr::Left,
        }
    }

    /// +1 à gauche (+X canonique), −1 à droite.
    pub fn sign(self) -> f32 {
        match self {
            Lr::Left => 1.0,
            Lr::Right => -1.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lr::Left => "Left",
            Lr::Right => "Right",
        }
    }
}

/// Chaîne d'os ayant un rôle commun.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Chain {
    /// Os au sol sous le bassin (« root » d'Unreal) : trajectoire.
    Ground,
    /// Bassin : porte le déplacement du corps (un seul os).
    Root,
    /// Colonne, du bassin à la poitrine (tout le corps d'un poisson, d'un serpent).
    Spine,
    Neck,
    Head,
    Jaw,
    Tail,
    Clavicle(Lr),
    /// Membre antérieur : bras, patte avant, aile (segments : haut, bas, main, doigt).
    Arm(Lr),
    /// Membre postérieur : jambe, patte arrière (cuisse, jambe, pied, orteils).
    Leg(Lr),
    /// Doigt : 0 pouce, 1 index, 2 majeur, 3 annulaire, 4 auriculaire.
    Finger(Lr, u8),
    /// Pattes des animaux à plus de quatre pattes (paire 0 à l'avant).
    LegPair(u8, Lr),
    /// Chaîne libre (végétaux, tentacules, cordes).
    Branch(u16),
}

impl Chain {
    /// Chaîne symétrique (gauche ↔ droite).
    pub fn mirror(self) -> Chain {
        match self {
            Chain::Clavicle(s) => Chain::Clavicle(s.mirror()),
            Chain::Arm(s) => Chain::Arm(s.mirror()),
            Chain::Leg(s) => Chain::Leg(s.mirror()),
            Chain::Finger(s, f) => Chain::Finger(s.mirror(), f),
            Chain::LegPair(p, s) => Chain::LegPair(p, s.mirror()),
            c => c,
        }
    }

    pub fn side(self) -> Option<Lr> {
        match self {
            Chain::Clavicle(s)
            | Chain::Arm(s)
            | Chain::Leg(s)
            | Chain::Finger(s, _)
            | Chain::LegPair(_, s) => Some(s),
            _ => None,
        }
    }

    /// Membre dont les segments ont un sens fixe (pas de rééchantillonnage).
    pub fn is_limb(self) -> bool {
        matches!(
            self,
            Chain::Clavicle(_) | Chain::Arm(_) | Chain::Leg(_) | Chain::Finger(..) | Chain::LegPair(..)
        )
    }

    /// Nom affiché (anglais).
    pub fn label(self) -> String {
        let fingers = [
            "thumb",
            "index finger",
            "middle finger",
            "ring finger",
            "little finger",
        ];
        match self {
            Chain::Ground => "Ground".into(),
            Chain::Root => "Hips".into(),
            Chain::Spine => "Spine".into(),
            Chain::Neck => "Neck".into(),
            Chain::Head => "Head".into(),
            Chain::Jaw => "Jaw".into(),
            Chain::Tail => "Tail".into(),
            Chain::Clavicle(s) => format!("{} clavicle", s.label()),
            Chain::Arm(s) => format!("{} arm", s.label()),
            Chain::Leg(s) => format!("{} leg", s.label()),
            Chain::Finger(s, f) => format!("{} {}", s.label(), fingers.get(f as usize).unwrap_or(&"finger")),
            Chain::LegPair(p, s) => format!("{} leg {}", s.label(), p + 1),
            Chain::Branch(b) => format!("Chain {}", b + 1),
        }
    }
}

/// Rôle d'un os : chaîne et rang dans la chaîne (0 côté racine).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Role {
    pub chain: Chain,
    pub index: u8,
}

impl Role {
    pub fn new(chain: Chain, index: u8) -> Self {
        Self { chain, index }
    }

    /// Nom affiché : « Left arm 2 », « Hips »…
    pub fn label(self) -> String {
        match self.chain {
            Chain::Root | Chain::Ground | Chain::Head | Chain::Jaw => self.chain.label(),
            c => format!("{} {}", c.label(), self.index + 1),
        }
    }
}

/// Famille de squelette, pour choisir les préréglages compatibles.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RigType {
    Humanoid,
    Quadruped,
    Bird,
    Fish,
    Serpent,
    MultiLegged,
    Plant,
    Chain,
    /// Convient à tout squelette.
    #[default]
    Any,
}

impl RigType {
    pub const ALL: [RigType; 9] = [
        RigType::Humanoid,
        RigType::Quadruped,
        RigType::Bird,
        RigType::Fish,
        RigType::Serpent,
        RigType::MultiLegged,
        RigType::Plant,
        RigType::Chain,
        RigType::Any,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RigType::Humanoid => "Humanoid",
            RigType::Quadruped => "Quadruped",
            RigType::Bird => "Bird",
            RigType::Fish => "Fish",
            RigType::Serpent => "Serpent",
            RigType::MultiLegged => "Multi-legged",
            RigType::Plant => "Plant / tree",
            RigType::Chain => "Chain",
            RigType::Any => "Any",
        }
    }

    /// Un préréglage prévu pour `self` s'applique-t-il à un rig `target` ?
    pub fn fits(self, target: RigType) -> bool {
        self == target || matches!(self, RigType::Any | RigType::Chain)
    }
}

/// Rôles de tous les os d'un squelette, type de rig et repère du corps.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoleMap {
    pub rig_type: RigType,
    /// Rôle de chaque os (indice = [`BoneId`]).
    pub roles: Vec<Option<Role>>,
    pub frame: BodyFrame,
}

impl RoleMap {
    /// Aucun rôle (repère par défaut : face à +Z).
    pub fn empty(skeleton: &Skeleton) -> Self {
        Self {
            rig_type: RigType::Any,
            roles: vec![None; skeleton.len()],
            frame: BodyFrame::from_skeleton(skeleton),
        }
    }

    /// Correspond au squelette (même nombre d'os).
    pub fn fits(&self, skeleton: &Skeleton) -> bool {
        self.roles.len() == skeleton.len()
    }

    pub fn role(&self, bone: BoneId) -> Option<Role> {
        self.roles.get(bone.index()).copied().flatten()
    }

    /// Change le rôle d'un os ; l'os qui avait ce rôle le perd.
    pub fn set(&mut self, bone: BoneId, role: Option<Role>) {
        if let Some(r) = role {
            for slot in &mut self.roles {
                if *slot == Some(r) {
                    *slot = None;
                }
            }
        }
        if let Some(slot) = self.roles.get_mut(bone.index()) {
            *slot = role;
        }
    }

    pub fn bone(&self, chain: Chain, index: u8) -> Option<BoneId> {
        self.roles
            .iter()
            .position(|r| *r == Some(Role { chain, index }))
            .map(|i| BoneId(i as u32))
    }

    /// Os d'une chaîne, de la racine vers le bout.
    pub fn chain(&self, chain: Chain) -> Vec<BoneId> {
        let mut bones: Vec<(u8, BoneId)> = self
            .roles
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                r.filter(|r| r.chain == chain)
                    .map(|r| (r.index, BoneId(i as u32)))
            })
            .collect();
        bones.sort_unstable();
        bones.into_iter().map(|(_, b)| b).collect()
    }

    /// Chaînes présentes, triées.
    pub fn chains(&self) -> Vec<Chain> {
        let mut chains: Vec<Chain> = self.roles.iter().flatten().map(|r| r.chain).collect();
        chains.sort_unstable();
        chains.dedup();
        chains
    }

    /// Bassin (porteur du déplacement).
    pub fn root(&self) -> Option<BoneId> {
        self.bone(Chain::Root, 0)
    }

    pub fn mapped_count(&self) -> usize {
        self.roles.iter().flatten().count()
    }
}
