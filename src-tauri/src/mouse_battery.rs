use hidapi::{HidApi, HidDevice};
use serde::{Deserialize, Serialize};
use std::thread;
use std::time::Duration;

// Centralized mouse configuration - add new mice here
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseConfig {
    pub product_id: u16,
    pub name: &'static str,
    pub is_wired: bool,
}

// All supported Glorious mice - add new entries here to extend support
pub const SUPPORTED_MICE: &[MouseConfig] = &[
    MouseConfig {
        product_id: 0x2011,
        name: "Model O Wired",
        is_wired: true,
    },
    MouseConfig {
        product_id: 0x2022,
        name: "Model O Wireless",
        is_wired: false,
    },
    MouseConfig {
        product_id: 0x2027,
        name: "Model O PRO Wireless",
        is_wired: false,
    },
    MouseConfig {
        product_id: 0x2034,
        name: "Model D 2 PRO Wireless",
        is_wired: false,
    },
];

impl MouseConfig {
    pub fn from_product_id(product_id: u16) -> Option<&'static Self> {
        SUPPORTED_MICE.iter().find(|m| m.product_id == product_id)
    }

    pub fn all_product_ids() -> Vec<u16> {
        SUPPORTED_MICE.iter().map(|m| m.product_id).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BatteryStatus {
    Normal {
        percentage: u8,
        mouse_model: MouseModel,
    },
    Charging {
        percentage: u8,
        mouse_model: MouseModel,
    },
    FullyCharged {
        mouse_model: MouseModel,
    },
    Asleep {
        mouse_model: MouseModel,
    },
    WakingUp {
        mouse_model: MouseModel,
    },
    NotFound,
    Unknown {
        raw_status: u8,
        raw_battery: u8,
        mouse_model: MouseModel,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseInfo {
    pub battery_status: BatteryStatus,
    pub firmware_version: Option<String>,
}

// Mouse model - stores index into SUPPORTED_MICE array or Unknown
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseModel {
    pub(crate) config_index: Option<usize>,
}

impl MouseModel {
    pub fn from_product_id(product_id: u16) -> Self {
        let index = SUPPORTED_MICE
            .iter()
            .position(|m| m.product_id == product_id);
        MouseModel {
            config_index: index,
        }
    }

    pub fn name(&self) -> &'static str {
        self.config().map(|c| c.name).unwrap_or("Unknown Mouse")
    }

    pub fn is_wired(&self) -> bool {
        self.config().map(|c| c.is_wired).unwrap_or(false)
    }

    fn config(&self) -> Option<&'static MouseConfig> {
        self.config_index.map(|i| &SUPPORTED_MICE[i])
    }
}

// Custom Serialize/Deserialize for MouseModel
impl Serialize for MouseModel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let Some(config) = self.config() {
            serializer.serialize_str(config.name)
        } else {
            serializer.serialize_str("Unknown Mouse")
        }
    }
}

