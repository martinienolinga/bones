//! Préréglages d'animation 3D (humains, animaux, végétaux, créatures).
//!
//! Moteur encore en pause (voir wip/LISEZMOI.md) : certaines parties ne sont
//! pas encore reliées, d'où les tolérances ci-dessous, à retirer à la reprise.
#![allow(dead_code, clippy::type_complexity, clippy::wrong_self_convention)]

pub mod apply;
pub mod create;
pub mod error;
pub mod library;
pub mod model;
pub mod params;
pub mod preview;
pub mod procedural;
pub mod reference;
pub mod retarget;
pub mod roles;
#[cfg(test)]
mod sheet;
#[cfg(test)]
mod tests;

pub use apply::{ApplyMode, ApplyOptions, ApplyReport};
pub use create::CreateOptions;
pub use error::PresetError;
pub use library::{builtin, find_builtin};
pub use model::{Category, KeyframeData, Preset, PresetInfo, PresetKind, PresetSource, Procedural, Section};
pub use params::{ParamSpec, Params};
pub use preview::Preview;
pub use procedural::Generator;
pub use reference::RefRig;
pub use roles::{BodyFrame, Chain, Lr, RigType, Role, RoleMap, detect};
