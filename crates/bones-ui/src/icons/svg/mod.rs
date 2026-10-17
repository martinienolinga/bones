//! Fragments SVG des icônes, regroupés par famille d'outils.

mod anim;
mod files;
mod misc;
mod motion;
mod rig;
mod view;

use super::Icon;
use super::hue::{self, Hue};

/// Corps SVG et famille de teintes d'une icône.
pub fn source(icon: Icon) -> (&'static str, Hue) {
    match icon {
        Icon::Gallery => (files::GALLERY, hue::FILES),
        Icon::Actions => (misc::ACTIONS, hue::NEUTRAL),
        Icon::Adjust => (misc::ADJUST, hue::NEUTRAL),
        Icon::Bone => (rig::BONE, hue::RIG),
        Icon::BoneAdd => (rig::BONE_ADD, hue::RIG),
        Icon::Move => (motion::MOVE, hue::MOVE),
        Icon::Rotate => (motion::ROTATE, hue::ROTATE),
        Icon::Pin => (motion::PIN, hue::PIN),
        Icon::Select => (motion::SELECT, hue::MOVE),
        Icon::Delete => (misc::DELETE, hue::DANGER),
        Icon::Undo => (misc::UNDO, hue::NEUTRAL),
        Icon::Redo => (misc::REDO, hue::NEUTRAL),
        Icon::Play => (anim::PLAY, hue::ANIM),
        Icon::Pause => (anim::PAUSE, hue::ANIM),
        Icon::PrevKey => (anim::PREV_KEY, hue::ANIM),
        Icon::NextKey => (anim::NEXT_KEY, hue::ANIM),
        Icon::Key => (anim::KEY, hue::ANIM),
        Icon::KeyRemove => (anim::KEY_REMOVE, hue::ANIM),
        Icon::Mirror => (rig::MIRROR, hue::RIG),
        Icon::Symmetrize => (rig::SYMMETRIZE, hue::RIG),
        Icon::AutoRig => (rig::AUTO_RIG, hue::RIG),
        Icon::Weights => (anim::WEIGHTS, hue::PHYSICS),
        Icon::Eye => (view::EYE, hue::VIEW),
        Icon::Hierarchy => (rig::HIERARCHY, hue::RIG),
        Icon::Reset => (motion::RESET, hue::ROTATE),
        Icon::Ballistic => (anim::BALLISTIC, hue::PHYSICS),
        Icon::Ghost => (view::GHOST, hue::VIEW),
        Icon::Trajectory => (view::TRAJECTORY, hue::VIEW),
        Icon::CenterOfMass => (anim::CENTER_OF_MASS, hue::PHYSICS),
        Icon::Export => (files::EXPORT, hue::FILES),
        Icon::Import => (files::IMPORT, hue::FILES),
        Icon::Save => (files::SAVE, hue::FILES),
        Icon::Folder => (files::FOLDER, hue::FILES),
        Icon::Settings => (misc::SETTINGS, hue::NEUTRAL),
        Icon::Ik => (motion::IK, hue::MOVE),
        Icon::Fk => (motion::FK, hue::ROTATE),
        Icon::Close => (misc::CLOSE, hue::NEUTRAL),
        Icon::Check => (motion::CHECK, hue::PIN),
        Icon::Frame => (view::FRAME, hue::VIEW),
        Icon::Grid => (view::GRID, hue::VIEW),
        Icon::Mesh => (view::MESH, hue::VIEW),
        Icon::Marker => (rig::MARKER, hue::PHYSICS),
        Icon::Chain => (motion::CHAIN, hue::MOVE),
        Icon::Pose => (rig::POSE, hue::RIG),
        Icon::Edit => (rig::EDIT, hue::RIG),
        Icon::Lock => (view::LOCK, hue::VIEW),
        Icon::Home => (view::HOME, hue::VIEW),
        Icon::Ortho => (view::ORTHO, hue::VIEW),
    }
}
