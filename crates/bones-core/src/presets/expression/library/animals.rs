//! Expressions animales, sur les seuls os du visage qui existent sur le rig
//! (mâchoire, oreilles, naseaux, yeux, lèvres…). Les os absents sont ignorés,
//! donc une même expression se dégrade proprement d'une espèce à l'autre.
//! Données CC0.

use super::super::builder::Expr;
use super::Spec;
use crate::presets::roles::RigType;

fn a(
    id: &'static str,
    name: &'static str,
    tags: &'static [&'static str],
    desc: &'static str,
    rig: RigType,
    b: Expr,
) -> Spec {
    Spec::new(id, name, "Animals", tags, desc, rig, b.finish())
}

pub(super) fn all() -> Vec<Spec> {
    vec![
        dog_snarl(),
        dog_pant(),
        cat_hiss(),
        cat_blink(),
        horse_ears_pinned(),
        horse_nostrils(),
        bird_beak_open(),
        fish_gape(),
    ]
}

fn dog_snarl() -> Spec {
    let mut b = Expr::new();
    b.nose_wrinkle(0.8);
    b.jaw_open(0.25);
    b.lips_part(0.6);
    b.nostrils_flare(0.5);
    b.ears_back(0.7);
    a(
        "dog_snarl",
        "Dog Snarl",
        &["dog", "teeth"],
        "Bared teeth, wrinkled muzzle, ears back.",
        RigType::Quadruped,
        b,
    )
}

fn dog_pant() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(0.9);
    b.tongue_out(1.0);
    b.ears_up(0.6);
    a(
        "dog_pant",
        "Dog Pant",
        &["dog", "happy"],
        "Open-mouth pant with the tongue out, ears up.",
        RigType::Quadruped,
        b,
    )
}

fn cat_hiss() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(0.6);
    b.nose_wrinkle(0.7);
    b.widen_mouth(0.4);
    b.ears_back(0.8);
    a(
        "cat_hiss",
        "Cat Hiss",
        &["cat", "teeth"],
        "Open mouth, flattened ears.",
        RigType::Quadruped,
        b,
    )
}

fn cat_blink() -> Spec {
    let mut b = Expr::new();
    b.lids_close(0.6);
    a(
        "cat_slow_blink",
        "Cat Slow Blink",
        &["cat", "calm"],
        "Half-closed, relaxed eyes.",
        RigType::Quadruped,
        b,
    )
}

fn horse_ears_pinned() -> Spec {
    let mut b = Expr::new();
    b.ears_back(1.0);
    b.nostrils_flare(0.6);
    b.lips_press(0.4);
    a(
        "horse_ears_pinned",
        "Horse Ears Pinned",
        &["horse", "anger"],
        "Ears pinned back, tight lips.",
        RigType::Quadruped,
        b,
    )
}

fn horse_nostrils() -> Spec {
    let mut b = Expr::new();
    b.nostrils_flare(1.0);
    b.jaw_open(0.15);
    b.ears_up(0.3);
    a(
        "horse_flared_nostrils",
        "Horse Flared Nostrils",
        &["horse"],
        "Wide-open nostrils.",
        RigType::Quadruped,
        b,
    )
}

fn bird_beak_open() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(0.8);
    a(
        "bird_beak_open",
        "Bird Beak Open",
        &["bird", "beak"],
        "Open beak (jaw).",
        RigType::Bird,
        b,
    )
}

fn fish_gape() -> Spec {
    let mut b = Expr::new();
    b.jaw_open(0.7);
    b.pucker(0.6);
    a(
        "fish_gape",
        "Fish Gape",
        &["fish"],
        "Rounded open mouth.",
        RigType::Fish,
        b,
    )
}
