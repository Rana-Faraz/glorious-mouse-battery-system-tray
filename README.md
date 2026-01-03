# Model D2 Pro Battery Monitor

A lightweight Windows system tray application that monitors the battery level of your Glorious Model D2 Pro Wireless gaming mouse.

![Battery Monitor](https://img.shields.io/badge/Status-Active-green)
![Platform](https://img.shields.io/badge/Platform-Windows-blue)

## Features

- 🔋 **Real-time Battery Monitoring** - Displays battery percentage with dynamic icons
- ⚡ **Charging Status** - Shows when the mouse is charging vs. on battery
- 🎨 **Visual Battery Indicators** - Color-coded icons (red/orange/yellow/green) based on battery level
- 🔄 **Auto-Refresh** - Checks battery status every 30 seconds automatically
- 🚀 **Startup Option** - Configure the app to run automatically when Windows starts
- 📊 **Firmware Info** - Check the current firmware version of your mouse
- 💪 **Lightweight** - Minimal resource usage, runs quietly in the system tray

## System Requirements

- Windows 10/11
- Glorious Model D2 Pro Wireless mouse (also supports Model O Wired and Model O Wireless)
- WebView2 Runtime (usually pre-installed on Windows 11)

## Installation

### Option 1: MSI Installer (Recommended)
1. Download `Model D2 Pro Battery Monitor_0.1.0_x64_en-US.msi` from the releases
2. Run the installer and follow the prompts
3. The app will start automatically after installation

### Option 2: NSIS Installer
1. Download `Model D2 Pro Battery Monitor_0.1.0_x64-setup.exe` from the releases
2. Run the setup executable
3. Follow the installation wizard

### Option 3: Portable Executable
1. Navigate to `target\release\`
2. Run `model-d2-pro-battery.exe` directly
3. No installation required

## Usage

Once running, the application will:
1. **Appear in the system tray** with a battery icon
2. **Update the icon** based on battery level:
   - 🔴 Red: 0-25% battery
   - 🟠 Orange: 26-50% battery
   - 🟡 Yellow: 51-75% battery
   - 🟢 Green: 76-100% battery
   - ⚡ Lightning bolt: Charging
   - ❓ Gray: Mouse not found/unknown status

### System Tray Menu

Right-click the tray icon to access:
- **Refresh** - Manually update battery status
- **Show Firmware Version** - Display current mouse firmware
- **Run at Startup** - Toggle automatic startup with Windows
- **Exit** - Close the application

### Tooltip

Hover over the tray icon to see:
- Current battery percentage
- Charging status
- Device connection status

## Building from Source

### Prerequisites
- Rust (latest stable version)
- Cargo
- Windows SDK

### Build Steps

```bash
# Clone the repository
git clone <repository-url>
cd model-d2-pro-battery

# Build the application
cargo tauri build

# Or for development
cargo tauri dev
```

The compiled executable will be in `target\release\model-d2-pro-battery.exe`
Installers will be in `target\release\bundle\`

## Technical Details

### Supported Devices
- **Glorious Model O Wired** (Product ID: 0x2011)
- **Glorious Model O Wireless** (Product ID: 0x2022)
- **Glorious Model D2 Pro Wireless** (Product ID: 0x2034)
- Vendor ID: 0x258A

### Battery Status Detection
The application uses HID (Human Interface Device) feature reports to communicate with the mouse:
- Sends command `0x02 0x02` with feature code `0x83`
- Reads battery percentage from response buffer position [8]
- Parses charging status from buffer position [1]

### Architecture
- **Backend**: Rust with Tauri framework
- **HID Communication**: hidapi library
- **Async Runtime**: Tokio for periodic updates
- **System Integration**: Windows system tray via tray-icon

## Troubleshooting

### Mouse Not Detected
- Ensure the mouse is powered on and connected (wired or wireless)
- Try the "Refresh" option from the tray menu
- Check if the mouse is recognized in Windows Device Manager
- Make sure you're using a supported Glorious mouse model

### App Won't Start
- Verify WebView2 Runtime is installed
- Check Windows Event Viewer for error messages
- Run from command line to see console output

### Battery Percentage Shows "Unknown"
- The mouse may be in sleep mode - move it to wake it up
- Try unplugging and replugging the USB receiver (wireless)
- Refresh the status manually from the tray menu

### Autostart Not Working
- Check if the app has permission to add startup entries
- Manually check Windows Task Manager > Startup tab
- Try toggling the "Run at Startup" option off and on again

## Uninstallation

### If installed via MSI:
1. Open Windows Settings > Apps > Installed apps
2. Find "Model D2 Pro Battery Monitor"
3. Click uninstall

### If installed via NSIS:
1. Use Windows Settings uninstaller, or
2. Run the uninstaller from the installation directory

### For portable version:
Simply delete the executable

## Credits

Battery detection logic based on Glorious mouse HID protocol research.

Built with:
- [Tauri](https://tauri.app/) - Desktop application framework
- [hidapi](https://github.com/libusb/hidapi) - USB HID communication
- [tokio](https://tokio.rs/) - Async runtime

## License

This project is provided as-is for personal use. Not officially affiliated with Glorious Gaming.

## Changelog

### Version 0.1.0 (Initial Release)
- Battery level monitoring with 30-second refresh
- Dynamic battery icons (0-25%, 26-50%, 51-75%, 76-100%, charging)
- Firmware version display
- Windows startup integration
- System tray interface
- Support for Model O and Model D2 Pro mice
