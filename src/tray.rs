use crate::battery::{BatterySnapshot, DetectedDevice};
use crate::icon;
use tray_icon::menu::{
    CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu,
};
use tray_icon::{TrayIcon, TrayIconBuilder};

pub const MENU_REFRESH: &str = "refresh";
pub const MENU_FIRMWARE: &str = "firmware";
pub const MENU_AUTOSTART: &str = "autostart";
pub const MENU_EXIT: &str = "exit";
pub const DEVICE_PREFIX: &str = "device:";

pub struct Tray {
    tray_icon: TrayIcon,
}

impl Tray {
    pub fn new(
        snapshot: &BatterySnapshot,
        autostart_enabled: bool,
        status_override: Option<&str>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let menu = build_menu(snapshot, autostart_enabled, status_override)?;
        let icon = icon::create_text_icon(&snapshot.status.icon_text())?;
        let tray_icon = TrayIconBuilder::new()
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_tooltip(snapshot.status.tooltip())
            .with_menu_on_left_click(false)
            .with_menu_on_right_click(true)
            .build()?;

        Ok(Self { tray_icon })
    }

    pub fn update(
        &self,
        snapshot: &BatterySnapshot,
        autostart_enabled: bool,
        status_override: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let icon = icon::create_text_icon(&snapshot.status.icon_text())?;
        let menu = build_menu(snapshot, autostart_enabled, status_override)?;
        self.tray_icon.set_icon(Some(icon))?;
        self.tray_icon
            .set_tooltip(Some(snapshot.status.tooltip()))?;
        self.tray_icon.set_menu(Some(Box::new(menu)));
        Ok(())
    }
}

pub fn menu_id(event: &MenuEvent) -> &str {
    event.id().as_ref()
}

pub fn device_key_from_menu_id(id: &str) -> Option<&str> {
    id.strip_prefix(DEVICE_PREFIX)
}

fn build_menu(
    snapshot: &BatterySnapshot,
    autostart_enabled: bool,
    status_override: Option<&str>,
) -> Result<Menu, Box<dyn std::error::Error>> {
    let menu = Menu::new();
    let status_text = status_override
        .map(str::to_string)
        .unwrap_or_else(|| snapshot.status.tooltip());
    let status_item = MenuItem::with_id(MenuId::new("status"), status_text, false, None);
    let devices_menu = build_devices_menu(&snapshot.devices, snapshot.selected_key.as_deref())?;
    let refresh_item = MenuItem::with_id(MenuId::new(MENU_REFRESH), "Refresh", true, None);
    let firmware_item = MenuItem::with_id(
        MenuId::new(MENU_FIRMWARE),
        "Show Firmware Version",
        true,
        None,
    );
    let autostart_item = CheckMenuItem::with_id(
        MenuId::new(MENU_AUTOSTART),
        "Run at Startup",
        true,
        autostart_enabled,
        None,
    );
    let separator = PredefinedMenuItem::separator();
    let exit_item = MenuItem::with_id(MenuId::new(MENU_EXIT), "Exit", true, None);

    menu.append_items(&[
        &status_item,
        &devices_menu,
        &refresh_item,
        &firmware_item,
        &autostart_item,
        &separator,
        &exit_item,
    ])?;

    Ok(menu)
}

fn build_devices_menu(
    devices: &[DetectedDevice],
    selected_key: Option<&str>,
) -> Result<Submenu, Box<dyn std::error::Error>> {
    let submenu = Submenu::with_id(MenuId::new("devices"), "Devices", !devices.is_empty());

    if devices.is_empty() {
        let none = MenuItem::with_id(
            MenuId::new("device:none"),
            "No supported mice found",
            false,
            None,
        );
        submenu.append(&none)?;
        return Ok(submenu);
    }

    for device in devices {
        let text = if Some(device.key.as_str()) == selected_key {
            format!("{} (selected)", device.name)
        } else {
            device.name.to_string()
        };
        let item = CheckMenuItem::with_id(
            MenuId::new(format!("{DEVICE_PREFIX}{}", device.key)),
            text,
            true,
            Some(device.key.as_str()) == selected_key,
            None,
        );
        submenu.append(&item)?;
    }

    Ok(submenu)
}
