//! Projection des sommets déformés (maillage posé) en espace écran, au début
//! d'un trait. La pose ne change pas pendant qu'on peint : ce calcul n'est
//! fait qu'une fois, en parallèle avec rayon.

use glam::{Mat4, Vec2};
use rayon::prelude::*;

use crate::{Mesh, OrbitCamera, Skin, Viewport};

/// Positions écran des sommets déformés (`None` : derrière la caméra) et, pour
/// chacun, « sa face est tournée vers la caméra » (normale déformée).
pub(crate) fn project_vertices(
    mesh: &Mesh,
    skin: &Skin,
    matrices: &[Mat4],
    camera: &OrbitCamera,
    vp: &Viewport,
) -> (Vec<Option<Vec2>>, Vec<bool>) {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    skin.deform_into(mesh, matrices, &mut positions, &mut normals);
    let forward = camera.forward();
    positions
        .par_iter()
        .zip(&normals)
        .map(|(p, n)| (camera.project(*p, vp), n.dot(forward) < 0.0))
        .unzip()
}
