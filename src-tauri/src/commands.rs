use crate::{
    backup, crypto, db,
    error::{err, AppError, AppResult},
    generator, security,
    totp,
    vault_query::{self, SortMode},
    models::{
        Category, ClipboardStatus, GeneratedSecret, ImportMode, ImportResult, ItemFields,
        ItemInput, ItemSummary, KdfParams, PassphraseOptions, PasswordOptions, VaultItem,
        VaultStatus,
    },
    state::VaultState,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use uuid::Uuid;
use zeroize::Zeroizing;

#[tauri::command]
pub fn vault_status(state: State<'_, VaultState>) -> AppResult<VaultStatus> {
    let conn = state.conn.lock().unwrap();
    let exists = db::get_meta(&conn, "kdf")?.is_some();
    let item_count = db::query_items(&conn)?.len();
    drop(conn);
    Ok(VaultStatus {
        exists,
        unlocked: state.is_unlocked(),
        auto_lock_minutes: state.auto_lock_minutes(),
        item_count,
        kdf: state.kdf(),
    })
}

#[tauri::command]
pub fn create_vault(
    state: State<'_, VaultState>,
    master_password: String,
) -> AppResult<VaultStatus> {
    if master_password.len() < 8 {
        return Err(AppError::new("Мастер-пароль должен содержать минимум 8 символов"));
    }
    if state.has_kdf() {
        return Err(AppError::new("Хранилище уже существует"));
    }

    let kdf = KdfParams::defaults(crypto::random_salt());
    let key = crypto::derive_key(&master_password, &kdf)?;
    let verifier = crypto::encrypt_field(&key, crypto::AAD_VERIFIER, crypto::VERIFIER_PLAINTEXT)?;

    {
        let conn = state.conn.lock().unwrap();
        let tx = conn.unchecked_transaction().map_err(err)?;
        db::set_meta(&tx, "schemaVersion", db::SCHEMA_VERSION)?;
        db::set_meta(&tx, "kdf", &serde_json::to_string(&kdf).map_err(err)?)?;
        db::set_meta(&tx, "verifier", &verifier)?;
        db::set_meta(&tx, "autoLockMinutes", "5")?;
        tx.commit().map_err(err)?;
    }

    state.unlock(key, kdf);
    vault_status(state)
}

#[tauri::command]
pub fn unlock_vault(
    state: State<'_, VaultState>,
    master_password: String,
) -> AppResult<VaultStatus> {
    state.unlock_limiter().ensure_allowed()?;

    let key = match security::derive_and_verify_master_key(&state, &master_password) {
        Ok(key) => key,
        Err(error) => {
            state.unlock_limiter().record_failure();
            return Err(error);
        }
    };
    let kdf = state
        .kdf()
        .ok_or_else(|| AppError::new("Хранилище не создано"))?;
    state.unlock_limiter().reset();
    state.unlock(key, kdf);
    vault_status(state)
}

#[tauri::command]
pub fn lock_vault(app: AppHandle, state: State<'_, VaultState>) -> AppResult<VaultStatus> {
    state.lock();
    if let Err(error) = app.clipboard().clear() {
        eprintln!("clipboard clear failed: {error}");
    }
    vault_status(state)
}

#[tauri::command]
pub fn set_auto_lock(
    state: State<'_, VaultState>,
    minutes: u64,
) -> AppResult<VaultStatus> {
    state.set_auto_lock_minutes(minutes)?;
    vault_status(state)
}

#[tauri::command]
pub fn touch_vault(state: State<'_, VaultState>) -> AppResult<()> {
    state.ensure_unlocked()?;
    Ok(())
}

fn decrypt_tags(key: &crypto::MasterKey, item_id: &str, blob: &str) -> AppResult<Vec<String>> {
    let aad = crypto::field_aad(item_id, "tags");
    let json = crypto::decrypt_field(key, &aad, blob)?;
    if json.is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&json).map_err(err)
}

fn stored_to_summary(
    key: &crypto::MasterKey,
    stored: &db::StoredItem,
) -> AppResult<ItemSummary> {
    let title = crypto::decrypt_field(key, &crypto::field_aad(&stored.id, "title"), &stored.title)?;
    let service =
        crypto::decrypt_field(key, &crypto::field_aad(&stored.id, "service"), &stored.service)?;
    let username =
        crypto::decrypt_field(key, &crypto::field_aad(&stored.id, "username"), &stored.username)?;
    let password =
        crypto::decrypt_field(key, &crypto::field_aad(&stored.id, "password"), &stored.password)?;
    let totp_secret = crypto::decrypt_field(
        key,
        &crypto::field_aad(&stored.id, "totpSecret"),
        &stored.totp_secret,
    )?;
    let tags = decrypt_tags(key, &stored.id, &stored.tags)?;

    let totp = if totp_secret.is_empty() {
        None
    } else {
        totp::generate(&totp_secret).ok()
    };

    Ok(ItemSummary {
        id: stored.id.clone(),
        category: Category::parse(&stored.category)?,
        title,
        service,
        username,
        favorite: stored.favorite,
        tags,
        has_password: !password.is_empty(),
        has_totp: !totp_secret.is_empty(),
        totp,
        created_at: stored.created_at.clone(),
        updated_at: stored.updated_at.clone(),
    })
}

