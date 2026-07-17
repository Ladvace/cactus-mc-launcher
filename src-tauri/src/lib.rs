mod auth;
mod commands;
mod content;
mod error;
mod instance;
mod launch;
mod loader;
mod minecraft;
mod modrinth;
mod paths;
mod board_auth;
mod settings;
mod players;
mod server_ping;
mod snapshot;
mod tunnel;
mod sources;
mod stickers;
mod worlds;

use auth::AccountStore;
use instance::store::InstanceStore;
use launch::{LaunchState, ServerState};
use settings::SettingsStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(InstanceStore::default())
        .manage(SettingsStore::default())
        .manage(LaunchState::default())
        .manage(ServerState::default())
        .manage(AccountStore::default())
        .manage(tunnel::TunnelState::default())
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
            commands::instance_folder,
            commands::get_settings,
            commands::save_settings,
            commands::get_minecraft_versions,
            commands::get_loader_versions,
            commands::launch_instance,
            commands::stop_instance,
            commands::send_server_command,
            commands::is_instance_running,
            commands::read_server_properties,
            commands::write_server_properties,
            commands::list_worlds,
            commands::backup_world,
            commands::delete_world,
            commands::get_local_ip,
            commands::read_ops,
            commands::read_whitelist,
            commands::add_op,
            commands::remove_op,
            commands::add_whitelist,
            commands::remove_whitelist,
            commands::setup_java,
            commands::login_microsoft,
            commands::get_accounts,
            commands::set_active_account,
            commands::remove_account,
            commands::list_sources,
            commands::search_content,
            commands::content_versions,
            commands::install_content,
            commands::list_content,
            commands::set_content_enabled,
            commands::remove_content,
            commands::install_modpack,
            commands::search_stickers,
            commands::download_image,
            commands::content_cache_stats,
            commands::export_setup,
            commands::import_setup,
            commands::board_login,
            commands::publish_setup,
            commands::instance_share_check,
            commands::set_skin,
            commands::get_capes,
            commands::set_cape,
            server_ping::ping_server,
            tunnel::tunnel_start,
            tunnel::tunnel_stop,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
