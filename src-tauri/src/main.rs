#![deny(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cal;
mod complex;
mod model;
mod nanovna;
mod store;
mod sweep;

use model::{CalMeta, Calibration, RawTrace, Reading, ReadingMeta, Standards};

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Run blocking work on a worker thread; flatten the join error into `String`.
async fn blocking<T, F>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("worker thread failed: {e}"))?
}

// ---- calibration commands --------------------------------------------

#[tauri::command]
async fn list_cals() -> Result<Vec<CalMeta>, String> {
    blocking(store::list_cals).await
}

#[tauri::command]
async fn get_cal(id: String) -> Result<Calibration, String> {
    blocking(move || store::read_cal(&id)).await
}

#[tauri::command(rename_all = "snake_case")]
async fn sweep_raw(start_hz: u64, stop_hz: u64, points: u32) -> Result<RawTrace, String> {
    blocking(move || nanovna::sweep_raw(start_hz, stop_hz, points)).await
}

#[tauri::command(rename_all = "snake_case")]
#[allow(clippy::too_many_arguments)]
async fn save_calibration(
    name: String,
    notes: String,
    start_hz: u64,
    stop_hz: u64,
    points: u32,
    open: RawTrace,
    short: RawTrace,
    load: RawTrace,
) -> Result<CalMeta, String> {
    blocking(move || {
        let standards = Standards { open, short, load };
        let error_terms = cal::solve_osl(&standards)?;
        let existing: std::collections::HashSet<String> =
            store::list_cals()?.into_iter().map(|m| m.id).collect();
        let id = store::slugify(&name, |s| existing.contains(s));
        let meta = CalMeta {
            id,
            name: name.trim().to_string(),
            notes: notes.trim().to_string(),
            created: now_rfc3339(),
            start_hz,
            stop_hz,
            points,
        };
        let cal = Calibration {
            meta: meta.clone(),
            standards,
            error_terms,
        };
        store::write_cal(&cal)?;
        Ok(meta)
    })
    .await
}

#[tauri::command]
async fn rename_cal(id: String, name: String) -> Result<CalMeta, String> {
    blocking(move || store::rename_cal(&id, &name)).await
}

#[tauri::command]
async fn delete_cal(id: String) -> Result<(), String> {
    blocking(move || store::delete_cal(&id)).await
}

#[tauri::command]
async fn cal_usage(id: String) -> Result<usize, String> {
    blocking(move || store::cal_usage(&id)).await
}

// ---- reading commands ----------------------------------------------

#[tauri::command(rename_all = "snake_case")]
async fn sweep(cal_id: String) -> Result<Reading, String> {
    blocking(move || {
        let cal = store::read_cal(&cal_id)?;
        let raw = nanovna::sweep_raw(cal.meta.start_hz, cal.meta.stop_hz, cal.meta.points)?;
        let corrected = cal::apply(&raw, &cal.error_terms)?;
        let trace = sweep::build_trace(&raw, &corrected);
        let min_swr = sweep::min_swr(&trace);
        let bandwidth = sweep::swr_bandwidth(&trace, 2.0);
        Ok(Reading {
            meta: ReadingMeta {
                id: String::new(),
                name: String::new(),
                notes: String::new(),
                created: now_rfc3339(),
                cal_id: cal.meta.id.clone(),
                cal_name: cal.meta.name.clone(),
                start_hz: cal.meta.start_hz,
                stop_hz: cal.meta.stop_hz,
                points: cal.meta.points,
            },
            trace,
            min_swr,
            bandwidth,
        })
    })
    .await
}

#[tauri::command]
async fn save_reading(
    reading: Reading,
    name: String,
    notes: String,
) -> Result<ReadingMeta, String> {
    blocking(move || {
        let existing: std::collections::HashSet<String> =
            store::list_readings()?.into_iter().map(|m| m.id).collect();
        let id = store::slugify(&name, |s| existing.contains(s));
        let mut r = reading;
        r.meta.id = id;
        r.meta.name = name.trim().to_string();
        r.meta.notes = notes.trim().to_string();
        store::write_reading(&r)
    })
    .await
}

#[tauri::command]
async fn list_readings() -> Result<Vec<ReadingMeta>, String> {
    blocking(store::list_readings).await
}

#[tauri::command]
async fn get_reading(id: String) -> Result<Reading, String> {
    blocking(move || store::read_reading(&id)).await
}

#[tauri::command]
async fn rename_reading(id: String, name: String) -> Result<ReadingMeta, String> {
    blocking(move || store::rename_reading(&id, &name)).await
}

#[tauri::command]
async fn delete_reading(id: String) -> Result<(), String> {
    blocking(move || store::delete_reading(&id)).await
}

#[tauri::command(rename_all = "snake_case")]
async fn export_reading(id: String, dest_path: String) -> Result<(), String> {
    blocking(move || {
        let src = store::home().join("readings").join(format!("{id}.s1p"));
        std::fs::copy(&src, &dest_path)
            .map(|_| ())
            .map_err(|e| format!("copying {} to {dest_path}: {e}", src.display()))
    })
    .await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_cals,
            get_cal,
            sweep_raw,
            save_calibration,
            rename_cal,
            delete_cal,
            cal_usage,
            sweep,
            save_reading,
            list_readings,
            get_reading,
            rename_reading,
            delete_reading,
            export_reading,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vnasweep");
}
