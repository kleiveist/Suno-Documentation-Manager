#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod application;
mod artwork;
mod certificate;
mod commands;
mod documents;
mod error;
mod evidence;
mod integrity;
mod model;
mod persistence;
mod security;
mod workflow;

fn main() {
    tauri::Builder::default()
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_workflow,
            commands::open_workspace,
            commands::create_workspace,
            commands::scan_workspace,
            commands::get_profile,
            commands::update_profile,
            commands::list_global_evidence,
            commands::import_global_evidence,
            commands::remove_global_evidence,
            commands::attach_global_evidence,
            commands::list_tracks,
            commands::list_albums,
            commands::create_album,
            commands::create_track,
            commands::load_track,
            commands::update_track,
            commands::update_track_library,
            commands::rename_album,
            commands::adopt_legacy_profile,
            commands::add_deviation,
            commands::resolve_deviation,
            commands::remove_deviation,
            commands::set_step_status,
            commands::import_evidence,
            commands::remove_evidence,
            commands::preview_evidence,
            commands::verify_evidence,
            commands::preview_documents,
            commands::generate_documents,
            commands::generate_artwork_disclosure,
            commands::calculate_hashes,
            commands::verify_hashes,
            commands::validate_track,
            commands::finalize_track,
            commands::invalidate_certificate,
            commands::create_revision,
            commands::re_evaluate_track,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Suno Documentation Manager");
}
