pub mod capture;
pub mod conversion;
pub mod games;

use capture::CaptureStatus;
use conversion::{CalibrateRequest, CalibrateResult, ConvertRequest, ConvertResult};
use games::Game;
use tauri::{AppHandle, Emitter};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

/// Tecla global para iniciar/detener la prueba de 360° desde dentro del juego.
const CALIBRATION_KEY: Code = Code::F8;

#[tauri::command]
fn list_games() -> Vec<Game> {
    games::all().to_vec()
}

#[tauri::command]
fn convert(request: ConvertRequest) -> Result<ConvertResult, String> {
    let from = games::find(&request.from_game)
        .ok_or_else(|| format!("Juego desconocido: {}", request.from_game))?;
    let to = games::find(&request.to_game)
        .ok_or_else(|| format!("Juego desconocido: {}", request.to_game))?;
    conversion::convert(from, to, &request)
}

/// Activa/desactiva el modo de prueba: inicia el listener de Raw Input y registra F8.
/// F8 solo queda registrado globalmente mientras la prueba está activa.
#[tauri::command]
fn calibration_arm(app: AppHandle, armed: bool) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    let key = Shortcut::new(None, CALIBRATION_KEY);
    if armed {
        capture::ensure_listener()?;
        if !shortcuts.is_registered(key) {
            shortcuts.register(key).map_err(|e| format!("No se pudo registrar F8: {e}"))?;
        }
    } else {
        capture::stop_recording();
        if shortcuts.is_registered(key) {
            shortcuts.unregister(key).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn capture_toggle() -> Result<CaptureStatus, String> {
    capture::ensure_listener()?;
    let recording = capture::toggle();
    capture::beep(recording);
    Ok(capture::status())
}

#[tauri::command]
fn capture_status() -> CaptureStatus {
    capture::status()
}

#[tauri::command]
fn calibrate(request: CalibrateRequest) -> Result<CalibrateResult, String> {
    let game = match &request.game_id {
        Some(id) => Some(games::find(id).ok_or_else(|| format!("Juego desconocido: {id}"))?),
        None => None,
    };
    conversion::calibrate(game, &request)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed
                        && shortcut.matches(Modifiers::empty(), CALIBRATION_KEY)
                    {
                        let recording = capture::toggle();
                        capture::beep(recording);
                        let _ = app.emit("capture-toggled", capture::status());
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            list_games,
            convert,
            calibration_arm,
            capture_toggle,
            capture_status,
            calibrate
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
