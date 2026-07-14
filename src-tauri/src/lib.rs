mod auth;
mod commands;
mod error;
mod instance;
mod launch;
mod loader;
mod minecraft;
mod paths;
mod settings;

use auth::AccountStore;
use instance::store::InstanceStore;
use launch::LaunchState;
use settings::SettingsStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(InstanceStore::default())
        .manage(SettingsStore::default())
        .manage(LaunchState::default())
        .manage(AccountStore::default())
        .setup(|app| {
            let handle = app.handle().clone();
            app.state::<InstanceStore>().load(&handle)?;
            app.state::<SettingsStore>().load(&handle)?;
            app.state::<AccountStore>().load(&handle)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_instances,
            commands::get_instance,
            commands::create_instance,
            commands::update_instance,
            commands::delete_instance,
            commands::get_settings,
            commands::save_settings,
            commands::get_minecraft_versions,
            commands::get_loader_versions,
            commands::launch_instance,
            commands::stop_instance,
            commands::is_instance_running,
            commands::setup_java,
            commands::login_microsoft,
            commands::get_accounts,
            commands::set_active_account,
            commands::remove_account,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
