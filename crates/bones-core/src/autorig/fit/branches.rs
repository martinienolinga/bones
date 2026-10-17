//! Branches d'une région de surface (doigts d'une main) : distance
//! géodésique depuis une frontière, puis arbre de fusion des sur-niveaux.
//!
//! En parcourant les sommets du plus lointain au plus proche, chaque bout de
//! doigt fait naître une composante ; deux composantes fusionnent au niveau
//! de la commissure qui les sépare. Les branches de faible persistance
//! (bosses, ongles) sont rattachées à leur voisine.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use glam::Vec3;

use super::surface::Surface;

/// Branche significative : bout, niveau de la commissure et axe.
#[derive(Clone, Debug)]
pub(super) struct Branch {
    /// Sommet le plus lointain (bout du doigt).
    pub tip: Vec3,
    /// Distance géodésique de la commissure (base visible du doigt).
    pub web: f32,
    /// Axe : centres des bandes de distance, de la commissure au bout.
    pub axis: Vec<Vec3>,
}

/// Distances géodésiques (le long des arêtes) depuis `sources`, limitées
/// aux sommets de `inside`. `f32::INFINITY` hors d'atteinte.
pub(super) fn geodesic(s: &Surface, sources: &[u32], inside: &[bool]) -> Vec<f32> {
    let mut dist = vec![f32::INFINITY; s.positions.len()];
    let mut heap = BinaryHeap::new();
    for &v in sources {
        dist[v as usize] = 0.0;
        heap.push(Reverse((0u32, v)));
    }
    while let Some(Reverse((bits, v))) = heap.pop() {
        let d = f32::from_bits(bits);
        if d > dist[v as usize] {
            continue;
        }
        for &u in s.neighbors(v) {
            if !inside[u as usize] {
                continue;
            }
            let nd = d + s.positions[v as usize].distance(s.positions[u as usize]);
            if nd < dist[u as usize] {
                dist[u as usize] = nd;
                // Les flottants positifs se comparent comme leurs bits.
                heap.push(Reverse((nd.to_bits(), u)));
            }
        }
    }
    dist
}

/// Branches de persistance au moins `min_length`, avec un pas de bande `band`.
pub(super) fn branches(s: &Surface, dist: &[f32], min_length: f32, band: f32) -> Vec<Branch> {
    let mut order: Vec<u32> = (0..dist.len() as u32)
        .filter(|v| dist[*v as usize].is_finite())
        .collect();
    order.sort_by(|a, b| dist[*b as usize].total_cmp(&dist[*a as usize]));

    // Union-find : chaque composante garde son bout (sommet le plus lointain).
    let n = dist.len();
    let mut parent: Vec<u32> = (0..n as u32).collect();
    let mut tip: Vec<u32> = (0..n as u32).collect();
    let mut added = vec![false; n];
    // Étiquette (bout de la composante) à l'insertion, et fusions : bout
    // absorbé → (bout survivant, niveau).
    let mut label = vec![u32::MAX; n];
    let mut merges: Vec<(u32, u32, f32)> = Vec::new();
    for &v in &order {
        added[v as usize] = true;
        for &u in s.neighbors(v) {
            if !added[u as usize] {
                continue;
            }
            let (rv, ru) = (find(&mut parent, v), find(&mut parent, u));
            if rv == ru {
                continue;
            }
            let (tv, tu) = (tip[rv as usize], tip[ru as usize]);
            let (old, young) = if dist[tv as usize] >= dist[tu as usize] {
                (tv, tu)
            } else {
                (tu, tv)
            };
            if young != v {
                merges.push((young, old, dist[v as usize]));
            }
            parent[rv as usize] = ru;
            tip[ru as usize] = old;
        }
        label[v as usize] = tip[find(&mut parent, v) as usize];
    }

    // Branches significatives ; les autres sont rattachées à leur survivant.
    let significant = |t: u32, level: f32| dist[t as usize] - level >= min_length;
    let mut absorbed_into: Vec<(u32, u32)> = Vec::new();
    let mut kept: Vec<(u32, f32)> = Vec::new();
    for &(young, old, level) in &merges {
        if significant(young, level) {
            kept.push((young, level));
        } else {
            absorbed_into.push((young, old));
        }
    }
    let resolve = |mut t: u32| {
        while let Some((_, old)) = absorbed_into.iter().find(|(y, _)| *y == t) {
            t = *old;
        }
        t
    };
    // La plus ancienne branche survit jusqu'au bout.
    if let Some(&v) = order.first() {
        let root = resolve(label[v as usize]);
        if !kept.iter().any(|(t, _)| *t == root) {
            kept.push((root, f32::NEG_INFINITY));
        }
    }
    // Commissure : plus haute fusion subie (avec une branche significative).
    let webs: Vec<f32> = kept
        .iter()
        .map(|&(t, level)| {
            merges
                .iter()
                .filter(|(young, old, _)| resolve(*old) == t && kept.iter().any(|(k, _)| k == young))
                .map(|m| m.2)
                .fold(level, f32::max)
        })
        .collect();

    kept.iter()
        .zip(webs)
        .filter(|((t, _), web)| web.is_finite() && significant(*t, *web))
        .map(|(&(t, _), web)| {
            let members: Vec<u32> = order
                .iter()
                .copied()
                .filter(|v| dist[*v as usize] >= web && resolve(label[*v as usize]) == t)
                .collect();
            Branch {
                tip: s.positions[t as usize],
                web,
                axis: band_centers(s, dist, &members, web, band),
            }
        })
        .collect()
}

/// Centres des bandes de distance `[web + k·band, web + (k + 1)·band)`,
/// lissés avec les bandes voisines (une bande seule ne couvre souvent qu'une
/// partie du tour du doigt).
fn band_centers(s: &Surface, dist: &[f32], members: &[u32], web: f32, band: f32) -> Vec<Vec3> {
    let mut sums: Vec<(Vec3, u32)> = Vec::new();
    for &v in members {
        let k = ((dist[v as usize] - web) / band) as usize;
        if sums.len() <= k {
            sums.resize(k + 1, (Vec3::ZERO, 0));
        }
        sums[k].0 += s.positions[v as usize];
        sums[k].1 += 1;
    }
    (0..sums.len())
        .filter(|k| sums[*k].1 > 0)
        .map(|k| {
            let window = &sums[k.saturating_sub(1)..(k + 2).min(sums.len())];
            let (sum, count) = window
                .iter()
                .fold((Vec3::ZERO, 0), |(s, c), (bs, bc)| (s + *bs, c + bc));
            sum / count as f32
        })
        .collect()
}

fn find(parent: &mut [u32], mut x: u32) -> u32 {
    while parent[x as usize] != x {
        parent[x as usize] = parent[parent[x as usize] as usize];
        x = parent[x as usize];
    }
    x
}
