use base64::{engine::general_purpose::STANDARD, Engine};
use keyring::Entry;
use rand::RngCore;
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::{Component, Path, PathBuf}, sync::Mutex, time::{SystemTime, UNIX_EPOCH}};
use tauri::State;

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS children (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL COLLATE NOCASE UNIQUE,
  allows_distribution INTEGER NOT NULL CHECK(allows_distribution IN (0,1))
);";

struct RosterDb(Mutex<Option<Connection>>);

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Child {
    id: i64,
    name: String,
    allows_distribution: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportImage {
    export_folder: String,
    child_folder: String,
    file_name: String,
    jpeg_base64: String,
}

fn data_dir() -> Result<PathBuf, String> {
    let path = dirs::data_local_dir()
        .ok_or("Impossible de déterminer le dossier de données local.")?
        .join("a-chacun-sa-photo");
    fs::create_dir_all(&path).map_err(|_| "Impossible de créer le dossier de données de l'application.".to_string())?;
    Ok(path)
}

fn roster_key() -> Result<(String, bool), String> {
    let entry = Entry::new("fr.achacunsaphoto", "roster-encryption-key")
        .map_err(|_| "Le coffre-fort sécurisé de cet ordinateur est indisponible.".to_string())?;
    match entry.get_password() {
        Ok(key) => Ok((key, true)),
        Err(keyring::Error::NoEntry) => {
            let mut bytes = [0_u8; 32];
            rand::rng().fill_bytes(&mut bytes);
            let key = STANDARD.encode(bytes);
            entry.set_password(&key)
                .map_err(|_| "Impossible de préparer le coffre-fort sécurisé de cet ordinateur.".to_string())?;
            Ok((key, false))
        }
        Err(_) => Err("Le coffre-fort sécurisé de cet ordinateur est verrouillé ou indisponible.".into()),
    }
}

fn safe_name(name: &str) -> Result<&str, String> {
    if name.is_empty() || name.len() > 120 || name == "." || name == ".."
        || name.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')) {
        return Err("Nom de dossier ou de fichier invalide.".into());
    }
    Ok(name)
}

/// Fichier témoin (sans aucun secret) : la classe est protégée par une phrase de passe saisie par l'enseignant,
/// parce que cet ordinateur n'offrait pas de coffre-fort (fréquent sous Linux sans gnome-keyring/KWallet).
const PASSPHRASE_MARKER: &str = "roster.passphrase-mode";

fn open_encrypted(path: &Path, key: &str) -> Result<Connection, ()> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE).map_err(|_| ())?;
    // SQLCipher chiffre les pages, y compris WAL/journal. La clé ne reste qu'en mémoire du processus.
    connection.pragma_update(None, "key", key).map_err(|_| ())?;
    connection.execute_batch(&format!("PRAGMA cipher_memory_security = ON; PRAGMA foreign_keys = ON; PRAGMA secure_delete = ON; {SCHEMA}"))
        .map_err(|_| ())?;
    Ok(connection)
}

fn store(db: &RosterDb, connection: Connection) -> Result<(), String> {
    *db.0.lock().map_err(|_| "Base de données indisponible.".to_string())? = Some(connection);
    Ok(())
}

/// Ouvre la classe avec la clé du coffre-fort du système. Erreurs sentinelles pour l'interface :
/// `LEGACY_ROSTER` (fichier illisible avec une clé neuve), `PASSPHRASE_SETUP` (pas de coffre-fort, aucune classe :
/// proposer de choisir une phrase de passe), `PASSPHRASE_REQUIRED` (classe protégée par phrase de passe).
#[tauri::command]
fn open_roster(db: State<RosterDb>) -> Result<(), String> {
    let directory = data_dir()?;
    let path = directory.join("roster.sqlite");
    if directory.join(PASSPHRASE_MARKER).exists() { return Err("PASSPHRASE_REQUIRED".into()); }
    let (key, already_existed) = match roster_key() {
        Ok(found) => found,
        Err(_) if !path.exists() => return Err("PASSPHRASE_SETUP".into()),
        Err(message) => return Err(message),
    };
    let connection = open_encrypted(&path, &key).map_err(|_| {
        if already_existed {
            "Impossible d'ouvrir la classe. Le coffre-fort sécurisé ou le fichier de la classe est peut-être endommagé.".to_string()
        } else {
            "LEGACY_ROSTER".to_string()
        }
    })?;
    store(&db, connection)
}

