//! STL (ASCII et binaire) via `stl_io`.
//!
//! Convention des logiciels d'impression 3D : Z vers le haut, millimètres.
//! À l'import, un modèle de plus de 20 unités est supposé en millimètres et
//! ramené en mètres ; à l'export, on écrit en millimètres, Z vers le haut.

use std::io::Cursor;

use glam::Vec3;

use super::rigging::posed_mesh;
use super::{Format, IoError};
use crate::{Mesh, Rig};

/// Seuil (en unités du fichier) au-delà duquel on suppose des millimètres.
const MM_THRESHOLD: f32 = 20.0;

fn z_up_to_y_up(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0], v[2], -v[1])
}

fn y_up_to_z_up(v: Vec3) -> [f32; 3] {
    [v.x, -v.z, v.y]
}

pub(super) fn import(bytes: &[u8]) -> Result<Mesh, IoError> {
    let stl = stl_io::read_stl(&mut Cursor::new(bytes)).map_err(|e| IoError::parse(Format::Stl, e))?;
    let mut mesh = Mesh {
        positions: stl.vertices.iter().map(|v| z_up_to_y_up(v.0)).collect(),
        indices: stl
            .faces
            .iter()
            .flat_map(|f| f.vertices.map(|i| i as u32))
            .collect(),
        ..Default::default()
    };
    if mesh.indices.is_empty() {
        return Err(IoError::parse(Format::Stl, "no triangle"));
    }
    if let Some((lo, hi)) = mesh.bounds()
        && (hi - lo).max_element() > MM_THRESHOLD
    {
        mesh.scale(0.001);
    }
    mesh.compute_normals();
    mesh.shrink_to_fit();
    Ok(mesh)
}

/// STL binaire du maillage dans la pose courante (millimètres, Z en haut).
pub(super) fn export(rig: &Rig) -> Vec<u8> {
    let mesh = posed_mesh(rig);
    let vertex = |i: u32| {
        let p = y_up_to_z_up(mesh.positions[i as usize] * 1000.0);
        stl_io::Vertex::new(p)
    };
    let triangles: Vec<stl_io::Triangle> = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| mesh.positions[i as usize]);
            let n = (b - a).cross(c - a).normalize_or_zero();
            stl_io::Triangle {
                normal: stl_io::Normal::new(y_up_to_z_up(n)),
                vertices: [vertex(t[0]), vertex(t[1]), vertex(t[2])],
            }
        })
        .collect();
    let mut out = Vec::with_capacity(84 + triangles.len() * 50);
    stl_io::write_stl(&mut out, triangles.iter()).expect("in-memory write");
    out
}
