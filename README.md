# À chacun sa photo

Application de bureau pour les enseignants : trier les photos de la classe (sortie, spectacle, vie de classe) et
préparer **un dossier par élève** à remettre aux familles, en **floutant les enfants dont la famille n’a pas autorisé
la diffusion**.

Elle fonctionne entièrement **hors ligne** : aucun compte, aucun service en ligne, aucune statistique d’utilisation,
aucune mise à jour automatique.

Logiciel libre sous licence [EUPL-1.2](LICENSE).

## Ce que fait l’application

- enregistre la liste des élèves (nom et accord de diffusion) **chiffrée** sur l’ordinateur ;
- importe des photos JPEG, PNG et HEIC (iPhone) **en mémoire seulement**, le temps de la session ;
- permet d’encadrer à la main le visage de chaque enfant, à la souris ou au clavier ;
- propose un « visage inconnu », toujours flouté, pour les enfants qui ne sont pas dans la classe ;
- exporte un dossier JPEG par élève : sur chaque photo, les enfants non autorisés sont floutés (ou masqués par un aplat),
  sauf dans leur propre dossier ;
- réencode chaque photo exportée, ce qui retire les métadonnées (date, appareil, position GPS) ;
- ne garde aucune copie des photos.

Guide d’utilisation : [docs/guide-enseignant.md](docs/guide-enseignant.md).

## Protection des données

La seule donnée conservée est la liste des élèves, dans `roster.sqlite`, dans le dossier de données local de
l’utilisateur (`a-chacun-sa-photo`). Ce fichier est chiffré avec SQLCipher :

- **cas général** : la clé est tirée au hasard par l’application et rangée dans le coffre-fort du système
  (Trousseau macOS, Gestionnaire d’identification Windows, Secret Service sous Linux). L’enseignant n’a rien à retenir ;
- **sans coffre-fort** (fréquent sous Linux) : l’application explique la situation et demande de choisir une phrase de
  passe, redemandée à chaque ouverture.

L’accord de diffusion est **non autorisé par défaut** : un élève est masqué pour les autres familles tant que
l’enseignant n’a pas coché l’accord. L’application aide à respecter les choix des familles ; elle ne remplace pas le
recueil écrit des autorisations et **ne détecte pas les visages** : l’enseignant doit encadrer chaque enfant non autorisé
visible.

Pour le directeur, la directrice ou le DPO : [kit RGPD](docs/kit-rgpd/).

## Fonctionnement hors ligne

La politique de sécurité du contenu (CSP) interdit tout script, image, formulaire ou connexion distants. Toutes les
dépendances sont embarquées (polices, décodeur HEIC en WebAssembly). L’installateur Windows embarque l’installateur hors
ligne de WebView2.

Chaque version publiée doit être vérifiée avec le réseau coupé, en contrôlant que ni l’application ni sa vue web ne tentent
de connexion.

## Développement

Prérequis : Node.js récent, Rust, et les [prérequis Tauri](https://tauri.app/start/prerequisites/) de votre système.

```sh
npm install
npm run tauri dev        # application complète
npm run build            # vérification des types + construction de l’interface
npm run tauri build      # installateur pour le système courant
npm test                 # tests des règles de l’interface (floutage, export, import)
cd src-tauri && cargo test
```

Avant chaque version publiée : dérouler la [recette manuelle](docs/recette.md) sur chaque système.

Architecture, contraintes et conventions : [CLAUDE.md](CLAUDE.md). Contribuer : [CONTRIBUTING.md](CONTRIBUTING.md).
Composants tiers et licences : [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).

## Avant un pilote public

- faire l’audit d’accessibilité RGAA ;
- faire valider la règle de floutage par des enseignants et un DPO ;
- signer les installateurs (Windows, notarisation macOS), rendre les constructions reproductibles, tester chaque version réseau coupé ;
- tester la conversion HEIC sur des photos d’iPhone récentes et des PC Windows peu puissants ;
- tester le repli par phrase de passe sur les distributions Linux des écoles (PrimTux…).
