use crate::{
    crypto::MasterKey,
    db,
    error::{AppError, AppResult},
    models::KdfParams,
};
use rusqlite::Connection;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, RwLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

const DEFAULT_AUTO_LOCK_MINUTES: u64 = 5;

pub struct VaultState {
    pub conn: Mutex<Connection>,
    key: RwLock<Option<MasterKey>>,
    kdf: RwLock<Option<KdfParams>>,
    last_activity: AtomicU64,
    auto_lock_ms: AtomicU64,
    backup_authorized_until: AtomicU64,
    pub clipboard_token: AtomicU64,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl VaultState {
    pub fn new(conn: Connection) -> AppResult<Self> {
        let auto_lock_minutes = db::get_meta(&conn, "autoLockMinutes")?
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|minutes| [1, 5, 15].contains(minutes))
            .unwrap_or(DEFAULT_AUTO_LOCK_MINUTES);

        let kdf = db::get_meta(&conn, "kdf")?
            .and_then(|value| serde_json::from_str::<KdfParams>(&value).ok());

        Ok(Self {
            conn: Mutex::new(conn),
            key: RwLock::new(None),
            kdf: RwLock::new(kdf),
            last_activity: AtomicU64::new(0),
            auto_lock_ms: AtomicU64::new(auto_lock_minutes * 60 * 1000),
            backup_authorized_until: AtomicU64::new(0),
            clipboard_token: AtomicU64::new(0),
        })
    }

    pub fn kdf(&self) -> Option<KdfParams> {
        self.kdf.read().unwrap().clone()
    }

    pub fn has_kdf(&self) -> bool {
        self.kdf.read().unwrap().is_some()
    }

    pub fn is_unlocked(&self) -> bool {
        self.key.read().unwrap().is_some()
    }

    pub fn unlock(&self, key: MasterKey, kdf: KdfParams) {
        *self.kdf.write().unwrap() = Some(kdf);
        *self.key.write().unwrap() = Some(key);
        self.touch();
    }

    pub fn lock(&self) {
        *self.key.write().unwrap() = None;
        self.backup_authorized_until.store(0, Ordering::SeqCst);
        self.last_activity.store(0, Ordering::SeqCst);
        self.clipboard_token.fetch_add(1, Ordering::SeqCst);
    }

    pub fn touch(&self) {
        self.last_activity.store(now_ms(), Ordering::SeqCst);
    }

    pub fn auto_lock_ms(&self) -> u64 {
        self.auto_lock_ms.load(Ordering::SeqCst)
    }

    pub fn auto_lock_minutes(&self) -> u64 {
        self.auto_lock_ms() / 60_000
    }

    pub fn set_auto_lock_minutes(&self, minutes: u64) -> AppResult<()> {
        if ![1, 5, 15].contains(&minutes) {
            return Err(AppError::new("Допустимые значения: 1, 5 или 15 минут"));
        }
        let conn = self.conn.lock().unwrap();
        db::set_meta(&conn, "autoLockMinutes", &minutes.to_string())?;
        drop(conn);
        self.auto_lock_ms
            .store(minutes * 60 * 1000, Ordering::SeqCst);
        self.touch();
        Ok(())
    }

    pub fn authorize_backup(&self, seconds: u64) {
        self.backup_authorized_until
            .store(now_ms() + seconds * 1000, Ordering::SeqCst);
        self.touch();
    }

    pub fn ensure_backup_authorized(&self) -> AppResult<()> {
        self.ensure_unlocked()?;
        let until = self.backup_authorized_until.load(Ordering::SeqCst);
        if until == 0 || now_ms() > until {
            return Err(AppError::new(
                "Подтвердите мастер-пароль для работы с бэкапами",
            ));
        }
        self.touch();
        Ok(())
    }

    pub fn ensure_unlocked(&self) -> AppResult<MasterKey> {
        let key = self
            .key
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| AppError::new("Хранилище заблокировано"))?;

        let last = self.last_activity.load(Ordering::SeqCst);
        let timeout = self.auto_lock_ms();
        if last == 0 || now_ms().saturating_sub(last) > timeout {
            drop(key);
            self.lock();
            return Err(AppError::new("Хранилище автоматически заблокировано"));
        }

        self.touch();
        Ok(key)
    }
}
