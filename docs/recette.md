# Recette avant publication

Liste de vérifications manuelles à dérouler **avant chaque version publiée**, sur chaque système visé (Windows, macOS,
Linux). Elle complète les tests automatiques (`npm test` pour les règles de l’interface, `cargo test` pour la liste
chiffrée et l’export), qui ne couvrent ni l’installateur, ni la vue web réelle, ni le rendu des photos.

**Jamais de vraies photos d’élèves ni de vrais noms.** Utilisez des photos d’adultes consentants ou libres de droits, et
des prénoms fictifs.

## Préparer le jeu d’essai

- 3 élèves fictifs : **Alice** (accord coché), **Bruno** (sans accord), **Chloé** (sans accord).
- Une photo de groupe avec au moins 4 visages, dont un visage à marquer « Visage inconnu ».
- Une photo portrait (verticale) et une photo avec un visage collé au bord de l’image.
- Au moins une photo JPEG, une PNG et une HEIC prise par un iPhone récent (iOS à jour).
- Un fichier de liste d’élèves avec colonnes « Nom » et « Prénom », enregistré depuis un tableur sous Windows
  (encodage Windows-1252, noms accentués) ; un second, un nom par ligne, en UTF-8.

Données de l’application à effacer entre deux passages (dossier `a-chacun-sa-photo`) :
Windows `%LOCALAPPDATA%\a-chacun-sa-photo`, macOS `~/Library/Application Support/a-chacun-sa-photo`,
Linux `~/.local/share/a-chacun-sa-photo`. Supprimer aussi l’entrée `fr.achacunsaphoto` du coffre-fort du système
(Gestionnaire d’identification Windows, Trousseau macOS, « Mots de passe et clés » sous Linux).

## 1. Installation et réseau coupé

- [ ] L’installateur provient de la CI de la version étiquetée ; son empreinte SHA-256 correspond à celle publiée.
- [ ] Installation sans droits particuliers autres que ceux demandés par le système ; l’application démarre.
- [ ] Windows : sur une machine sans WebView2, l’installation fonctionne **hors ligne** (installateur embarqué).
- [ ] **Réseau coupé** (Wi-Fi et câble débranchés) : tout le parcours ci-dessous fonctionne, police comprise.
- [ ] **Réseau branché** : aucune connexion ouverte par l’application ni par sa vue web pendant tout le parcours.
  - macOS / Linux : `lsof -i -n -P | grep -i -E "chacun|WebKit|WebContent"` ne renvoie rien (à relancer pendant l’import
    HEIC et l’export).
  - Windows (PowerShell) : `Get-NetTCPConnection | Where-Object OwningProcess -in (Get-Process "À chacun sa photo", msedgewebview2).Id`
    ne renvoie rien (fermer au préalable les autres applications qui utilisent WebView2 : Teams, Outlook…).

## 2. Ouverture de la classe

- [ ] Premier lancement, « Ouvrir ma classe » : la classe s’ouvre sans rien demander (clé rangée dans le coffre-fort du
  système).
- [ ] Relancer et ouvrir : les élèves sont toujours là, sans rien saisir.
- [ ] Le fichier `roster.sqlite` n’est pas lisible en clair (`sqlite3 roster.sqlite ".tables"` → erreur « file is not a
  database »).

### Repli par phrase de passe (Linux sans coffre-fort, PrimTux…)

Sur PrimTux ou une distribution sans coffre-fort, ou en simulant son absence sur n’importe quel Linux :

```sh
env -u DBUS_SESSION_BUS_ADDRESS ./a-chacun-sa-photo.AppImage
```

- [ ] Sans classe existante : l’écran explique la situation et propose de choisir une phrase de passe.
- [ ] Moins de 8 caractères, ou deux saisies différentes : message clair, rien n’est créé.
- [ ] Après création : le fichier `roster.passphrase-mode` existe dans le dossier de données.
- [ ] Relancer : la phrase de passe est demandée ; une mauvaise phrase est refusée avec un message en français ; la bonne
  ouvre la classe.
- [ ] Relancer **avec** le coffre-fort disponible (sans `env -u …`) : la phrase de passe est toujours demandée (pas de
  bascule silencieuse).
- [ ] « J’ai oublié ma phrase de passe » : confirmation, puis l’ancien fichier est déplacé dans `archive/` (pas supprimé) et
  une nouvelle classe démarre.

### Ancien fichier illisible

- [ ] Supprimer l’entrée du coffre-fort en gardant `roster.sqlite`, relancer : l’écran « Une ancienne classe ne peut pas être
  ouverte » s’affiche ; « Commencer une nouvelle classe » archive l’ancien fichier dans `archive/`.

## 3. Liste des élèves

- [ ] Ajouter un élève : accord **décoché par défaut**. Un nom déjà présent est refusé avec un message en français.
- [ ] Importer le fichier Windows-1252 : accents corrects, « Prénom Nom », tous **masqués** ; le message indique le
  nombre d’ajouts et de doublons ignorés. Idem avec le fichier UTF-8.
- [ ] Modifier un nom et l’accord ; la pastille de l’élève change (net / flouté).
- [ ] Sauvegarder la classe avec une phrase de passe, supprimer un élève, restaurer : l’élève revient, avec l’accord de
  la sauvegarde. Une mauvaise phrase de passe est refusée.

