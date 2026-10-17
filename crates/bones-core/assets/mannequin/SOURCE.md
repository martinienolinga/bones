# Mannequin par défaut — provenance

| | |
|---|---|
| Fichier | `mannequin.glb` (≈ 420 Ko) |
| Modèle d'origine | « Body Male - Realistic » (et ses deux yeux), *Realistic scan data model with multiresolution details* |
| Auteur | Dan Ulrich, pour Blender Studio (métadonnées d'asset du fichier `.blend`) |
| Collection | **Human Base Meshes v1.4.1** — « by Blender Studio and community contributions », mise à jour du 20 janvier 2025 |
| Licence | **CC0 1.0 Universal** (domaine public) : aucune attribution exigée ; texte complet dans `LICENSE.txt` |
| Page | <https://www.blender.org/download/demo-files/> (section *Asset Bundles*, « Human Base Meshes v1.4.1 … CC0 ») |
| Archive | <https://mirror.blender.org/demo/asset-bundles/human-base-meshes/human-base-meshes-bundle-v1.4.1.zip> |
| Dépôt | <https://projects.blender.org/studio/asset-bundles> (`human_base_meshes/human_base_meshes_bundle.blend`) |
| Récupéré le | 17 octobre 2026 |

La licence figure sur la page de téléchargement de blender.org (« 49 MB – CC0 ») et
dans les métadonnées de l'asset dans le fichier `.blend` (`license: CC0`,
`author: Dan Ulrich`).

## Modifications

Faites avec Blender 4.5 LTS en ligne de commande, par le script
`export_mannequin.py` (dans ce dossier) :

1. seuls le corps masculin réaliste et ses deux yeux sont gardés ;
2. maillage de base du modificateur multirésolution (niveau 0 : la topologie
   d'origine en quads, sans les détails sculptés des niveaux supérieurs) ;
3. fusion corps + yeux, transformations appliquées, sommets confondus soudés,
   triangulation (11 674 sommets, 23 336 triangles) ;
4. mise à l'échelle × 1,0355 pour une taille de **1,75 m**, pieds en y = 0,
   centré en X et en Z ;
5. normales lissées ; coordonnées de texture, matériaux et attributs
   supprimés (l'app affiche la couleur argile par défaut) ;
6. export GLB : Y vers le haut, mètres, le personnage regarde vers +Z (repère
   de l'app), pose en A d'origine.

Le squelette et les poids ne sont pas dans le fichier : ils sont générés au
chargement par l'auto-rig de l'app (`bones_core::samples::mannequin`).

## Modèles écartés

- Autres corps du même bundle (stylisés, féminin réaliste, primitives) : le
  corps masculin réaliste a la silhouette la plus neutre pour un mannequin.
- Quaternius « Universal Base Characters » (CC0) : style low poly de jeu,
  archive de 122 Mo à télécharger via itch.io.
- Personnages Kenney (CC0) : style cubique, trop éloigné d'un mannequin.
- Exports MakeHuman (CC0) : nécessitent l'application MakeHuman et ses
  géométries d'aide ; topologie moins propre que les bases de Blender Studio.
- Personnages d'exemple de three.js, Mixamo, Sketchfab « Standard » : licences
  incompatibles ou provenance incertaine.
