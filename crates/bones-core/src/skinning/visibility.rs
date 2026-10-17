//! Visibilité sommet → os pour la diffusion de chaleur.
//!
//! Comme dans Baran & Popović (et Blender), un os ne « chauffe » un sommet
//! que si le segment qui les relie reste dans le volume : il ne doit pas
//! traverser la surface. Sans ce test, en pose en A, les flancs du torse
//! seraient attribués aux bras tout proches, et l'intérieur des cuisses à
//! la jambe opposée.
//!
//! Les segments sont testés contre une hiérarchie de boîtes englobantes
//! (BVH) des triangles ; les triangles du sommet de départ sont ignorés.

use glam::Vec3;
use rayon::prelude::*;

use super::envelope::distance_to_segment;
use crate::Mesh;

/// Triangles par feuille de la hiérarchie.
const LEAF: usize = 4;
/// Marge (part du segment) ignorée aux extrémités.
const EPSILON: f32 = 1e-3;
/// Tolérance sur les coordonnées barycentriques des intersections.
const BARY_EPSILON: f32 = 1e-4;

struct Node {
    lo: Vec3,
    hi: Vec3,
    /// Feuille : plage `start..start + count` de `order` ; sinon `count == 0`
    /// et les enfants sont `start` et `start + 1`.
    start: u32,
    count: u32,
}

pub(super) struct Occluder<'a> {
    positions: &'a [Vec3],
    tris: Vec<[u32; 3]>,
    order: Vec<u32>,
    nodes: Vec<Node>,
}

impl<'a> Occluder<'a> {
    pub fn new(mesh: &'a Mesh) -> Self {
        let tris: Vec<[u32; 3]> = mesh.indices.as_chunks::<3>().0.to_vec();
        let mut occluder = Self {
            positions: &mesh.positions,
            order: (0..tris.len() as u32).collect(),
            tris,
            nodes: Vec::new(),
        };
        if !occluder.tris.is_empty() {
            occluder.nodes.push(Node {
                lo: Vec3::ZERO,
                hi: Vec3::ZERO,
                start: 0,
                count: occluder.tris.len() as u32,
            });
            occluder.split(0);
        }
        occluder
    }

    fn centroid(&self, t: u32) -> Vec3 {
        let [a, b, c] = self.tris[t as usize];
        (self.positions[a as usize] + self.positions[b as usize] + self.positions[c as usize]) / 3.0
    }

    /// Subdivise un nœud (partage au milieu de l'axe le plus long).
    fn split(&mut self, index: usize) {
        let (start, count) = (self.nodes[index].start as usize, self.nodes[index].count as usize);
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for &t in &self.order[start..start + count] {
            for v in self.tris[t as usize] {
                lo = lo.min(self.positions[v as usize]);
                hi = hi.max(self.positions[v as usize]);
            }
        }
        self.nodes[index].lo = lo;
        self.nodes[index].hi = hi;
        if count <= LEAF {
            return;
        }
        let axis = (hi - lo).max_position();
        let mut range: Vec<u32> = self.order[start..start + count].to_vec();
        let mid = count / 2;
        range.select_nth_unstable_by(mid, |a, b| {
            self.centroid(*a)[axis].total_cmp(&self.centroid(*b)[axis])
        });
        self.order[start..start + count].copy_from_slice(&range);
        let left = self.nodes.len();
        self.nodes.push(Node {
            lo,
            hi,
            start: start as u32,
            count: mid as u32,
        });
        self.nodes.push(Node {
            lo,
            hi,
            start: (start + mid) as u32,
            count: (count - mid) as u32,
        });
        self.nodes[index].start = left as u32;
        self.nodes[index].count = 0;
        self.split(left);
        self.split(left + 1);
    }

    /// Vrai si le segment du sommet `vertex` (position `from`) au point `to`
    /// ne traverse aucun triangle (hors ceux du sommet).
    pub fn visible(&self, vertex: u32, from: Vec3, to: Vec3) -> bool {
        if self.nodes.is_empty() {
            return true;
        }
        let dir = to - from;
        let inv = dir.recip();
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            let node = &self.nodes[i];
            if !hits_box(from, inv, node.lo, node.hi) {
                continue;
            }
            if node.count == 0 {
                stack.push(node.start as usize);
                stack.push(node.start as usize + 1);
                continue;
            }
            let range = node.start as usize..(node.start + node.count) as usize;
            for &t in &self.order[range] {
                let tri = self.tris[t as usize];
                if tri.contains(&vertex) {
                    continue;
                }
                let [a, b, c] = tri.map(|v| self.positions[v as usize]);
                if let Some(s) = intersect(from, dir, a, b, c)
                    && s > EPSILON
                    && s < 1.0 - EPSILON
                {
                    return false;
                }
            }
        }
        true
    }
}