## 4. Photos et cadres

- [ ] Importer JPEG, PNG et HEIC : toutes s’affichent ; le message indique les HEIC converties (Windows / Linux).
- [ ] **PC modeste** (4 Go de RAM, processeur d’entrée de gamme) : importer 50 photos HEIC d’un coup. Noter la durée et la
  mémoire maximale (Gestionnaire des tâches) ; l’application ne doit ni planter ni geler le système.
- [ ] Dessiner un cadre à la souris pour chaque enfant ; déplacer et redimensionner par les poignées ; réattribuer un
  cadre à un autre élève.
- [ ] **Clavier seul** : Tab jusqu’à la photo, Entrée ajoute un cadre au centre, flèches déplacent, Maj + flèches
  redimensionnent, Suppr retire ; ←/→ changent de photo quand aucun cadre n’a le focus.
- [ ] Zoom (boutons, `+`/`-`/`0`, Ctrl/⌘ + molette) : les cadres restent sur les visages.
- [ ] **Clavier seul, sans souris** : « Ajouter des photos » (en-tête et écran vide) et « Gérer la classe » →
  « Importer une liste » s’atteignent avec Tab et ouvrent le sélecteur de fichiers avec Entrée ou Espace (sous macOS,
  Tab ne passe sur les boutons que si « Navigation au clavier » est activée dans Réglages Système > Clavier ; sinon
  utiliser Option + Tab).
- [ ] **Taille du texte** : Ctrl/⌘ + `=` agrandit tout le texte de l’application (jusqu’à 200 % au moins), Ctrl/⌘ + `-`
  le réduit, Ctrl/⌘ + `0` revient à 100 %, sans zoomer la photo ; à 200 %, rien n’est coupé ni inaccessible. Fenêtre réduite à 640 px de large :
  la liste des élèves passe au-dessus de la photo.
- [ ] **Liens d’évitement** : la première tabulation affiche « Aller à la photo », puis « Aller à la liste des élèves » ;
  Entrée place le focus au bon endroit.
- [ ] **Erreurs de saisie** : une phrase de passe trop courte, ou un élève déjà présent, affiche le message sous le champ
  et y replace le focus.
- [ ] **Focus jamais perdu** : après « Ouvrir ma classe », la modification ou la suppression d’un élève, l’export ou
  « Effacer la session », la tabulation suivante reste à un endroit logique.
- [ ] **macOS, langue** : les menus sont en français (« Édition », « Fenêtre »), le sélecteur de fichiers aussi
  (« Annuler », « Ouvrir »), et VoiceOver lit l’application avec une voix française.
- [ ] ⌘Q avec des photos identifiées non exportées : l’avertissement « Quitter sans exporter ? » s’affiche.
- [ ] **Lecteur d’écran** (NVDA sous Windows, VoiceOver sous macOS) : les messages du bandeau (« La classe est prête »,
  « 3 photo(s) ajoutée(s) », fin d’export, erreurs) sont lus automatiquement ; un cadre est annoncé « Cadre de …,
  cadre » et ses boutons × / ⇄ sont atteignables avec Tab juste après.
- [ ] Supprimer l’élève Chloé : ses cadres deviennent « Visage inconnu », toujours floutés.

## 5. Export (vérification essentielle)

- [ ] L’export crée un dossier « Photos du JJ-MM-AAAA à HHhMM » avec un sous-dossier par élève ayant au moins une photo,
  et **aucun** dossier « Visage inconnu ».
- [ ] Ouvrir chaque photo exportée et vérifier, **en zoomant** :

  | Dans le dossier de… | Alice (accord) | Bruno (sans accord) | Chloé (sans accord) | Visage inconnu |
  |---|---|---|---|---|
  | Alice | net | flouté | flouté | flouté |
  | Bruno | net | **net** | flouté | flouté |
  | Chloé | net | flouté | **net** | flouté |

- [ ] Aucun trait du visage n’est reconnaissable sous le flou, y compris sur une photo en haute résolution.
- [ ] Le visage collé au bord est entièrement couvert.
- [ ] Refaire l’export en mode « aplat » : ellipses opaques aux mêmes endroits.
- [ ] Métadonnées retirées : `exiftool photo.jpg` n’affiche ni GPS, ni appareil, ni date de prise de vue (ou Propriétés
  → Détails sous Windows).
- [ ] Exporter deux fois dans le même dossier : un second dossier « … (2) » est créé, rien n’est écrasé.
- [ ] Les fichiers d’origine sont intacts et aucun fichier photo n’a été écrit ailleurs que dans le dossier choisi
  (dossier de données de l’application : seulement `roster.sqlite` et ses fichiers associés).

## 6. Fin de session et fin d’année

- [ ] Modifier un cadre après l’export puis fermer la fenêtre : la question « Quitter sans exporter ? » apparaît ;
  « Rester » annule la fermeture. Juste après un export, la fenêtre se ferme sans question.
- [ ] « Effacer la session » vide la liste des photos.
- [ ] « Nouvelle année : effacer la classe » efface tous les élèves après confirmation.
- [ ] « À propos » affiche le bon numéro de version et la liste des licences.

## Compte rendu

Reporter dans le ticket de la version : système et version, machine (processeur, mémoire), version testée, cases non
cochées et problèmes rencontrés (sans photo ni nom réel).
