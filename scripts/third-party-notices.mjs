// Génère THIRD-PARTY-NOTICES.md : composants tiers embarqués dans l'application et leurs licences.
// À relancer après toute mise à jour des dépendances : node scripts/third-party-notices.mjs
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const root = new URL("..", import.meta.url).pathname;

// Dépendances npm de production (hors outils de build), d'après package-lock.json.
const lock = JSON.parse(readFileSync(`${root}package-lock.json`, "utf8"));
const npm = Object.entries(lock.packages)
  .filter(([path, info]) => path && !info.dev && !info.devOptional)
  .map(([path, info]) => ({ name: path.replace(/^.*node_modules\//, ""), version: info.version, license: info.license ?? "voir le paquet" }));

// Dépendances Rust normales (toutes plateformes), d'après le graphe résolu par Cargo.
const metadata = JSON.parse(execFileSync("cargo", ["metadata", "--format-version", "1", "--locked"], { cwd: `${root}src-tauri`, maxBuffer: 64 << 20 }));
const packages = new Map(metadata.packages.map((pkg) => [pkg.id, pkg]));
const nodes = new Map(metadata.resolve.nodes.map((node) => [node.id, node]));
const seen = new Set();
const queue = [metadata.resolve.root];
while (queue.length) {
  const node = nodes.get(queue.pop());
  for (const dep of node.deps) {
    if (seen.has(dep.pkg) || !dep.dep_kinds.some((kind) => kind.kind === null)) continue;
    seen.add(dep.pkg);
    queue.push(dep.pkg);
  }
}
const rust = [...seen].map((id) => packages.get(id)).map((pkg) => ({ name: pkg.name, version: pkg.version, license: pkg.license ?? "voir le paquet" }));

const byName = (a, b) => a.name.localeCompare(b.name) || a.version.localeCompare(b.version);
const table = (rows) => ["| Composant | Version | Licence |", "|---|---|---|", ...rows.sort(byName).map((r) => `| ${r.name} | ${r.version} | ${r.license} |`)].join("\n");

writeFileSync(`${root}THIRD-PARTY-NOTICES.md`, `# Composants tiers

À chacun sa photo est distribué sous licence EUPL-1.2 (voir \`LICENSE\`). L’application embarque les composants
tiers ci-dessous, chacun sous sa propre licence. Fichier généré par \`node scripts/third-party-notices.mjs\`.

## Bibliothèques natives embarquées

| Composant | Fourni par | Licence |
|---|---|---|
| SQLCipher (chiffrement de la liste des élèves) | crate \`libsqlite3-sys\` (bundled-sqlcipher) | BSD-3-Clause (Zetetic LLC) ; SQLite : domaine public |
| OpenSSL (primitives cryptographiques de SQLCipher) | crate \`openssl-src\` | Apache-2.0 |
| libheif et libde265 (lecture des photos HEIC), compilés en WebAssembly | paquet npm \`heic-to\` | LGPL-3.0 |
| Atkinson Hyperlegible Next (police) | paquet npm \`@fontsource-variable/atkinson-hyperlegible-next\` | OFL-1.1 |

**Bibliothèques LGPL (heic-to, libheif, libde265).** Elles sont livrées sans modification, dans un fichier séparé
chargé à la demande (\`heic-to\` dans le dossier \`assets\` de l’application). Leur code source est disponible
sur <https://github.com/hoppergee/heic-to>, <https://github.com/strukturag/libheif> et
<https://github.com/strukturag/libde265>. Vous pouvez remplacer ce fichier par une version modifiée en
reconstruisant l’application depuis ses sources (voir \`README.md\`).

## Paquets JavaScript intégrés à l’interface

${table(npm)}

## Crates Rust intégrées au programme

Liste pour toutes les plateformes (Windows, macOS, Linux) ; chaque installateur n’en contient qu’une partie.

${table(rust)}
`);
console.log(`THIRD-PARTY-NOTICES.md : ${npm.length} paquets npm, ${rust.length} crates Rust.`);
