# Travaux en pause

Deux chantiers ont été interrompus à la demande de l'utilisateur. Leur code,
inachevé et non vérifié, est rangé ici sous forme de correctifs : il n'est pas
compilé avec l'application, qui reste dans son dernier état vérifié.

Reprise : `git apply wip/<nom>.patch` depuis la racine du dépôt, puis
poursuivre le travail et supprimer le correctif. Les deux correctifs
s'appliquent ensemble ou séparément sur le commit qui les ajoute.

## `presets-mannequin.patch` — vignettes rendues avec le mannequin

Demande : les vignettes des galeries (bâtons pour les animations, points pour
les expressions) ne permettent pas de distinguer les presets ; les représenter
avec le mannequin par défaut (`samples::mannequin()`).

- Expressions : gros plan du visage, expression appliquée à pleine intensité.
- Animations : corps entier jouant l'animation en boucle, en direct, sur place
  (déplacement horizontal annulé) ; repli sur l'aperçu en bâtons pour les
  presets d'animaux qui ne peuvent pas animer un humain.
- Feuille d'application : même rendu, en plus grand.
- Technique : seules les cartes visibles sont rendues, en tuiles dans une
  texture GPU partagée avec egui (`bones-render/src/tiles.rs`), sans relecture
  des pixels ; poses calculées sans bloquer l'interface
  (`bones-core/src/presets/stage/`, `bones-app/src/showcase/`,
  `ui/popovers/presets/stage.rs`).
- Critère principal : chaque expression reconnaissable à la taille d'une
  carte ; augmenter les amplitudes des expressions si besoin
  (`presets/expression/`).
- État à l'arrêt : en cours, non vérifié ; `ui/popovers/presets/mod.rs`
  dépassait 250 lignes et devait être découpé ; clippy signalait
  `neg_multiply` dans `presets/expression/actions.rs`.

## `scene-objects.patch` — objets de scène (étape 1 : cœur)

Demande : améliorer l'outliner et pouvoir manipuler plusieurs objets 3D et
leur hiérarchie en interaction avec les os.

Plan en trois étapes :
1. Cœur (`bones-core/src/scene/`, commencé) : `rig.objects` en plus du corps
   (`rig.mesh`) ; parent monde, objet ou os (suivi des modifications du
   squelette) ; liaison rigide ou déformée (poids propres, transfert depuis le
   corps : `skinning/closest.rs`, `skinning/transfer.rs`) ; ajout de
   primitives ou depuis un fichier, duplication, suppression, renommage,
   reparentage en gardant la position ; sélection et gizmo ; enregistrement
   dans le `.kbrig` (anciens projets compatibles, fichiers hostiles refusés) ;
   import glTF en objets séparés. Pas de clés d'animation propres aux objets
   pour l'instant.
2. Rendu des objets et surbrillance de la sélection (`bones-render`).
3. Nouvel outliner (arbre unique os + objets rangés sous leur os, branches
   repliables, recherche, œil et cadenas, renommage, glisser-déposer,
   menu contextuel, IK/FK) et mode Objet dans la barre du haut.

État à l'arrêt : début de l'étape 1 (modèle de données), ne compile pas
encore.
