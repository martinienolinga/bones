//! Point le plus proche sur un maillage : hiérarchie de boîtes englobantes
//! (BVH, partage médian sur l'axe le plus long) parcourue par « séparation
//! et évaluation » (branch and bound), et point le plus proche d'un
//! triangle selon Ericson, *Real-Time Collision Detection* (2005), § 5.1.5.

use glam::Vec3;

/// Triangles par feuille.
const LEAF: usize = 4;

struct Node {
    lo: Vec3,
    hi: Vec3,
    /// Feuille : plage `start..start + count` de `order` ; sinon `count == 0`
    /// et les enfants sont `start` et `start + 1`.
    start: u32,
    count: u32,
}

/// Point trouvé : triangle, coordonnées barycentriques et distance au carré.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hit {
    pub triangle: [u32; 3],
    pub bary: Vec3,
    pub distance2: f32,
}

pub(crate) struct ClosestTriangles<'a> {
    positions: &'a [Vec3],
    tris: Vec<[u32; 3]>,
    order: Vec<u32>,
    nodes: Vec<Node>,
}

impl<'a> ClosestTriangles<'a> {
    /// Construit la hiérarchie ; les triangles d'indices invalides sont ignorés.
    pub fn new(positions: &'a [Vec3], indices: &[u32]) -> Self {
        let n = positions.len();
        let tris: Vec<[u32; 3]> = indices
            .as_chunks::<3>()
            .0
            .iter()
            .filter(|t| t.iter().all(|i| (*i as usize) < n))
            .copied()
            .collect();
        let mut tree = Self {
            positions,
            order: (0..tris.len() as u32).collect(),
            tris,
            nodes: Vec::new(),
        };
        if !tree.tris.is_empty() {
            tree.nodes.push(Node {
                lo: Vec3::ZERO,
                hi: Vec3::ZERO,
                start: 0,
                count: tree.tris.len() as u32,
            });
            // Pile explicite : pas de récursion profonde.
            let mut stack = vec![0usize];
            while let Some(i) = stack.pop() {
                stack.extend(tree.split(i));
            }
        }
        tree
    }

    fn corners(&self, t: u32) -> [Vec3; 3] {
        self.tris[t as usize].map(|v| self.positions[v as usize])
    }

    /// Calcule la boîte d'un nœud et le coupe en deux s'il est trop grand ;
    /// retourne les enfants à traiter.
    fn split(&mut self, index: usize) -> Vec<usize> {
        let (start, count) = (self.nodes[index].start as usize, self.nodes[index].count as usize);
        let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
        for &t in &self.order[start..start + count] {
            for p in self.corners(t) {
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
        self.nodes[index].lo = lo;
        self.nodes[index].hi = hi;
        if count <= LEAF {
            return Vec::new();
        }
        let axis = (hi - lo).max_position();
        let mid = count / 2;
        let centroid = |t: &u32| self.corners(*t).iter().sum::<Vec3>()[axis];
        let mut range: Vec<u32> = self.order[start..start + count].to_vec();
        range.select_nth_unstable_by(mid, |a, b| centroid(a).total_cmp(&centroid(b)));
        self.order[start..start + count].copy_from_slice(&range);
        let left = self.nodes.len();
        for (s, c) in [(start, mid), (start + mid, count - mid)] {
            self.nodes.push(Node {
                lo,
                hi,
                start: s as u32,
                count: c as u32,
            });
        }
        self.nodes[index].start = left as u32;
        self.nodes[index].count = 0;
        vec![left, left + 1]
    }

    /// Point du maillage le plus proche de `p` ; `None` sans triangle.
    pub fn closest(&self, p: Vec3) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            let Some(node) = self.nodes.get(i) else { break };
            let bound = best.map_or(f32::INFINITY, |b| b.distance2);
            if box_distance2(p, node.lo, node.hi) >= bound {
                continue;
            }
            if node.count == 0 {
                // Enfant le plus proche visité en premier (empilé en dernier).
                let (a, b) = (node.start as usize, node.start as usize + 1);
                let da = box_distance2(p, self.nodes[a].lo, self.nodes[a].hi);
                let db = box_distance2(p, self.nodes[b].lo, self.nodes[b].hi);
                stack.extend(if da <= db { [b, a] } else { [a, b] });
                continue;
            }
            for &t in &self.order[node.start as usize..(node.start + node.count) as usize] {
                let [a, b, c] = self.corners(t);
                let bary = closest_on_triangle(p, a, b, c);
                let q = a * bary.x + b * bary.y + c * bary.z;
                let d2 = (q - p).length_squared();
                if best.is_none_or(|h| d2 < h.distance2) {
                    best = Some(Hit {
                        triangle: self.tris[t as usize],
                        bary,
                        distance2: d2,
                    });
                }
            }
        }
        best
    }
}

/// Distance au carré d'un point à une boîte (0 à l'intérieur).
fn box_distance2(p: Vec3, lo: Vec3, hi: Vec3) -> f32 {
    let d = (lo - p).max(Vec3::ZERO).max(p - hi);
    d.length_squared()
}

/// Coordonnées barycentriques du point du triangle `abc` le plus proche de
/// `p` (Ericson, § 5.1.5 : régions de Voronoï des sommets, arêtes, face).
fn closest_on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Vec3 {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return Vec3::X;
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return Vec3::Y;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return Vec3::new(1.0 - v, v, 0.0);
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return Vec3::Z;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return Vec3::new(1.0 - w, 0.0, w);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return Vec3::new(0.0, 1.0 - w, w);
    }
    let denom = va + vb + vc;
    if denom.abs() < 1e-30 || !denom.is_finite() {
        // Triangle dégénéré : sommet le plus proche.
        let d = [a, b, c].map(|q| (q - p).length_squared());
        let i = (0..3).min_by(|x, y| d[*x].total_cmp(&d[*y])).unwrap_or(0);
        return [Vec3::X, Vec3::Y, Vec3::Z][i];
    }
    let (v, w) = (vb / denom, vc / denom);
    Vec3::new(1.0 - v - w, v, w)
}
