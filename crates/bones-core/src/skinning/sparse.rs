//! Algèbre linéaire creuse pour la diffusion de chaleur : Laplacien
//! cotangent au format CSR et gradient conjugué préconditionné (Jacobi).

use rayon::prelude::*;

use crate::Mesh;

const MAX_ITERATIONS: usize = 600;
const TOLERANCE: f64 = 1e-6;

/// Matrice creuse symétrique au format CSR (hors diagonale).
pub(super) struct Csr {
    row_start: Vec<u32>,
    cols: Vec<u32>,
    values: Vec<f64>,
}

impl Csr {
    fn mul_add(&self, diag: &[f64], x: &[f64], out: &mut [f64]) {
        out.par_iter_mut().enumerate().for_each(|(i, o)| {
            let (s, e) = (self.row_start[i] as usize, self.row_start[i + 1] as usize);
            let mut acc = diag[i] * x[i];
            for k in s..e {
                acc += self.values[k] * x[self.cols[k] as usize];
            }
            *o = acc;
        });
    }
}

/// Laplacien cotangent (matrice de raideur, semi-définie positive) et masses
/// barycentriques des sommets.
pub(super) fn cotangent_laplacian(mesh: &Mesh) -> (Csr, Vec<f64>, Vec<f64>) {
    let n = mesh.positions.len();
    let mut entries: Vec<(u32, u32, f64)> = Vec::with_capacity(mesh.indices.len() * 2);
    let mut mass = vec![0.0f64; n];
    for tri in mesh.indices.as_chunks::<3>().0 {
        let idx = [tri[0], tri[1], tri[2]];
        let p = idx.map(|i| mesh.positions[i as usize].as_dvec3());
        let area2 = (p[1] - p[0]).cross(p[2] - p[0]).length();
        if area2 < 1e-12 {
            continue;
        }
        for k in 0..3 {
            mass[idx[k] as usize] += area2 / 6.0;
            // Arête opposée au sommet k, pondérée par la cotangente de son angle.
            let (a, b) = ((k + 1) % 3, (k + 2) % 3);
            let (u, v) = (p[a] - p[k], p[b] - p[k]);
            let cot = (u.dot(v) / u.cross(v).length()).clamp(0.0, 1e4);
            let w = 0.5 * cot;
            entries.push((idx[a], idx[b], w));
            entries.push((idx[b], idx[a], w));
        }
    }
    entries.par_sort_unstable_by_key(|(r, c, _)| (*r, *c));

    let mut row_start = vec![0u32; n + 1];
    let mut cols = Vec::with_capacity(entries.len());
    let mut values = Vec::with_capacity(entries.len());
    let mut diag = vec![0.0f64; n];
    let mut last: Option<(u32, u32)> = None;
    for (r, c, w) in entries {
        diag[r as usize] += w;
        if last == Some((r, c)) {
            *values.last_mut().unwrap() -= w;
        } else {
            cols.push(c);
            values.push(-w);
            row_start[r as usize + 1] = cols.len() as u32;
            last = Some((r, c));
        }
    }
    // Lignes vides : on propage le début de ligne.
    for i in 1..=n {
        row_start[i] = row_start[i].max(row_start[i - 1]);
    }
    (
        Csr {
            row_start,
            cols,
            values,
        },
        diag,
        mass,
    )
}

/// Gradient conjugué préconditionné par la diagonale.
pub(super) fn conjugate_gradient(a: &Csr, diag: &[f64], b: &[f64], x: &mut [f64]) {
    let n = b.len();
    let mut r = vec![0.0; n];
    a.mul_add(diag, x, &mut r);
    r.iter_mut().zip(b).for_each(|(ri, bi)| *ri = bi - *ri);
    let mut z: Vec<f64> = r.iter().zip(diag).map(|(ri, d)| ri / d).collect();
    let mut p = z.clone();
    let mut ap = vec![0.0; n];
    let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
    let b_norm = b.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-30);

    for _ in 0..MAX_ITERATIONS {
        a.mul_add(diag, &p, &mut ap);
        let pap: f64 = p.iter().zip(&ap).map(|(a, b)| a * b).sum();
        if pap <= 0.0 {
            break;
        }
        let alpha = rz / pap;
        for i in 0..n {
            x[i] += alpha * p[i];
            r[i] -= alpha * ap[i];
        }
        if r.iter().map(|v| v * v).sum::<f64>().sqrt() / b_norm < TOLERANCE {
            break;
        }
        for i in 0..n {
            z[i] = r[i] / diag[i];
        }
        let rz_new: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
        let beta = rz_new / rz;
        rz = rz_new;
        for i in 0..n {
            p[i] = z[i] + beta * p[i];
        }
    }
}
