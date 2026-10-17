//! Détection par la topologie et la géométrie de repos, pour les squelettes
//! aux noms quelconques (« Bone.012 », « joint7 ») : le bassin est le premier
//! embranchement, les paires de branches symétriques sont des membres, la
//! branche principale restante est la colonne, qui mène à un second
//! embranchement (épaules) puis au cou et à la tête.

use glam::Vec3;

use super::names::{NameHint, Part};
use super::{Chain, Lr, Role, RoleMap};
use crate::{BoneId, Skeleton, head_tail};

/// Données géométriques du squelette au repos.
pub(super) struct Geo {
    pub heads: Vec<Vec3>,
    pub tails: Vec<Vec3>,
    pub children: Vec<Vec<BoneId>>,
    /// Plus longue distance (somme des longueurs) jusqu'au bout d'une branche.
    pub reach: Vec<f32>,
    pub lo: Vec3,
    pub size: f32,
}

impl Geo {
    pub fn new(skeleton: &Skeleton) -> Self {
        let world = skeleton.rest_world();
        let n = skeleton.len();
        let (mut heads, mut tails) = (Vec::with_capacity(n), Vec::with_capacity(n));
        let mut children = vec![Vec::new(); n];
        for (i, (w, b)) in world.iter().zip(skeleton.bones()).enumerate() {
            let (h, t) = head_tail(w, b.length);
            heads.push(h);
            tails.push(t);
            if let Some(p) = b.parent() {
                children[p.index()].push(BoneId(i as u32));
            }
        }
        let mut reach: Vec<f32> = skeleton.bones().iter().map(|b| b.length).collect();
        for i in (0..n).rev() {
            if let Some(p) = skeleton.bones()[i].parent() {
                let via = skeleton.bones()[p.index()].length + reach[i];
                reach[p.index()] = reach[p.index()].max(via);
            }
        }
        let (lo, hi) = super::frame::extent(skeleton);
        Self {
            heads,
            tails,
            children,
            reach,
            lo,
            size: (hi - lo).max_element().max(1e-3),
        }
    }

    pub fn dir(&self, b: BoneId) -> Vec3 {
        (self.tails[b.index()] - self.heads[b.index()]).normalize_or(Vec3::Y)
    }

    pub fn len(&self, b: BoneId) -> f32 {
        (self.tails[b.index()] - self.heads[b.index()]).length()
    }

    /// Enfants qui comptent (assez longs, ni auxiliaires ni bouts de chaîne).
    pub fn significant(&self, b: BoneId, hints: &[NameHint]) -> Vec<BoneId> {
        self.children[b.index()]
            .iter()
            .copied()
            .filter(|c| {
                self.reach[c.index()] >= 0.02 * self.size
                    && !matches!(hints[c.index()].part, Some(Part::Ignore | Part::End))
            })
            .collect()
    }

    /// Chemin principal depuis `b` : l'enfant de plus longue portée à chaque pas.
    pub fn main_path(&self, b: BoneId, hints: &[NameHint]) -> Vec<BoneId> {
        let mut path = vec![b];
        let mut cur = b;
        while let Some(next) = self
            .significant(cur, hints)
            .into_iter()
            .max_by(|x, y| self.reach[x.index()].total_cmp(&self.reach[y.index()]))
        {
            path.push(next);
            cur = next;
        }
        path
    }
}

/// Branche partant d'un embranchement.
struct Branch {
    path: Vec<BoneId>,
    end: Vec3,
}

