// src/caret.rs
// Определение позиции текстового курсора (каретки)

use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetForegroundWindow, GetGUIThreadInfo, GetWindowThreadProcessId, GUITHREADINFO,
};

/// Возвращает координаты (X, Y) текстового курсора на экране
/// Если каретка не найдена (или программа рисует её кастомно, как Chromium), возвращает None
pub fn get_caret_pos() -> Option<(i32, i32)> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() {
            return None;
        }

        let thread_id = GetWindowThreadProcessId(hwnd, None);
        if thread_id == 0 {
            return None;
        }

        let mut gti: GUITHREADINFO = std::mem::zeroed();
        gti.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;

        if GetGUIThreadInfo(thread_id, &mut gti).is_ok() {
            // Флаг GUI_CARETBLINKING (0x00000001) или наличие hwndCaret говорит о фокусе каретки
            if !gti.hwndCaret.0.is_null() {
                let mut pt = POINT {
                    x: gti.rcCaret.left,
                    y: gti.rcCaret.bottom, // Используем нижний край каретки
                };

                // Переводим клиентские координаты окна в глобальные координаты экрана
                if ClientToScreen(gti.hwndCaret, &mut pt).into() {
                    return Some((pt.x, pt.y));
                }
            }
        }

        // Если каретка не найдена, используем координаты курсора мыши
        let mut pt = POINT::default();
        if GetCursorPos(&mut pt).is_ok() {
            return Some((pt.x, pt.y));
        }

        None
    }
}
