//! Windows-only dependency/interface smoke check (without a GUI running).
#[allow(dead_code)]
pub fn compile_interfaces() {
    use tray_icon::{TrayIconBuilder, Icon, menu::{Menu, MenuItem, MenuEvent}};
    let menu = Menu::new();
    let open = MenuItem::with_id("open", "Open", true, None);
    let _ = menu.append(&open);
    let icon = Icon::from_rgba(vec![0u8; 16 * 16 * 4], 16, 16).expect("valid icon buffer");
    let _builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Meeting Recorder")
        .with_icon(icon);
    let _receiver = MenuEvent::receiver();
    let _key = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
}