/// Rôles devinés par la topologie seule.
pub(super) fn detect(skeleton: &Skeleton, hints: &[NameHint], geo: &Geo) -> RoleMap {
    let mut map = RoleMap::empty(skeleton);
    let Some(main) = skeleton
        .ids()
        .filter(|b| skeleton.bone(*b).parent().is_none())
        .max_by_key(|b| skeleton.subtree(*b).len())
    else {
        return map;
    };
    // Descente jusqu'au premier embranchement : le bassin.
    let mut hub = main;
    let mut kids = geo.significant(hub, hints);
    while kids.len() == 1 {
        hub = kids[0];
        kids = geo.significant(hub, hints);
    }
    if kids.is_empty() {
        return map;
    }
    set(&mut map, hub, Chain::Root, 0);
    if main != hub && geo.heads[main.index()].y < geo.lo.y + 0.1 * geo.size {
        set(&mut map, main, Chain::Ground, 0);
    }
    let branches: Vec<Branch> = kids.iter().map(|k| branch(geo, hints, *k)).collect();
    let (pairs, rest) = pair_up(geo, hub, &branches);
    // Colonne : branche non appariée la plus fournie ; queue : une autre.
    let mut rest = rest;
    rest.sort_by_key(|i| std::cmp::Reverse(skeleton.subtree(branches[*i].path[0]).len()));
    let mut all_pairs: Vec<(usize, usize, bool)> = pairs.iter().map(|(a, b)| (*a, *b, true)).collect();
    let mut groups: Vec<&[Branch]> = vec![&branches];
    let chest_branches;
    if let Some(&spine) = rest.first() {
        let path = &branches[spine].path;
        // Second embranchement le long de la colonne : épaules.
        let chest = path.iter().position(|b| geo.significant(*b, hints).len() >= 2);
        let spine_end = chest.unwrap_or(path.len().saturating_sub(1));
        let neck_head: Vec<BoneId> = match chest {
            Some(c) => {
                let kids = geo.significant(path[c], hints);
                chest_branches = kids.iter().map(|k| branch(geo, hints, *k)).collect::<Vec<_>>();
                let (p, r) = pair_up(geo, path[c], &chest_branches);
                all_pairs.extend(p.iter().map(|(a, b)| (*a, *b, false)));
                groups.push(&chest_branches);
                r.into_iter()
                    .max_by(|a, b| geo.reach[chest_branches[*a].path[0].index()].total_cmp(&geo.reach[chest_branches[*b].path[0].index()]))
                    .map(|i| chest_branches[i].path.clone())
                    .unwrap_or_default()
            }
            None => Vec::new(),
        };
        let spine_bones = if chest.is_some() { &path[..=spine_end] } else { &path[..path.len().saturating_sub(1)] };
        for (i, b) in spine_bones.iter().enumerate() {
            set(&mut map, *b, Chain::Spine, i);
        }
        let head = match chest {
            Some(_) => neck_head.last().copied(),
            None => path.last().copied().filter(|_| path.len() > 1),
        };
        for (i, b) in neck_head.iter().take(neck_head.len().saturating_sub(1)).enumerate() {
            set(&mut map, *b, Chain::Neck, i);
        }
        if let Some(h) = head {
            set(&mut map, h, Chain::Head, 0);
        }
        if let Some(&tail) = rest.get(1) {
            for (i, b) in branches[tail].path.iter().enumerate() {
                set(&mut map, *b, Chain::Tail, i);
            }
        }
    }
    assign_limbs(&mut map, geo, hints, &groups, &all_pairs);
    map
}

fn set(map: &mut RoleMap, b: BoneId, chain: Chain, index: usize) {
    map.roles[b.index()] = Some(Role::new(chain, index.min(255) as u8));
}

fn branch(geo: &Geo, hints: &[NameHint], start: BoneId) -> Branch {
    let path = geo.main_path(start, hints);
    let end = geo.tails[path.last().unwrap().index()];
    Branch { path, end }
}

