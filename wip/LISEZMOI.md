# Travaux en cours (mis en pause)

Les travaux ont été mis en pause à la demande de l'utilisateur, et tout a été commité tel
quel à sa demande, terminé ou non. Le travail inachevé de quatre agents est donc dans le
code : il peut ne pas passer les vérifications (format, clippy, tests) ni la CI.

## Reprendre

Terminer chaque chantier ci-dessous, puis vérifier : `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
`cargo check -p bones-app --target aarch64-apple-ios`. Supprimer ce dossier `wip/` quand
tout est vert.

## Chantiers inachevés (106 fichiers)

1. **Moteur de presets d'animation** (`crates/bones-core/src/presets/**`, `editor/presets.rs`,
   parties animation de `io/`) : bibliothèque procédurale (humain, animaux, végétation, divers),
   rôles et détection du type de squelette, reciblage, application et création de presets,
   format `.kb3da` (conteneur de `project::container`). Arrêté pendant l'écriture des tests de
   la couche d'application et de l'intégration à l'éditeur. Reste aussi : import/export BVH,
   glTF, FBX, Collada des presets, planches de contrôle visuel.
2. **Mannequin** (`samples/`, `autorig/`, `editor/rig_ops.rs`, `project/` : chargement en
   mémoire) : option `AutoRigOptions.fit_to_mesh` et précalcul du mannequin en `.kbrig` embarqué,
   en cours. Reste : **rig facial** (contrat de noms ci-dessous), poids du visage, puis
   précalcul du mannequin avec le visage.
3. **Weight Paint, moteur** (`crates/bones-core/src/weight_paint/**`, `editor/weight_paint.rs`,
   `Mode::WeightPaint` et ses branches de `match`) : arrêté pendant l'ajout des branches de
   `match` dans le cœur. Voir le contrat d'API dans l'historique de la session : pinceaux Draw
   (Mix, Add, Subtract, Lighten, Darken, Multiply), Blur, Average, Smear, Gradient, Sample ;
   atténuations ; miroir X ; normalisation automatique ; limite de 4 influences ; poids peints
   jamais écrasés automatiquement ; outils de poids.
4. **Weight Paint, interface et rendu** (`bones-app/src/ui/**`, `bones-render` : carte de
   chaleur) : arrêté pendant la mise à jour des constructions de `Scene`.

## Contrat de noms des os du visage

Enfants de `head.x` sauf mention : `jaw.x` (charnière de la mâchoire), `chin.x` (enfant de
`jaw.x`), `tongue_01.x` → `tongue_02.x` → `tongue_03.x` (enfants de `jaw.x`), `teeth_top.x`,
`teeth_bot.x` (enfant de `jaw.x`), `eye.l/.r` (centre du globe), `eyelid_top.l/.r`,
`eyelid_bot.l/.r` (pivot au centre du globe), `brow_in.l/.r`, `brow_mid.l/.r`,
`brow_out.l/.r`, `cheek.l/.r`, `nose.x`, `nostril.l/.r`, `lip_top.x`, `lip_top.l/.r`,
`lip_bot.x`, `lip_bot.l/.r` (enfants de `jaw.x`), `lip_corner.l/.r`, `ear.l/.r`.

## Chantiers suivants déjà demandés par l'utilisateur

- Galerie des presets d'animation (style bibliothèque de pinceaux de Procreate), création,
  import et export de presets.
- Galerie des expressions faciales (humains et animaux : émotions d'Ekman, expressions
  courantes, visèmes de Preston Blair, chien, chat, cheval, oiseau, poisson), sur le contrat
  de noms ci-dessus ; étape des marqueurs du visage dans le mode Auto-Rig.
- Proposition en attente de décision : morph targets (52 expressions standard d'ARKit).
- Petits restes : glissement vers la gauche sur une carte de la galerie, vignette floue au
  début du zoom d'ouverture, filtre `Format::Bones` dans `ui/popovers/actions.rs`, mise à
  jour du README et de `docs/ARCHITECTURE.md` (galerie, `.kbrig`, Weight Paint).
