//! Skinning linéaire sur le CPU.

use glam::{Mat3, Mat4, Vec3};
use rayon::prelude::*;

use crate::{Mesh, Skin};

impl Skin {
    /// Skinning linéaire (LBS) des positions et normales sur le CPU, en
    /// parallèle. Le rendu utilise la version GPU ; celle-ci sert à l'export,
    /// au picking sur le maillage et aux tests.
    pub fn deform_into(
        &self,
        mesh: &Mesh,
        matrices: &[Mat4],
        positions: &mut Vec<Vec3>,
        normals: &mut Vec<Vec3>,
    ) {
        positions.resize(mesh.positions.len(), Vec3::ZERO);
        normals.resize(mesh.normals.len(), Vec3::ZERO);
        positions
            .par_iter_mut()
            .zip(normals.par_iter_mut())
            .enumerate()
            .for_each(|(i, (out_p, out_n))| {
                let (p, n) = (mesh.positions[i], mesh.normals[i]);
                let vw = self.weights.get(i).copied().unwrap_or_default();
                let mut pos = Vec3::ZERO;
                let mut nrm = Vec3::ZERO;
                let mut total = 0.0;
                for (bone, w) in vw.iter() {
                    let Some(m) = matrices.get(bone.index()) else {
                        continue;
                    };
                    pos += w * m.transform_point3(p);
                    nrm += w * (Mat3::from_mat4(*m) * n);
                    total += w;
                }
                if total < 1e-6 {
                    *out_p = p;
                    *out_n = n;
                } else {
                    *out_p = pos / total;
                    *out_n = nrm.try_normalize().unwrap_or(n);
                }
            });
    }
}
