// src/layout.rs
// Переключение раскладки клавиатуры через Win32 API

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, GetKeyboardLayoutList, HKL,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, WM_INPUTLANGCHANGEREQUEST,
};

/// Получить список всех установленных раскладок клавиатуры
fn get_layout_list() -> Vec<HKL> {
    unsafe {
        let count = GetKeyboardLayoutList(None) as usize;
        if count == 0 {
            return Vec::new();
        }
        let mut list = vec![HKL::default(); count];
        GetKeyboardLayoutList(Some(&mut list));
        list
    }
}

/// Получить текущую раскладку активного окна
fn get_current_layout() -> HKL {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        let thread_id = GetWindowThreadProcessId(hwnd, None);
        GetKeyboardLayout(thread_id)
    }
}

/// Получить код языка из HKL (нижние 16 бит — LANGID)
fn hkl_to_langid(hkl: HKL) -> u16 {
    (hkl.0 as usize & 0xFFFF) as u16
}

/// Преобразовать LANGID в читаемое название (EN/RU/etc.)
pub fn langid_to_name(langid: u16) -> String {
    // Primary language ID — нижние 10 бит
    let primary = langid & 0x3FF;
    match primary {
        0x09 => "EN".to_string(), // English
        0x19 => "RU".to_string(), // Russian
        0x04 => "ZH".to_string(), // Chinese
        0x07 => "DE".to_string(), // German
        0x0C => "FR".to_string(), // French
        0x0A => "ES".to_string(), // Spanish
        0x11 => "JA".to_string(), // Japanese
        0x12 => "KO".to_string(), // Korean
        0x1D => "SV".to_string(), // Swedish
        0x14 => "PL".to_string(), // Polish
        0x05 => "CS".to_string(), // Czech
        0x22 => "UK".to_string(), // Ukrainian
        _ => format!("{:03X}", primary),
    }
}

/// Переключить на следующую раскладку и вернуть название новой
pub fn switch_to_next_layout() -> String {
    let layouts = get_layout_list();
    if layouts.len() < 2 {
        let current = get_current_layout();
        return langid_to_name(hkl_to_langid(current));
    }

    let current = get_current_layout();
    let current_id = hkl_to_langid(current);

    // Найти текущую и взять следующую
    let next_hkl = layouts
        .iter()
        .position(|&hkl| hkl_to_langid(hkl) == current_id)
        .map(|idx| layouts[(idx + 1) % layouts.len()])
        .unwrap_or(layouts[0]);

    unsafe {
        // Отправляем сообщение о смене языка в активное окно пользователя
        let hwnd = GetForegroundWindow();
        let _ = PostMessageW(
            hwnd,
            WM_INPUTLANGCHANGEREQUEST,
            WPARAM(0),
            LPARAM(next_hkl.0 as isize),
        );
    }

    langid_to_name(hkl_to_langid(next_hkl))
}

/// Получить название текущей раскладки (без переключения)
#[allow(dead_code)]
pub fn get_current_layout_name() -> String {
    let current = get_current_layout();
    langid_to_name(hkl_to_langid(current))
}
