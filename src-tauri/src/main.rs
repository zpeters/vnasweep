#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod nanovna;

use nanovna::BoardInfo;

/// Read the attached NanoVNA's board identity. Runs the blocking serial work on
/// a worker thread so the UI stays responsive.
#[tauri::command]
async fn board_info() -> Result<BoardInfo, String> {
    tauri::async_runtime::spawn_blocking(nanovna::board_info)
        .await
        .map_err(|e| format!("worker thread failed: {e}"))?
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![board_info])
        .run(tauri::generate_context!())
        .expect("error while running vnasweep");
}
