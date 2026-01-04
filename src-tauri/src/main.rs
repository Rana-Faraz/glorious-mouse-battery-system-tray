// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn log_startup() {
    #[cfg(windows)]
    {
        use std::fs::{create_dir_all, OpenOptions};
        use std::io::Write;
        if let Ok(appdata) = std::env::var("LOCALAPPDATA") {
            let log_dir = format!("{}\\ModelD2ProBattery", appdata);
            let _ = create_dir_all(&log_dir);
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(format!("{}\\startup.log", log_dir))
            {
                let _ = writeln!(file, "=== Application Started ===");
                let _ = writeln!(file, "Time: {:?}", std::time::SystemTime::now());
                let _ = writeln!(file, "Exe Path: {:?}", std::env::current_exe());
                let _ = writeln!(file, "Working Dir: {:?}", std::env::current_dir());
            }
        }
    }
}

fn main() {
    log_startup();
    model_d2_pro_battery_lib::run()
}
