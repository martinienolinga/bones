//! Détection des rôles : noms d'abord, topologie pour compléter, puis type
//! de rig et repère du corps.

use super::names::{self, NameHint, Part};
use super::topology::{self, Geo};
use super::{Chain, Lr, Role, RoleMap, classify, frame};
use crate::{BoneId, Skeleton};

/// Devine le type de rig, le rôle de chaque os et le repère du corps.
pub fn detect(skeleton: &Skeleton) -> RoleMap {
    let mut map = RoleMap::empty(skeleton);
    if skeleton.is_empty() {
        return map;
    }
    let hints: Vec<NameHint> = skeleton.bones().iter().map(|b| names::parse(&b.name)).collect();
    let geo = Geo::new(skeleton);
    from_names(skeleton, &hints, &geo, &mut map);
    let topo = topology::detect(skeleton, &hints, &geo);
    if topo.chains().iter().any(|c| matches!(c, Chain::LegPair(..))) {
        // Plus de deux paires de pattes : la topologie l'emporte pour les membres.
        for role in &mut map.roles {
            if role.is_some_and(|r| matches!(r.chain, Chain::Arm(_) | Chain::Leg(_) | Chain::Clavicle(_))) {
                *role = None;
            }
        }
    }
    merge(&mut map, topo);
    map.rig_type = classify::rig_type(skeleton, &hints, &geo, &map);
    classify::finish(skeleton, &hints, &geo, &mut map);
    map.frame = frame::estimate(skeleton, &map);
    map
}

/// Complète les chaînes absentes avec celles trouvées par la topologie,
/// si leurs os sont libres.
fn merge(map: &mut RoleMap, topo: RoleMap) {
    let named_chains = map.chains();
    for chain in topo.chains() {
        if named_chains.contains(&chain) {
            continue;
        }
        let bones = topo.chain(chain);
        if bones.iter().all(|b| map.role(*b).is_none()) {
            for b in bones {
                map.roles[b.index()] = topo.role(b);
            }
        }
    }
}

fn depth_sorted(skeleton: &Skeleton, mut bones: Vec<(BoneId, Option<u8>)>) -> Vec<(BoneId, Option<u8>)> {
    bones.sort_by_key(|(b, _)| (skeleton.depth(*b), *b));
    bones
}