/// Repli sans coffre-fort : `create` choisit la phrase de passe d'une nouvelle classe, sinon elle déverrouille la classe existante.
#[tauri::command]
fn open_roster_with_passphrase(passphrase: String, create: bool, db: State<RosterDb>) -> Result<(), String> {
    valid_passphrase(&passphrase)?;
    let directory = data_dir()?;
    let path = directory.join("roster.sqlite");
    let marker = directory.join(PASSPHRASE_MARKER);
    if create {
        if path.exists() { return Err("Une classe existe déjà sur cet ordinateur.".into()); }
        let connection = open_encrypted(&path, &passphrase).map_err(|_| "Impossible de créer la classe.".to_string())?;
        fs::write(&marker, "La liste des élèves est protégée par une phrase de passe.\n")
            .map_err(|_| "Impossible d'enregistrer le mode de protection de la classe.".to_string())?;
        return store(&db, connection);
    }
    if !marker.exists() { return Err("Cette classe n'est pas protégée par une phrase de passe.".into()); }
    let connection = open_encrypted(&path, &passphrase).map_err(|_| "Phrase de passe incorrecte.".to_string())?;
    store(&db, connection)
}

#[tauri::command]
fn archive_legacy_roster() -> Result<(), String> {
    let directory = data_dir()?;
    let roster = directory.join("roster.sqlite");
    let marker = directory.join(PASSPHRASE_MARKER);
    if marker.exists() { fs::remove_file(&marker).map_err(|_| "Impossible de réinitialiser la protection de la classe.".to_string())?; }
    if !roster.exists() { return Ok(()); }
    let archive = directory.join("archive");
    fs::create_dir_all(&archive).map_err(|_| "Impossible de créer le dossier d'archive.".to_string())?;
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| "Horloge de l'ordinateur invalide.".to_string())?.as_secs();
    fs::rename(&roster, archive.join(format!("roster-before-device-key-{timestamp}.sqlite")))
        .map_err(|_| "Impossible d'archiver l'ancienne classe.".to_string())?;
    for suffix in ["-wal", "-shm"] {
        let sidecar = directory.join(format!("roster.sqlite{suffix}"));
        if sidecar.exists() {
            fs::rename(&sidecar, archive.join(format!("roster-before-device-key-{timestamp}.sqlite{suffix}")))
                .map_err(|_| "Impossible d'archiver les fichiers associés à l'ancienne classe.".to_string())?;
        }
    }
    Ok(())
}

fn with_db<T>(state: &RosterDb, action: impl FnOnce(&Connection) -> Result<T, String>) -> Result<T, String> {
    let guard = state.0.lock().map_err(|_| "Base de données indisponible.".to_string())?;
    action(guard.as_ref().ok_or("La classe n'est pas encore ouverte.")?)
}

#[tauri::command]
fn list_children(db: State<RosterDb>) -> Result<Vec<Child>, String> {
    with_db(&db, |conn| {
        let mut statement = conn.prepare("SELECT id, name, allows_distribution FROM children ORDER BY name COLLATE NOCASE")
            .map_err(db_error)?;
        let rows = statement.query_map([], |row| Ok(Child { id: row.get(0)?, name: row.get(1)?, allows_distribution: row.get(2)? }))
            .map_err(db_error)?
            .collect::<Result<Vec<_>, _>>().map_err(db_error);
        rows
    })
}

fn db_error(error: rusqlite::Error) -> String {
    match error {
        rusqlite::Error::SqliteFailure(e, _) if e.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_UNIQUE =>
            "Un élève porte déjà ce nom.".into(),
        _ => "La classe n'a pas pu être enregistrée.".into(),
    }
}

fn valid_child_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() || name.len() > 100 { return Err("Le nom doit contenir entre 1 et 100 caractères.".into()); }
    Ok(name)
}

fn valid_passphrase(passphrase: &str) -> Result<(), String> {
    if passphrase.chars().count() < 8 { return Err("La phrase de passe doit contenir au moins 8 caractères.".into()); }
    Ok(())
}

