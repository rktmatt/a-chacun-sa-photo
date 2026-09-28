# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

"À chacun sa photo" (package name `edu-pic`): offline desktop app for school teachers to sort class photos into one folder per pupil. Licensed EUPL-1.2 (`LICENSE`). Intended for open-source release and listing on French public education tool registries. All UI text and user-facing error messages (including those returned from Rust) are in French; keep it that way.

Stack: Tauri 2 (Rust backend) + Vue 3 `<script setup>` + Vite + TypeScript. No Nuxt, no router, no state library — the whole frontend is `src/App.vue`.

## Commands

```sh
npm install
npm run tauri dev        # full app (starts Vite on 127.0.0.1:1420, strictPort)
npm run build            # vue-tsc type check + Vite build into dist/
npm run tauri build      # release installer for the current platform
npx vue-tsc --noEmit     # type check frontend only
cd src-tauri && cargo check    # / cargo clippy for the Rust side
cd src-tauri && cargo test     # Rust tests (backup/restore, export no-overwrite)
```

No frontend tests or linter yet.

## Non-negotiable constraints

- **Zero network calls, ever.** No telemetry, updater, CDN, remote fonts, or online APIs. The CSP in `src-tauri/tauri.conf.json` only allows `self`, `ipc:`, `blob:`, `data:`, `asset:`; do not loosen it. All dependencies must be bundled (e.g. `heic-to/csp` — recent libheif as embedded wasm — is lazy-loaded from the local bundle, which is why CSP has `wasm-unsafe-eval`). Windows installer embeds the WebView2 offline installer.
- **Photos are never persisted by the app.** They live only as `File` objects + `blob:` URLs in frontend memory. Only at export are they re-encoded and written to the teacher-chosen folder. Never cache, thumbnail to disk, or store photos/tags in SQLite.
- **Only persistent data is the roster** (`children` table: `name`, `allows_distribution`) in `<data_local_dir>/edu-pic/roster.sqlite`, encrypted with SQLCipher.
- **Tagging is manual only.** No face detection / ML.
- **Minimal Tauri permissions.** `src-tauri/capabilities/default.json` exposes only core + dialog open/save/ask + `core:window:allow-destroy` (needed by the close-request guard). Any new plugin needs an explicit, minimal capability entry.
- Must run on low-end school hardware: avoid heavy deps, keep memory in check (export processes one image at a time on purpose).

## Architecture

### Rust backend (`src-tauri/src/lib.rs`, single file)
- `RosterDb(Mutex<Option<Connection>>)` managed state; `with_db` helper errors if roster not opened.
- Encryption key: 32 random bytes, base64, generated on first run and stored in the OS credential vault via `keyring` (service `fr.achacunsaphoto`, same as the Tauri identifier, account `roster-encryption-key`). Teacher never sees or enters a secret. Key is applied with `PRAGMA key`; `cipher_memory_security = ON`.
- `open_roster` returns the sentinel error string `"LEGACY_ROSTER"` when a DB exists but the key was freshly generated (can't decrypt). `archive_legacy_roster` then moves the old file + `-wal`/`-shm` into `archive/` (never deletes).
- Commands: `open_roster`, `archive_legacy_roster`, `list_children`, `save_child` (insert; duplicate name → French error), `update_child`, `delete_child`, `clear_roster` (end of school year; `secure_delete` + `VACUUM`), `import_children` (names list, consent off, duplicates skipped), `backup_roster` / `restore_roster` (SQLCipher `ATTACH … KEY` with a teacher passphrase + `sqlcipher_export`; restore merges, backup consent wins), `prepare_export_folder`, `write_export_image`. Register new commands in `generate_handler!`.
- Export never overwrites: `prepare_export_folder` creates a fresh `Photos du JJ-MM-AAAA à HHhMM` folder (suffix ` (2)`… if taken) and `write_export_image` opens files with `create_new`. Paths go through `absolute_path` + `safe_name` (rejects separators, reserved Windows chars, `.`/`..`). Keep this validation on any new file-writing command.
- All error strings returned to the frontend are French (`db_error` maps SQLite errors); don't return raw `e.to_string()`.

### Frontend (`src/App.vue`)
- `UNKNOWN_ID = 0` is a frontend-only pseudo-pupil « Visage inconnu »: always blurred, never gets a folder. Tags of a deleted pupil (or after `clear_roster`) are converted to it. `renderFor` blurs any tag whose child is missing. Beware `selectedChildId` can be `0`: compare with `null`, never truthiness.
- New pupils default to consent **off** (`childAllowed = false`); CSV/TXT roster import (one name per line or « Nom »/« Prénom » columns, UTF-8 or Windows-1252) also imports as masked.
- Close guard: `onCloseRequested` asks before quitting if tags changed since last successful export (`tagSignature` vs `exportedSignature`). Confirmations use plugin-dialog `ask`, not `window.confirm`.
- Zoom (1–6×, buttons, `+`/`-`/`0`, Ctrl/⌘+wheel) resizes the `<img>`; tag coordinates stay valid because `pointerPosition` uses the frame's bounding rect. Mask mode: blur (default) or opaque fill.
- Types: `Child {id, name, allowsDistribution}` (camelCase via serde `rename_all`), `Tag {childId, x, y, width, height}`, `Photo {file, url, tags}`.
- Tag coordinates are **normalized 0–1** relative to the displayed image, so they map directly to natural image pixels at export.
- HEIC/HEIF on import: kept as-is if the webview decodes it natively (`createImageBitmap`, e.g. WebKit on macOS), else converted to JPEG via lazy `import("heic-to/csp")`. (`heic2any` was dropped: its 2020 libheif fails on recent iPhone HEIC with `ERR_LIBHEIF format not supported`.)
- Export (`exportPhotos` → `renderFor`): for each pupil, every photo containing a tag for that pupil is drawn to a canvas; for every *other* tagged child with `allowsDistribution === false`, the ellipse inscribed in the tag box (padded by 1.5% each side) is blurred. The recipient's own face is never blurred in their own folder. Output is always re-encoded JPEG (strips EXIF/GPS), filenames `NNN-<name>.jpg`, one folder per child name.
- `discardSession` revokes blob URLs and empties the photo list.

## Known gaps / inconsistencies
- README "Before a public pilot" list: RGAA accessibility audit, DPO review of blur rule, signed/reproducible builds, network-denied release test, HEIC testing on low-memory Windows.
