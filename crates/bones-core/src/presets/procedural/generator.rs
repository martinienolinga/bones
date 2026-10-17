//! Liste des générateurs : nom, famille de squelette, squelette de
//! référence, paramètres et construction.

use serde::{Deserialize, Serialize};

use super::ctx::Rigging;
use super::{Motion, animal, human, misc, plant};
use crate::presets::error::PresetError;
use crate::presets::params::{ParamSpec, Params};
use crate::presets::reference::RefRig;
use crate::presets::roles::RigType;

/// Générateur procédural.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Generator {
    /// Attente : respiration, transferts de poids.
    HumanIdle,
    /// Regard à gauche puis à droite.
    HumanLookAround,
    /// Marche, jogging, course, sprint (paramètre `run`).
    HumanLocomotion,
    /// Saut avec préparation, vol balistique et réception.
    HumanJump,
    /// Salut de la main.
    HumanWave,
    /// Gestes de conversation.
    HumanTalk,
    /// S'accroupir puis se relever.
    HumanCrouch,
    /// S'asseoir (ou se relever).
    HumanSit,
    /// Pas chassé rythmé.
    HumanDance,
    /// Garde, direct avant, direct arrière.
    HumanPunch,
    /// Coup de pied de face.
    HumanKick,
    /// Pas, trot, galop (paramètre `gait`).
    QuadrupedGait,
    /// Quadrupède à l'arrêt.
    QuadrupedIdle,
    /// Quadrupède qui s'assoit.
    QuadrupedSit,
    /// Battement d'ailes.
    BirdFlap,
    /// Vol plané.
    BirdGlide,
    /// Sautillement.
    BirdHop,
    /// Picorage.
    BirdPeck,
    /// Nage par ondulation.
    FishSwim,
    /// Reptation latérale.
    SerpentSlither,
    /// Marche à six ou huit pattes.
    MultiLegWalk,
    /// Queue (ou chaîne) qui remue.
    TailWag,
    /// Végétal dans le vent (arbre, herbe, fleur, palmier, feuilles).
    PlantSway,
    /// Tentacule ondulant.
    Tentacle,
    /// Corde qui se balance.
    Rope,
    /// Pendule.
    Pendulum,
    /// Roues et pistons.
    Machine,
}

impl Generator {
    pub const ALL: [Generator; 27] = [
        Generator::HumanIdle,
        Generator::HumanLookAround,
        Generator::HumanLocomotion,
        Generator::HumanJump,
        Generator::HumanWave,
        Generator::HumanTalk,
        Generator::HumanCrouch,
        Generator::HumanSit,
        Generator::HumanDance,
        Generator::HumanPunch,
        Generator::HumanKick,
        Generator::QuadrupedGait,
        Generator::QuadrupedIdle,
        Generator::QuadrupedSit,
        Generator::BirdFlap,
        Generator::BirdGlide,
        Generator::BirdHop,
        Generator::BirdPeck,
        Generator::FishSwim,
        Generator::SerpentSlither,
        Generator::MultiLegWalk,
        Generator::TailWag,
        Generator::PlantSway,
        Generator::Tentacle,
        Generator::Rope,
        Generator::Pendulum,
        Generator::Machine,
    ];

