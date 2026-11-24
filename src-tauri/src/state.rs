use crate::{
    crypto::MasterKey,
    db,
    error::{AppError, AppResult},
    models::KdfParams,
    security::AttemptLimiter,
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
    unlock_limiter: AttemptLimiter,
    backup_limiter: AttemptLimiter,
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
            unlock_limiter: AttemptLimiter::default(),
            backup_limiter: AttemptLimiter::default(),
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

    /// Миллисекунды с последнего подтверждения активности.
    pub fn idle_ms(&self) -> u64 {
        let last = self.last_activity.load(Ordering::SeqCst);
        if last == 0 {
            return u64::MAX;
        }
        now_ms().saturating_sub(last)
    }

    /// Проверяет таймаут в Rust и обнуляет ключ, если пора блокироваться.
    /// Возвращает true только в момент фактической блокировки, чтобы не
    /// засыпать frontend повторяющимися событиями.
    pub fn auto_lock_if_idle(&self) -> bool {
        if !self.is_unlocked() {
            return false;
        }
        if self.idle_ms() >= self.auto_lock_ms() {
            self.lock();
            return true;
        }
        false
    }

    pub fn unlock_limiter(&self) -> &AttemptLimiter {
        &self.unlock_limiter
    }

    pub fn backup_limiter(&self) -> &AttemptLimiter {
        &self.backup_limiter
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
        if until == 0 || now_ms() >= until {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{random_salt, MasterKey};
    use crate::models::KdfParams;
    use uuid::Uuid;

    fn state() -> (VaultState, std::path::PathBuf) {
        let path = std::env::temp_dir().join(format!(
            "voidvault-state-test-{}.sqlite3",
            Uuid::new_v4()
        ));
        let conn = db::open(&path).unwrap();
        let state = VaultState::new(conn).unwrap();
        state.unlock(
            MasterKey::from_bytes(&[1u8; 32]),
            KdfParams::defaults(random_salt()),
        );
        (state, path)
    }

    fn cleanup(path: std::path::PathBuf) {
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn auto_lock_zeroizes_key_and_emits_once() {
        let (state, path) = state();
        assert!(state.is_unlocked());
        state.auto_lock_ms.store(0, Ordering::SeqCst);
        assert!(state.auto_lock_if_idle());
        assert!(!state.is_unlocked());
        assert!(!state.auto_lock_if_idle());
        cleanup(path);
    }

    #[test]
    fn auto_lock_ignores_recent_activity() {
        let (state, path) = state();
        state.touch();
        assert!(!state.auto_lock_if_idle());
        assert!(state.is_unlocked());
        cleanup(path);
    }

    #[test]
    fn auto_lock_minutes_are_validated() {
        let (state, path) = state();
        assert!(state.set_auto_lock_minutes(2).is_err());
        assert!(state.set_auto_lock_minutes(15).is_ok());
        assert_eq!(state.auto_lock_minutes(), 15);
        cleanup(path);
    }

    #[test]
    fn backup_authorization_expires_and_is_revoked_on_lock() {
        let (state, path) = state();
        state.authorize_backup(0);
        assert!(state.ensure_backup_authorized().is_err());

        state.authorize_backup(60);
        assert!(state.ensure_backup_authorized().is_ok());

        state.lock();
        assert!(state.ensure_backup_authorized().is_err());
        cleanup(path);
    }

    #[test]
    fn ensure_unlocked_enforces_timeout() {
        let (state, path) = state();
        state.auto_lock_ms.store(0, Ordering::SeqCst);
        state.last_activity.store(now_ms() - 10, Ordering::SeqCst);
        assert!(state.ensure_unlocked().is_err());
        assert!(!state.is_unlocked());
        cleanup(path);
    }
}
