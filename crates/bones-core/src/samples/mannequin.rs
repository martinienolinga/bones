//! Mannequin par défaut : maillage embarqué (Blender Studio, CC0), squelette
//! humanoïde ajusté à son volume et poids par diffusion de chaleur.
//!
//! [`build_mannequin`] refait tout le rigging au chargement (import, marqueurs,
//! ajustement des articulations, liaison). Pour une ouverture instantanée, son
//! résultat peut être précalculé une fois pour toutes (projet `.kbrig`
//! embarqué) : seul le corps de [`mannequin`] change alors, l'API reste la même.

use crate::autorig::{self, AutoRigOptions};
use crate::io::{Format, Imported, import_bytes};
use crate::{BindMethod, Markers, Mesh, Rig};

/// Maillage du mannequin (GLB, mètres, Y vers le haut, regard vers +Z,
/// pieds en y = 0). Provenance et licence : `assets/mannequin/SOURCE.md`.
pub const MANNEQUIN_GLB: &[u8] = include_bytes!("../../assets/mannequin/mannequin.glb");

/// Nom du document.
const NAME: &str = "Mannequin";

/// Maillage seul du mannequin, sans squelette.
pub fn mannequin_mesh() -> Mesh {
    match import_bytes(Format::Glb, MANNEQUIN_GLB, NAME, None) {
        Ok(Imported::Rig(rig)) => (*rig.mesh).clone(),
        Ok(Imported::Mesh(mesh)) => mesh,
        Err(e) => panic!("mannequin embarqué illisible : {e}"),
    }
}

/// Mannequin prêt à l'emploi : maillage, squelette humanoïde (noms
/// `.l`/`.r`/`.x` comme [`Rig::demo`], doigts et orteils), membres IK/FK et
/// poids.
pub fn mannequin() -> Rig {
    build_mannequin()
}

/// Construit le mannequin de zéro (quelques secondes en debug, une fraction
/// de seconde en release ; la diffusion de chaleur domine).
pub fn build_mannequin() -> Rig {
    let mut rig = Rig::new(NAME, mannequin_mesh());
    let markers = Markers::guess(&rig.mesh).expect("maillage du mannequin vide");
    let (skeleton, limbs) = autorig::generate_fitted(&markers, &AutoRigOptions::default(), &rig.mesh);
    rig.set_skeleton(skeleton, limbs);
    rig.bind(BindMethod::Heat);
    rig
}
