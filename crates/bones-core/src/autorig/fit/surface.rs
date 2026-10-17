//! Surface du maillage préparée pour l'analyse : sommets soudés, voisinage
//! et coupes planes.

use std::collections::HashMap;
use std::f32::consts::TAU;

use glam::Vec3;

use crate::Mesh;

/// Contour fermé d'une coupe plane (une « tranche » de membre ou de tronc).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ring {
    /// Centre du contour (moyenne pondérée par la longueur des segments).
    pub center: Vec3,
    pub perimeter: f32,
}

impl Ring {
    /// Rayon du cercle de même périmètre.
    pub fn radius(&self) -> f32 {
        self.perimeter / TAU
    }
}

/// Contour dont le centre est le plus proche de `p`.
pub(crate) fn nearest(rings: &[Ring], p: Vec3) -> Option<Ring> {
    rings
        .iter()
        .min_by(|a, b| {
            a.center
                .distance_squared(p)
                .total_cmp(&b.center.distance_squared(p))
        })
        .copied()
}

pub(crate) struct Surface<'a> {
    pub positions: &'a [Vec3],
    /// Triangles, en indices de sommets soudés.
    pub tris: Vec<[u32; 3]>,
    /// Voisins de chaque sommet (format compressé : `adj[start[v]..start[v + 1]]`).
    start: Vec<u32>,
    adj: Vec<u32>,
}

impl<'a> Surface<'a> {
    pub fn new(mesh: &'a Mesh) -> Self {
        let (map, _) = mesh.weld_map();
        let tris: Vec<[u32; 3]> = mesh
            .indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| t.map(|i| map[i as usize]))
            .filter(|t| t[0] != t[1] && t[1] != t[2] && t[0] != t[2])
            .collect();
        let n = mesh.positions.len();
        let mut pairs: Vec<(u32, u32)> = Vec::with_capacity(tris.len() * 6);
        for t in &tris {
            for k in 0..3 {
                let (a, b) = (t[k], t[(k + 1) % 3]);
                pairs.push((a, b));
                pairs.push((b, a));
            }
        }
        pairs.sort_unstable();
        pairs.dedup();
        let mut start = vec![0u32; n + 1];
        for (a, _) in &pairs {
            start[*a as usize + 1] += 1;
        }
        for v in 0..n {
            start[v + 1] += start[v];
        }
        Self {
            positions: &mesh.positions,
            tris,
            start,
            adj: pairs.into_iter().map(|(_, b)| b).collect(),
        }
    }

    pub fn neighbors(&self, v: u32) -> &[u32] {
        &self.adj[self.start[v as usize] as usize..self.start[v as usize + 1] as usize]
    }

    /// Sommets utilisés par au moins un triangle.
    pub fn vertices(&self) -> impl Iterator<Item = u32> + '_ {
        (0..self.positions.len() as u32).filter(|v| !self.neighbors(*v).is_empty())
    }

    /// Coupe par le plan passant par `point`, de normale `normal` : un
    /// contour par boucle fermée (ou ouverte, au bord d'un trou).
    pub fn section(&self, point: Vec3, normal: Vec3) -> Vec<Ring> {
        let side = |v: u32| (self.positions[v as usize] - point).dot(normal);
        let mut nodes: HashMap<(u32, u32), u32> = HashMap::new();
        let mut parent: Vec<u32> = Vec::new();
        let mut segments: Vec<(u32, Vec3, f32)> = Vec::new();
        for t in &self.tris {
            let d = t.map(side);
            let above = d.map(|x| x >= 0.0);
            if above[0] == above[1] && above[1] == above[2] {
                continue;
            }
            let mut ends = [(0u32, Vec3::ZERO); 2];
            let mut count = 0;
            for k in 0..3 {
                let (i, j) = (k, (k + 1) % 3);
                if above[i] == above[j] || count == 2 {
                    continue;
                }
                let (a, b) = (t[i], t[j]);
                let s = d[i] / (d[i] - d[j]);
                let p = self.positions[a as usize].lerp(self.positions[b as usize], s);
                let key = (a.min(b), a.max(b));
                let next = parent.len() as u32;
                let node = *nodes.entry(key).or_insert(next);
                if node == next {
                    parent.push(next);
                }
                ends[count] = (node, p);
                count += 1;
            }
            union(&mut parent, ends[0].0, ends[1].0);
            let len = ends[0].1.distance(ends[1].1);
            segments.push((ends[0].0, (ends[0].1 + ends[1].1) * 0.5, len));
        }
        let mut rings: HashMap<u32, (Vec3, f32)> = HashMap::new();
        for (node, mid, len) in segments {
            let e = rings.entry(find(&mut parent, node)).or_insert((Vec3::ZERO, 0.0));
            e.0 += mid * len;
            e.1 += len;
        }
        rings
            .into_values()
            .filter(|(_, len)| *len > 1e-6)
            .map(|(sum, len)| Ring {
                center: sum / len,
                perimeter: len,
            })
            .collect()
    }
}

fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        parent[x as usize] = parent[parent[x as usize] as usize];
        x = parent[x as usize];
    }
    x
}

fn union(parent: &mut [u32], a: u32, b: u32) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    if ra != rb {
        parent[ra.max(rb) as usize] = ra.min(rb);
    }
}
