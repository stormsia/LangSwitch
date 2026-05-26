// src/osd.rs
// OSD-окно на базе Slint

use crate::caret::get_caret_pos;
use crate::theme::get_system_theme;
use slint::{ComponentHandle, Timer};
use windows::core::w;
use windows::Win32::Foundation::POINT;
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetWindowLongW, SetWindowLongW, SetWindowPos, ShowWindow, GWL_EXSTYLE, SW_HIDE, SW_SHOWNA,
    WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_EX_APPWINDOW,
    HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE, SWP_NOACTIVATE, SWP_FRAMECHANGED,
};

slint::slint! {
    export component OsdWindow inherits Window {
        title: "LangSwitch_OSD_Hidden";
        width: 220px;
        height: 140px;
        no-frame: true;
        always-on-top: true;
        background: transparent;
        
        in property <string> lang_text: "";
        in property <float> window_opacity: 0.0;
        
        in property <color> sys_bg: rgba(18, 16, 38, 0.85);
        in property <color> sys_accent: rgba(162, 57, 202, 0.7);
        in property <bool> is_light: false;

        Rectangle {
            // Размеры самой плашки
            width: 180px;
            height: 100px;
            x: 20px;
            y: 20px;

            opacity: root.window_opacity;
            // Анимация прозрачности — 1200мс плавного затухания
            animate opacity { duration: 1200ms; easing: ease-out; }
            
            // Тень (блюр вокруг фона)
            drop-shadow-blur: 15px;
            drop-shadow-color: rgba(0, 0, 0, 0.6);
            drop-shadow-offset-y: 4px;
            
            border-radius: 18px;
            background: root.sys_bg;
            border-width: 1.5px;
            border-color: root.sys_accent;
            
            // Внутренняя подсветка
            Rectangle {
                x: 12px;
                y: 4px;
                width: parent.width - 24px;
                height: 2px;
                background: root.sys_accent.with-alpha(0.3);
                border-radius: 1px;
            }

            Text {
                y: parent.height / 2 - 24px;
                text: root.lang_text;
                color: root.is_light ? #202020 : #f0ebff;
                font-size: 52px;
                horizontal-alignment: center;
            }

            Text {
                y: parent.height - 25px;
                text: "Language";
                color: root.sys_accent;
                font-size: 11px;
                horizontal-alignment: center;
            }
        }
    }
}

/// Получение реальных физических границ рабочей области монитора (без панели задач)
fn get_monitor_bounds(x: i32, y: i32) -> (i32, i32, i32, i32) {
    unsafe {
        let hmonitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(hmonitor, &mut mi).into() {
            (mi.rcWork.left, mi.rcWork.top, mi.rcWork.right, mi.rcWork.bottom)
        } else {
            (0, 0, 1920, 1080) // Fallback (никогда не должно произойти)
        }
    }
}

/// Инициализация: применить Win32-стили один раз при старте.
/// Вызвать из главного потока сразу после создания OsdWindow.
pub fn init_osd_window(window: &OsdWindow) {
    // Показываем окно через Slint, чтобы Win32-хэндл был создан
    window.window().show().unwrap();
    // Применяем стили через Win32 (убираем из taskbar, сквозной, always-on-top)
    // и сразу же скрываем через Win32 — НЕ через Slint, чтобы event loop не завершился
    apply_win32_styles_and_hide();
}

