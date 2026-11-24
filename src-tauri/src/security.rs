use crate::{
    crypto::{decrypt_bytes, derive_key, MasterKey, AAD_VERIFIER, VERIFIER_PLAINTEXT},
    db,
    error::{AppError, AppResult},
    state::VaultState,
};
use serde::Serialize;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

/// Результат оценки пароля. Расчёт полностью выполняется в Rust: frontend
/// получает уже готовые числа и подсказки и не дублирует криптографическую
/// логику на JavaScript.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordAssessment {
    pub entropy_bits: f64,
    pub score: u8,
    pub label: &'static str,
    pub suggestions: Vec<&'static str>,
}

fn is_sequential(window: &[char]) -> bool {
    if window.len() != 3 {
        return false;
    }
    let first = window[0] as i32;
    let second = window[1] as i32;
    let third = window[2] as i32;
    let forward = second - first == 1 && third - second == 1;
    let backward = first - second == 1 && second - third == 1;
    forward || backward
}

fn contains_repeat(password: &str) -> bool {
    password
        .chars()
        .collect::<Vec<_>>()
        .windows(3)
        .any(|window| window[0] == window[1] && window[1] == window[2])
}

fn contains_sequence(password: &str) -> bool {
    password
        .chars()
        .collect::<Vec<_>>()
        .windows(3)
        .any(is_sequential)
}

/// Оценка энтропии с учётом размера алфавита и типичных слабых паттернов.
/// Значение приблизительное, но консервативное: штрафы только уменьшают
/// оценку, никогда не завышают её.
pub fn assess_password(password: &str) -> PasswordAssessment {
    let mut pool = 0usize;
    if password.chars().any(|c| c.is_ascii_lowercase()) {
        pool += 26;
    }
    if password.chars().any(|c| c.is_ascii_uppercase()) {
        pool += 26;
    }
    if password.chars().any(|c| c.is_ascii_digit()) {
        pool += 10;
    }
    if password.chars().any(|c| !c.is_ascii_alphanumeric()) {
        // Оценка для символов намеренно занижена: реальная раскладка дольше.
        pool += 24;
    }

    let length = password.chars().count();
    let raw = if pool == 0 || length == 0 {
        0.0
    } else {
        length as f64 * (pool as f64).log2()
    };

    let repeated = contains_repeat(password);
    let sequenced = contains_sequence(password);
    let digits_only = password.chars().all(|c| c.is_ascii_digit());
    let mut penalty = 0.0;
    if repeated {
        penalty += 4.0;
    }
    if sequenced {
        penalty += 3.0;
    }
    if length < 12 {
        penalty += 5.0;
    }
    if digits_only {
        penalty += 8.0;
    }
    let lowered = password.to_lowercase();
    for pattern in ["qwerty", "abcdef", "123456", "password"] {
        if lowered.contains(pattern) {
            penalty += 6.0;
        }
    }

    let mut entropy_bits = (raw - penalty).max(0.0);
    // Цифровой PIN остаётся слабым независимо от длины: перебор идёт по 10
    // символам, а не по полному алфавиту пароля.
    if digits_only {
        entropy_bits = entropy_bits.min(30.0);
    }
    let (score, label) = if entropy_bits < 28.0 {
        (0, "Очень слабый")
    } else if entropy_bits < 45.0 {
        (1, "Слабый")
    } else if entropy_bits < 65.0 {
        (2, "Достаточный")
    } else if entropy_bits < 90.0 {
        (3, "Сильный")
    } else {
        (4, "Очень сильный")
    };

    let mut suggestions = Vec::new();
    if pool < 62 {
        suggestions.push("Добавьте заглавные буквы, цифры и символы");
    }
    if length < 12 {
        suggestions.push("Увеличьте длину минимум до 12 символов");
    }
    if repeated {
        suggestions.push("Избегайте повторов подряд");
    }
    if sequenced {
        suggestions.push("Избегайте последовательностей");
    }
    if entropy_bits < 45.0 {
        suggestions.push("Используйте случайную фразу из нескольких слов");
    }
    suggestions.truncate(4);

    PasswordAssessment {
        entropy_bits,
        score,
        label,
        suggestions,
    }
}

/// Сравнение верификатора без раннего выхода: время не зависит от того,
/// какой байт совпал первым.
pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    bool::from(left.ct_eq(right))
}

