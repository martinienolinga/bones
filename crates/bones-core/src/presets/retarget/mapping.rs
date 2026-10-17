//! Correspondance des os : par rôle (chaînes rééchantillonnées), puis par
//! nom ; alignement des directions de repos.

use glam::{Quat, Vec3};

use super::{Report, RetargetOptions, Sample};
use crate::math::rotation_arc;
use crate::presets::roles::{Chain, RoleMap};
use crate::rig::mirror::mirror_name;
use crate::{BoneId, Skeleton};

/// Correspondance os cible → os source.
#[derive(Clone, Debug)]
pub struct Mapping {
    pub(crate) samples: Vec<Option<Sample>>,
    pub(crate) report: Report,
}

/// Nom sans espace de noms (`mixamorig:Hips` → `hips`).
fn plain(name: &str) -> String {
    name.rsplit([':', '|']).next().unwrap_or(name).to_lowercase()
}

/// Reflet d'un vecteur canonique (plan sagittal).
pub(crate) fn mirror_vec(v: Vec3) -> Vec3 {
    Vec3::new(-v.x, v.y, v.z)
}

/// Reflet d'une rotation canonique (plan sagittal).
pub(crate) fn mirror_quat(q: Quat) -> Quat {
    Quat::from_xyzw(q.x, -q.y, -q.z, q.w)
}

impl Mapping {
    /// `animated` : os source animés (vide : tous).
    pub fn new(
        source: (&Skeleton, &RoleMap),
        animated: &[BoneId],
        target: (&Skeleton, &RoleMap),
        options: &RetargetOptions,
    ) -> Self {
        let (src, src_roles) = source;
        let (tgt, tgt_roles) = target;
        let usable = |b: BoneId| animated.is_empty() || animated.contains(&b);
        let src_rest = src.rest_world();
        let tgt_rest = tgt.rest_world();
        let (bs, bt) = (src_roles.frame.basis(), tgt_roles.frame.basis());
        let mut samples = vec![None; tgt.len()];
        let mut report = Report::default();
        for t in tgt.ids() {
            let by_role = tgt_roles.role(t).and_then(|role| {
                let chain = if options.mirror {
                    role.chain.mirror()
                } else {
                    role.chain
                };
                let bones = src_roles.chain(chain);
                let n_t = tgt_roles.chain(role.chain).len();
                pick(&bones, role.index as usize, n_t, chain.is_limb())
            });
            let by_name = || {
                let name = &tgt.bone(t).name;
                let wanted = if options.mirror {
                    mirror_name(name).unwrap_or(name.clone())
                } else {
                    name.clone()
                };
                let wanted = plain(&wanted);
                src.ids()
                    .find(|s| plain(&src.bone(*s).name) == wanted)
                    .map(|s| (s, s, 0.0))
            };
            let Some((a, b, w)) = by_role
                .or_else(by_name)
                .filter(|(a, b, _)| usable(*a) && usable(*b))
            else {
                report.unmapped.push(tgt.bone(t).name.clone());
                continue;
            };
            let role = tgt_roles.role(t).map(|r| r.chain);
            let translate = matches!(role, Some(Chain::Root | Chain::Ground))
                || (role.is_none() && tgt.bone(t).parent().is_none());
            // Alignement de repos : direction source reportée dans le repère cible.
            let dir = |x: BoneId| src_rest[x.index()].rotation * Vec3::Y;
            let src_dir = dir(a).lerp(dir(b), w);
            let canon = bs.inverse() * src_dir;
            let wanted = bt * if options.mirror { mirror_vec(canon) } else { canon };
            let tgt_dir = tgt_rest[t.index()].rotation * Vec3::Y;
            let limit: f32 = match role {
                Some(Chain::Root | Chain::Ground) => 0.0,
                Some(c) if c.is_limb() => 100.0,
                _ => 60.0,
            };
            let angle = tgt_dir.angle_between(wanted).to_degrees();
            let align = if options.align_rest && angle < limit {
                rotation_arc(tgt_dir, wanted)
            } else {
                Quat::IDENTITY
            };
            samples[t.index()] = Some(Sample {
                a,
                b,
                w,
                align,
                translate,
            });
            report
                .mapped
                .push((tgt.bone(t).name.clone(), src.bone(a).name.clone()));
        }
        Self { samples, report }
    }

    /// Nombre d'os cible animés.
    pub fn mapped(&self) -> usize {
        self.samples.iter().flatten().count()
    }
}

/// Os source pour le rang `i` d'une chaîne cible de `n_t` os : même rang
/// pour un membre, position relative interpolée pour une colonne, une
/// queue, un cou.
fn pick(bones: &[BoneId], i: usize, n_t: usize, limb: bool) -> Option<(BoneId, BoneId, f32)> {
    let n_s = bones.len();
    if n_s == 0 {
        return None;
    }
    if limb || n_s == n_t {
        return bones.get(i).map(|b| (*b, *b, 0.0));
    }
    let x = ((i as f32 + 0.5) / n_t.max(1) as f32 * n_s as f32 - 0.5).clamp(0.0, (n_s - 1) as f32);
    let a = x.floor() as usize;
    let b = (a + 1).min(n_s - 1);
    Some((bones[a], bones[b], x - a as f32))
}
