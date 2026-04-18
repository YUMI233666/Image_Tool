pub mod commands;
pub mod core;

pub fn run() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
      commands::batch::list_processors,
      commands::batch::start_batch_job,
      commands::batch::cancel_batch_job,
      commands::batch::open_path_in_system,
    ])
    .run(tauri::generate_context!())
    .expect("failed to run tauri app");
}