impl<'de> Deserialize<'de> for MouseModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        let index = SUPPORTED_MICE.iter().position(|m| m.name == name);
        Ok(MouseModel {
            config_index: index,
        })
    }
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
        let supported_pids = MouseConfig::all_product_ids();

        self.hid_api
            .device_list()
            .filter(|d| {
                // Glorious' vendor id
                d.vendor_id() == 0x258A &&
                // Check if product ID is in our supported list
                supported_pids.contains(&d.product_id()) &&
                // Feature report interface
                d.interface_number() == 0x02
            })
            // Prefer wired mice (lower product ID typically means wired)
            .min_by(|a, b| {
                let a_wired = MouseConfig::from_product_id(a.product_id())
                    .map(|c| if c.is_wired { 0 } else { 1 })
                    .unwrap_or(2);
                let b_wired = MouseConfig::from_product_id(b.product_id())
                    .map(|c| if c.is_wired { 0 } else { 1 })
                    .unwrap_or(2);
                a_wired
                    .cmp(&b_wired)
                    .then_with(|| a.product_id().cmp(&b.product_id()))
            })
            .map(|d| d.clone())
    }

    pub fn get_detected_model(&self) -> Option<MouseModel> {
        self.find_device()
            .map(|info| MouseModel::from_product_id(info.product_id()))
    }

    pub fn get_battery_status(&self) -> BatteryStatus {
        let device_info = match self.find_device() {
            Some(info) => info,
            None => return BatteryStatus::NotFound,
        };

        let mouse_model = MouseModel::from_product_id(device_info.product_id());
        let wired = mouse_model.is_wired();

        let device = match device_info.open_device(&self.hid_api) {
            Ok(dev) => dev,
            Err(_) => return BatteryStatus::NotFound,
        };

        self.read_battery_status(&device, wired, mouse_model)
    }

    fn read_battery_status(
        &self,
        device: &HidDevice,
        wired: bool,
        mouse_model: MouseModel,
    ) -> BatteryStatus {
        let mut bfr_w = [0u8; 65];

        bfr_w[3] = 0x02;
        bfr_w[4] = 0x02;
        bfr_w[6] = 0x83;

        if device.send_feature_report(&bfr_w).is_err() {
            return BatteryStatus::Unknown {
                raw_status: 0,
                raw_battery: 0,
                mouse_model,
            };
        }

        thread::sleep(Duration::from_millis(50));

        let mut bfr_r = [0u8; 65];

        if device.get_feature_report(&mut bfr_r).is_err() {
            return BatteryStatus::Unknown {
                raw_status: 0,
                raw_battery: 0,
                mouse_model,
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
            (Some(0), false) => BatteryStatus::Normal {
                percentage,
                mouse_model,
            },
            (Some(0), true) => {
                if percentage >= 100 {
                    BatteryStatus::FullyCharged { mouse_model }
                } else {
                    BatteryStatus::Charging {
                        percentage,
                        mouse_model,
                    }
                }
            }
            (Some(1), _) => BatteryStatus::Asleep { mouse_model },
            (Some(3), _) => BatteryStatus::WakingUp { mouse_model },
            _ => BatteryStatus::Unknown {
                raw_status: bfr_r[1],
                raw_battery: bfr_r[8],
                mouse_model,
            },
        }
    }

    pub fn get_firmware_version(&self) -> Option<String> {
        let device_info = self.find_device()?;
        let mouse_model = MouseModel::from_product_id(device_info.product_id());
        let wired = mouse_model.is_wired();
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
            BatteryStatus::Charging { percentage, .. } => {
                if *percentage >= 100 {
                    "battery_100"
                } else {
                    "battery_charging"
                }
            }
            BatteryStatus::Normal { percentage, .. } => {
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
            BatteryStatus::FullyCharged { .. } => "battery_100",
            _ => "battery_unknown",
        }
    }

    pub fn get_mouse_model(&self) -> Option<MouseModel> {
        match self {
            BatteryStatus::Normal { mouse_model, .. } => Some(*mouse_model),
            BatteryStatus::Charging { mouse_model, .. } => Some(*mouse_model),
            BatteryStatus::FullyCharged { mouse_model } => Some(*mouse_model),
            BatteryStatus::Asleep { mouse_model } => Some(*mouse_model),
            BatteryStatus::WakingUp { mouse_model } => Some(*mouse_model),
            BatteryStatus::Unknown { mouse_model, .. } => Some(*mouse_model),
            BatteryStatus::NotFound => None,
        }
    }

    pub fn get_tooltip(&self) -> String {
        let mouse_name = self.get_mouse_model().map(|m| m.name()).unwrap_or("Mouse");

        match self {
            BatteryStatus::Normal { percentage, .. } => {
                format!("{}: {}%", mouse_name, percentage)
            }
            BatteryStatus::Charging { percentage, .. } => {
                format!("{}: {}% (Charging)", mouse_name, percentage)
            }
            BatteryStatus::FullyCharged { .. } => {
                format!("{}: Fully Charged", mouse_name)
            }
            BatteryStatus::Asleep { .. } => {
                format!("{}: Mouse is asleep", mouse_name)
            }
            BatteryStatus::WakingUp { .. } => {
                format!("{}: Waking up...", mouse_name)
            }
            BatteryStatus::NotFound => "Mouse: Device not found".to_string(),
            BatteryStatus::Unknown { .. } => {
                format!("{}: Unknown status", mouse_name)
            }
        }
    }
}
