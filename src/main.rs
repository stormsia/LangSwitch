// src/main.rs
// LangSwitch — глобальный переключатель раскладки клавиатуры для Windows
// Стиль: KDE Plasma OSD-индикатор

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hook;
mod layout;
mod osd;
mod tray;
mod theme;
mod caret;

use crossbeam_channel::bounded;
use slint::ComponentHandle;
use std::thread;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, PeekMessageW, TranslateMessage, PM_REMOVE, WM_QUIT,
};

fn main() {
    // Инициализация UI
    let osd_window = osd::OsdWindow::new().unwrap();
    let weak_osd = osd_window.as_weak();

    // Применяем Win32-стили при старте: убираем из taskbar, делаем сквозным и always-on-top
    {
        let weak = weak_osd.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(win) = weak.upgrade() {
                osd::init_osd_window(&win);
            }
        });
    }

    // Канал: хук -> обработчик (событие нажатия)
    let (hook_tx, hook_rx) = bounded::<()>(32);

    // Поток 1: Хук клавиатуры (блокирующий message loop)
    thread::Builder::new()
        .name("keyboard-hook".into())
        .spawn(move || {
            hook::run_hook(hook_tx);
        })
        .expect("Failed to spawn hook thread");

    // Поток 2: Обработчик событий хука → переключение языка → уведомление OSD
    thread::Builder::new()
        .name("layout-switcher".into())
        .spawn(move || {
            for () in hook_rx.iter() {
                let new_lang = layout::switch_to_next_layout();
                
                // Отправляем команду обновления в главный поток (Slint)
                let weak = weak_osd.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(win) = weak.upgrade() {
                        osd::show_osd(&win, new_lang);
                    }
                });
            }
        })
        .expect("Failed to spawn layout-switcher thread");

    // Поток 3: Трей (со своим циклом обработки)
    thread::Builder::new()
        .name("tray-icon".into())
        .spawn(move || {
            let tray = tray::AppTray::new();
            unsafe {
                let mut msg = std::mem::zeroed();
                loop {
                    // Читаем события окон (необходимо для работы трея и меню на Windows)
                    if PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
                        if msg.message == WM_QUIT {
                            break;
                        }
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    } else {
                        // Если системных событий нет, проверяем наши (клики по меню)
                        if tray.check_events() {
                            let _ = slint::invoke_from_event_loop(|| {
                                let _ = slint::quit_event_loop();
                            });
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
        })
        .expect("Failed to spawn tray thread");

    // Главный поток: Слушаем события (окно изначально скрыто, покажется при первом хоткее)
    slint::run_event_loop().unwrap();
}
