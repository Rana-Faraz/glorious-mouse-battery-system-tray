mod mouse_battery;

use ab_glyph::{FontRef, PxScale};
use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::drawing::draw_text_mut;
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

fn log_error(msg: &str) {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            use std::fs::{create_dir_all, OpenOptions};
            use std::io::Write;
            let log_dir = format!("{}\\ModelD2ProBattery", appdata);
            let _ = create_dir_all(&log_dir);
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(format!("{}\\error.log", log_dir))
            {
                let _ = writeln!(file, "{}", msg);
            }
        }
    }
    eprintln!("{}", msg);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    log_error("Starting application...");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(|app| {
            log_error("Setting up application...");

            // Initialize mouse battery monitor
            let mouse_battery = match MouseBattery::new() {
                Ok(mb) => {
                    log_error("Mouse battery monitor initialized successfully");
                    Arc::new(Mutex::new(mb))
                }
                Err(e) => {
                    let err_msg = format!("Failed to initialize mouse battery: {}", e);
                    log_error(&err_msg);
                    return Err(e.into());
                }
            };

            // Check autostart status
            let autostart_manager = app.autolaunch();
            let autostart_enabled =
                Arc::new(Mutex::new(autostart_manager.is_enabled().unwrap_or(false)));

            // Store state in app
            app.manage(AppState {
                mouse_battery: mouse_battery.clone(),
                autostart_enabled: autostart_enabled.clone(),
            });

            log_error("Setting up system tray...");
            // Setup system tray
            if let Err(e) = setup_tray(app.handle(), autostart_enabled.clone()) {
                let err_msg = format!("Failed to setup tray: {}", e);
                log_error(&err_msg);
                return Err(e);
            }

            log_error("Starting battery monitoring task...");
            // Start periodic battery monitoring
            let app_handle = app.handle().clone();
            let mouse_battery_clone = mouse_battery.clone();

            tauri::async_runtime::spawn(async move {
                battery_monitor_task(app_handle, mouse_battery_clone).await;
            });

            log_error("Setup complete!");
            Ok(())
        })
        .build(tauri::generate_context!())
        .map_err(|e| {
            let err_msg = format!("Failed to build Tauri app: {}", e);
            log_error(&err_msg);
            e
        })
        .expect("error while running tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}

fn build_menu_with_status<R: Runtime>(
    app: &AppHandle<R>,
    battery_status: &BatteryStatus,
    autostart_enabled: bool,
) -> Result<tauri::menu::Menu<R>, Box<dyn std::error::Error>> {
    // Create status menu item at the top showing mouse name and percentage
    let status_text = battery_status.get_tooltip();
    let status_item = MenuItemBuilder::with_id("status", &status_text)
        .enabled(false) // Make it non-clickable (display only)
        .build(app)?;

    // Create other menu items
    let refresh_item = MenuItemBuilder::with_id("refresh", "Refresh").build(app)?;
    let firmware_item = MenuItemBuilder::with_id("firmware", "Show Firmware Version").build(app)?;

    let autostart_item = CheckMenuItemBuilder::with_id("autostart", "Run at Startup")
        .checked(autostart_enabled)
        .build(app)?;

    let quit_item = MenuItemBuilder::with_id("quit", "Exit").build(app)?;

    // Build menu with status at the top
    let menu = MenuBuilder::new(app)
        .items(&[
            &status_item,
            &refresh_item,
            &firmware_item,
            &autostart_item,
            &quit_item,
        ])
        .build()?;

    Ok(menu)
}

fn setup_tray<R: Runtime>(
    app: &AppHandle<R>,
    autostart_enabled: Arc<Mutex<bool>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Get initial battery status
    let state = app.state::<AppState>();
    let battery_status = state.mouse_battery.lock().unwrap().get_battery_status();
    let autostart_checked = *autostart_enabled.lock().unwrap();

    // Build menu with status at top
    let menu = build_menu_with_status(app, &battery_status, autostart_checked)?;

    // Generate text icon
    let icon = create_text_icon(&battery_status)?;

    // Create tray icon
    let tray = TrayIconBuilder::new()
        .icon(icon)
        .menu(&menu)
        .tooltip(battery_status.get_tooltip())
        .on_menu_event(move |app, event| match event.id.as_ref() {
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

fn create_text_icon(status: &BatteryStatus) -> Result<Image<'static>, Box<dyn std::error::Error>> {
    let text = match status {
        BatteryStatus::Normal { percentage, .. } => format!("{}", percentage),
        BatteryStatus::Charging { percentage, .. } => format!("{}", percentage),
        BatteryStatus::FullyCharged { .. } => "100".to_string(),
        BatteryStatus::Asleep { .. } => "ZZZ".to_string(),
        BatteryStatus::WakingUp { .. } => "...".to_string(),
        BatteryStatus::NotFound => "N/A".to_string(),
        BatteryStatus::Unknown { .. } => "???".to_string(),
    };

    // Create a larger 256x256 image with transparent background for better quality
    let mut img: RgbaImage = ImageBuffer::from_pixel(256, 256, Rgba([0, 0, 0, 0]));

    // Load a font
    let font_data = include_bytes!("../assets/DejaVuSans.ttf");
    let font = FontRef::try_from_slice(font_data).map_err(|_| "Failed to load font")?;

    // Use much larger font size for better readability in system tray
    let scale = if text.len() <= 2 {
        PxScale::from(200.0) // Very large for 2 characters (like "68")
    } else if text.len() == 3 {
        PxScale::from(110.0) // Large for 3 characters (like "100")
    } else {
        PxScale::from(80.0) // Smaller for 4+ characters (like "N/A")
    };

    // Draw white text with good visibility
    let white = Rgba([255u8, 255u8, 255u8, 255u8]);

    // Better centering for larger canvas
    let x_offset = if text.len() <= 2 {
        40
    } else if text.len() == 3 {
        30
    } else {
        20
    };
    let y_offset = 40;

    draw_text_mut(&mut img, white, x_offset, y_offset, scale, &font, &text);

    // Convert to PNG bytes
    let mut png_bytes = Vec::new();
    img.write_to(
        &mut std::io::Cursor::new(&mut png_bytes),
        image::ImageFormat::Png,
    )?;

    let icon = Image::from_bytes(&png_bytes)?;
    Ok(icon)
}

fn update_tray_status<R: Runtime>(app: &AppHandle<R>) -> Result<(), Box<dyn std::error::Error>> {
    let state = app.state::<AppState>();
    let battery_status = state.mouse_battery.lock().unwrap().get_battery_status();
    let autostart_enabled = state.autostart_enabled.lock().unwrap();
    let autostart_checked = *autostart_enabled;

    // Get tray icon
    if let Some(tray) = app.try_state::<tauri::tray::TrayIcon>() {
        let icon = create_text_icon(&battery_status)?;

        // Rebuild menu with updated status
        let menu = build_menu_with_status(app, &battery_status, autostart_checked)?;

        tray.set_icon(Some(icon))?;
        tray.set_tooltip(Some(&battery_status.get_tooltip()))?;
        tray.set_menu(Some(menu))?;
    }

    Ok(())
}

fn show_firmware_version<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let mouse_battery = state.mouse_battery.lock().unwrap();

    // Get the detected mouse model
    let mouse_name = mouse_battery
        .get_detected_model()
        .map(|m| m.name())
        .unwrap_or("Mouse");

    let firmware = mouse_battery.get_firmware_version();

    let message = match firmware {
        Some(version) => format!("Firmware Version: {}", version),
        None => "Unable to retrieve firmware version. Is the mouse connected?".to_string(),
    };

    // Use a notification or dialog
    #[cfg(windows)]
    {
        use std::process::Command;
        let _ = Command::new("msg")
            .args(&["*", &format!("{}\n\n{}", mouse_name, message)])
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
