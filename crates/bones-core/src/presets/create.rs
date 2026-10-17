//! Création de préréglages utilisateur : à partir de l'animation du projet
//! (intervalle d'images, éventuellement une partie des os), enregistrée
//! avec son squelette pour être reciblée plus tard ; ou générateur
//! procédural avec des paramètres personnalisés.

use super::error::PresetError;
use super::model::{
    Category, KeyframeData, Preset, PresetInfo, PresetKind, PresetSource, Procedural, new_id,
};
use super::params::Params;
use super::procedural::Generator;
use super::roles::{RoleMap, detect};
use crate::project::unix_now;
use crate::{BoneId, Clip, Pose, Skeleton};

/// Description d'un préréglage à créer.
#[derive(Clone, Debug, PartialEq)]
pub struct CreateOptions {
    pub name: String,
    pub category: Category,
    pub group: String,
    pub tags: Vec<String>,
    pub description: String,
    pub author: String,
    pub license: String,
    /// Intervalle d'images [first, last] de l'animation.
    pub first: u32,
    pub last: u32,
    /// Os à enregistrer (vide : tous).
    pub bones: Vec<BoneId>,
    pub looping: bool,
    /// Rôles corrigés à la main (sinon détection automatique).
    pub roles: Option<RoleMap>,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            name: "My Preset".into(),
            category: Category::Human,
            group: String::new(),
            tags: Vec::new(),
            description: String::new(),
            author: String::new(),
            license: "CC0-1.0".into(),
            first: 0,
            last: u32::MAX,
            bones: Vec::new(),
            looping: false,
            roles: None,
        }
    }
}

fn info(options: &CreateOptions, roles: &RoleMap) -> PresetInfo {
    let now = unix_now();
    PresetInfo {
        id: new_id("user"),
        name: crate::project::store::sanitize_name(&options.name),
        category: options.category,
        group: options.group.trim().to_owned(),
        tags: options
            .tags
            .iter()
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
            .collect(),
        description: options.description.clone(),
        author: options.author.clone(),
        license: options.license.clone(),
        source: PresetSource::User,
        rig_type: roles.rig_type,
        looping: options.looping,
        created: now,
        modified: now,
        ..PresetInfo::default()
    }
}

/// Préréglage à clés tiré de l'animation `clip` du squelette `skeleton`.
pub fn from_animation(
    skeleton: &Skeleton,
    clip: &Clip,
    options: &CreateOptions,
) -> Result<Preset, PresetError> {
    if skeleton.is_empty() {
        return Err(PresetError::Empty("the model has no skeleton".into()));
    }
    if clip.is_empty() {
        return Err(PresetError::Empty("the animation has no keys".into()));
    }
    let last = options.last.min(clip.frame_count.saturating_sub(1));
    let first = options.first.min(last);
    if last <= first {
        return Err(PresetError::Empty("choose at least two frames".into()));
    }
    // Clés de l'intervalle, plus les bornes échantillonnées : le clip est
    // autonome et commence à l'image 0.
    let mut out = Clip::new(clip.fps, last - first + 1);
    for f in [first, last] {
        let mut pose = Pose::rest(skeleton);
        clip.sample_into(f as f32, &mut pose);
        out.set_key(f - first, pose);
    }
    for key in clip.keys().iter().filter(|k| k.frame > first && k.frame < last) {
        out.set_key(key.frame - first, key.pose.clone());
    }
    for key in clip.keys() {
        if key.frame >= first
            && key.frame <= last
            && let Some(k) = out.key_at_mut(key.frame - first)
        {
            k.interpolation = key.interpolation;
        }
    }
    out.sync_len(skeleton);
    let roles = options
        .roles
        .clone()
        .filter(|r| r.fits(skeleton))
        .unwrap_or_else(|| detect(skeleton));
    let mut info = info(options, &roles);
    info.fps = clip.fps;
    info.frames = last - first;
    info.bones = skeleton.len() as u32;
    let bones: Vec<BoneId> = options
        .bones
        .iter()
        .copied()
        .filter(|b| skeleton.contains(*b))
        .collect();
    let data = KeyframeData {
        skeleton: skeleton.clone(),
        roles,
        clip: out,
        bones,
    };
    info.root_motion = travel(&data) > 0.01 * data.roles.frame.size.max(1e-3);
    Ok(Preset {
        info,
        kind: PresetKind::Keyframes(Box::new(data)),
    })
}

/// Déplacement horizontal du bassin entre la première et la dernière clé.
pub(crate) fn travel(data: &KeyframeData) -> f32 {
    let s = &data.skeleton;
    let root = data.roles.root().unwrap_or(BoneId(0));
    let at = |frame: f32| {
        let mut pose = Pose::rest(s);
        data.clip.sample_into(frame, &mut pose);
        pose.world(s)[root.index()].translation
    };
    let last = data.clip.keys().last().map_or(0, |k| k.frame) as f32;
    let d = at(last) - at(0.0);
    let up = data.roles.frame.up;
    (d - up * d.dot(up)).length()
}

/// Préréglage procédural aux paramètres personnalisés.
pub fn procedural(generator: Generator, params: Params, options: &CreateOptions) -> Preset {
    let p = Procedural {
        generator,
        params,
        reference: None,
    };
    let skeleton = p.reference().skeleton();
    let roles = detect(&skeleton);
    let mut info = info(options, &roles);
    let (frames, root_motion) = super::library::measure(&p);
    info.rig_type = generator.rig_type();
    info.fps = 30.0;
    info.frames = frames;
    info.root_motion = root_motion;
    info.looping = generator.looping();
    info.procedural = true;
    Preset {
        info,
        kind: PresetKind::Procedural(p),
    }
}