/// Показать индикатор языка
pub fn show_osd(window: &OsdWindow, lang: String) {
    // Убеждаемся, что окно показано
    window.window().show().unwrap();
    let theme = get_system_theme();

    let bg_color = if theme.is_light {
        slint::Color::from_argb_u8(215, 240, 240, 245)
    } else {
        slint::Color::from_argb_u8(215, 18, 16, 38)
    };
    let accent_color =
        slint::Color::from_argb_u8(200, theme.accent_r, theme.accent_g, theme.accent_b);

    window.set_sys_bg(bg_color);
    window.set_sys_accent(accent_color);
    window.set_is_light(theme.is_light);
    window.set_lang_text(lang.into());
    window.set_window_opacity(1.0);

    // Установка позиции (каретка или центр мыши)
    let (mut px, mut py) = if let Some((cx, cy)) = get_caret_pos() {
        (cx + 10, cy + 25) // Чуть правее и ниже курсора
    } else {
        (0, 0)
    };

    // Получаем реальный физический размер окна (с учетом DPI масштабирования)
    let win_size = window.window().size();
    let win_w = win_size.width as i32;
    let win_h = win_size.height as i32;

    // Границы текущего монитора
    let (m_left, m_top, m_right, m_bottom) = get_monitor_bounds(px, py);

    // Проверка границ экрана
    if px + win_w > m_right { px = m_right - win_w; }
    if py + win_h > m_bottom { py = m_bottom - win_h; }
    if px < m_left { px = m_left; }
    if py < m_top { py = m_top; }

    // Позиция окна
    window
        .window()
        .set_position(slint::PhysicalPosition::new(px, py));

    // Показываем через Win32 (Slint думает что окно уже показано — мы скрыли его через Win32)
    unsafe {
        if let Ok(hwnd) = FindWindowW(None, w!("LangSwitch_OSD_Hidden")) {
            if !hwnd.0.is_null() {
                let _ = ShowWindow(hwnd, SW_SHOWNA);
            }
        }
    }

    let weak = window.as_weak();
    // Через 600мс ставим opacity=0 (Slint анимирует затухание за 1200мс),
    // а через 1800мс скрываем через Win32, чтобы не было прозрачного оверлея
    Timer::single_shot(std::time::Duration::from_millis(600), move || {
        if let Some(win) = weak.upgrade() {
            win.set_window_opacity(0.0);
            // Скрываем через Win32 после завершения анимации (1200мс)
            let weak2 = win.as_weak();
            Timer::single_shot(std::time::Duration::from_millis(1200), move || {
                if weak2.upgrade().is_some() {
                    unsafe {
                        if let Ok(hwnd) = FindWindowW(None, w!("LangSwitch_OSD_Hidden")) {
                            if !hwnd.0.is_null() {
                                let _ = ShowWindow(hwnd, SW_HIDE);
                            }
                        }
                    }
                }
            });
        }
    });
}

/// Настраивает Win32-стили и скрывает окно через Win32 (не через Slint).
/// Slint не знает о скрытии → event loop продолжает работать.
fn apply_win32_styles_and_hide() {
    unsafe {
        if let Ok(hwnd) = FindWindowW(None, w!("LangSwitch_OSD_Hidden")) {
            if !hwnd.0.is_null() {
                // Скрываем, чтобы применить TOOLWINDOW (требование Windows)
                let _ = ShowWindow(hwnd, SW_HIDE);
                // Устанавливаем стили: убрать из taskbar, сделать сквозным для кликов
                // Сначала убираем флаг APPWINDOW (он заставляет появляться в таскбаре),
                // затем добавляем TOOLWINDOW и TRANSPARENT.
                let mut style = GetWindowLongW(hwnd, GWL_EXSTYLE);
                // Сбрасываем WS_EX_APPWINDOW на случай, если Slint установил его по умолчанию
                style &= !(WS_EX_APPWINDOW.0 as i32);
                // Устанавливаем нужные расширенные стили (Tool window + transparent)
                style |= WS_EX_TRANSPARENT.0 as i32 | WS_EX_TOOLWINDOW.0 as i32;
                SetWindowLongW(hwnd, GWL_EXSTYLE, style);
                // Применяем изменения фрейма/стилей и делаем окно topmost через SetWindowPos
                let _ = SetWindowPos(
                    hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                );
                // Оставляем скрытым — show_osd покажет его при необходимости
                // SW_HIDE уже применён выше, больше ничего не нужно
            }
        }
    }
}
