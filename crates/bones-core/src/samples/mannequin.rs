//! Mannequin par défaut : maillage embarqué (Blender Studio, CC0), squelette
//! humanoïde ajusté à son volume et poids par diffusion de chaleur.
//!
//! Le rigging complet (import, marqueurs, ajustement, liaison) prend plusieurs
//! secondes : il est fait une fois pour toutes par [`build_mannequin`], et son
//! résultat est embarqué sous forme de projet `.kbrig` ([`MANNEQUIN_KBRIG`]),
//! que [`mannequin`] relit en quelques millisecondes. Après une modification
//! du maillage, de l'auto-rig ou des poids, régénérer ce fichier avec
//! `cargo test -p bones-core --lib samples::tests::rebuild_embedded_mannequin -- --ignored`
//! (un test signale quand il ne correspond plus à l'auto-rig actuel).

use std::path::Path;

use crate::autorig::{self, AutoRigOptions};
use crate::io::{Format, Imported, import_bytes};
use crate::project::{self, ProjectError, ProjectInfo, ProjectSnapshot};
use crate::{BindMethod, Editor, Markers, Mesh, Rig};

/// Maillage source du mannequin (GLB, mètres, Y vers le haut, regard vers
/// +Z, pieds en y = 0). Provenance et licence : `assets/mannequin/SOURCE.md`.
pub const MANNEQUIN_GLB: &[u8] = include_bytes!("../../assets/mannequin/mannequin.glb");

/// Mannequin précalculé par [`build_mannequin`] (projet `.kbrig` : maillage,
/// squelette, membres IK/FK et poids).
pub const MANNEQUIN_KBRIG: &[u8] = include_bytes!("../../assets/mannequin/mannequin.kbrig");

/// Nom du document.
const NAME: &str = "Mannequin";

/// Mannequin prêt à l'emploi : maillage, squelette humanoïde (noms
/// `.l`/`.r`/`.x` comme [`Rig::demo`], doigts et orteils), membres IK/FK et
/// poids. Lu depuis le projet embarqué : quelques millisecondes.
pub fn mannequin() -> Rig {
    load_mannequin(MANNEQUIN_KBRIG).expect("mannequin embarqué illisible")
}

/// Lit un mannequin précalculé (projet `.kbrig` en mémoire).
pub fn load_mannequin(bytes: &[u8]) -> Result<Rig, ProjectError> {
    let mut rig = project::load_bytes(bytes, NAME)?.snapshot.rig;
    rig.name = NAME.to_owned();
    // Cibles IK et orientation des mains et des pieds recalées sur la pose
    // (l'orientation n'est pas enregistrée dans les projets).
    let world = rig.world();
    for limb in &mut rig.limbs {
        limb.snap_to_pose(&rig.skeleton, &world);
    }
    Ok(rig)
}

/// Maillage seul du mannequin, sans squelette (depuis le GLB source).
pub fn mannequin_mesh() -> Mesh {
    match import_bytes(Format::Glb, MANNEQUIN_GLB, NAME, None) {
        Ok(Imported::Rig(rig)) => (*rig.mesh).clone(),
        Ok(Imported::Mesh(mesh)) => mesh,
        Err(e) => panic!("mannequin embarqué illisible : {e}"),
    }
}

/// Construit le mannequin de zéro depuis le GLB source (quelques secondes en
/// debug ; la diffusion de chaleur domine).
pub fn build_mannequin() -> Rig {
    let mut rig = Rig::new(NAME, mannequin_mesh());
    let markers = Markers::guess(&rig.mesh).expect("maillage du mannequin vide");
    let (skeleton, limbs) = autorig::generate_fitted(&markers, &AutoRigOptions::default(), &rig.mesh);
    rig.set_skeleton(skeleton, limbs);
    rig.bind(BindMethod::Heat);
    rig
}

/// Construit le mannequin et l'enregistre comme projet `.kbrig` (sans
/// vignette, date de création nulle).
pub fn bake_mannequin(path: &Path) -> Result<ProjectInfo, ProjectError> {
    let editor = Editor::new(build_mannequin());
    ProjectSnapshot::capture(&editor).save(path, NAME, 0, None)
}
