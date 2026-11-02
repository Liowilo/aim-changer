//! Captura pasiva del movimiento crudo del mouse (Raw Input de Windows) para la
//! prueba de 360°. Solo **lee** los counts que envía el mouse; nunca inyecta input,
//! así que es seguro junto a anti-cheats (Vanguard, VAC, etc.).
//!
//! Un hilo dedicado crea una ventana message-only registrada con `RIDEV_INPUTSINK`,
//! que recibe `WM_INPUT` aunque el juego tenga el foco.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::OnceLock;

static RECORDING: AtomicBool = AtomicBool::new(false);
static SUM_X: AtomicI64 = AtomicI64::new(0);
static SUM_Y: AtomicI64 = AtomicI64::new(0);
static EVENTS: AtomicU64 = AtomicU64::new(0);
/// Movimiento inyectado por software (SendInput, macros): se ignora en la medición.
static INJECTED_X: AtomicI64 = AtomicI64::new(0);
static LISTENER: OnceLock<Result<(), String>> = OnceLock::new();

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureStatus {
    pub recording: bool,
    pub counts_x: i64,
    pub counts_y: i64,
    pub events: u64,
}

pub fn status() -> CaptureStatus {
    CaptureStatus {
        recording: RECORDING.load(Ordering::SeqCst),
        counts_x: SUM_X.load(Ordering::SeqCst),
        counts_y: SUM_Y.load(Ordering::SeqCst),
        events: EVENTS.load(Ordering::SeqCst),
    }
}

pub fn start_recording() {
    SUM_X.store(0, Ordering::SeqCst);
    SUM_Y.store(0, Ordering::SeqCst);
    EVENTS.store(0, Ordering::SeqCst);
    INJECTED_X.store(0, Ordering::SeqCst);
    RECORDING.store(true, Ordering::SeqCst);
}

pub fn stop_recording() {
    RECORDING.store(false, Ordering::SeqCst);
}

/// Alterna grabación. Devuelve `true` si quedó grabando.
pub fn toggle() -> bool {
    if RECORDING.load(Ordering::SeqCst) {
        stop_recording();
        false
    } else {
        start_recording();
        true
    }
}

/// `physical = false` cuando el evento no viene de un dispositivo real (hDevice nulo).
fn record(dx: i32, dy: i32, physical: bool) {
    if !RECORDING.load(Ordering::SeqCst) {
        return;
    }
    if !physical {
        INJECTED_X.fetch_add(dx as i64, Ordering::SeqCst);
        return;
    }
    SUM_X.fetch_add(dx as i64, Ordering::SeqCst);
    SUM_Y.fetch_add(dy as i64, Ordering::SeqCst);
    EVENTS.fetch_add(1, Ordering::SeqCst);
}

/// Inicia (una sola vez por proceso) el hilo que escucha Raw Input.
pub fn ensure_listener() -> Result<(), String> {
    LISTENER
        .get_or_init(|| {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::Builder::new()
                .name("raw-input".into())
                .spawn(move || platform::run(tx))
                .map_err(|e| e.to_string())?;
            rx.recv()
                .map_err(|_| "El hilo de captura terminó inesperadamente".to_string())?
        })
        .clone()
}

/// Pitido corto (no bloqueante) para confirmar inicio/fin mientras se está en el juego.
pub fn beep(start: bool) {
    platform::beep(if start { 880 } else { 440 });
}

#[cfg(windows)]
mod platform {
    use std::mem::{size_of, zeroed};
    use std::ptr::{null, null_mut};
    use std::sync::mpsc::Sender;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::Diagnostics::Debug::Beep;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Input::{
        GetRawInputData, RegisterRawInputDevices, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER,
        MOUSE_MOVE_ABSOLUTE, RIDEV_INPUTSINK, RID_INPUT, RIM_TYPEMOUSE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, RegisterClassW,
        TranslateMessage, HWND_MESSAGE, MSG, WM_INPUT, WNDCLASSW,
    };

    unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if msg == WM_INPUT {
            let mut raw: RAWINPUT = unsafe { zeroed() };
            let mut size = size_of::<RAWINPUT>() as u32;
            let n = unsafe {
                GetRawInputData(
                    lparam as _,
                    RID_INPUT,
                    &mut raw as *mut RAWINPUT as *mut _,
                    &mut size,
                    size_of::<RAWINPUTHEADER>() as u32,
                )
            };
            if n != u32::MAX && raw.header.dwType == RIM_TYPEMOUSE {
                let m = unsafe { raw.data.mouse };
                if (m.usFlags as u32) & (MOUSE_MOVE_ABSOLUTE as u32) == 0 {
                    super::record(m.lLastX, m.lLastY, !raw.header.hDevice.is_null());
                }
            }
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    pub fn run(tx: Sender<Result<(), String>>) {
        unsafe {
            let hinst = GetModuleHandleW(null());
            let class: Vec<u16> = "AimChangerRawInput\0".encode_utf16().collect();
            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(wndproc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinst,
                hIcon: null_mut(),
                hCursor: null_mut(),
                hbrBackground: null_mut(),
                lpszMenuName: null(),
                lpszClassName: class.as_ptr(),
            };
            if RegisterClassW(&wc) == 0 {
                let _ = tx.send(Err("No se pudo registrar la clase de ventana".into()));
                return;
            }
            let hwnd = CreateWindowExW(
                0, class.as_ptr(), class.as_ptr(), 0, 0, 0, 0, 0,
                HWND_MESSAGE, null_mut(), hinst, null(),
            );
            if hwnd.is_null() {
                let _ = tx.send(Err("No se pudo crear la ventana de captura".into()));
                return;
            }
            let device = RAWINPUTDEVICE {
                usUsagePage: 0x01, // Generic Desktop
                usUsage: 0x02,     // Mouse
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            };
            if RegisterRawInputDevices(&device, 1, size_of::<RAWINPUTDEVICE>() as u32) == 0 {
                let _ = tx.send(Err("No se pudo registrar Raw Input para el mouse".into()));
                return;
            }
            let _ = tx.send(Ok(()));

            let mut msg: MSG = zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    pub fn beep(freq: u32) {
        std::thread::spawn(move || unsafe {
            Beep(freq, 120);
        });
    }
}

#[cfg(not(windows))]
mod platform {
    use std::sync::mpsc::Sender;
    pub fn run(tx: Sender<Result<(), String>>) {
        let _ = tx.send(Err("La captura de mouse solo está disponible en Windows".into()));
    }
    pub fn beep(_freq: u32) {}
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::mem::size_of;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT,
    };

    fn send_move(dx: i32) {
        let input = INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 {
                mi: MOUSEINPUT { dx, dy: 0, mouseData: 0, dwFlags: MOUSEEVENTF_MOVE, time: 0, dwExtraInfo: 0 },
            },
        };
        unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) };
    }

    /// Mueve el cursor real unos píxeles (y lo regresa); por eso es `#[ignore]`.
    /// Ejecutar con: cargo test capture -- --ignored
    #[test]
    #[ignore]
    fn captures_injected_relative_motion() {
        ensure_listener().unwrap();
        start_recording();
        send_move(37);
        send_move(-12);
        std::thread::sleep(std::time::Duration::from_millis(300));
        stop_recording();
        send_move(-25);
        // La entrada inyectada se separa de la física (que puede variar si el usuario mueve el mouse).
        assert_eq!(INJECTED_X.load(Ordering::SeqCst), 25, "{:?}", status());
    }
}