/// Rôles d'après les noms.
fn from_names(skeleton: &Skeleton, hints: &[NameHint], geo: &Geo, map: &mut RoleMap) {
    let with = |pred: &dyn Fn(&NameHint) -> Option<Option<u8>>| -> Vec<(BoneId, Option<u8>)> {
        let bones = skeleton
            .ids()
            .filter_map(|b| pred(&hints[b.index()]).map(|seg| (b, seg)))
            .collect();
        depth_sorted(skeleton, bones)
    };
    // Côté d'un os sans suffixe : d'après sa position (+X à gauche).
    let side_of = |b: BoneId, h: &NameHint| {
        h.side.unwrap_or(if geo.heads[b.index()].x >= 0.0 { Lr::Left } else { Lr::Right })
    };

    // Bassin : os « hips/pelvis », sinon ancêtre commun des deux jambes,
    // sinon premier os « root » qui a des enfants.
    let legs = with(&|h| matches!(h.part, Some(Part::Leg(_))).then_some(None));
    let left_leg = legs.iter().find(|(b, _)| side_of(*b, &hints[b.index()]) == Lr::Left);
    let right_leg = legs.iter().find(|(b, _)| side_of(*b, &hints[b.index()]) == Lr::Right);
    let root = with(&|h| (h.part == Some(Part::Hips)).then_some(None))
        .first()
        .map(|(b, _)| *b)
        .or_else(|| match (left_leg, right_leg) {
            (Some((l, _)), Some((r, _))) => common_ancestor(skeleton, *l, *r),
            _ => None,
        })
        .or_else(|| {
            with(&|h| (h.part == Some(Part::RootWord)).then_some(None))
                .into_iter()
                .map(|(b, _)| b)
                .find(|b| !geo.children[b.index()].is_empty())
        });
    if let Some(r) = root {
        map.roles[r.index()] = Some(Role::new(Chain::Root, 0));
        let ground = skeleton.ids().filter(|b| *b != r && skeleton.is_ancestor(*b, r)).find(|b| {
            hints[b.index()].part == Some(Part::RootWord)
        });
        if let Some(g) = ground {
            map.roles[g.index()] = Some(Role::new(Chain::Ground, 0));
        }
    }
    let under_root = |b: BoneId| root.is_none_or(|r| skeleton.is_ancestor(r, b) && b != r);

    let simple = [
        (Part::Spine, Chain::Spine),
        (Part::Neck, Chain::Neck),
        (Part::Head, Chain::Head),
        (Part::Jaw, Chain::Jaw),
        (Part::Tail, Chain::Tail),
    ];
    for (part, chain) in simple {
        let bones: Vec<_> = with(&|h| (h.part == Some(part)).then_some(None))
            .into_iter()
            .filter(|(b, _)| under_root(*b) && map.role(*b).is_none())
            .collect();
        let limit = if matches!(chain, Chain::Head | Chain::Jaw) { 1 } else { usize::MAX };
        assign_chain(skeleton, geo, map, chain, bones.into_iter().take(limit).collect());
    }
    for side in Lr::BOTH {
        let sided = |map: &RoleMap, pred: &dyn Fn(Part) -> Option<Option<u8>>| {
            with(&|h| h.part.and_then(pred))
                .into_iter()
                .filter(|(b, _)| side_of(*b, &hints[b.index()]) == side && map.role(*b).is_none())
                .collect::<Vec<_>>()
        };
        let clavicle = sided(map, &|p| (p == Part::Clavicle).then_some(None));
        assign_chain(skeleton, geo, map, Chain::Clavicle(side), clavicle.into_iter().take(1).collect());
        let arm = sided(map, &|p| match p {
            Part::Arm(seg) => Some(seg),
            Part::Wing => Some(None),
            _ => None,
        });
        assign_chain(skeleton, geo, map, Chain::Arm(side), arm);
        let leg = sided(map, &|p| if let Part::Leg(seg) = p { Some(seg) } else { None });
        assign_chain(skeleton, geo, map, Chain::Leg(side), leg);
        for finger in 0..5u8 {
            let bones = sided(map, &|p| (p == Part::Finger(finger)).then_some(None));
            assign_chain(skeleton, geo, map, Chain::Finger(side, finger), bones);
        }
        // `LeftShoulder / LeftElbow / LeftWrist` : l'épaule est le haut du bras.
        let arm = map.chain(Chain::Arm(side));
        if let (Some(first), Some(c)) = (arm.first(), map.bone(Chain::Clavicle(side), 0))
            && map.role(*first).is_some_and(|r| r.index == 1)
            && skeleton.bone(*first).parent() == Some(c)
        {
            map.roles[c.index()] = Some(Role::new(Chain::Arm(side), 0));
        }
    }
}

/// Numérote une chaîne (os triés par profondeur) : chaque os doit
/// descendre du précédent ; un segment déjà vu n'est accepté que s'il
/// change de direction (orteil après le pied), sinon c'est un os de
/// découpe (`thigh.L.001`) laissé sans rôle.
fn assign_chain(
    skeleton: &Skeleton,
    geo: &Geo,
    map: &mut RoleMap,
    chain: Chain,
    bones: Vec<(BoneId, Option<u8>)>,
) {
    let mut next = 0u8;
    let mut last: Option<BoneId> = None;
    for (b, hint) in bones {
        if last.is_some_and(|l| !skeleton.is_ancestor(l, b)) {
            continue;
        }
        let index = match hint {
            Some(h) if h < next => {
                let bent = last.is_some_and(|l| geo.dir(l).dot(geo.dir(b)) < 0.94);
                if !bent {
                    continue;
                }
                next
            }
            Some(h) => h,
            None => next,
        };
        map.roles[b.index()] = Some(Role::new(chain, index));
        next = index.saturating_add(1);
        last = Some(b);
    }
}

fn common_ancestor(skeleton: &Skeleton, a: BoneId, b: BoneId) -> Option<BoneId> {
    let mut cur = skeleton.bone(a).parent();
    while let Some(p) = cur {
        if skeleton.is_ancestor(p, b) {
            return Some(p);
        }
        cur = skeleton.bone(p).parent();
    }
    None
}
