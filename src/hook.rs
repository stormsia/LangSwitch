// src/hook.rs
// Глобальный низкоуровневый перехват клавиатуры через WH_KEYBOARD_LL

use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::OnceLock;
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Foundation::LRESULT;
use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_RCONTROL, VK_RMENU, VK_RSHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;

// Глобальный хэндл хука и канал для отправки событий
// HHOOK содержит *mut c_void — не Sync, храним как AtomicUsize
static HOOK_HANDLE: AtomicUsize = AtomicUsize::new(0);
pub static HOOK_ENABLED: AtomicBool = AtomicBool::new(true);
static SENDER: OnceLock<Sender<()>> = OnceLock::new();

// Состояние нажатых клавиш-модификаторов
static LCTRL_DOWN: AtomicBool = AtomicBool::new(false);
static RCTRL_DOWN: AtomicBool = AtomicBool::new(false);
static LSHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static RSHIFT_DOWN: AtomicBool = AtomicBool::new(false);
static LALT_DOWN: AtomicBool = AtomicBool::new(false);
static RALT_DOWN: AtomicBool = AtomicBool::new(false);

// Флаг чтобы не триггерить несколько раз на одно удержание
static COMBO_FIRED: AtomicBool = AtomicBool::new(false);

unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 || !HOOK_ENABLED.load(Ordering::Relaxed) {
        return CallNextHookEx(None, n_code, w_param, l_param);
    }

    let kb = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
    let vk = kb.vkCode as u16;
    let is_down = w_param.0 as u32 == WM_KEYDOWN || w_param.0 as u32 == WM_SYSKEYDOWN;
    let is_up = !is_down;

    // Обновляем состояние модификаторов
    match vk {
        v if v == VK_LCONTROL.0 => LCTRL_DOWN.store(is_down, Ordering::SeqCst),
        v if v == VK_RCONTROL.0 => RCTRL_DOWN.store(is_down, Ordering::SeqCst),
        v if v == VK_LSHIFT.0 => LSHIFT_DOWN.store(is_down, Ordering::SeqCst),
        v if v == VK_RSHIFT.0 => RSHIFT_DOWN.store(is_down, Ordering::SeqCst),
        v if v == VK_LMENU.0 => LALT_DOWN.store(is_down, Ordering::SeqCst),
        v if v == VK_RMENU.0 => RALT_DOWN.store(is_down, Ordering::SeqCst),
        _ => {}
    }

    let lctrl = LCTRL_DOWN.load(Ordering::SeqCst);
    let rctrl = RCTRL_DOWN.load(Ordering::SeqCst);
    let lshift = LSHIFT_DOWN.load(Ordering::SeqCst);
    let rshift = RSHIFT_DOWN.load(Ordering::SeqCst);
    let lalt = LALT_DOWN.load(Ordering::SeqCst);
    let ralt = RALT_DOWN.load(Ordering::SeqCst);

    // Проверяем комбинации:
    // Ctrl+Shift (одна сторона) или Shift+Alt (одна сторона)
    let combo_ctrl_shift = (lctrl && lshift) || (rctrl && rshift);
    let combo_shift_alt = (lshift && lalt) || (rshift && ralt);
    let combo_active = combo_ctrl_shift || combo_shift_alt;

    if combo_active && !COMBO_FIRED.load(Ordering::SeqCst) {
        COMBO_FIRED.store(true, Ordering::SeqCst);
        if let Some(tx) = SENDER.get() {
            let _ = tx.try_send(());
        }
        // Блокируем стандартное поведение (Win+Space и т.д.)
        return LRESULT(1);
    }

    // Сбрасываем флаг когда все модификаторы отпущены
    if !lctrl && !rctrl && !lshift && !rshift && !lalt && !ralt {
        COMBO_FIRED.store(false, Ordering::SeqCst);
    }

    // Если была активна комбинация — продолжаем блокировать до отпускания
    if is_up && COMBO_FIRED.load(Ordering::SeqCst) {
        // Пересчитаем — уже обновили состояние выше
    }

    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

/// Запустить хук в текущем потоке (блокирующий вызов — нужен message loop)
pub fn run_hook(tx: Sender<()>) {
    SENDER.set(tx).expect("SENDER already initialized");

    unsafe {
        let hmod = GetModuleHandleW(None).expect("GetModuleHandleW failed");
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(low_level_keyboard_proc), hmod, 0)
            .expect("SetWindowsHookExW failed");

        // Сохраняем хэндл как usize — безопасно т.к. используем только в этом потоке
        HOOK_HANDLE.store(hook.0 as usize, Ordering::SeqCst);

        // Message loop — необходим для WH_KEYBOARD_LL
        let mut msg = MSG::default();
        loop {
            let ret = GetMessageW(&mut msg, None, 0, 0);
            if ret.0 <= 0 {
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let raw = HOOK_HANDLE.load(Ordering::SeqCst);
        if raw != 0 {
            let _ = UnhookWindowsHookEx(HHOOK(raw as *mut _));
        }
    }
}
