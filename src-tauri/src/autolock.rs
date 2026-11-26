use crate::state::VaultState;
use std::{
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_clipboard_manager::ClipboardExt;

/// Событие посылается frontend только после того, как Rust уже обнулил ключ.
/// Даже если JavaScript скомпрометирован, секрет к этому моменту недоступен.
pub const AUTO_LOCK_EVENT: &str = "vault-auto-locked";

/// Запускает фоновый таймер, который проверяет простой независимо от вызовов
/// интерфейса. Раньше автоблокировка срабатывала только когда frontend сам
/// дёргал IPC; теперь граница находится в Rust.
pub fn spawn_watcher(app: AppHandle, interval: Duration) {
    let name = "voidvault-autolock";
    let result = thread::Builder::new().name(name.to_string()).spawn(move || loop {
        thread::sleep(interval);
        let state = app.state::<VaultState>();
        if state.auto_lock_if_idle() {
            // Ключ уже обнулён в state.lock(); дополнительно убираем секрет,
            // который мог остаться в системном буфере после копирования.
            let _ = app.clipboard().clear();
            let _ = app.emit(AUTO_LOCK_EVENT, ());
        }
    });

    if let Err(error) = result {
        eprintln!("failed to start auto-lock watcher: {error}");
    }
}
