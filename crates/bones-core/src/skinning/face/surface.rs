//! Surface soudée pour les poids du visage : sommets confondus fusionnés,
//! voisinage, composantes connexes et distance géodésique bornée (Dijkstra sur
//! les arêtes), base de la liaison localisée du visage.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use glam::Vec3;

use crate::Mesh;

pub(super) const UNREACHED: f32 = f32::INFINITY;

pub(super) struct FaceSurface {
    /// Positions soudées (un point par sommet distinct).
    pub positions: Vec<Vec3>,
    /// Sommet d'origine → indice soudé dense.
    pub welded: Vec<u32>,
    start: Vec<u32>,
    adj: Vec<u32>,
    /// Sommet soudé → identifiant de composante connexe.
    comp: Vec<u32>,
}

impl FaceSurface {
    pub fn new(mesh: &Mesh) -> Self {
        let (map, unique) = mesh.weld_map();
        // Représentant d'origine → indice dense.
        let mut dense = vec![u32::MAX; map.len()];
        let mut positions = Vec::with_capacity(unique);
        for (i, rep) in map.iter().enumerate() {
            if *rep as usize == i {
                dense[i] = positions.len() as u32;
                positions.push(mesh.positions[i]);
            }
        }
        let welded: Vec<u32> = map.iter().map(|rep| dense[*rep as usize]).collect();

        // Arêtes des triangles (indices soudés), dédupliquées dans les deux sens.
        let n = positions.len();
        let mut pairs: Vec<(u32, u32)> = Vec::new();
        for t in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = [
                welded[t[0] as usize],
                welded[t[1] as usize],
                welded[t[2] as usize],
            ];
            if a == b || b == c || a == c {
                continue;
            }
            for (i, j) in [(a, b), (b, c), (c, a)] {
                pairs.push((i, j));
                pairs.push((j, i));
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
        let adj: Vec<u32> = pairs.into_iter().map(|(_, b)| b).collect();

        let comp = components(n, &start, &adj);
        Self {
            positions,
            welded,
            start,
            adj,
            comp,
        }
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn neighbors(&self, v: u32) -> &[u32] {
        &self.adj[self.start[v as usize] as usize..self.start[v as usize + 1] as usize]
    }

    pub fn component(&self, v: u32) -> u32 {
        self.comp[v as usize]
    }

    /// Sommet soudé le plus proche de `p` (dans la composante d'un `hint`
    /// facultatif, pour éviter d'attraper une pièce voisine).
    pub fn nearest(&self, p: Vec3) -> Option<u32> {
        (0..self.positions.len() as u32).min_by(|a, b| {
            self.positions[*a as usize]
                .distance_squared(p)
                .total_cmp(&self.positions[*b as usize].distance_squared(p))
        })
    }

    /// Distance géodésique (le long des arêtes) à l'ensemble `seeds`, bornée
    /// par `radius` : au-delà, le sommet reste à [`UNREACHED`]. Dijkstra à tas
    /// binaire ; les distances sont positives, donc ordonnées par leurs bits.
    pub fn geodesic(&self, seeds: &[u32], radius: f32) -> Vec<f32> {
        let mut dist = vec![UNREACHED; self.positions.len()];
        let mut heap: BinaryHeap<Reverse<(u32, u32)>> = BinaryHeap::new();
        for &s in seeds {
            if (s as usize) < dist.len() && dist[s as usize] != 0.0 {
                dist[s as usize] = 0.0;
                heap.push(Reverse((0, s)));
            }
        }
        while let Some(Reverse((bits, v))) = heap.pop() {
            let d = f32::from_bits(bits);
            if d > dist[v as usize] {
                continue;
            }
            let p = self.positions[v as usize];
            for &u in self.neighbors(v) {
                let nd = d + p.distance(self.positions[u as usize]);
                if nd <= radius && nd < dist[u as usize] {
                    dist[u as usize] = nd;
                    heap.push(Reverse((nd.to_bits(), u)));
                }
            }
        }
        dist
    }
}

/// Composantes connexes par union-find sur les arêtes soudées.
fn components(n: usize, start: &[u32], adj: &[u32]) -> Vec<u32> {
    let mut parent: Vec<u32> = (0..n as u32).collect();
    fn find(p: &mut [u32], mut x: u32) -> u32 {
        while p[x as usize] != x {
            p[x as usize] = p[p[x as usize] as usize];
            x = p[x as usize];
        }
        x
    }
    for v in 0..n as u32 {
        for &u in &adj[start[v as usize] as usize..start[v as usize + 1] as usize] {
            let (ra, rb) = (find(&mut parent, v), find(&mut parent, u));
            if ra != rb {
                parent[ra.max(rb) as usize] = ra.min(rb);
            }
        }
    }
    (0..n as u32).map(|v| find(&mut parent, v)).collect()
}
