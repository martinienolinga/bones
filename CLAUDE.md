# PaintSharp Bones — consignes pour Claude

Application iPad de rigging et d'animation 3D, écrite **entièrement en Rust** (aucun Swift ni
Objective-C). Inspirations : interface de **Procreate** et **Nomad Sculpt**, animation de
**Cascadeur** (clés, physique, pose assistée, trajectoires), rigging de **Auto-Rig Pro** (Blender).
Objectif : le niveau d'une application professionnelle, avec des fonctions avancées et complètes.

Lire `docs/ARCHITECTURE.md` avant toute modification importante, et `README.md` pour les
fonctions, les formats et les gestes.

## Langue

- Converser avec l'utilisateur **en français**.
- Commentaires, documentation du code, messages de commit et docs du dépôt : **en français**.
- Textes visibles dans l'application (interface, erreurs, noms de presets) : **en anglais** pour
  l'instant.

## Espace de travail

| Crate | Rôle |
|---|---|
| `crates/bones-core` | Toute la logique, sans GPU ni plateforme : squelette, pose, IK, skinning, auto-rig, animation, physique, manipulateurs, presets, formats de fichiers, éditeur et gestes. |
| `crates/bones-render` | Rendu wgpu (Metal sur iPad) : skinning GPU, os en rayons X, MSAA, vignettes hors écran, flou d'arrière-plan. |
| `crates/bones-ui` | Kit d'interface egui : thème, glyphes SVG monochromes, widgets, verre dépoli. |
| `crates/bones-app` | Application winit + egui : galerie de projets, éditeur, gestes, tâches de fond. |

Chaîne d'outils figée sur **Rust 1.97** (`rust-toolchain.toml`), édition 2024, `rustfmt`
avec `max_width = 110`.

## Commandes

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p bones-app --target aarch64-apple-ios   # la cible iPad doit toujours compiler
cargo run -p bones-app                                 # app sur poste (macOS / Linux)
ios/build.sh sim debug && ios/run-sim.sh               # simulateur iPad (Mac)
```

La commande `/verify` lance toutes les vérifications. Ne jamais committer un état qui ne les
passe pas : la CI (`.github/workflows/ci.yml`) les rejoue et construit l'app iPad.

## Règles de code

- **Fichiers courts et bien rangés** : environ 250 lignes au plus ; découper par responsabilité.
  Les tests vont dans des fichiers `tests.rs` séparés.
- **Toujours optimisé** : multithreading avec rayon quand c'est utile, données lourdes partagées
  par `Arc` (l'historique d'annulation ne copie que l'état léger), caches GPU pilotés par des
  numéros de révision, calculs longs sur des fils de fond qui réveillent la boucle par
  `EventLoopProxy`. Ne jamais bloquer le fil de l'interface (disque, décodage, poids).
- **Dépendances en Rust pur** (compilation iOS) : pas de bibliothèque C. Préférer des crates et
  des algorithmes éprouvés (et les citer dans la doc du code) à du code maison.
- Conventions 3D : Y vers le haut, personnage tourné vers +Z, os le long de leur +Y local, noms
  `.l` / `.r` / `.x` à la Auto-Rig Pro, pose stockée en écarts par rapport au repos.
- Chaque modification de l'utilisateur passe par l'historique (`checkpoint`) ; un simple toucher
  ne crée pas d'étape d'annulation.
- Ne pas modifier `Rig::demo()` : de nombreux tests s'appuient sur lui. Le mannequin par défaut de
  l'application est `samples::mannequin()`.

## Interface (exigences fortes de l'utilisateur)

- Finition au niveau de **Procreate** : barre du haut pleine, bandeaux collés aux bords,
  popovers à encoche, listes groupées, « studios » de réglages, transitions animées, cibles
  tactiles d'au moins 44 pt, police Inter.
- **Aucun dégradé** : aplats uniquement.
- **Glyphes monochromes** dans le style de Procreate : blancs au repos, teintés en bleu
  d'accent `#0A84FF` quand ils sont actifs. Nouveaux glyphes dans `crates/bones-ui/src/icons/svg/`.
- Panneaux flottants au-dessus de la vue 3D : `bones_ui::glass` (verre dépoli sombre).
- La **galerie de projets** est la page d'accueil, comme dans Procreate.
- Guide de rotation : cube de vue à la Nomad (coins arrondis, faces en aplats) ou axes à la
  Blender ; Apple Pencil exclusif réglable par tâche.
- Toujours vérifier visuellement par des captures et les juger avec un œil critique.

## Fichiers et formats

- Projets : `.kbrig` (`bones_core::project`), un fichier ZIP compressé par projet, enregistré de
  façon atomique, avec une vignette lisible sans tout décompresser.
- Animations et presets : `.kb3da`, avec le même conteneur (`project::container`).
- Échange : glTF/GLB, FBX (import), OBJ, STL, PLY, BVH ; textures incluses.
- Les fichiers reçus d'ailleurs ne sont pas fiables : tailles bornées, validation, erreurs sans
  panique.

## Règles légales

- Modèles, animations et presets livrés avec l'app : créations propres ou **CC0** (CC-BY
  seulement avec crédit). Jamais de Mixamo/Adobe, de Sketchfab « standard », ni de licence NC ou
  ND. Documenter la provenance à côté de chaque asset (`SOURCE.md`, `LICENSE.txt`).
- Formats d'autres logiciels : uniquement les formats ouverts ou documentés ; pas de
  rétro-ingénierie de formats propriétaires (`.casc`, `.blend`…).

## Tests et vérification visuelle

- Gestes multi-doigts : tests unitaires (`crates/bones-app/src/input/tests.rs`), car X11 ne sait
  pas simuler le multitouch.
- Interface sans GPU : harnais egui de `crates/bones-app/src/ui/tests.rs`.
- Rendu sans écran : `crates/bones-render/tests/` (lavapipe sous Linux).
- Captures de l'app : `tools/screenshots/shot.sh` et `tools/screenshots/interact.sh` (Xvfb,
  lavapipe, xdotool ; voir l'en-tête des scripts).

## Git et travail en parallèle

- Travailler sur la branche indiquée par la session ; pas de pull request sans demande explicite.
- Commits en français, petits et vérifiés ; `Cargo.lock` est versionné.
- Plusieurs sous-agents peuvent travailler en même temps : répartir les fichiers par zones,
  relire un fichier partagé juste avant de le modifier, ne jamais annuler le travail d'un autre,
  et laisser la session principale vérifier et committer. Le dossier `target/` est partagé : ne
  pas créer d'autres dossiers de compilation (disque limité), attendre les verrous de cargo.
- Agents spécialisés disponibles dans `.claude/agents/`.