/// Выводит ключ из пароля и проверяет его через зашифрованный верификатор.
/// Возвращает тот же самый ключ, который затем можно положить в состояние —
/// Argon2id выполняется один раз. Расшифрованный верификатор живёт в Zeroizing
/// и никогда не уходит в frontend.
pub fn derive_and_verify_master_key(state: &VaultState, password: &str) -> AppResult<MasterKey> {
    let kdf = state
        .kdf()
        .ok_or_else(|| AppError::new("Хранилище не создано"))?;
    let key = derive_key(password, &kdf)?;
    let verifier = {
        let conn = state.conn.lock().unwrap();
        db::get_meta(&conn, "verifier")?
            .ok_or_else(|| AppError::new("Отсутствует верификатор хранилища"))?
    };
    let plain = Zeroizing::new(
        decrypt_bytes(&key, AAD_VERIFIER, &verifier)
            .map_err(|_| AppError::new("Неверный мастер-пароль"))?,
    );
    if !constant_time_eq(&plain, VERIFIER_PLAINTEXT.as_bytes()) {
        return Err(AppError::new("Неверный мастер-пароль"));
    }
    Ok(key)
}

pub fn verify_master_password(state: &VaultState, password: &str) -> AppResult<()> {
    drop(derive_and_verify_master_key(state, password)?);
    Ok(())
}

#[derive(Debug)]
struct AttemptState {
    failures: u32,
    locked_until: Option<Instant>,
}

/// Ограничитель попыток с экспоненциальной задержкой. Используется и для
/// разблокировки, и для повторного подтверждения перед работами с бэкапами.
pub struct AttemptLimiter {
    state: Mutex<AttemptState>,
    base_delay: Duration,
    max_delay: Duration,
}

impl AttemptLimiter {
    pub fn new(base_delay: Duration, max_delay: Duration) -> Self {
        Self {
            state: Mutex::new(AttemptState {
                failures: 0,
                locked_until: None,
            }),
            base_delay,
            max_delay,
        }
    }

    fn lock(state: &mut AttemptState, base: Duration, max: Duration) {
        let shift = state.failures.saturating_sub(1).min(8);
        let delay = base
            .checked_mul(1u32 << shift)
            .unwrap_or(max)
            .min(max);
        state.locked_until = Some(Instant::now() + delay);
    }

    pub fn ensure_allowed(&self) -> AppResult<()> {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(until) = state.locked_until {
            if Instant::now() < until {
                let seconds = until.duration_since(Instant::now()).as_secs() + 1;
                return Err(AppError::new(format!(
                    "Слишком много неудачных попыток. Повторите через {seconds} с"
                )));
            }
            state.locked_until = None;
        }
        Ok(())
    }

    pub fn record_failure(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.failures = state.failures.saturating_add(1);
        Self::lock(&mut state, self.base_delay, self.max_delay);
    }

    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.failures = 0;
        state.locked_until = None;
    }

    #[allow(dead_code)] // используется в unit-тестах и диагностике
    pub fn failures(&self) -> u32 {
        self.state
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .failures
    }
}

impl Default for AttemptLimiter {
    fn default() -> Self {
        Self::new(Duration::from_millis(400), Duration::from_secs(15 * 60))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_varied_password_scores_higher_than_short_one() {
        let short = assess_password("aa1");
        let long = assess_password("raven-LEDGER-42!quartz");
        assert!(long.entropy_bits > short.entropy_bits);
        assert!(long.score > short.score);
        assert_eq!(long.label, "Очень сильный");
    }

    #[test]
    fn sequences_and_repeats_are_penalized() {
        let clean = assess_password("mQ8!vT2#kL9@rF4");
        let repeated = assess_password("mmmQQQ888!!!");
        let sequenced = assess_password("abcdefghijkl");
        assert!(clean.entropy_bits > repeated.entropy_bits);
        assert!(clean.entropy_bits > sequenced.entropy_bits);
        assert!(repeated.suggestions.iter().any(|item| item.contains("повтор")));
        assert!(sequenced.suggestions.iter().any(|item| item.contains("последователь")));
    }

    #[test]
    fn digits_only_password_is_weak_even_when_long() {
        let assessment = assess_password("012345678901234567890123456789");
        assert!(assessment.score <= 1);
        assert!(assessment.entropy_bits < 45.0);
    }

    #[test]
    fn constant_time_eq_checks_length_and_content() {
        assert!(constant_time_eq(b"voidvault", b"voidvault"));
        assert!(!constant_time_eq(b"voidvault", b"voidvaulT"));
        assert!(!constant_time_eq(b"void", b"voidvault"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn limiter_locks_then_allows_after_cooldown() {
        let limiter = AttemptLimiter::new(Duration::from_millis(20), Duration::from_millis(20));
        assert!(limiter.ensure_allowed().is_ok());
        limiter.record_failure();
        assert!(limiter.ensure_allowed().is_err());
        std::thread::sleep(Duration::from_millis(35));
        assert!(limiter.ensure_allowed().is_ok());
        assert_eq!(limiter.failures(), 1);
    }

    #[test]
    fn limiter_reset_clears_cooldown() {
        let limiter = AttemptLimiter::new(Duration::from_secs(60), Duration::from_secs(60));
        limiter.record_failure();
        assert!(limiter.ensure_allowed().is_err());
        limiter.reset();
        assert!(limiter.ensure_allowed().is_ok());
        assert_eq!(limiter.failures(), 0);
    }
}
