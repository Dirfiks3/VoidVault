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