fn absolute_path(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("Emplacement de fichier invalide.".into());
    }
    Ok(path)
}

#[tauri::command]
fn save_child(name: String, allows_distribution: bool, db: State<RosterDb>) -> Result<(), String> {
    let name = valid_child_name(&name)?;
    with_db(&db, |conn| conn.execute(
        "INSERT INTO children (name, allows_distribution) VALUES (?1, ?2)",
        params![name, allows_distribution]
    ).map(|_| ()).map_err(db_error))
}

#[tauri::command]
fn update_child(id: i64, name: String, allows_distribution: bool, db: State<RosterDb>) -> Result<(), String> {
    let name = valid_child_name(&name)?;
    with_db(&db, |conn| conn.execute(
        "UPDATE children SET name = ?1, allows_distribution = ?2 WHERE id = ?3",
        params![name, allows_distribution, id]
    ).map(|_| ()).map_err(db_error))
}

#[tauri::command]
fn delete_child(id: i64, db: State<RosterDb>) -> Result<(), String> {
    with_db(&db, |conn| conn.execute("DELETE FROM children WHERE id = ?1", params![id]).map(|_| ()).map_err(db_error))
}

/// Fin d'année : efface tous les élèves. `secure_delete` écrase le contenu supprimé dans le fichier.
#[tauri::command]
fn clear_roster(db: State<RosterDb>) -> Result<(), String> {
    with_db(&db, |conn| conn.execute_batch("DELETE FROM children; VACUUM;").map_err(db_error))
}

/// Ajoute une liste de noms (import ONDE / tableur). Les nouveaux élèves sont masqués par défaut ;
/// les noms déjà présents sont ignorés. Renvoie le nombre d'élèves ajoutés.
#[tauri::command]
fn import_children(names: Vec<String>, db: State<RosterDb>) -> Result<usize, String> {
    let mut guard = db.0.lock().map_err(|_| "Base de données indisponible.".to_string())?;
    let conn = guard.as_mut().ok_or("La classe n'est pas encore ouverte.")?;
    let tx = conn.transaction().map_err(db_error)?;
    let mut added = 0;
    for name in &names {
        let Ok(name) = valid_child_name(name) else { continue };
        added += tx.execute("INSERT INTO children (name, allows_distribution) VALUES (?1, 0) ON CONFLICT(name) DO NOTHING", params![name])
            .map_err(db_error)?;
    }
    tx.commit().map_err(db_error)?;
    Ok(added)
}

/// Sauvegarde chiffrée par une phrase de passe choisie par l'enseignant, pour changer d'ordinateur.
#[tauri::command]
fn backup_roster(destination: String, passphrase: String, db: State<RosterDb>) -> Result<(), String> {
    valid_passphrase(&passphrase)?;
    let destination = absolute_path(&destination)?;
    let temporary = destination.with_extension("tmp");
    if temporary.exists() { fs::remove_file(&temporary).map_err(|_| "Impossible de préparer la sauvegarde.".to_string())?; }
    with_db(&db, |conn| export_encrypted(conn, &temporary, &passphrase))?;
    fs::rename(&temporary, &destination).map_err(|_| "Impossible d'enregistrer la sauvegarde à cet emplacement.".to_string())
}

/// Restaure une sauvegarde : les élèves sont fusionnés avec la classe actuelle (l'accord de la sauvegarde prévaut).
#[tauri::command]
fn restore_roster(source: String, passphrase: String, db: State<RosterDb>) -> Result<usize, String> {
    valid_passphrase(&passphrase)?;
    let source = absolute_path(&source)?;
    if !source.is_file() { return Err("Fichier de sauvegarde introuvable.".into()); }
    with_db(&db, |conn| import_encrypted(conn, &source, &passphrase))
}

fn export_encrypted(conn: &Connection, target: &Path, passphrase: &str) -> Result<(), String> {
    let target = target.to_str().ok_or("Emplacement de fichier invalide.")?;
    conn.execute("ATTACH DATABASE ?1 AS backup KEY ?2", params![target, passphrase])
        .map_err(|_| "Impossible de créer la sauvegarde.".to_string())?;
    let exported = conn.query_row("SELECT sqlcipher_export('backup')", [], |_| Ok(()));
    let _ = conn.execute("DETACH DATABASE backup", []);
    exported.map_err(|_| "Impossible d'écrire la sauvegarde.".to_string())
}

