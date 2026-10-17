//! Type de rig d'après les rôles trouvés et la forme du squelette, puis
//! rôles des corps sans membres (serpent, poisson) et chaînes libres
//! (végétaux, cordes).

use glam::Vec3;

use super::names::{NameHint, Part};
use super::topology::Geo;
use super::{Chain, Lr, RigType, Role, RoleMap};
use crate::{BoneId, Skeleton};

pub(super) fn rig_type(skeleton: &Skeleton, hints: &[NameHint], geo: &Geo, map: &RoleMap) -> RigType {
    let word = |p: Part| hints.iter().any(|h| h.part == Some(p));
    let has = |c: Chain| map.bone(c, 0).is_some() || !map.chain(c).is_empty();
    // Noms de végétaux majoritaires (tronc, branches, feuilles).
    let plant_words = hints.iter().filter(|h| h.part == Some(Part::Plant)).count();
    if plant_words * 3 >= skeleton.len() {
        return RigType::Plant;
    }
    // Pattes multiples, posées au sol.
    let legs_down = map
        .chains()
        .into_iter()
        .filter(|c| matches!(c, Chain::LegPair(_, Lr::Left)))
        .filter(|c| {
            let tip = map.chain(*c).last().map(|b| geo.tails[b.index()].y);
            tip.is_some_and(|y| y < geo.lo.y + 0.25 * geo.size)
        })
        .count();
    if legs_down >= 3 {
        return RigType::MultiLegged;
    }
    let legs = has(Chain::Leg(Lr::Left)) && has(Chain::Leg(Lr::Right));
    let arms = has(Chain::Arm(Lr::Left)) && has(Chain::Arm(Lr::Right));
    let ground = geo.lo.y;
    let height = map
        .root()
        .map_or(geo.size, |r| (geo.heads[r.index()].y - ground).max(1e-3));
    let reaches_ground = |side| {
        map.chain(Chain::Arm(side))
            .last()
            .is_some_and(|b| geo.tails[b.index()].y < ground + 0.2 * height)
    };
    // Colonne verticale : du bassin à la poitrine (ou à la tête).
    let vertical = match (
        map.root(),
        map.chain(Chain::Spine).last().or(map.chain(Chain::Head).first()),
    ) {
        (Some(r), Some(top)) => {
            let d = geo.heads[top.index()] - geo.heads[r.index()];
            d.normalize_or(Vec3::Y).y > 0.7
        }
        _ => true,
    };
    if word(Part::Wing) && legs {
        return RigType::Bird;
    }
    if legs && arms {
        return if reaches_ground(Lr::Left) && reaches_ground(Lr::Right) {
            RigType::Quadruped
        } else if vertical {
            RigType::Humanoid
        } else {
            RigType::Bird
        };
    }
    if legs {
        return if vertical {
            RigType::Humanoid
        } else {
            RigType::Bird
        };
    }
    if word(Part::Fin) {
        return RigType::Fish;
    }
    if arms {
        return RigType::Any;
    }
    // Long corps sans membres : serpent au sol, poisson dans l'eau.
    let body = map.chain(Chain::Spine);
    if body.len() >= 5 {
        let (first, last) = (
            geo.heads[body[0].index()],
            geo.tails[body[body.len() - 1].index()],
        );
        let length: f32 = body.iter().map(|b| geo.len(*b)).sum();
        if (last - first).normalize_or(Vec3::Y).y.abs() < 0.4 {
            let top = body
                .iter()
                .map(|b| geo.heads[b.index()].y)
                .fold(f32::MIN, f32::max);
            return if top - geo.lo.y < 0.15 * length {
                RigType::Serpent
            } else {
                RigType::Fish
            };
        }
    }
    if word(Part::Plant) {
        return RigType::Plant;
    }
    shape_type(skeleton, hints, geo)
}

