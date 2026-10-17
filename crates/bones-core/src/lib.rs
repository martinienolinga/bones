//! Cœur de PaintSharp Bones : tout ce qui concerne le rigging et l'animation,
//! sans aucune dépendance au GPU ni à la plateforme. Ce crate est entièrement
//! testable sur un poste de développement (`cargo test -p bones-core`).
//!
//! Conventions :
//! - repère main droite, Y vers le haut ;
//! - un os pointe le long de son axe +Y local : la tête est à l'origine de
//!   l'os, la queue en `(0, longueur, 0)` ;
//! - les transformations locales d'un os sont relatives à l'espace de son
//!   parent (donc à la tête du parent).
//!
//! Organisation :
//! - [`math`] : transformations, caméra, sélection à l'écran ;
//! - [`mesh`] : maillages et primitives ;
//! - [`rig`] : squelette, pose, symétrie, document de travail ;
//! - [`skinning`] : poids (enveloppes, diffusion de chaleur) et déformation ;
//! - [`ik`] : FABRIK, IK à deux os, points épinglés ;
//! - [`autorig`] : rigging assisté façon Auto-Rig Pro ;
//! - [`anim`] : clés, interpolation, physique (Cascadeur) ;
//! - [`io`] : fichiers `.bones`, import OBJ, export glTF ;
//! - [`editor`] : état de l'éditeur et gestes, indépendants de la plateforme.

pub mod anim;
pub mod autorig;
pub mod editor;
pub mod ik;
pub mod io;
pub mod math;
pub mod mesh;
pub mod rig;
pub mod skinning;

pub use glam;

pub use anim::{Clip, Interpolation, Key};
pub use autorig::{AutoRigOptions, Limb, LimbMode, MarkerKind, Markers};
pub use editor::{BindJob, EditTool, Editor, Handle, Mode, PoseTool, ViewOptions};
pub use ik::{IkSettings, Pin};
pub use io::rig_file::RigError;
pub use math::{OrbitCamera, Ray, Transform, Viewport};
pub use mesh::Mesh;
pub use rig::mirror::{Side, mirror_name, side_of};
pub use rig::{Bone, BoneId, BoneRemap, Pose, Revisions, Rig, Skeleton, head_tail, next_revision};
pub use skinning::{BindMethod, Skin, VertexWeights};
