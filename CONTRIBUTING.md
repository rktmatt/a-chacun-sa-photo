# Contribuer

Merci de votre intérêt ! Les retours d’enseignants sont aussi précieux que le code : n’hésitez pas à ouvrir un ticket
pour signaler un problème ou proposer une amélioration, en décrivant votre situation (système, type de photos, étape).

**Ne joignez jamais de vraies photos d’élèves ni de listes nominatives à un ticket.**

## Règles du projet

Ces contraintes sont non négociables ; une contribution qui les enfreint ne pourra pas être acceptée :

- **aucune connexion réseau**, jamais : pas de télémétrie, de mise à jour automatique, de CDN, de police distante ni
  d’API en ligne. Ne pas assouplir la CSP de `src-tauri/tauri.conf.json` ;
- **les photos ne sont jamais enregistrées par l’application** (ni cache, ni miniature sur disque, ni base de données) ;
- **identification manuelle uniquement** : pas de détection ou de reconnaissance de visage ;
- **permissions Tauri minimales** : toute nouvelle permission dans `src-tauri/capabilities/default.json` doit être justifiée ;
- **textes de l’interface et messages d’erreur en français**, y compris ceux renvoyés par Rust ;
- rester utilisable sur les ordinateurs modestes des écoles : dépendances légères, mémoire maîtrisée.

Le détail de l’architecture est dans [CLAUDE.md](CLAUDE.md).

## Avant de proposer une modification

```sh
npm run build                         # types + construction de l’interface
cd src-tauri && cargo clippy --all-targets && cargo test
```

Si vous ajoutez ou mettez à jour une dépendance, régénérez la liste des licences :

```sh
node scripts/third-party-notices.mjs
```

Ajoutez une ligne au [CHANGELOG](CHANGELOG.md) sous « Non publié ».

## Licence

En contribuant, vous acceptez que votre contribution soit distribuée sous licence [EUPL-1.2](LICENSE).