/// Squelette sans membres : arbre, tige, corde, serpent ou poisson.
fn shape_type(skeleton: &Skeleton, hints: &[NameHint], geo: &Geo) -> RigType {
    let Some(main) = main_root(skeleton) else {
        return RigType::Any;
    };
    let path = geo.main_path(main, hints);
    let branching = skeleton
        .ids()
        .filter(|b| geo.significant(*b, hints).len() >= 2)
        .count();
    let start = geo.heads[main.index()];
    let end = geo.tails[path.last().unwrap().index()];
    let d = (end - start).normalize_or(Vec3::Y);
    let length: f32 = path.iter().map(|b| geo.len(*b)).sum();
    if path.len() < 2 && branching == 0 {
        return RigType::Any;
    }
    if d.y > 0.6 && start.y < geo.lo.y + 0.15 * length {
        return RigType::Plant;
    }
    if branching > 0 {
        return if d.y > 0.3 { RigType::Plant } else { RigType::Any };
    }
    if d.y < -0.6 {
        return RigType::Chain;
    }
    if d.y.abs() < 0.4 && path.len() >= 6 {
        let top = path
            .iter()
            .map(|b| geo.heads[b.index()].y)
            .fold(f32::MIN, f32::max);
        return if top - geo.lo.y < 0.15 * length {
            RigType::Serpent
        } else {
            RigType::Fish
        };
    }
    RigType::Chain
}

fn main_root(skeleton: &Skeleton) -> Option<BoneId> {
    skeleton
        .ids()
        .filter(|b| skeleton.bone(*b).parent().is_none())
        .max_by_key(|b| skeleton.subtree(*b).len())
}

/// Rôles des corps sans membres et chaînes libres pour les os restants.
pub(super) fn finish(skeleton: &Skeleton, hints: &[NameHint], geo: &Geo, map: &mut RoleMap) {
    let Some(main) = main_root(skeleton) else { return };
    let free = |map: &RoleMap, b: BoneId| map.role(b).is_none();
    match map.rig_type {
        RigType::Serpent | RigType::Fish if map.chain(Chain::Spine).is_empty() => {
            // Corps : chemin principal, la racine côté tête (convention la
            // plus courante), sauf si un os « tail » est près de la racine.
            let mut path = geo.main_path(main, hints);
            path.retain(|b| free(map, *b));
            let tail_first = path
                .iter()
                .take(path.len() / 2)
                .any(|b| hints[b.index()].part == Some(Part::Tail));
            if let Some((first, rest)) = path.split_first() {
                if map.root().is_none() {
                    map.roles[first.index()] = Some(Role::new(Chain::Root, 0));
                }
                for (i, b) in rest.iter().enumerate() {
                    map.roles[b.index()] = Some(Role::new(Chain::Spine, i.min(255) as u8));
                }
                let head = if tail_first { rest.last() } else { None };
                if let Some(h) = head.filter(|_| map.chain(Chain::Head).is_empty()) {
                    map.roles[h.index()] = Some(Role::new(Chain::Head, 0));
                }
            }
        }
        RigType::Plant | RigType::Chain | RigType::Any => {
            if map.rig_type != RigType::Any {
                // Végétaux et cordes : racine à la base, tout le reste en chaînes.
                map.roles.iter_mut().for_each(|r| *r = None);
            }
            if map.root().is_none() && free(map, main) {
                map.roles[main.index()] = Some(Role::new(Chain::Root, 0));
            }
            branches(skeleton, hints, geo, map);
        }
        _ => {}
    }
}

/// Découpe les os restants en chaînes : les parents précédant leurs
/// enfants, le premier os libre rencontré commence une chaîne qui suit le
/// chemin principal (le tronc d'abord, puis chaque branche latérale).
fn branches(skeleton: &Skeleton, hints: &[NameHint], geo: &Geo, map: &mut RoleMap) {
    let mut next: u16 = 0;
    for b in skeleton.ids() {
        if map.role(b).is_some() || matches!(hints[b.index()].part, Some(Part::Ignore)) {
            continue;
        }
        let path = geo.main_path(b, hints);
        let mut index = 0u8;
        for bone in path {
            if map.role(bone).is_some() {
                break;
            }
            map.roles[bone.index()] = Some(Role::new(Chain::Branch(next), index));
            index = index.saturating_add(1);
        }
        next = next.saturating_add(1);
    }
}
