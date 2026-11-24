mod backup;
mod commands;
mod crypto;
mod db;
mod error;
mod generator;
mod models;
mod state;
mod totp;

use state::VaultState;
use tauri::Manager;

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = app.path().app_data_dir()?;
    let connection = db::open(&data_dir.join("voidvault.sqlite3"))?;
    app.manage(VaultState::new(connection)?);

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::vault_status,
            commands::create_vault,
            commands::unlock_vault,
            commands::lock_vault,
            commands::set_auto_lock,
            commands::touch_vault,
            commands::list_items,
            commands::get_item,
            commands::save_item,
            commands::delete_item,
            commands::toggle_favorite,
            commands::generate_password,
            commands::generate_passphrase,
            commands::copy_secret,
            commands::clear_clipboard,
            commands::authorize_backup,
            commands::export_backup,
            commands::import_backup,
            commands::export_backup_file,
            commands::import_backup_file,
            commands::otpauth_uri,
        ])
        .run(tauri::generate_context!())
        .expect("error while running VoidVault");
}
