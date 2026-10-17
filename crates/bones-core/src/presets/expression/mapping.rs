//! Reconnaissance des os du visage d'autres logiciels (best-effort), pour
//! poser le visage d'un humanoïde importé. Table modeste, volontairement
//! prudente : un nom ne correspond que s'il contient un mot-clé facial clair,
//! afin de ne jamais attraper un os du corps.
//!
//! Schémas couverts : Rigify / Auto-Rig Pro (`eye.L`, `jaw_master`,
//! `lid.T.L`, `brow.T.L`, `lip_corner.L`…), Character Creator / iClone
//! (`CC_Base_JawRoot`, `CC_Base_L_Eye`, `CC_Base_Teeth01`, `CC_Base_Tongue01`),
//! MetaHuman / divers (`FACIAL_C_Jaw`, `FACIAL_L_Eye`…), et les variantes
//! anglaises courantes (`LeftEye`, `upperLip`, `eyeLidLower`…).

use crate::autorig::face::FaceBone;
use crate::rig::mirror::{Side, side_of};

/// Découpe un nom en mots minuscules (espace de noms, séparateurs, camelCase,
/// frontières lettre/chiffre).
fn tokens(name: &str) -> Vec<String> {
    let base = name.rsplit([':', '|']).next().unwrap_or(name);
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev: Option<char> = None;
    for c in base.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev = None;
            continue;
        }
        if let Some(p) = prev {
            let boundary = (p.is_lowercase() && c.is_uppercase()) || (p.is_alphabetic() != c.is_alphabetic());
            if boundary && !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        }
        cur.push(c.to_ascii_lowercase());
        prev = Some(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Côté d'après le nom : suffixe canonique (`.l`/`.r`…), sinon un mot `l`/`r`.
fn side(name: &str, words: &[String]) -> Option<Side> {
    side_of(name).or_else(|| {
        words.iter().find_map(|w| match w.as_str() {
            "l" | "left" | "lf" => Some(Side::Left),
            "r" | "right" | "rt" => Some(Side::Right),
            _ => None,
        })
    })
}

fn pick(left: FaceBone, right: FaceBone, side: Option<Side>) -> Option<FaceBone> {
    match side? {
        Side::Left => Some(left),
        Side::Right => Some(right),
    }
}

/// Os du visage canonique correspondant à un nom étranger, ou `None`.
pub fn foreign(name: &str) -> Option<FaceBone> {
    use FaceBone::*;
    let w = tokens(name);
    let has = |k: &str| w.iter().any(|t| t == k);
    let any = |ks: &[&str]| ks.iter().any(|k| has(k));
    let s = side(name, &w);
    let upper = any(&["upper", "top", "t", "up"]);
    let lower = any(&["lower", "bottom", "bot", "b", "low"]);

    if any(&["eyelid", "lid"]) {
        return pick(
            if lower { EyelidBottomL } else { EyelidTopL },
            if lower { EyelidBottomR } else { EyelidTopR },
            s,
        );
    }
    if any(&["brow", "eyebrow"]) {
        let (l, r) = if any(&["inner", "in"]) {
            (BrowInL, BrowInR)
        } else if any(&["outer", "out"]) {
            (BrowOutL, BrowOutR)
        } else {
            (BrowMidL, BrowMidR)
        };
        return pick(l, r, s);
    }
    if has("eye") {
        return pick(EyeL, EyeR, s);
    }
    if has("cheek") {
        return pick(CheekL, CheekR, s);
    }
    if has("nostril") {
        return pick(NostrilL, NostrilR, s);
    }
    if any(&["nose", "nasal"]) {
        return Some(Nose);
    }
    if any(&["corner", "mouthcorner", "lipcorner"]) {
        return pick(LipCornerL, LipCornerR, s);
    }
    if has("lip") {
        return match (s, upper, lower) {
            (Some(Side::Left), true, _) => Some(LipTopL),
            (Some(Side::Right), true, _) => Some(LipTopR),
            (Some(Side::Left), _, true) => Some(LipBottomL),
            (Some(Side::Right), _, true) => Some(LipBottomR),
            (_, _, true) => Some(LipBottom),
            _ => Some(LipTop),
        };
    }
    if has("teeth") || has("tooth") {
        return Some(if lower || any(&["02", "2"]) {
            TeethBottom
        } else {
            TeethTop
        });
    }
    if has("tongue") {
        return Some(if any(&["03", "3"]) {
            Tongue3
        } else if any(&["02", "2"]) {
            Tongue2
        } else {
            Tongue1
        });
    }
    if has("ear") {
        return pick(EarL, EarR, s);
    }
    if has("chin") {
        return Some(Chin);
    }
    if any(&["jaw", "mandible"]) {
        return Some(Jaw);
    }
    None
}