#[tauri::command]
pub fn list_items(
    state: State<'_, VaultState>,
    query: Option<String>,
    category: Option<Category>,
    favorites_only: Option<bool>,
    sort: Option<SortMode>,
) -> AppResult<Vec<ItemSummary>> {
    let key = state.ensure_unlocked()?;
    let conn = state.conn.lock().unwrap();
    let stored_items = db::query_items(&conn)?;
    drop(conn);

    let mut summaries = Vec::with_capacity(stored_items.len());
    for stored in &stored_items {
        summaries.push(stored_to_summary(&key, stored)?);
    }

    let filter = vault_query::ItemFilter::new(query, category, favorites_only);
    Ok(vault_query::apply_filter(
        summaries,
        &filter,
        sort.unwrap_or_default(),
    ))
}

#[tauri::command]
pub fn get_item(state: State<'_, VaultState>, id: String) -> AppResult<VaultItem> {
    let key = state.ensure_unlocked()?;
    let conn = state.conn.lock().unwrap();
    let stored = db::query_item(&conn, &id)?
        .ok_or_else(|| AppError::new("Запись не найдена"))?;
    drop(conn);

    let title = crypto::decrypt_field(&key, &crypto::field_aad(&id, "title"), &stored.title)?;
    let service = crypto::decrypt_field(&key, &crypto::field_aad(&id, "service"), &stored.service)?;
    let username =
        crypto::decrypt_field(&key, &crypto::field_aad(&id, "username"), &stored.username)?;
    let password = crypto::decrypt_field(&key, &crypto::field_aad(&id, "password"), &stored.password)?;
    let totp_secret =
        crypto::decrypt_field(&key, &crypto::field_aad(&id, "totpSecret"), &stored.totp_secret)?;
    let note = crypto::decrypt_field(&key, &crypto::field_aad(&id, "note"), &stored.note)?;
    let tags = decrypt_tags(&key, &id, &stored.tags)?;

    Ok(VaultItem {
        id: stored.id,
        created_at: stored.created_at,
        updated_at: stored.updated_at,
        fields: ItemFields {
            category: Category::parse(&stored.category)?,
            title,
            service,
            username,
            password,
            totp_secret,
            note,
            favorite: stored.favorite,
            tags,
        },
    })
}

