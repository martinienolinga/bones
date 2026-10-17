//! Modèles d'exemple embarqués dans l'application, prêts à l'emploi.
//!
//! - [`mannequin`] : corps humain réaliste (Blender Studio, CC0), riggé par
//!   notre auto-rig et lié par diffusion de chaleur, précalculé en projet
//!   `.kbrig` embarqué.

mod mannequin;
#[cfg(test)]
mod tests;

pub use mannequin::{
    MANNEQUIN_GLB, MANNEQUIN_KBRIG, bake_mannequin, build_mannequin, load_mannequin, mannequin, mannequin_mesh,
};
