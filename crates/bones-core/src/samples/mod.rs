//! Modèles d'exemple embarqués dans l'application, prêts à l'emploi.
//!
//! - [`mannequin`] : corps humain réaliste (Blender Studio, CC0), riggé par
//!   notre auto-rig et lié par diffusion de chaleur.

mod mannequin;
#[cfg(test)]
mod tests;

pub use mannequin::{MANNEQUIN_GLB, mannequin, mannequin_mesh};