    /// Nom affiché (anglais).
    pub fn label(self) -> &'static str {
        match self {
            Generator::HumanIdle => "Idle (human)",
            Generator::HumanLookAround => "Look around",
            Generator::HumanLocomotion => "Walk / run",
            Generator::HumanJump => "Jump",
            Generator::HumanWave => "Wave hand",
            Generator::HumanTalk => "Talking gestures",
            Generator::HumanCrouch => "Crouch",
            Generator::HumanSit => "Sit down / stand up",
            Generator::HumanDance => "Dance",
            Generator::HumanPunch => "Punches",
            Generator::HumanKick => "Front kick",
            Generator::QuadrupedGait => "Quadruped gait",
            Generator::QuadrupedIdle => "Idle (quadruped)",
            Generator::QuadrupedSit => "Sit (quadruped)",
            Generator::BirdFlap => "Wing flap",
            Generator::BirdGlide => "Glide",
            Generator::BirdHop => "Hop",
            Generator::BirdPeck => "Peck",
            Generator::FishSwim => "Swim",
            Generator::SerpentSlither => "Slither",
            Generator::MultiLegWalk => "Multi-legged walk",
            Generator::TailWag => "Tail wag",
            Generator::PlantSway => "Plant in the wind",
            Generator::Tentacle => "Tentacle wave",
            Generator::Rope => "Rope swing",
            Generator::Pendulum => "Pendulum",
            Generator::Machine => "Mechanical loop",
        }
    }

    /// Famille de squelette attendue.
    pub fn rig_type(self) -> RigType {
        match self {
            Generator::HumanIdle => RigType::Humanoid,
            Generator::HumanLookAround => RigType::Humanoid,
            Generator::HumanLocomotion => RigType::Humanoid,
            Generator::HumanJump => RigType::Humanoid,
            Generator::HumanWave => RigType::Humanoid,
            Generator::HumanTalk => RigType::Humanoid,
            Generator::HumanCrouch => RigType::Humanoid,
            Generator::HumanSit => RigType::Humanoid,
            Generator::HumanDance => RigType::Humanoid,
            Generator::HumanPunch => RigType::Humanoid,
            Generator::HumanKick => RigType::Humanoid,
            Generator::QuadrupedGait => RigType::Quadruped,
            Generator::QuadrupedIdle => RigType::Quadruped,
            Generator::QuadrupedSit => RigType::Quadruped,
            Generator::BirdFlap => RigType::Bird,
            Generator::BirdGlide => RigType::Bird,
            Generator::BirdHop => RigType::Bird,
            Generator::BirdPeck => RigType::Bird,
            Generator::FishSwim => RigType::Fish,
            Generator::SerpentSlither => RigType::Serpent,
            Generator::MultiLegWalk => RigType::MultiLegged,
            Generator::TailWag => RigType::Any,
            Generator::PlantSway => RigType::Plant,
            Generator::Tentacle => RigType::Chain,
            Generator::Rope => RigType::Chain,
            Generator::Pendulum => RigType::Chain,
            Generator::Machine => RigType::Any,
        }
    }

    /// Squelette d'aperçu et d'export.
    pub fn reference(self) -> RefRig {
        match self {
            Generator::HumanIdle => RefRig::Humanoid,
            Generator::HumanLookAround => RefRig::Humanoid,
            Generator::HumanLocomotion => RefRig::Humanoid,
            Generator::HumanJump => RefRig::Humanoid,
            Generator::HumanWave => RefRig::Humanoid,
            Generator::HumanTalk => RefRig::Humanoid,
            Generator::HumanCrouch => RefRig::Humanoid,
            Generator::HumanSit => RefRig::Humanoid,
            Generator::HumanDance => RefRig::Humanoid,
            Generator::HumanPunch => RefRig::Humanoid,
            Generator::HumanKick => RefRig::Humanoid,
            Generator::QuadrupedGait => RefRig::Quadruped,
            Generator::QuadrupedIdle => RefRig::Quadruped,
            Generator::QuadrupedSit => RefRig::Quadruped,
            Generator::BirdFlap => RefRig::Bird,
            Generator::BirdGlide => RefRig::Bird,
            Generator::BirdHop => RefRig::Bird,
            Generator::BirdPeck => RefRig::Bird,
            Generator::FishSwim => RefRig::Fish,
            Generator::SerpentSlither => RefRig::Serpent,
            Generator::MultiLegWalk => RefRig::Spider,
            Generator::TailWag => RefRig::Quadruped,
            Generator::PlantSway => RefRig::Tree,
            Generator::Tentacle => RefRig::Tentacle,
            Generator::Rope => RefRig::Rope,
            Generator::Pendulum => RefRig::Pendulum,
            Generator::Machine => RefRig::Machine,
        }
    }

    /// Paramètres réglables.
    pub fn params(self) -> &'static [ParamSpec] {
        match self {
            Generator::HumanIdle => human::idle::IDLE,
            Generator::HumanLookAround => human::idle::LOOK,
            Generator::HumanLocomotion => human::locomotion::PARAMS,
            Generator::HumanJump => human::jump::PARAMS,
            Generator::HumanWave => human::gestures::WAVE,
            Generator::HumanTalk => human::gestures::TALK,
            Generator::HumanCrouch => human::postures::CROUCH,
            Generator::HumanSit => human::postures::SIT,
            Generator::HumanDance => human::dance::PARAMS,
            Generator::HumanPunch => human::fight::PUNCH,
            Generator::HumanKick => human::fight::KICK,
            Generator::QuadrupedGait => animal::quadruped::PARAMS,
            Generator::QuadrupedIdle => animal::quad_rest::IDLE,
            Generator::QuadrupedSit => animal::quad_rest::SIT,
            Generator::BirdFlap => animal::bird::FLAP,
            Generator::BirdGlide => animal::bird::GLIDE,
            Generator::BirdHop => animal::bird::HOP,
            Generator::BirdPeck => animal::bird::PECK,
            Generator::FishSwim => animal::swim::SWIM,
            Generator::SerpentSlither => animal::swim::SLITHER,
            Generator::MultiLegWalk => animal::crawl::PARAMS,
            Generator::TailWag => misc::WAG,
            Generator::PlantSway => plant::PARAMS,
            Generator::Tentacle => misc::TENTACLE,
            Generator::Rope => misc::SWING,
            Generator::Pendulum => misc::SWING,
            Generator::Machine => misc::MACHINE,
        }
    }

    /// Cycle qui boucle (sinon action qui se joue une fois).
    pub fn looping(self) -> bool {
        !matches!(
            self,
            Generator::HumanJump | Generator::HumanCrouch | Generator::HumanSit | Generator::QuadrupedSit
        )
    }

    pub(crate) fn build<'r>(
        self,
        rig: &'r Rigging<'_>,
        p: &Params,
    ) -> Result<Box<dyn Motion + 'r>, PresetError> {
        Ok(match self {
            Generator::HumanIdle => Box::new(human::idle::Idle::new(rig, p)?),
            Generator::HumanLookAround => Box::new(human::idle::LookAround::new(rig, p)?),
            Generator::HumanLocomotion => Box::new(human::locomotion::Locomotion::new(rig, p)?),
            Generator::HumanJump => Box::new(human::jump::Jump::new(rig, p)?),
            Generator::HumanWave => Box::new(human::gestures::Wave::new(rig, p)?),
            Generator::HumanTalk => Box::new(human::gestures::Talk::new(rig, p)?),
            Generator::HumanCrouch => Box::new(human::postures::Crouch::new(rig, p)?),
            Generator::HumanSit => Box::new(human::postures::Sit::new(rig, p)?),
            Generator::HumanDance => Box::new(human::dance::Dance::new(rig, p)?),
            Generator::HumanPunch => Box::new(human::fight::Punch::new(rig, p)?),
            Generator::HumanKick => Box::new(human::fight::Kick::new(rig, p)?),
            Generator::QuadrupedGait => Box::new(animal::quadruped::Gait::new(rig, p)?),
            Generator::QuadrupedIdle => Box::new(animal::quad_rest::QuadIdle::new(rig, p)?),
            Generator::QuadrupedSit => Box::new(animal::quad_rest::QuadSit::new(rig, p)?),
            Generator::BirdFlap => Box::new(animal::bird::Flap::new(rig, p, false)?),
            Generator::BirdGlide => Box::new(animal::bird::Flap::new(rig, p, true)?),
            Generator::BirdHop => Box::new(animal::bird::Hop::new(rig, p)?),
            Generator::BirdPeck => Box::new(animal::bird::Peck::new(rig, p)?),
            Generator::FishSwim => Box::new(animal::swim::Swim::new(rig, p)?),
            Generator::SerpentSlither => Box::new(animal::swim::Slither::new(rig, p)?),
            Generator::MultiLegWalk => Box::new(animal::crawl::Crawl::new(rig, p)?),
            Generator::TailWag => Box::new(misc::Wave::new(rig, p, false)?),
            Generator::PlantSway => Box::new(plant::Sway::new(rig, p)?),
            Generator::Tentacle => Box::new(misc::Wave::new(rig, p, true)?),
            Generator::Rope => Box::new(misc::Swing::new(rig, p, true)?),
            Generator::Pendulum => Box::new(misc::Swing::new(rig, p, false)?),
            Generator::Machine => Box::new(misc::Machine::new(rig, p)?),
        })
    }
}
