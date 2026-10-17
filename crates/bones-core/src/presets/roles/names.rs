//! Lecture des noms d'os : découpage en mots (camelCase, séparateurs,
//! chiffres), côté gauche/droit et partie du corps, quelles que soient les
//! conventions (Auto-Rig Pro `arm.l`, Rigify `DEF-upper_arm.L.001`, Mixamo
//! `mixamorig:LeftForeArm`, Unreal `lowerarm_l`, 3ds Max `Bip01 L Forearm`,
//! BVH CMU `LHipJoint`, VRM `J_Bip_L_UpperArm`…).

use super::Lr;

/// Partie du corps suggérée par un nom. Pour les membres, le segment
/// (0 haut, 1 bas, 2 main/pied, 3 doigt/orteil) quand le nom le précise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Part {
    /// « root », « ground », « master » : bassin ou os au sol selon la hiérarchie.
    RootWord,
    Hips,
    Spine,
    Neck,
    Head,
    Jaw,
    Tail,
    Clavicle,
    Arm(Option<u8>),
    Leg(Option<u8>),
    Wing,
    Finger(u8),
    Fin,
    /// Tronc, branche, feuille…
    Plant,
    /// Tentacule, corde, chaîne.
    Generic,
    /// Bout de chaîne sans influence (`HeadTop_End`, `_nub`).
    End,
    /// Os auxiliaire (torsion, contrôleur, visage…) : jamais associé.
    Ignore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NameHint {
    pub side: Option<Lr>,
    pub part: Option<Part>,
}

/// Préfixes et mots sans signification anatomique.
const NOISE: &[&str] = &[
    "mixamorig",
    "def",
    "org",
    "mch",
    "bip",
    "b",
    "j",
    "jnt",
    "bn",
    "cc",
    "base",
    "skeleton",
    "armature",
    "sk",
    "skel",
    "character",
    "rig",
    "deform",
    "valvebiped",
    "x",
    "c",
    "m",
    "bone",
    "anim",
];

const IGNORE: &[&str] = &[
    "twist",
    "roll",
    "helper",
    "pole",
    "target",
    "ik",
    "ctrl",
    "control",
    "ctr",
    "socket",
    "weapon",
    "prop",
    "attach",
    "corrective",
    "eye",
    "eyes",
    "ear",
    "ears",
    "tongue",
    "hair",
    "brow",
    "lid",
    "lip",
    "lips",
    "cheek",
    "nose",
    "teeth",
    "breast",
    "belt",
    "cloth",
    "skirt",
    "cape",
    "bag",
    "camera",
    "light",
    "mesh",
    "geo",
];

const END: &[&str] = &["end", "nub", "tip", "site", "top"];

/// Découpe un nom en mots minuscules (préfixe d'espace de noms retiré).
pub(super) fn tokens(name: &str) -> Vec<String> {
    let name = name.rsplit([':', '|']).next().unwrap_or(name);
    let chars: Vec<char> = name.chars().collect();
    let mut out = Vec::new();
    let mut cur = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            flush(&mut cur, &mut out);
            continue;
        }
        if let Some(p) = cur.chars().last() {
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            let boundary = (p.is_lowercase() && c.is_uppercase())
                || (p.is_alphabetic() != c.is_alphabetic())
                || (p.is_uppercase() && c.is_uppercase() && next_lower);
            if boundary {
                flush(&mut cur, &mut out);
            }
        }
        cur.push(c);
    }
    flush(&mut cur, &mut out);
    out
}

fn flush(cur: &mut String, out: &mut Vec<String>) {
    if !cur.is_empty() {
        out.push(cur.to_lowercase());
        cur.clear();
    }
}

fn side_of(token: &str) -> Option<Lr> {
    match token {
        "left" | "l" | "lf" | "lft" | "izq" | "gauche" => Some(Lr::Left),
        "right" | "r" | "rt" | "rgt" | "der" | "droite" => Some(Lr::Right),
        _ => None,
    }
}

/// Analyse un nom d'os.
pub(super) fn parse(name: &str) -> NameHint {
    let mut side = None;
    let mut words: Vec<String> = Vec::new();
    for t in tokens(name) {
        if let Some(s) = side_of(&t) {
            side = side.or(Some(s));
        } else if !NOISE.contains(&t.as_str()) && !t.chars().all(|c| c.is_ascii_digit()) {
            words.push(t);
        }
    }
    NameHint {
        side,
        part: classify(&words, side.is_some()),
    }
}

