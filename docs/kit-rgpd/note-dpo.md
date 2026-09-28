# Note technique à l’attention du DPO

## Objet

« À chacun sa photo » est une application de bureau libre (EUPL-1.2) qui aide un·e enseignant·e à remettre à chaque
famille les photos de classe où figure son enfant, en floutant les enfants dont la famille n’a pas autorisé la diffusion.

## Données et flux

| Donnée | Où | Combien de temps |
|---|---|---|
| Nom de l’élève, accord de diffusion (booléen) | Fichier `roster.sqlite` dans le dossier de données local du compte utilisateur, chiffré | Jusqu’à suppression par l’enseignant (fonction « Nouvelle année » pour tout effacer) |
| Photos d’origine | Restent à leur emplacement ; lues en mémoire vive le temps de la session | Libérées à la fermeture ou via « Effacer la session » |
| Cadres des visages | Mémoire vive uniquement | Idem |
| Photos exportées | Dossier choisi par l’enseignant | Sous sa responsabilité ; suppression recommandée après remise |
| Sauvegarde de la liste (facultative) | Fichier choisi par l’enseignant, chiffré par une phrase de passe | Sous sa responsabilité |

Aucune donnée ne quitte l’ordinateur par le logiciel : pas de compte, pas de télémétrie, pas de mise à jour automatique,
pas de ressource distante. La politique de sécurité du contenu de la vue web interdit toute connexion réseau.

## Mesures de sécurité

- **Chiffrement** de la liste des élèves avec SQLCipher (AES-256, dérivation PBKDF2). La clé est aléatoire (256 bits)
  et conservée dans le coffre-fort du système (Trousseau macOS, Gestionnaire d’identification Windows, Secret Service
  Linux). En l’absence de coffre-fort, l’enseignant choisit une phrase de passe demandée à chaque ouverture.
- **Minimisation** : seuls le nom et l’accord sont conservés ; aucune photo, aucune donnée biométrique, aucune
  détection automatique de visage.
- **Protection par défaut** : un élève nouvellement saisi ou importé est considéré comme non autorisé.
- **Export** : chaque photo est réencodée en JPEG, ce qui supprime les métadonnées EXIF (dont la géolocalisation). Un
  export n’écrase jamais un fichier existant.
- **Effacement** : la suppression d’élèves écrase les données supprimées dans le fichier (`secure_delete`).

## Risques résiduels et mesures organisationnelles

| Risque | Mesure |
|---|---|
| Un enfant non autorisé visible mais non encadré par l’enseignant (arrière-plan) apparaît net chez les autres familles | Le logiciel ne détecte pas les visages, par choix de minimisation. Un rappel est affiché avant chaque export ; un « visage inconnu » permet de flouter toute personne hors liste. Relecture des photos par l’enseignant |
| Poste partagé sur une même session utilisateur | La clé du coffre-fort est liée à la session : toute personne utilisant la même session peut ouvrir la classe. Utiliser une session personnelle |
| Dossiers d’export conservés sur le poste ou une clé USB | Consigne de suppression après remise (guide de l’enseignant) |
| Rediffusion par une famille | Engagement de non-publication dans le formulaire d’autorisation |
| Flou insuffisant | Le flou réduit la zone à environ 6 × 6 cellules avant agrandissement ; un masquage opaque est disponible |

## Code source et vérification

Le code source est public ; le fonctionnement décrit ici peut être vérifié dans `src-tauri/src/lib.rs` (stockage et
écriture des fichiers) et `src/App.vue` (règle de floutage, fonction `renderFor`).