/// Paires de branches symétriques (même portée, extrémités à même hauteur,
/// écartées latéralement). Retourne les paires et les branches seules.
fn pair_up(geo: &Geo, hub: BoneId, branches: &[Branch]) -> (Vec<(usize, usize)>, Vec<usize>) {
    let center = geo.heads[hub.index()];
    let mut candidates = Vec::new();
    for i in 0..branches.len() {
        for j in i + 1..branches.len() {
            let (a, b) = (&branches[i], &branches[j]);
            let (ra, rb) = (geo.reach[a.path[0].index()], geo.reach[b.path[0].index()]);
            let reach_gap = (ra - rb).abs() / ra.max(rb).max(1e-6);
            let height_gap = (a.end.y - b.end.y).abs() / geo.size;
            let spread = (a.end - b.end).length();
            // Les deux extrémités à égale distance du centre.
            let balance = ((a.end - center).length() - (b.end - center).length()).abs() / spread.max(1e-6);
            // Des branches qui montent (arbre) ne sont pas des membres.
            let rise = (a.end.y + b.end.y) * 0.5 - center.y;
            let limb = rise < 0.5 * ra.max(rb);
            if limb && reach_gap < 0.2 && height_gap < 0.08 && spread > 0.05 * geo.size && balance < 0.25 {
                candidates.push((reach_gap + height_gap + balance, i, j));
            }
        }
    }
    candidates.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut used = vec![false; branches.len()];
    let mut pairs = Vec::new();
    for (_, i, j) in candidates {
        if !used[i] && !used[j] {
            used[i] = true;
            used[j] = true;
            pairs.push((i, j));
        }
    }
    let rest = (0..branches.len()).filter(|i| !used[*i]).collect();
    (pairs, rest)
}

/// Membres : jambes au bassin, bras aux épaules ; au-delà de deux paires,
/// pattes numérotées de l'avant vers l'arrière.
fn assign_limbs(
    map: &mut RoleMap,
    geo: &Geo,
    hints: &[NameHint],
    groups: &[&[Branch]],
    pairs: &[(usize, usize, bool)],
) {
    let spine = map.chain(Chain::Spine);
    let forward = match (map.root(), spine.last()) {
        (Some(r), Some(s)) => {
            let d = geo.heads[s.index()] - geo.heads[r.index()];
            let flat = Vec3::new(d.x, 0.0, d.z);
            if flat.length() > 0.5 * d.length() { flat.normalize() } else { Vec3::Z }
        }
        _ => Vec3::Z,
    };
    let left = Vec3::Y.cross(forward).normalize_or(Vec3::X);
    let group_of = |at_hub: bool| if at_hub { groups[0] } else { groups[groups.len() - 1] };
    let multi = pairs.len() > 2;
    let mut order: Vec<&(usize, usize, bool)> = pairs.iter().collect();
    // De l'avant vers l'arrière (position de l'attache le long du corps).
    order.sort_by(|x, y| {
        let at = |p: &(usize, usize, bool)| geo.heads[group_of(p.2)[p.0].path[0].index()].dot(forward);
        at(y).total_cmp(&at(x))
    });
    for (n, &&(a, b, at_hub)) in order.iter().enumerate() {
        let group = group_of(at_hub);
        let (ba, bb) = (&group[a], &group[b]);
        let a_left = (ba.end - bb.end).dot(left) >= 0.0;
        for (br, is_left) in [(ba, a_left), (bb, !a_left)] {
            let side = if is_left { Lr::Left } else { Lr::Right };
            let chain = if multi {
                Chain::LegPair(n as u8, side)
            } else if at_hub {
                Chain::Leg(side)
            } else {
                Chain::Arm(side)
            };
            let mut path: &[BoneId] = &br.path;
            // Os court en tête de membre : clavicule (bras) ou attache (jambe).
            if !multi && path.len() >= 4 && geo.len(path[0]) < 0.6 * geo.len(path[1]) {
                if matches!(chain, Chain::Arm(_)) {
                    set(map, path[0], Chain::Clavicle(side), 0);
                }
                path = &path[1..];
            }
            // La main (os d'où partent plusieurs doigts) termine le bras.
            let hand = path.iter().position(|b| geo.significant(*b, hints).len() >= 3);
            let count = hand.map_or(4, |h| h + 1).min(4);
            for (i, bone) in path.iter().take(count).enumerate() {
                set(map, *bone, chain, i);
            }
        }
    }
}
