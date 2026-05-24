// src/tray.rs
// Иконка в системном трее с контекстным меню

use std::sync::atomic::Ordering;
use tray_icon::{
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder,
};

pub struct AppTray {
    _tray: TrayIcon,
    toggle_id: tray_icon::menu::MenuId,
    quit_id: tray_icon::menu::MenuId,
}

impl AppTray {
    pub fn new() -> Self {
        let icon = load_icon();

        let tray_menu = Menu::new();
        
        let toggle_item = CheckMenuItem::new("Intercept keys", true, true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        let _ = tray_menu.append(&toggle_item);
        let _ = tray_menu.append(&PredefinedMenuItem::separator());
        let _ = tray_menu.append(&quit_item);

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("LangSwitch")
            .with_icon(icon)
            .build()
            .expect("Failed to build tray icon");

        AppTray { 
            _tray: tray,
            toggle_id: toggle_item.id().clone(),
            quit_id: quit_item.id().clone(),
        }
    }

    /// Проверяем, нажал ли пользователь Quit, и заодно обрабатываем toggle
    pub fn check_events(&self) -> bool {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.quit_id {
                return true;
            }
            if event.id == self.toggle_id {
                // Переключаем глобальный флаг
                let current = crate::hook::HOOK_ENABLED.load(Ordering::Relaxed);
                crate::hook::HOOK_ENABLED.store(!current, Ordering::Relaxed);
            }
        }
        false
    }
}

fn load_icon() -> tray_icon::Icon {
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes)
        .expect("Failed to load icon")
        .into_rgba8();
    let (w, h) = img.dimensions();
    tray_icon::Icon::from_rgba(img.into_raw(), w, h).expect("Failed to create tray icon")
}
