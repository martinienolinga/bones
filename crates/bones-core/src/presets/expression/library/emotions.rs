//! Émotions humaines de base : les six de Paul Ekman, plus neutre et mépris.
//! Références : Ekman & Friesen, *Facial Action Coding System* (unités
//! d'action combinées ici à la main). Données CC0.

use super::super::builder::Expr;
use super::Spec;
use crate::presets::roles::RigType;

fn e(
    id: &'static str,
    name: &'static str,
    tags: &'static [&'static str],
    desc: &'static str,
    b: Expr,
) -> Spec {
    Spec::new(id, name, "Emotions", tags, desc, RigType::Humanoid, b.finish())
}

pub(super) fn all() -> Vec<Spec> {
    vec![
        e(
            "neutral",
            "Neutral",
            &["rest", "calm"],
            "Relaxed resting face.",
            Expr::new(),
        ),
        happy(),
        sad(),
        angry(),
        fearful(),
        surprised(),
        disgusted(),
        contempt(),
    ]
}

fn happy() -> Spec {
    let mut b = Expr::new();
    b.smile(0.9);
    b.squint(0.25);
    e(
        "happy",
        "Happy",
        &["smile", "joy", "au6", "au12"],
        "Genuine smile with raised cheeks.",
        b,
    )
}

fn sad() -> Spec {
    let mut b = Expr::new();
    b.brows_inner_up(0.75);
    b.frown_mouth(0.55);
    b.lids_close(0.15);
    e(
        "sad",
        "Sad",
        &["sorrow", "au1", "au15"],
        "Inner brows up, mouth corners down.",
        b,
    )
}

fn angry() -> Spec {
    let mut b = Expr::new();
    b.brows_down(0.85);
    b.squint(0.25);
    b.nostrils_flare(0.4);
    b.frown_mouth(0.25);
    e(
        "angry",
        "Angry",
        &["anger", "au4", "au23"],
        "Brows pulled down, lips pressed, nostrils flared.",
        b,
    )
}

fn fearful() -> Spec {
    let mut b = Expr::new();
    b.brows_inner_up(0.55);
    b.brows_up(0.35);
    b.lids_widen(0.8);
    b.jaw_open(0.25);
    b.widen_mouth(0.35);
    e(
        "fear",
        "Fearful",
        &["fear", "au1", "au2", "au5"],
        "Raised brows, wide eyes, stretched mouth.",
        b,
    )
}

fn surprised() -> Spec {
    let mut b = Expr::new();
    b.brows_up(0.95);
    b.lids_widen(0.9);
    b.jaw_open(0.6);
    e(
        "surprise",
        "Surprised",
        &["surprise", "au1", "au2", "au26"],
        "Brows up, eyes wide, jaw dropped.",
        b,
    )
}

fn disgusted() -> Spec {
    let mut b = Expr::new();
    b.nose_wrinkle(0.85);
    b.brows_down(0.4);
    b.frown_mouth(0.2);
    b.lips_part(0.3);
    e(
        "disgust",
        "Disgusted",
        &["disgust", "au9", "au10"],
        "Wrinkled nose and raised upper lip.",
        b,
    )
}

fn contempt() -> Spec {
    let mut b = Expr::new();
    b.corner_up(true, 0.7);
    b.squint(0.1);
    e(
        "contempt",
        "Contempt",
        &["contempt", "au12", "au14"],
        "One-sided lip-corner raise.",
        b,
    )
}