/// Le segment `from + s·dir`, s ∈ [0, 1], touche-t-il la boîte ?
fn hits_box(from: Vec3, inv: Vec3, lo: Vec3, hi: Vec3) -> bool {
    let t1 = (lo - from) * inv;
    let t2 = (hi - from) * inv;
    let near = t1.min(t2);
    let far = t1.max(t2);
    // Les NaN (rayon parallèle à une face, sur le bord) ne doivent pas exclure.
    let enter = [near.x, near.y, near.z, 0.0]
        .into_iter()
        .filter(|v| !v.is_nan())
        .fold(f32::MIN, f32::max);
    let exit = [far.x, far.y, far.z, 1.0]
        .into_iter()
        .filter(|v| !v.is_nan())
        .fold(f32::MAX, f32::min);
    enter <= exit
}

/// Intersection rayon–triangle (Möller–Trumbore, deux faces) : paramètre `s`
/// le long de `dir`.
fn intersect(from: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let e1 = b - a;
    let e2 = c - a;
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let t = from - a;
    // Triangle très légèrement agrandi : un segment qui passe pile sur une
    // arête commune ne doit pas glisser entre les deux triangles.
    let u = t.dot(p) * inv;
    if !(-BARY_EPSILON..=1.0 + BARY_EPSILON).contains(&u) {
        return None;
    }
    let q = t.cross(e1);
    let v = dir.dot(q) * inv;
    if v < -BARY_EPSILON || u + v > 1.0 + BARY_EPSILON {
        return None;
    }
    Some(e2.dot(q) * inv)
}

/// Point du segment [a, b] le plus proche de `p`.
pub(super) fn closest_on_segment(p: Vec3, a: Vec3, b: Vec3) -> Vec3 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 <= 0.0 {
        return a;
    }
    a + ab * ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
}

/// Os les plus proches d'un sommet.
pub(super) struct Nearest {
    /// Distance à l'os le plus proche.
    pub distance: f32,
    /// Os à égalité (0,1 %) avec le plus proche et visibles depuis le sommet ;
    /// vide si l'os le plus proche est caché : le sommet ne reçoit alors ses
    /// poids que par diffusion.
    pub ties: Vec<u16>,
    /// Os le plus proche, visible ou non (repli des parties isolées).
    pub closest: u16,
}

/// Pour chaque sommet, les os les plus proches qu'il voit (segments
/// tête–queue en pose de repos). Comme dans Blender, seul l'os le plus proche
/// (et ses égaux) peut chauffer un sommet : un os plus lointain mais visible
/// ne compte pas (sinon un œil, isolé dans l'orbite, pourrait suivre l'épaule).
/// Calcul parallèle sur les sommets.
pub(super) fn nearest_visible(mesh: &Mesh, segments: &[(Vec3, Vec3)]) -> Vec<Nearest> {
    let occluder = Occluder::new(mesh);
    mesh.positions
        .par_iter()
        .enumerate()
        .map(|(v, &p)| {
            let d: Vec<f32> = segments
                .iter()
                .map(|(a, b)| distance_to_segment(p, *a, *b))
                .collect();
            let mut order: Vec<u16> = (0..d.len() as u16).collect();
            order.sort_by(|a, b| d[*a as usize].total_cmp(&d[*b as usize]));
            let min = d[order[0] as usize];
            let ties = order
                .iter()
                .copied()
                .take_while(|i| d[*i as usize] <= min * 1.001 + 1e-7)
                .filter(|i| {
                    let (a, b) = segments[*i as usize];
                    occluder.visible(v as u32, p, closest_on_segment(p, a, b))
                })
                .collect();
            Nearest {
                distance: min.max(1e-4),
                ties,
                closest: order[0],
            }
        })
        .collect()
}