fn import_encrypted(conn: &Connection, source: &Path, passphrase: &str) -> Result<usize, String> {
    let source = source.to_str().ok_or("Emplacement de fichier invalide.")?;
    conn.execute("ATTACH DATABASE ?1 AS backup KEY ?2", params![source, passphrase])
        .map_err(|_| "Impossible d'ouvrir la sauvegarde.".to_string())?;
    let restored = conn.query_row("SELECT count(*) FROM backup.children", [], |row| row.get::<_, i64>(0))
        .map_err(|_| "Phrase de passe incorrecte, ou ce fichier n'est pas une sauvegarde de classe.".to_string())
        .and_then(|_| conn.execute(
            "INSERT INTO main.children (name, allows_distribution)
             SELECT name, allows_distribution FROM backup.children WHERE true
             ON CONFLICT(name) DO UPDATE SET allows_distribution = excluded.allows_distribution", []
        ).map_err(db_error));
    let _ = conn.execute("DETACH DATABASE backup", []);
    restored
}

/// Crée un dossier d'export neuf dans le dossier choisi (suffixe « (2) », « (3) »… s'il existe déjà)
/// pour ne jamais mélanger ni écraser un export précédent. Renvoie le nom retenu.
#[tauri::command]
fn prepare_export_folder(destination: String, name: String) -> Result<String, String> {
    let root = absolute_path(&destination)?;
    let name = safe_name(&name)?;
    for attempt in 1..100 {
        let candidate = if attempt == 1 { name.to_owned() } else { format!("{name} ({attempt})") };
        match fs::create_dir(root.join(&candidate)) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("Impossible de créer le dossier d'export à cet emplacement.".into()),
        }
    }
    Err("Trop de dossiers d'export portent déjà ce nom.".into())
}

#[tauri::command]
fn write_export_image(destination: String, image: ExportImage) -> Result<(), String> {
    let root = absolute_path(&destination)?;
    let output = root.join(safe_name(&image.export_folder)?).join(safe_name(&image.child_folder)?).join(safe_name(&image.file_name)?);
    fs::create_dir_all(output.parent().ok_or("Dossier d'export invalide.")?)
        .map_err(|_| "Impossible de créer le dossier de l'élève.".to_string())?;
    let bytes = STANDARD.decode(image.jpeg_base64).map_err(|_| "Image exportée invalide.".to_string())?;
    if bytes.len() > 100_000_000 { return Err("Image trop volumineuse pour l'export.".into()); }
    // create_new : un fichier existant n'est jamais écrasé.
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(&output).map_err(|e| match e.kind() {
        std::io::ErrorKind::AlreadyExists => format!("Le fichier {} existe déjà.", image.file_name),
        _ => "Impossible d'écrire la photo dans le dossier d'export.".to_string(),
    })?;
    file.write_all(&bytes).map_err(|_| "Impossible d'écrire la photo (disque plein ?).".to_string())
}

/// Menu macOS en français (le menu par défaut de Tauri est en anglais : « File, Edit… »).
/// « Quitter » ferme la fenêtre, pour passer par l'avertissement « Quitter sans exporter ? ».
#[cfg(target_os = "macos")]
fn french_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem as P, Submenu};
    let name = "À chacun sa photo";
    let quit = MenuItem::with_id(app, "quitter", format!("Quitter {name}"), true, Some("CmdOrCtrl+Q"))?;
    let app_menu = Submenu::with_items(app, name, true, &[
        &P::hide(app, Some(&format!("Masquer {name}")))?,
        &P::hide_others(app, Some("Masquer les autres"))?,
        &P::show_all(app, Some("Tout afficher"))?,
        &P::separator(app)?,
        &quit,
    ])?;
    let edit = Submenu::with_items(app, "Édition", true, &[
        &P::undo(app, Some("Annuler"))?,
        &P::redo(app, Some("Rétablir"))?,
        &P::separator(app)?,
        &P::cut(app, Some("Couper"))?,
        &P::copy(app, Some("Copier"))?,
        &P::paste(app, Some("Coller"))?,
        &P::select_all(app, Some("Tout sélectionner"))?,
    ])?;
    let window = Submenu::with_items(app, "Fenêtre", true, &[
        &P::minimize(app, Some("Réduire"))?,
        &P::fullscreen(app, Some("Plein écran"))?,
        &P::separator(app)?,
        &P::close_window(app, Some("Fermer la fenêtre"))?,
    ])?;
    Menu::with_items(app, &[&app_menu, &edit, &window])
}

pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "macos")]
    let builder = builder.menu(french_menu).on_menu_event(|app, event| {
        if event.id() == "quitter" {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.close();
            }
        }
    });
    builder
        .manage(RosterDb(Mutex::new(None)))
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            open_roster, open_roster_with_passphrase, archive_legacy_roster, list_children, save_child, update_child, delete_child,
            clear_roster, import_children, backup_roster, restore_roster, prepare_export_folder, write_export_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running À chacun sa photo");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roster(path: &Path, key: &str) -> Connection {
        let conn = Connection::open(path).unwrap();
        conn.pragma_update(None, "key", key).unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        conn
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("a-chacun-sa-photo-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn backup_roundtrip_merges_and_rejects_wrong_passphrase() {
        let dir = scratch("backup");
        let source = roster(&dir.join("a.sqlite"), "device-key-a");
        source.execute("INSERT INTO children (name, allows_distribution) VALUES ('Léa Martin', 1), ('Tom Petit', 0)", []).unwrap();
        let backup = dir.join("classe.sauvegarde");
        export_encrypted(&source, &backup, "phrase secrète").unwrap();

        let target = roster(&dir.join("b.sqlite"), "device-key-b");
        target.execute("INSERT INTO children (name, allows_distribution) VALUES ('tom petit', 1)", []).unwrap();
        assert!(import_encrypted(&target, &backup, "mauvaise phrase").is_err());
        assert_eq!(import_encrypted(&target, &backup, "phrase secrète").unwrap(), 2);
        let rows: Vec<(String, bool)> = target.prepare("SELECT name, allows_distribution FROM children ORDER BY name").unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
        // Le nom existant est conservé ; l'accord de la sauvegarde prévaut.
        assert_eq!(rows, vec![("Léa Martin".into(), true), ("tom petit".into(), false)]);
        // La sauvegarde n'est pas lisible sans phrase de passe.
        let plain = Connection::open(&backup).unwrap();
        assert!(plain.query_row("SELECT count(*) FROM children", [], |r| r.get::<_, i64>(0)).is_err());
        // Windows refuse de supprimer un fichier encore ouvert.
        drop((source, target, plain));
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn passphrase_roster_rejects_wrong_passphrase() {
        let dir = scratch("passphrase");
        let path = dir.join("roster.sqlite");
        let conn = open_encrypted(&path, "le chat dort au soleil").unwrap();
        conn.execute("INSERT INTO children (name, allows_distribution) VALUES ('Léa', 0)", []).unwrap();
        drop(conn);
        assert!(open_encrypted(&path, "mauvaise phrase").is_err());
        let conn = open_encrypted(&path, "le chat dort au soleil").unwrap();
        assert_eq!(conn.query_row("SELECT count(*) FROM children", [], |r| r.get::<_, i64>(0)).unwrap(), 1);
        drop(conn);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn export_never_overwrites() {
        let dir = scratch("export");
        let root = dir.to_str().unwrap().to_owned();
        let first = prepare_export_folder(root.clone(), "Photos".into()).unwrap();
        let second = prepare_export_folder(root.clone(), "Photos".into()).unwrap();
        assert_eq!((first.as_str(), second.as_str()), ("Photos", "Photos (2)"));
        let image = |name: &str| ExportImage { export_folder: first.clone(), child_folder: "Léa".into(), file_name: name.into(), jpeg_base64: STANDARD.encode(b"jpeg") };
        write_export_image(root.clone(), image("001-a.jpg")).unwrap();
        assert!(write_export_image(root.clone(), image("001-a.jpg")).is_err());
        assert!(write_export_image(root.clone(), image("../x.jpg")).is_err());
        assert!(write_export_image("relatif".into(), image("002-a.jpg")).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
