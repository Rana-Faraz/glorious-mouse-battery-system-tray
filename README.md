# Glorious Mouse Battery Monitor

A lightweight native Rust Windows system tray application that monitors the battery level of supported Glorious gaming mice.

Current release: `0.2.0`

## Features

- Text battery percentage in the system tray, such as `68`, `100`, `ZZZ`, `N/A`, or `???`
- Automatic detection of Glorious mice on the battery HID interface
- Manual device selection from the tray menu when multiple supported mice are connected
- Battery, charging, asleep, waking, not found, and unknown status display
- Automatic refresh every 30 seconds by default
- Firmware version lookup from the tray menu
- Per-user Windows startup toggle
- Single-instance guard to avoid duplicate tray icons
- File logging under `%LOCALAPPDATA%\ModelD2ProBattery\logs`

## Supported Devices

The app detects Glorious HID devices by vendor ID `0x258A` and feature-report interface `0x02`. Known product IDs below get polished names and wired/wireless hints; unknown Glorious product IDs are still detected and shown using their HID product string or a `Glorious Mouse 0xNNNN` fallback.

- Glorious Model O Wired, product ID `0x2011`
- Glorious Model O Wireless, product ID `0x2022`
- Glorious Model O PRO Wireless, product ID `0x2027`
- Glorious Model D 2 PRO Wireless, product ID `0x2034`

## Usage

Run `model-d2-pro-battery.exe`. The app has no main window and lives in the Windows system tray.

Right-click the tray icon for:

- Current mouse battery/status
- Device selection
- Manual refresh
- Firmware version lookup
- Run at Startup toggle
- Exit

The app stores minimal config at:

```text
%LOCALAPPDATA%\ModelD2ProBattery\config.toml
```

The config currently contains:

```toml
selected_device_key = "..."
refresh_interval_seconds = 30
```

Startup state is stored separately in the Windows registry:

```text
HKCU\Software\Microsoft\Windows\CurrentVersion\Run
```

## Building

Prerequisites:

- Rust stable
- Windows
- Windows SDK/build tools compatible with `hidapi`

Development checks:

```powershell
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Release build:

```powershell
cargo build --release
```

The executable is written to:

```text
target\release\model-d2-pro-battery.exe
```

## Technical Notes

The application uses HID feature reports to communicate with the mouse:

- Battery command writes `0x02 0x02` with feature code `0x83`
- Battery percentage is read from response buffer index `8`
- Charging/status state is parsed from response buffer index `1`
- Firmware lookup uses feature code `0x81`

The runtime is native Rust:

- `tray-icon` for the tray icon and context menu
- `winit` for the event loop
- `hidapi` for USB HID communication
- `image`, `imageproc`, and `ab_glyph` for dynamic text tray icons
- `winreg` for per-user startup integration
- `tracing` for file logging

## Troubleshooting

If the mouse is not detected:

- Confirm the mouse or receiver is connected and powered on
- Move the mouse to wake it if it is asleep
- Use the tray menu's Refresh action
- Check Windows Device Manager
- Confirm the device is one of the supported models above

If the app does not appear:

- Check whether another instance is already running
- Check logs under `%LOCALAPPDATA%\ModelD2ProBattery\logs`
- Run a debug build from PowerShell to see console output

## License

This project is provided as-is for personal use. It is not officially affiliated with Glorious Gaming.