fn classify(words: &[String], sided: bool) -> Option<Part> {
    let has = |w: &str| words.iter().any(|x| x == w);
    let s: String = words.concat();
    let any = |keys: &[&str]| keys.iter().any(|k| s.contains(k));
    if s.is_empty() {
        return None;
    }
    if words.iter().any(|w| IGNORE.contains(&w.as_str())) {
        return Some(Part::Ignore);
    }
    if words.iter().any(|w| END.contains(&w.as_str())) {
        return Some(Part::End);
    }
    let fingers = [
        (&["thumb"][..], 0),
        (&["index"][..], 1),
        (&["middle"][..], 2),
        (&["ring"][..], 3),
        (&["pinky", "pinkie", "little"][..], 4),
    ];
    if let Some((_, f)) = fingers.iter().find(|(k, _)| any(k)) {
        return Some(Part::Finger(*f));
    }
    if any(&["finger"]) {
        return Some(Part::Ignore);
    }
    if any(&["wing"]) {
        return Some(Part::Wing);
    }
    let leg_words = [
        "leg", "thigh", "shin", "calf", "foot", "paw", "toe", "hoof", "knee", "ankle",
    ];
    let front = has("front") || has("fore") || any(&["frontleg", "foreleg", "frontpaw"]);
    let hind = has("back") || has("hind") || has("rear") || any(&["hindleg", "backleg", "rearleg"]);
    if (front || hind) && any(&leg_words) {
        let seg = leg_segment(&s);
        return Some(if front { Part::Arm(seg) } else { Part::Leg(seg) });
    }
    if any(&["hip"]) && !any(&["hips"]) && sided {
        // `LHipJoint` (BVH CMU) : court os vers la cuisse ; `LeftHip` : la cuisse.
        return Some(if has("joint") {
            Part::Ignore
        } else {
            Part::Leg(Some(0))
        });
    }
    if any(&leg_words)
        || any(&[
            "upleg", "femur", "tibia", "crus", "shank", "heel", "tarsus", "ball", "digit",
        ])
    {
        return Some(Part::Leg(leg_segment(&s)));
    }
    if any(&["forearm", "lowerarm", "elbow", "ulna", "radius"]) {
        return Some(Part::Arm(Some(1)));
    }
    if any(&["hand", "wrist", "palm", "carpal", "carpus"]) {
        return Some(Part::Arm(Some(2)));
    }
    if any(&["upperarm", "uparm", "humerus"]) {
        return Some(Part::Arm(Some(0)));
    }
    if has("arm") {
        // `arm.l`, `LeftArm` : le haut du bras, mais `arm_joint_L_2`… aussi.
        return Some(Part::Arm(None));
    }
    let table: &[(&[&str], Part)] = &[
        (&["clavicle", "collar", "shoulder", "scapula"], Part::Clavicle),
        (&["neck"], Part::Neck),
        (&["head", "skull", "cranium"], Part::Head),
        (&["jaw", "chin", "mouth", "mandible"], Part::Jaw),
        (&["tail"], Part::Tail),
        (&["hips", "pelvis", "cog", "hip"], Part::Hips),
        (
            &[
                "spine", "chest", "abdomen", "torso", "belly", "thorax", "ribcage", "waist",
            ],
            Part::Spine,
        ),
        (&["root", "ground", "master", "trajectory"], Part::RootWord),
        (
            &[
                "trunk", "stem", "stalk", "branch", "twig", "leaf", "leaves", "frond", "petal", "flower",
            ],
            Part::Plant,
        ),
        (
            &["tentacle", "rope", "chain", "pendulum", "cable", "whip"],
            Part::Generic,
        ),
    ];
    if let Some((_, part)) = table.iter().find(|(k, _)| any(k)) {
        return Some(*part);
    }
    if has("back") {
        return Some(Part::Spine);
    }
    if has("fin") || any(&["pectoral", "dorsal", "caudal"]) {
        return Some(Part::Fin);
    }
    None
}

/// Segment d'une jambe d'après son nom (`None` : à déduire de l'ordre).
fn leg_segment(s: &str) -> Option<u8> {
    let any = |keys: &[&str]| keys.iter().any(|k| s.contains(k));
    if any(&["toe", "digit", "ball", "phalan"]) {
        Some(3)
    } else if any(&["foot", "ankle", "heel", "tarsus", "hoof", "paw"]) {
        Some(2)
    } else if any(&["calf", "shin", "lowerleg", "knee", "tibia", "crus", "shank"]) {
        Some(1)
    } else if any(&["thigh", "upleg", "upperleg", "femur"]) {
        Some(0)
    } else {
        None
    }
}
