//! Caches dérivés du maillage (indexés par la révision du maillage) : sommets
//! symétriques (« X Mirror ») et voisinage par arêtes (lissage, flou).
//! Reconstruits quand la révision du maillage change.

use std::collections::HashMap;

use glam::Vec3;
use rayon::prelude::*;

use crate::Mesh;

pub(crate) struct MeshCaches {
    pub rev: u64,
    /// Pour chaque sommet, son symétrique par rapport au plan X = 0
    /// (lui-même s'il n'en a pas, p. ex. sur le plan de symétrie).
    pub mirror: Vec<u32>,
    /// Représentant soudé de chaque sommet (positions confondues réunies).
    weld: Vec<u32>,
    /// Voisins (représentants) de chaque représentant ; vide pour les autres.
    neighbors: Vec<Vec<u32>>,
}

impl MeshCaches {
    pub fn build(mesh: &Mesh, rev: u64) -> Self {
        let (weld, neighbors) = adjacency(mesh);
        Self {
            rev,
            mirror: mirror_map(mesh),
            weld,
            neighbors,
        }
    }

    /// Voisins (par arête) du sommet `v`, en indices de représentants soudés.
    pub fn neighbors_of(&self, v: usize) -> &[u32] {
        match self.weld.get(v) {
            Some(rep) => &self.neighbors[*rep as usize],
            None => &[],
        }
    }
}

/// Correspondance des sommets symétriques (plan X = 0) par hachage spatial,
/// avec une tolérance relative à la taille du modèle. Un sommet sans
/// symétrique (maillage asymétrique, ou sur le plan) se renvoie lui-même.
fn mirror_map(mesh: &Mesh) -> Vec<u32> {
    let n = mesh.positions.len();
    let Some((lo, hi)) = mesh.bounds() else {
        return Vec::new();
    };
    let tol = ((hi - lo).length() * 1e-4).max(1e-6);
    let key = |p: Vec3| {
        [
            (p.x / tol).round() as i64,
            (p.y / tol).round() as i64,
            (p.z / tol).round() as i64,
        ]
    };
    let mut table: HashMap<[i64; 3], u32> = HashMap::with_capacity(n);
    for (i, p) in mesh.positions.iter().enumerate() {
        table.entry(key(*p)).or_insert(i as u32);
    }
    mesh.positions
        .par_iter()
        .enumerate()
        .map(|(i, p)| {
            let m = Vec3::new(-p.x, p.y, p.z);
            let k = key(m);
            let mut best = i as u32;
            let mut best_d = tol * 2.0;
            for dz in -1..=1 {
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if let Some(&j) = table.get(&[k[0] + dx, k[1] + dy, k[2] + dz]) {
                            let d = (mesh.positions[j as usize] - m).length();
                            if d < best_d {
                                best_d = d;
                                best = j;
                            }
                        }
                    }
                }
            }
            best
        })
        .collect()
}

/// Voisinage par arêtes des triangles, réuni par représentant soudé pour ne
/// pas se fendre aux coutures UV (comme la diffusion de chaleur).
fn adjacency(mesh: &Mesh) -> (Vec<u32>, Vec<Vec<u32>>) {
    let (weld, _) = mesh.weld_map();
    let mut neighbors: Vec<Vec<u32>> = vec![Vec::new(); mesh.positions.len()];
    let link = |a: u32, b: u32, nb: &mut Vec<Vec<u32>>| {
        let (a, b) = (weld[a as usize], weld[b as usize]);
        if a != b && !nb[a as usize].contains(&b) {
            nb[a as usize].push(b);
        }
    };
    for tri in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = *tri;
        link(a, b, &mut neighbors);
        link(b, a, &mut neighbors);
        link(b, c, &mut neighbors);
        link(c, b, &mut neighbors);
        link(c, a, &mut neighbors);
        link(a, c, &mut neighbors);
    }
    (weld, neighbors)
}