#[tauri::command]
pub fn save_item(
    state: State<'_, VaultState>,
    payload: ItemInput,
) -> AppResult<String> {
    let key = state.ensure_unlocked()?;
    let id = payload.id.filter(|value| !value.is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = Utc::now().to_rfc3339();
    let conn = state.conn.lock().unwrap();
    let previous = db::query_item(&conn, &id)?;
    let created_at = previous
        .as_ref()
        .map(|item| item.created_at.clone())
        .unwrap_or_else(|| now.clone());
    drop(conn);

    let encrypt = |name: &str, value: &str| {
        crypto::encrypt_field(&key, &crypto::field_aad(&id, name), value)
    };
    let tags_json = serde_json::to_string(&payload.fields.tags).map_err(err)?;
    let stored = db::StoredItem {
        id: id.clone(),
        category: payload.fields.category.as_str().to_string(),
        title: encrypt("title", &payload.fields.title)?,
        service: encrypt("service", &payload.fields.service)?,
        username: encrypt("username", &payload.fields.username)?,
        password: encrypt("password", &payload.fields.password)?,
        totp_secret: encrypt("totpSecret", &payload.fields.totp_secret)?,
        note: encrypt("note", &payload.fields.note)?,
        favorite: payload.fields.favorite,
        tags: encrypt("tags", &tags_json)?,
        created_at,
        updated_at: now,
    };

    let conn = state.conn.lock().unwrap();
    db::save_item(&conn, &stored)?;
    drop(conn);
    state.touch();
    Ok(id)
}

#[tauri::command]
pub fn delete_item(state: State<'_, VaultState>, id: String) -> AppResult<()> {
    state.ensure_unlocked()?;
    let conn = state.conn.lock().unwrap();
    db::delete_item(&conn, &id)?;
    drop(conn);
    state.touch();
    Ok(())
}

#[tauri::command]
pub fn toggle_favorite(state: State<'_, VaultState>, id: String) -> AppResult<()> {
    state.ensure_unlocked()?;
    let conn = state.conn.lock().unwrap();
    let mut stored = db::query_item(&conn, &id)?
        .ok_or_else(|| AppError::new("Запись не найдена"))?;
    stored.favorite = !stored.favorite;
    stored.updated_at = Utc::now().to_rfc3339();
    db::save_item(&conn, &stored)?;
    drop(conn);
    state.touch();
    Ok(())
}

#[tauri::command]
pub fn password_strength(password: String) -> security::PasswordAssessment {
    security::assess_password(&password)
}

#[tauri::command]
pub fn generate_password(options: PasswordOptions) -> AppResult<GeneratedSecret> {
    generator::generate_password(&options)
}

#[tauri::command]
pub fn generate_passphrase(options: PassphraseOptions) -> AppResult<GeneratedSecret> {
    generator::generate_passphrase(&options)
}

#[tauri::command]
pub fn copy_secret(
    app: AppHandle,
    state: State<'_, VaultState>,
    text: String,
) -> AppResult<ClipboardStatus> {
    state.ensure_unlocked()?;
    let secret = Zeroizing::new(text);
    if secret.is_empty() {
        return Err(AppError::new("Нечего копировать"));
    }
    let token = state.clipboard_token.fetch_add(1, Ordering::SeqCst) + 1;
    app.clipboard()
        .write_secret(&*secret)
        .map_err(|error| AppError::new(format!("Не удалось скопировать: {error}")))?;

    let app_handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(30));
        let current = app_handle
            .state::<VaultState>()
            .clipboard_token
            .load(Ordering::SeqCst);
        if current == token {
            let _ = app_handle.clipboard().clear();
            let _ = app_handle.emit("clipboard-cleared", ());
        }
    });

    Ok(ClipboardStatus {
        expires_in_seconds: 30,
    })
}

#[tauri::command]
pub fn clear_clipboard(app: AppHandle, state: State<'_, VaultState>) -> AppResult<()> {
    state.clipboard_token.fetch_add(1, Ordering::SeqCst);
    app.clipboard().clear().map_err(err)?;
    Ok(())
}

#[tauri::command]
pub fn authorize_backup(
    state: State<'_, VaultState>,
    master_password: String,
) -> AppResult<u64> {
    const WINDOW_SECONDS: u64 = 120;

    state.backup_limiter().ensure_allowed()?;
    if let Err(error) = security::verify_master_password(&state, &master_password) {
        state.backup_limiter().record_failure();
        return Err(error);
    }
    state.backup_limiter().reset();
    state.authorize_backup(WINDOW_SECONDS);
    Ok(WINDOW_SECONDS)
}

#[tauri::command]
pub fn export_backup(state: State<'_, VaultState>) -> AppResult<String> {
    state.ensure_backup_authorized()?;
    backup::export_vault(&state)
}

#[tauri::command]
pub fn import_backup(
    state: State<'_, VaultState>,
    backup_text: String,
    mode: ImportMode,
    backup_password: Option<String>,
) -> AppResult<ImportResult> {
    state.ensure_backup_authorized()?;
    backup::import_vault(&state, &backup_text, mode, backup_password)
}

#[tauri::command]
pub fn export_backup_file(
    state: State<'_, VaultState>,
    path: String,
) -> AppResult<()> {
    state.ensure_backup_authorized()?;
    backup::export_to_file(&state, &path)
}

#[tauri::command]
pub fn import_backup_file(
    state: State<'_, VaultState>,
    path: String,
    mode: ImportMode,
    backup_password: Option<String>,
) -> AppResult<ImportResult> {
    state.ensure_backup_authorized()?;
    backup::import_from_file(&state, &path, mode, backup_password)
}

#[tauri::command]
pub fn otpauth_uri(state: State<'_, VaultState>, id: String) -> AppResult<String> {
    let key = state.ensure_unlocked()?;
    let conn = state.conn.lock().unwrap();
    let stored = db::query_item(&conn, &id)?
        .ok_or_else(|| AppError::new("Запись не найдена"))?;
    drop(conn);
    let secret = crypto::decrypt_field(
        &key,
        &crypto::field_aad(&id, "totpSecret"),
        &stored.totp_secret,
    )?;
    let title = crypto::decrypt_field(&key, &crypto::field_aad(&id, "title"), &stored.title)?;
    let encoded_secret = STANDARD.encode(secret.as_bytes());
    let label = urlencode(&format!("VoidVault:{title}"));
    Ok(format!(
        "otpauth://totp/{label}?secret={encoded_secret}&issuer=VoidVault&algorithm=SHA1&digits=6&period=30"
    ))
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
