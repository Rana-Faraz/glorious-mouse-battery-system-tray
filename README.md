# Glorious Mouse Battery Monitor

A lightweight Windows system tray application that monitors the battery level of your Glorious gaming mice.

![Battery Monitor](https://img.shields.io/badge/Status-Active-green)
![Platform](https://img.shields.io/badge/Platform-Windows-blue)

## Features

- 🔋 **Real-time Battery Monitoring** - Displays battery percentage as text in system tray (e.g., "68", "100")
- 🎯 **Automatic Mouse Detection** - Automatically detects and displays the correct mouse model name
- 📱 **Dynamic Mouse Names** - Shows accurate mouse model name in tooltips and context menu (Model O Wired, Model O Wireless, Model O PRO Wireless, Model D 2 PRO Wireless)
- ⚡ **Charging Status** - Displays when the mouse is charging vs. running on battery
- 🔄 **Auto-Refresh** - Checks battery status every 30 seconds automatically
- 📊 **Status Menu** - Right-click context menu shows mouse name and current battery percentage
- 🔧 **Firmware Version** - Check the current firmware version of your mouse
- 🚀 **Startup Integration** - Toggle automatic startup with Windows
- 💪 **Lightweight & Minimal** - No windows, runs quietly in system tray with minimal resource usage
- 🔌 **Multi-Mouse Support** - Supports multiple Glorious mouse models with centralized, extensible configuration

## System Requirements

- Windows 10/11
- One of the following Glorious gaming mice:
  - Model O Wired
  - Model O Wireless
  - Model O PRO Wireless
  - Model D 2 PRO Wireless
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
1. **Appear in the system tray** with a battery percentage displayed as text (e.g., "68", "100")
2. **Automatically detect your mouse model** and display the correct name
3. **Update the percentage** every 30 seconds automatically
4. **Display status information** in tooltips and context menu

### System Tray Display

The tray icon shows:
- **Battery percentage** as large, readable text (e.g., "68" for 68%, "100" for fully charged)
- **Status indicators**: "ZZZ" when mouse is asleep, "N/A" when not found, "???" for unknown status
- **Text-only display** for maximum readability in the system tray

### Context Menu (Right-Click)

Right-click the tray icon to access:
- **Status Display** (top of menu) - Shows mouse model name and current battery percentage (e.g., "Model D 2 PRO Wireless: 68%")
- **Refresh** - Manually update battery status immediately
- **Show Firmware Version** - Display current mouse firmware version in a notification
- **Run at Startup** - Toggle automatic startup with Windows (checkmark indicates if enabled)
- **Exit** - Close the application

### Tooltip (Hover)

Hover over the tray icon to see:
- Mouse model name (e.g., "Model D 2 PRO Wireless")
- Current battery percentage
- Charging status (if charging)
- Connection status

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
- **Glorious Model O PRO Wireless** (Product ID: 0x2027)
- **Glorious Model D 2 PRO Wireless** (Product ID: 0x2034)
- Vendor ID: 0x258A (Glorious Gaming)

**Note**: Adding support for new mouse models is simple - just add a configuration entry. See the source code for details.

### Battery Status Detection
The application uses HID (Human Interface Device) feature reports to communicate with the mouse:
- Sends command `0x02 0x02` with feature code `0x83`
- Reads battery percentage from response buffer position [8]
- Parses charging status from buffer position [1]

### Architecture
- **Backend**: Rust with Tauri framework
- **HID Communication**: hidapi library (version 2.6) for USB device communication
- **Async Runtime**: Tokio for periodic battery checks every 30 seconds
- **System Integration**: Windows system tray via tray-icon
- **Text Rendering**: Dynamic icon generation with text using imageproc and ab_glyph
- **Configuration**: Centralized mouse configuration for easy extensibility

### Extensibility

The application uses a centralized configuration system. To add support for a new Glorious mouse model, simply add a new entry to the `SUPPORTED_MICE` array in the source code. The app will automatically:
- Detect the new mouse model by product ID
- Display the correct name in tooltips and menus
- Handle wired/wireless detection correctly

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

### Version 0.1.0 (Current Release)
- Text-based battery percentage display in system tray for maximum readability
- Automatic mouse model detection and dynamic name display
- Real-time battery monitoring with 30-second automatic refresh
- Context menu with mouse status, refresh, firmware info, and startup toggle
- Support for multiple Glorious mouse models:
  - Model O Wireless
  - Model O PRO Wireless
  - Model D 2 PRO Wireless
- Windows startup integration
- System tray-only interface (no windows)
- Centralized configuration system for easy extensibility
