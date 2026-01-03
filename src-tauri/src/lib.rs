mod mouse_battery;

use mouse_battery::{BatteryStatus, MouseBattery};
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tokio::time::{interval, Duration};

pub struct AppState {
    mouse_battery: Arc<Mutex<MouseBattery>>,
    autostart_enabled: Arc<Mutex<bool>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(|app| {
            // Initialize mouse battery monitor
            let mouse_battery = match MouseBattery::new() {
                Ok(mb) => Arc::new(Mutex::new(mb)),
                Err(e) => {
                    eprintln!("Failed to initialize mouse battery: {}", e);
                    return Err(e.into());
                }
            };

            // Check autostart status
            let autostart_manager = app.autolaunch();
            let autostart_enabled = Arc::new(Mutex::new(autostart_manager.is_enabled().unwrap_or(false)));

            // Store state in app
            app.manage(AppState {
                mouse_battery: mouse_battery.clone(),
                autostart_enabled: autostart_enabled.clone(),
            });

            // Setup system tray
            setup_tray(app.handle(), autostart_enabled.clone())?;

            // Start periodic battery monitoring
            let app_handle = app.handle().clone();
            let mouse_battery_clone = mouse_battery.clone();
            
            tauri::async_runtime::spawn(async move {
                battery_monitor_task(app_handle, mouse_battery_clone).await;
            });

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}

fn setup_tray<R: Runtime>(
    app: &AppHandle<R>,
    autostart_enabled: Arc<Mutex<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create menu items
    let refresh_item = MenuItemBuilder::with_id("refresh", "Refresh").build(app)?;
    let firmware_item = MenuItemBuilder::with_id("firmware", "Show Firmware Version").build(app)?;
    
    let autostart_checked = *autostart_enabled.lock().unwrap();
    let autostart_item = CheckMenuItemBuilder::with_id("autostart", "Run at Startup")
        .checked(autostart_checked)
        .build(app)?;
    
    let quit_item = MenuItemBuilder::with_id("quit", "Exit").build(app)?;

    // Build menu
    let menu = MenuBuilder::new(app)
        .items(&[&refresh_item, &firmware_item, &autostart_item, &quit_item])
        .build()?;

    // Get initial battery status and icon
    let state = app.state::<AppState>();
    let battery_status = state.mouse_battery.lock().unwrap().get_battery_status();
    let icon_path = get_icon_path(&battery_status);
    
    // Load icon
    let icon = load_icon(&icon_path)?;

    // Create tray icon
    let tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip(battery_status.get_tooltip())
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "refresh" => {
                    if let Err(e) = update_tray_status(app) {
                        eprintln!("Failed to refresh status: {}", e);
                    }
                }
                "firmware" => {
                    show_firmware_version(app);
                }
                "autostart" => {
                    toggle_autostart(app);
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|_tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                // Optional: Show menu on left click
                // tray.popup_menu();
            }
        })
        .build(app)?;

    // Store tray icon in app state
    app.manage(tray);

    Ok(())
}

fn get_icon_path(status: &BatteryStatus) -> String {
    let icon_name = match status {
        BatteryStatus::Charging { percentage } => {
            if *percentage >= 100 {
                "battery_100"
            } else {
                "battery_charging"
            }
        }
        BatteryStatus::Normal { percentage } => {
            if *percentage <= 25 {
                "battery_0"
            } else if *percentage <= 50 {
                "battery_25"
            } else if *percentage <= 75 {
                "battery_50"
            } else {
                "battery_75"
            }
        }
        BatteryStatus::FullyCharged => "battery_100",
        _ => "battery_unknown",
    };
    
    format!("battery/{}.png", icon_name)
}

fn load_icon(path: &str) -> Result<Image<'static>, Box<dyn std::error::Error>> {
    // Load icon from resources
    let icon_bytes = std::fs::read(path)?;
    let icon = Image::from_bytes(&icon_bytes)?;
    Ok(icon)
}

fn update_tray_status<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let state = app.state::<AppState>();
    let battery_status = state.mouse_battery.lock().unwrap().get_battery_status();
    
    // Get tray icon
    if let Some(tray) = app.try_state::<tauri::tray::TrayIcon>() {
        let icon_path = get_icon_path(&battery_status);
        let icon = load_icon(&icon_path)?;
        
        tray.set_icon(Some(icon))?;
        tray.set_tooltip(Some(&battery_status.get_tooltip()))?;
    }
    
    Ok(())
}

fn show_firmware_version<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let firmware = state.mouse_battery.lock().unwrap().get_firmware_version();
    
    let message = match firmware {
        Some(version) => format!("Firmware Version: {}", version),
        None => "Unable to retrieve firmware version. Is the mouse connected?".to_string(),
    };
    
    // Use a notification or dialog
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("msg")
            .args(&["*", &format!("Model D2 Pro\n\n{}", message)])
            .spawn();
    }
    
    println!("{}", message);
}

fn toggle_autostart<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let autostart_manager = app.autolaunch();
    
    let mut autostart_enabled = state.autostart_enabled.lock().unwrap();
    let new_state = !*autostart_enabled;
    
    let result = if new_state {
        autostart_manager.enable()
    } else {
        autostart_manager.disable()
    };
    
    match result {
        Ok(_) => {
            *autostart_enabled = new_state;
            println!("Autostart toggled to: {}", new_state);
        }
        Err(e) => {
            eprintln!("Failed to toggle autostart: {}", e);
        }
    }
}

async fn battery_monitor_task<R: Runtime>(
    app: AppHandle<R>,
    _mouse_battery: Arc<Mutex<MouseBattery>>,
) {
    let mut interval = interval(Duration::from_secs(30));
    
    loop {
        interval.tick().await;
        
        if let Err(e) = update_tray_status(&app) {
            eprintln!("Failed to update tray status: {}", e);
        }
    }
}
