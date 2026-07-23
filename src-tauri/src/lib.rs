mod achievements;
mod auth;
mod commands;
mod content;
mod error;
mod friends;
mod http;
mod instance;
mod launch;
mod loader;
mod minecraft;
mod modrinth;
mod news;
mod paths;
mod board_auth;
mod settings;
mod players;
mod server_browse;
mod server_ping;
mod servers_dat;
mod snapshot;
mod tunnel;
mod sources;
mod stickers;
mod tuneup;
mod worlds;

use auth::AccountStore;
use instance::store::InstanceStore;
use launch::{LaunchState, ServerState};
use settings::SettingsStore;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ngrok (aws-lc-rs) and reqwest (ring) both pull in rustls 0.23, so no
    // single crypto provider is chosen automatically. Pick one explicitly before
    // any TLS is used, otherwise rustls panics on the first connection.
    let _ = rustls::crypto::ring::default_provider().install_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
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
            commands::is_flatpak,
            commands::get_instance,
            commands::create_instance,
            commands::update_instance,
            commands::delete_instance,
            commands::create_server_from,
            commands::instance_folder,
            commands::set_instance_game_dir,
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
            commands::add_server_to_instance,
            commands::get_achievements,
            commands::backend_base,
            commands::get_news,
            commands::get_friends,
            commands::friend_update,
            commands::get_friend_prefs,
            commands::set_friend_prefs,
            commands::read_ops,
            commands::read_whitelist,
            commands::add_op,
            commands::remove_op,
            commands::add_whitelist,
            commands::remove_whitelist,
            commands::setup_java,
            commands::resolved_java_paths,
            commands::login_microsoft,
            commands::cancel_login,
            commands::get_accounts,
            commands::set_active_account,
            commands::remove_account,
            commands::list_sources,
            commands::search_content,
            commands::get_content_categories,
            commands::content_versions,
            commands::install_content,
            commands::list_content,
            commands::set_content_enabled,
            commands::remove_content,
            commands::install_modpack,
            commands::tuneup_recommend,
            commands::tuneup_apply,
            commands::search_stickers,
            commands::download_image,
            commands::content_cache_stats,
            commands::clear_content_cache,
            commands::reset_app_data,
            commands::get_data_dir,
            commands::set_data_dir,
            commands::export_setup,
            commands::import_setup,
            commands::board_login,
            commands::publish_setup,
            commands::instance_share_check,
            commands::set_skin,
            commands::reset_skin,
            commands::get_capes,
            commands::set_cape,
            server_ping::ping_server,
            server_browse::browse_servers,
            tunnel::tunnel_start,
            tunnel::tunnel_stop,
            tunnel::tunnel_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
