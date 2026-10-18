//! Glyphes SVG des icônes, regroupés par famille d'outils.

mod anim;
mod files;
mod misc;
mod motion;
mod paint;
mod rig;
mod shapes;
mod view;

use super::Icon;

/// Corps SVG d'une icône.
pub fn source(icon: Icon) -> &'static str {
    match icon {
        Icon::Gallery => files::GALLERY,
        Icon::Actions => misc::ACTIONS,
        Icon::Adjust => misc::ADJUST,
        Icon::Bone => rig::BONE,
        Icon::BoneAdd => rig::BONE_ADD,
        Icon::Move => motion::MOVE,
        Icon::Rotate => motion::ROTATE,
        Icon::Pin => motion::PIN,
        Icon::Select => motion::SELECT,
        Icon::Delete => misc::DELETE,
        Icon::Undo => misc::UNDO,
        Icon::Redo => misc::REDO,
        Icon::Play => anim::PLAY,
        Icon::Pause => anim::PAUSE,
        Icon::PrevKey => anim::PREV_KEY,
        Icon::NextKey => anim::NEXT_KEY,
        Icon::Key => anim::KEY,
        Icon::KeyRemove => anim::KEY_REMOVE,
        Icon::Mirror => rig::MIRROR,
        Icon::Symmetrize => rig::SYMMETRIZE,
        Icon::AutoRig => rig::AUTO_RIG,
        Icon::Weights => anim::WEIGHTS,
        Icon::Eye => view::EYE,
        Icon::Hierarchy => rig::HIERARCHY,
        Icon::Reset => motion::RESET,
        Icon::Ballistic => anim::BALLISTIC,
        Icon::Ghost => view::GHOST,
        Icon::Trajectory => view::TRAJECTORY,
        Icon::CenterOfMass => anim::CENTER_OF_MASS,
        Icon::Export => files::EXPORT,
        Icon::Import => files::IMPORT,
        Icon::Save => files::SAVE,
        Icon::Folder => files::FOLDER,
        Icon::Settings => misc::SETTINGS,
        Icon::Ik => motion::IK,
        Icon::Fk => motion::FK,
        Icon::Close => misc::CLOSE,
        Icon::Check => misc::CHECK,
        Icon::Frame => view::FRAME,
        Icon::Grid => view::GRID,
        Icon::Mesh => view::MESH,
        Icon::Marker => rig::MARKER,
        Icon::Chain => motion::CHAIN,
        Icon::Pose => rig::POSE,
        Icon::Edit => rig::EDIT,
        Icon::Lock => view::LOCK,
        Icon::Home => view::HOME,
        Icon::Ortho => view::ORTHO,
        Icon::BoneHead => rig::BONE_HEAD,
        Icon::Duplicate => rig::DUPLICATE,
        Icon::Subdivide => rig::SUBDIVIDE,
        Icon::Flip => rig::FLIP,
        Icon::XMirror => rig::XMIRROR,
        Icon::Plus => misc::PLUS,
        Icon::Scale => motion::SCALE,
        Icon::Transform => motion::TRANSFORM,
        Icon::Globe => view::GLOBE,
        Icon::Local => view::LOCAL,
        Icon::Magnet => misc::MAGNET,
        Icon::Brush => paint::BRUSH,
        Icon::Blur => paint::BLUR,
        Icon::Average => paint::AVERAGE,
        Icon::Smear => paint::SMEAR,
        Icon::Gradient => paint::GRADIENT,
        Icon::Sample => paint::SAMPLE,
        Icon::Normalize => paint::NORMALIZE,
        Icon::Motion => anim::MOTION,
        Icon::Face => rig::FACE,
        Icon::Object => shapes::OBJECT,
        Icon::Cube => shapes::CUBE,
        Icon::Sphere => shapes::SPHERE,
        Icon::Cylinder => shapes::CYLINDER,
        Icon::Cone => shapes::CONE,
        Icon::Plane => shapes::PLANE,
        Icon::Torus => shapes::TORUS,
    }
}
