// build.rs — embed иконки и версии в .exe (Windows)
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        // Игнорируем ошибку если .ico не найден (PNG тоже ок)
        let _ = res.compile();
    }
}
