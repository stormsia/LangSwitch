// src/theme.rs
// Получение системных цветов Windows из реестра

use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

#[derive(Clone, Copy)]
pub struct SystemTheme {
    pub is_light: bool,
    pub accent_r: u8,
    pub accent_g: u8,
    pub accent_b: u8,
}

pub fn get_system_theme() -> SystemTheme {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);

    // 1. Читаем светлую/тёмную тему
    let mut is_light = false;
    if let Ok(personalize) =
        hkcu.open_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
    {
        if let Ok(apps_use_light) = personalize.get_value::<u32, _>("AppsUseLightTheme") {
            is_light = apps_use_light == 1;
        }
    }

    // 2. Читаем акцентный цвет (DWM ColorizationColor)
    // Формат: 0xAARRGGBB
    let mut accent_r = 162;
    let mut accent_g = 57;
    let mut accent_b = 202; // По умолчанию фиолетовый

    if let Ok(dwm) = hkcu.open_subkey("Software\\Microsoft\\Windows\\DWM") {
        if let Ok(color) = dwm.get_value::<u32, _>("ColorizationColor") {
            accent_r = ((color >> 16) & 0xFF) as u8;
            accent_g = ((color >> 8) & 0xFF) as u8;
            accent_b = (color & 0xFF) as u8;
        }
    }

    SystemTheme {
        is_light,
        accent_r,
        accent_g,
        accent_b,
    }
}
