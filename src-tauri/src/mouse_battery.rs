use hidapi::{HidApi, HidDevice};
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatteryStatus {
    Normal { percentage: u8 },
    Charging { percentage: u8 },
    FullyCharged,
    Asleep,
    WakingUp,
    NotFound,
    Unknown { raw_status: u8, raw_battery: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseInfo {
    pub battery_status: BatteryStatus,
    pub firmware_version: Option<String>,
}

pub struct MouseBattery {
    hid_api: HidApi,
}

impl MouseBattery {
    pub fn new() -> Result<Self, String> {
        let hid_api = HidApi::new().map_err(|e| format!("Failed to initialize HID API: {}", e))?;
        Ok(Self { hid_api })
    }

    pub fn find_device(&self) -> Option<hidapi::DeviceInfo> {
        self.hid_api
            .device_list()
            .filter(|d| {
                // Glorious' vendor id
                d.vendor_id() == 0x258A &&
                // Supported Glorious mice product IDs:
                // 0x2011 = Model O Wired
                // 0x2022 = Model O Wireless
                // 0x2034 = Model D 2 PRO (Wireless)
                [0x2011, 0x2022, 0x2034].contains(&d.product_id()) &&
                // Feature report interface
                d.interface_number() == 0x02
            })
            // Get wired (0x2011) if available
            .min_by(|a, b| a.product_id().cmp(&b.product_id()))
            .map(|d| d.clone())
    }

    pub fn get_battery_status(&self) -> BatteryStatus {
        let device_info = match self.find_device() {
            Some(info) => info,
            None => return BatteryStatus::NotFound,
        };

        // Product id indicates whether wired
        // Model O Wired: 0x2011
        // Model O Wireless: 0x2022
        // Model D 2 PRO: 0x2034 (wireless)
        let wired = device_info.product_id() == 0x2011;

        let device = match device_info.open_device(&self.hid_api) {
            Ok(dev) => dev,
            Err(_) => return BatteryStatus::NotFound,
        };

        self.read_battery_status(&device, wired)
    }

    fn read_battery_status(&self, device: &HidDevice, wired: bool) -> BatteryStatus {
        let mut bfr_w = [0u8; 65];

        bfr_w[3] = 0x02;
        bfr_w[4] = 0x02;
        bfr_w[6] = 0x83;

        if device.send_feature_report(&bfr_w).is_err() {
            return BatteryStatus::Unknown {
                raw_status: 0,
                raw_battery: 0,
            };
        }

        thread::sleep(Duration::from_millis(50));

        let mut bfr_r = [0u8; 65];

        if device.get_feature_report(&mut bfr_r).is_err() {
            return BatteryStatus::Unknown {
                raw_status: 0,
                raw_battery: 0,
            };
        }

        let mut percentage = bfr_r[8];

        if percentage == 0 {
            percentage = 1;
        }

        let status = [0xA1, 0xA4, 0xA2, 0xA0, 0xA3]
            .iter()
            .position(|&s| s == bfr_r[1]);

        let status = if bfr_r[6] != 0x83 { Some(2) } else { status };

        match (status, wired) {
            (Some(0), false) => BatteryStatus::Normal { percentage },
            (Some(0), true) => {
                if percentage >= 100 {
                    BatteryStatus::FullyCharged
                } else {
                    BatteryStatus::Charging { percentage }
                }
            }
            (Some(1), _) => BatteryStatus::Asleep,
            (Some(3), _) => BatteryStatus::WakingUp,
            _ => BatteryStatus::Unknown {
                raw_status: bfr_r[1],
                raw_battery: bfr_r[8],
            },
        }
    }

    pub fn get_firmware_version(&self) -> Option<String> {
        let device_info = self.find_device()?;
        let wired = device_info.product_id() == 0x2011;
        let device = device_info.open_device(&self.hid_api).ok()?;

        let mut bfr_w = [0u8; 65];

        if wired {
            bfr_w[3] = 0x02;
        }

        bfr_w[4] = 0x03;
        bfr_w[6] = 0x81;

        device.send_feature_report(&bfr_w).ok()?;

        thread::sleep(Duration::from_millis(50));

        let mut bfr_r = [0u8; 65];

        device.get_feature_report(&mut bfr_r).ok()?;

        Some(format!(
            "{}.{}.{}.{}",
            bfr_r[7], bfr_r[8], bfr_r[9], bfr_r[10]
        ))
    }

    pub fn get_mouse_info(&self) -> MouseInfo {
        let battery_status = self.get_battery_status();
        let firmware_version = self.get_firmware_version();

        MouseInfo {
            battery_status,
            firmware_version,
        }
    }
}

impl BatteryStatus {
    pub fn get_icon_name(&self) -> &'static str {
        match self {
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
        }
    }

    pub fn get_tooltip(&self) -> String {
        match self {
            BatteryStatus::Normal { percentage } => {
                format!("Model D2 Pro: {}%", percentage)
            }
            BatteryStatus::Charging { percentage } => {
                format!("Model D2 Pro: {}% (Charging)", percentage)
            }
            BatteryStatus::FullyCharged => "Model D2 Pro: Fully Charged".to_string(),
            BatteryStatus::Asleep => "Model D2 Pro: Mouse is asleep".to_string(),
            BatteryStatus::WakingUp => "Model D2 Pro: Waking up...".to_string(),
            BatteryStatus::NotFound => "Model D2 Pro: Device not found".to_string(),
            BatteryStatus::Unknown { .. } => "Model D2 Pro: Unknown status".to_string(),
        }
    }
}
