use crate::{
    crypto::{
        decrypt_bytes, decrypt_field, derive_key, encrypt_bytes, encrypt_field, unwrap_base64,
        wrap_base64, AAD_BACKUP, AAD_VERIFIER, BACKUP_HEADER, VERIFIER_PLAINTEXT,
    },
    db::{self, StoredItem},
    error::{err, AppError, AppResult},
    models::{ImportMode, ImportResult, KdfParams},
    state::VaultState,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupEnvelope {
    format: String,
    kdf: KdfParams,
    data: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackupPayload {
    version: u32,
    exported_at: String,
    kdf: KdfParams,
    verifier: String,
    auto_lock_minutes: u64,
    items: Vec<StoredItem>,
}

pub fn export_vault(state: &VaultState) -> AppResult<String> {
    let key = state.ensure_unlocked()?;
    let kdf = state.kdf().ok_or_else(|| AppError::new("Параметры KDF не найдены"))?;
    let conn = state.conn.lock().unwrap();
    let verifier = db::get_meta(&conn, "verifier")?
        .ok_or_else(|| AppError::new("Отсутствует верификатор хранилища"))?;
    let auto_lock_minutes = db::get_meta(&conn, "autoLockMinutes")?
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(5);
    let items = db::query_items(&conn)?;
    drop(conn);

    let payload = BackupPayload {
        version: 1,
        exported_at: Utc::now().to_rfc3339(),
        kdf: kdf.clone(),
        verifier,
        auto_lock_minutes,
        items,
    };
    let json = serde_json::to_vec(&payload).map_err(err)?;
    let encrypted = encrypt_bytes(&key, AAD_BACKUP, &json)?;
    let envelope = BackupEnvelope {
        format: BACKUP_HEADER.to_string(),
        kdf,
        data: encrypted,
    };
    let envelope_json = serde_json::to_vec(&envelope).map_err(err)?;
    let encoded = STANDARD.encode(envelope_json);

    Ok(format!(
        "{}\n{}",
        BACKUP_HEADER,
        wrap_base64(&encoded, 76)
    ))
}

pub fn import_vault(
    state: &VaultState,
    text: &str,
    mode: ImportMode,
    backup_password: Option<String>,
) -> AppResult<ImportResult> {
    let mut lines = text.lines();
    let header = lines
        .next()
        .map(|line| line.trim())
        .ok_or_else(|| AppError::new("Пустой файл бэкапа"))?;
    if header != BACKUP_HEADER {
        return Err(AppError::new("Неизвестный формат бэкапа"));
    }
    let envelope_raw: String = lines.collect();
    let envelope_raw = unwrap_base64(&envelope_raw);
    let envelope: BackupEnvelope =
        serde_json::from_slice(&STANDARD.decode(envelope_raw).map_err(err)?).map_err(err)?;
    if envelope.format != BACKUP_HEADER {
        return Err(AppError::new("Неверная версия формата бэкапа"));
    }

    let backup_key = if let Some(password) = backup_password.filter(|value| !value.is_empty()) {
        derive_key(&password, &envelope.kdf)?
    } else {
        state.ensure_unlocked()?
    };

    let payload_json = decrypt_bytes(&backup_key, AAD_BACKUP, &envelope.data)?;
    let payload: BackupPayload = serde_json::from_slice(&payload_json).map_err(err)?;
    if payload.version != 1 {
        return Err(AppError::new("Неподдерживаемая версия бэкапа"));
    }
    let verifier = decrypt_field(&backup_key, AAD_VERIFIER, &payload.verifier)?;
    if verifier != VERIFIER_PLAINTEXT {
        return Err(AppError::new("Неверный пароль бэкапа"));
    }

    let current_key = state.ensure_unlocked()?;
    let mut reencrypted = Vec::with_capacity(payload.items.len());
    for item in payload.items {
        reencrypted.push(reencrypt_item(&backup_key, &current_key, item)?);
    }

    let conn = state.conn.lock().unwrap();
    let tx = conn.unchecked_transaction().map_err(err)?;
    if mode == ImportMode::Replace {
        db::delete_all_items(&tx)?;
    }
    let mut imported = 0usize;
    let mut skipped = 0usize;
    let mut seen: HashMap<String, ()> = HashMap::new();
    for item in reencrypted {
        if seen.insert(item.id.clone(), ()).is_some() {
            skipped += 1;
            continue;
        }
        db::save_item(&tx, &item)?;
        imported += 1;
    }
    tx.commit().map_err(err)?;
    drop(conn);
    state.touch();

    Ok(ImportResult {
        imported,
        skipped,
        mode,
    })
}

fn reencrypt_item(
    backup_key: &crate::crypto::MasterKey,
    current_key: &crate::crypto::MasterKey,
    item: StoredItem,
) -> AppResult<StoredItem> {
    let id = item.id.clone();
    let field = |name: &str, value: &str| {
        let aad = crate::crypto::field_aad(&id, name);
        let plaintext = decrypt_field(backup_key, &aad, value)?;
        encrypt_field(current_key, &aad, &plaintext)
    };

    Ok(StoredItem {
        id: id.clone(),
        category: item.category,
        title: field("title", &item.title)?,
        service: field("service", &item.service)?,
        username: field("username", &item.username)?,
        password: field("password", &item.password)?,
        totp_secret: field("totpSecret", &item.totp_secret)?,
        note: field("note", &item.note)?,
        favorite: item.favorite,
        tags: field("tags", &item.tags)?,
        created_at: item.created_at,
        updated_at: item.updated_at,
    })
}

pub fn export_to_file(state: &VaultState, path: &str) -> AppResult<()> {
    let contents = export_vault(state)?;
    std::fs::write(path, contents).map_err(err)
}

pub fn import_from_file(
    state: &VaultState,
    path: &str,
    mode: ImportMode,
    backup_password: Option<String>,
) -> AppResult<ImportResult> {
    let contents = std::fs::read_to_string(path).map_err(err)?;
    import_vault(state, &contents, mode, backup_password)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        crypto::{field_aad, random_salt, MasterKey},
        models::{ImportMode, KdfParams},
    };
    use uuid::Uuid;

    struct Fixture {
        state: VaultState,
        path: std::path::PathBuf,
        key: MasterKey,
        password: String,
    }

    fn fixture() -> Fixture {
        let path = std::env::temp_dir().join(format!(
            "voidvault-backup-test-{}.sqlite3",
            Uuid::new_v4()
        ));
        let conn = db::open(&path).unwrap();
        let password = Uuid::new_v4().to_string();
        let kdf = KdfParams {
            algorithm: "argon2id".to_string(),
            memory_kib: 1024,
            iterations: 1,
            parallelism: 1,
            hash_len: 32,
            salt: random_salt(),
        };
        let key = derive_key(&password, &kdf).unwrap();
        let verifier = encrypt_field(&key, AAD_VERIFIER, VERIFIER_PLAINTEXT).unwrap();
        db::set_meta(&conn, "kdf", &serde_json::to_string(&kdf).unwrap()).unwrap();
        db::set_meta(&conn, "verifier", &verifier).unwrap();
        db::set_meta(&conn, "autoLockMinutes", "5").unwrap();
        let state = VaultState::new(conn).unwrap();
        state.unlock(key.clone(), kdf);
        Fixture {
            state,
            path,
            key,
            password,
        }
    }

    fn store_item(fixture: &Fixture, id: &str, title: &str) {
        let encrypt = |field: &str, value: &str| {
            encrypt_field(&fixture.key, &field_aad(id, field), value).unwrap()
        };
        let stored = StoredItem {
            id: id.to_string(),
            category: "login".to_string(),
            title: encrypt("title", title),
            service: encrypt("service", "service.example"),
            username: encrypt("username", "user"),
            password: encrypt("password", "ciphertext"),
            totp_secret: encrypt("totpSecret", ""),
            note: encrypt("note", "note"),
            favorite: false,
            tags: encrypt("tags", "[]"),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let conn = fixture.state.conn.lock().unwrap();
        db::save_item(&conn, &stored).unwrap();
    }

    fn stored_titles(fixture: &Fixture) -> Vec<String> {
        let conn = fixture.state.conn.lock().unwrap();
        db::query_items(&conn)
            .unwrap()
            .into_iter()
            .map(|item| {
                decrypt_field(
                    &fixture.key,
                    &field_aad(&item.id, "title"),
                    &item.title,
                )
                .unwrap()
            })
            .collect()
    }

    fn cleanup(fixture: Fixture) {
        drop(fixture.state);
        let _ = std::fs::remove_file(fixture.path);
    }

    #[test]
    fn export_has_expected_header_and_envelope() {
        let fixture = fixture();
        store_item(&fixture, "id-1", "Запись");
        let backup = export_vault(&fixture.state).unwrap();
        assert!(backup.starts_with(BACKUP_HEADER));
        assert!(backup.lines().count() > 2);
        assert!(!backup.contains("Запись"));
        cleanup(fixture);
    }

    #[test]
    fn import_reencrypts_with_different_master_key() {
        let source = fixture();
        store_item(&source, "id-1", "Переносимая запись");
        let backup = export_vault(&source.state).unwrap();

        let target = fixture();
        let result = import_vault(
            &target.state,
            &backup,
            ImportMode::Merge,
            Some(source.password.clone()),
        )
        .unwrap();
        assert_eq!(result.imported, 1);
        assert_eq!(result.skipped, 0);
        assert_eq!(stored_titles(&target), vec!["Переносимая запись"]);

        // Запись расшифровывается ключом целевого хранилища, не ключом бэкапа.
        let conn = target.state.conn.lock().unwrap();
        let stored = db::query_item(&conn, "id-1").unwrap().unwrap();
        drop(conn);
        let title = decrypt_field(&target.key, &field_aad("id-1", "title"), &stored.title).unwrap();
        assert_eq!(title, "Переносимая запись");
        assert!(decrypt_field(&source.key, &field_aad("id-1", "title"), &stored.title).is_err());

        cleanup(source);
        cleanup(target);
    }

    #[test]
    fn wrong_backup_password_is_rejected() {
        let source = fixture();
        let backup = export_vault(&source.state).unwrap();
        let target = fixture();
        let wrong = Uuid::new_v4().to_string();
        assert!(import_vault(
            &target.state,
            &backup,
            ImportMode::Merge,
            Some(wrong)
        )
        .is_err());
        assert!(stored_titles(&target).is_empty());
        cleanup(source);
        cleanup(target);
    }

    #[test]
    fn merge_keeps_existing_items_and_overwrites_matching_ids() {
        let source = fixture();
        store_item(&source, "shared", "Общая");
        store_item(&source, "new", "Новая");
        let backup = export_vault(&source.state).unwrap();

        let target = fixture();
        store_item(&target, "shared", "Локальная версия");
        let result = import_vault(
            &target.state,
            &backup,
            ImportMode::Merge,
            Some(source.password.clone()),
        )
        .unwrap();
        assert_eq!(result.imported, 2);
        assert_eq!(result.skipped, 0);
        let mut titles = stored_titles(&target);
        titles.sort();
        assert_eq!(titles, vec!["Новая", "Общая"]);
        cleanup(source);
        cleanup(target);
    }

    #[test]
    fn replace_removes_existing_items_before_import() {
        let source = fixture();
        store_item(&source, "incoming", "Из бэкапа");
        let backup = export_vault(&source.state).unwrap();

        let target = fixture();
        store_item(&target, "old", "Устарелая");
        let result = import_vault(
            &target.state,
            &backup,
            ImportMode::Replace,
            Some(source.password.clone()),
        )
        .unwrap();
        assert_eq!(result.imported, 1);
        assert_eq!(stored_titles(&target), vec!["Из бэкапа"]);
        cleanup(source);
        cleanup(target);
    }

    #[test]
    fn malformed_backup_is_rejected_before_decryption() {
        let fixture = fixture();
        assert!(import_vault(&fixture.state, "", ImportMode::Merge, None).is_err());
        assert!(import_vault(
            &fixture.state,
            "NOT A BACKUP\nZm9v",
            ImportMode::Merge,
            None
        )
        .is_err());
        cleanup(fixture);
    }
}
