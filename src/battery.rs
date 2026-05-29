use hidapi::{DeviceInfo, HidApi, HidDevice};
use serde::{Deserialize, Serialize};
use std::ffi::CStr;
use std::thread;
use std::time::Duration;

pub const GLORIOUS_VENDOR_ID: u16 = 0x258A;
const FEATURE_INTERFACE: i32 = 0x02;
const REPORT_LEN: usize = 65;
const BATTERY_COMMAND: (usize, u8, usize, u8, usize, u8) = (3, 0x02, 4, 0x02, 6, 0x83);
const FIRMWARE_COMMAND: (usize, u8, usize, u8) = (4, 0x03, 6, 0x81);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseConfig {
    pub product_id: u16,
    pub name: &'static str,
    pub is_wired: bool,
}

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
        SUPPORTED_MICE
            .iter()
            .find(|mouse| mouse.product_id == product_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MouseModel {
    product_id: u16,
    name: String,
    is_wired: bool,
    known: bool,
}

impl MouseModel {
    #[cfg(test)]
    pub fn from_product_id(product_id: u16) -> Self {
        if let Some(config) = MouseConfig::from_product_id(product_id) {
            Self {
                product_id,
                name: config.name.to_string(),
                is_wired: config.is_wired,
                known: true,
            }
        } else {
            Self {
                product_id,
                name: fallback_mouse_name(product_id),
                is_wired: false,
                known: false,
            }
        }
    }

    fn from_device_info(info: &DeviceInfo) -> Self {
        if let Some(config) = MouseConfig::from_product_id(info.product_id()) {
            Self {
                product_id: info.product_id(),
                name: config.name.to_string(),
                is_wired: config.is_wired,
                known: true,
            }
        } else {
            Self {
                product_id: info.product_id(),
                name: device_product_name(info),
                is_wired: false,
                known: false,
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn is_wired(&self) -> bool {
        self.is_wired
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedDevice {
    pub key: String,
    pub product_id: u16,
    pub name: String,
    pub is_wired: bool,
    pub known: bool,
}

impl DetectedDevice {
    fn from_info(info: &DeviceInfo) -> Self {
        let model = MouseModel::from_device_info(info);
        Self {
            key: device_key(info),
            product_id: info.product_id(),
            name: model.name,
            is_wired: model.is_wired,
            known: model.known,
        }
    }

    pub fn model(&self) -> MouseModel {
        MouseModel {
            product_id: self.product_id,
            name: self.name.clone(),
            is_wired: self.is_wired,
            known: self.known,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl BatteryStatus {
    pub fn icon_text(&self) -> String {
        match self {
            BatteryStatus::Normal { percentage, .. } => percentage.to_string(),
            BatteryStatus::Charging { percentage, .. } => percentage.to_string(),
            BatteryStatus::FullyCharged { .. } => "100".to_string(),
            BatteryStatus::Asleep { .. } => "ZZZ".to_string(),
            BatteryStatus::WakingUp { .. } => "...".to_string(),
            BatteryStatus::NotFound => "N/A".to_string(),
            BatteryStatus::Unknown { .. } => "???".to_string(),
        }
    }

    pub fn tooltip(&self) -> String {
        let mouse_name = self.mouse_model().map(MouseModel::name).unwrap_or("Mouse");

        match self {
            BatteryStatus::Normal { percentage, .. } => format!("{mouse_name}: {percentage}%"),
            BatteryStatus::Charging { percentage, .. } => {
                format!("{mouse_name}: {percentage}% (Charging)")
            }
            BatteryStatus::FullyCharged { .. } => format!("{mouse_name}: Fully Charged"),
            BatteryStatus::Asleep { .. } => format!("{mouse_name}: Mouse is asleep"),
            BatteryStatus::WakingUp { .. } => format!("{mouse_name}: Waking up..."),
            BatteryStatus::NotFound => "Mouse: Device not found".to_string(),
            BatteryStatus::Unknown {
                raw_status,
                raw_battery,
                ..
            } => {
                format!("{mouse_name}: Unknown status ({raw_status:#04X}, battery {raw_battery})")
            }
        }
    }

    pub fn mouse_model(&self) -> Option<&MouseModel> {
        match self {
            BatteryStatus::Normal { mouse_model, .. }
            | BatteryStatus::Charging { mouse_model, .. }
            | BatteryStatus::FullyCharged { mouse_model }
            | BatteryStatus::Asleep { mouse_model }
            | BatteryStatus::WakingUp { mouse_model }
            | BatteryStatus::Unknown { mouse_model, .. } => Some(mouse_model),
            BatteryStatus::NotFound => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BatterySnapshot {
    pub devices: Vec<DetectedDevice>,
    pub selected_key: Option<String>,
    pub status: BatteryStatus,
}

pub struct MouseBattery {
    hid_api: HidApi,
}

impl MouseBattery {
    pub fn new() -> Result<Self, String> {
        let hid_api =
            HidApi::new().map_err(|error| format!("failed to initialize HID API: {error}"))?;
        Ok(Self { hid_api })
    }

    pub fn snapshot(&self, preferred_key: Option<&str>) -> BatterySnapshot {
        let devices = self.detect_devices();
        let selected = select_device(&devices, preferred_key);
        let status = selected
            .as_ref()
            .map(|device| self.get_battery_status_for(device))
            .unwrap_or(BatteryStatus::NotFound);

        BatterySnapshot {
            selected_key: selected.map(|device| device.key.clone()),
            devices,
            status,
        }
    }

    pub fn detect_devices(&self) -> Vec<DetectedDevice> {
        let mut devices = self
            .hid_api
            .device_list()
            .filter(|info| is_supported_feature_interface(info))
            .map(DetectedDevice::from_info)
            .collect::<Vec<_>>();

        devices.sort_by_key(|device| {
            (
                u8::from(!device.is_wired),
                device.product_id,
                device.key.clone(),
            )
        });
        devices
    }

    pub fn get_firmware_version(
        &self,
        selected_key: Option<&str>,
    ) -> Option<(DetectedDevice, String)> {
        let devices = self.detect_devices();
        let selected = select_device(&devices, selected_key)?;
        let info = self.find_device_info(&selected.key)?;
        let device = info.open_device(&self.hid_api).ok()?;
        let version = read_firmware_version(&device, selected.model())?;
        Some((selected.clone(), version))
    }

    fn get_battery_status_for(&self, selected: &DetectedDevice) -> BatteryStatus {
        let Some(info) = self.find_device_info(&selected.key) else {
            return BatteryStatus::NotFound;
        };
        let mouse_model = selected.model();
        let device = match info.open_device(&self.hid_api) {
            Ok(device) => device,
            Err(error) => {
                tracing::warn!(%error, device = %selected.name, "failed to open HID device");
                return BatteryStatus::NotFound;
            }
        };

        match request_battery_report(&device) {
            Ok(report) => parse_battery_report(&report, mouse_model),
            Err(error) => {
                tracing::warn!(%error, device = %selected.name, "failed to read battery report");
                BatteryStatus::Unknown {
                    raw_status: 0,
                    raw_battery: 0,
                    mouse_model,
                }
            }
        }
    }

    fn find_device_info(&self, key: &str) -> Option<DeviceInfo> {
        self.hid_api
            .device_list()
            .find(|info| is_supported_feature_interface(info) && device_key(info) == key)
            .cloned()
    }
}

pub fn select_device<'a>(
    devices: &'a [DetectedDevice],
    preferred_key: Option<&str>,
) -> Option<&'a DetectedDevice> {
    if let Some(key) = preferred_key {
        if let Some(device) = devices.iter().find(|device| device.key == key) {
            return Some(device);
        }
    }

    devices.iter().min_by_key(|device| {
        (
            u8::from(!device.is_wired),
            device.product_id,
            device.key.clone(),
        )
    })
}

pub fn parse_battery_report(report: &[u8; REPORT_LEN], mouse_model: MouseModel) -> BatteryStatus {
    let mut percentage = report[8];
    if percentage == 0 {
        percentage = 1;
    }

    let status = [0xA1, 0xA4, 0xA2, 0xA0, 0xA3]
        .iter()
        .position(|&status| status == report[1]);

    let status = if report[6] != 0x83 { Some(2) } else { status };

    match (status, mouse_model.is_wired()) {
        (Some(0), false) => BatteryStatus::Normal {
            percentage,
            mouse_model,
        },
        (Some(0), true) if percentage >= 100 => BatteryStatus::FullyCharged { mouse_model },
        (Some(0), true) => BatteryStatus::Charging {
            percentage,
            mouse_model,
        },
        (Some(1), _) => BatteryStatus::Asleep { mouse_model },
        (Some(3), _) => BatteryStatus::WakingUp { mouse_model },
        _ => BatteryStatus::Unknown {
            raw_status: report[1],
            raw_battery: report[8],
            mouse_model,
        },
    }
}

fn request_battery_report(device: &HidDevice) -> Result<[u8; REPORT_LEN], hidapi::HidError> {
    let mut request = [0_u8; REPORT_LEN];
    request[BATTERY_COMMAND.0] = BATTERY_COMMAND.1;
    request[BATTERY_COMMAND.2] = BATTERY_COMMAND.3;
    request[BATTERY_COMMAND.4] = BATTERY_COMMAND.5;

    device.send_feature_report(&request)?;
    thread::sleep(Duration::from_millis(50));

    let mut response = [0_u8; REPORT_LEN];
    device.get_feature_report(&mut response)?;
    Ok(response)
}

fn read_firmware_version(device: &HidDevice, mouse_model: MouseModel) -> Option<String> {
    let mut request = [0_u8; REPORT_LEN];
    if mouse_model.is_wired() {
        request[3] = 0x02;
    }
    request[FIRMWARE_COMMAND.0] = FIRMWARE_COMMAND.1;
    request[FIRMWARE_COMMAND.2] = FIRMWARE_COMMAND.3;

    device.send_feature_report(&request).ok()?;
    thread::sleep(Duration::from_millis(50));

    let mut response = [0_u8; REPORT_LEN];
    device.get_feature_report(&mut response).ok()?;

    Some(format!(
        "{}.{}.{}.{}",
        response[7], response[8], response[9], response[10]
    ))
}

fn is_supported_feature_interface(info: &DeviceInfo) -> bool {
    info.vendor_id() == GLORIOUS_VENDOR_ID && info.interface_number() == FEATURE_INTERFACE
}

fn device_key(info: &DeviceInfo) -> String {
    let path = cstr_to_string(info.path());
    if !path.is_empty() {
        path
    } else {
        format!("vid_{:04x}_pid_{:04x}", info.vendor_id(), info.product_id())
    }
}

fn cstr_to_string(value: &CStr) -> String {
    value.to_string_lossy().into_owned()
}

fn device_product_name(info: &DeviceInfo) -> String {
    info.product_string()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| fallback_mouse_name(info.product_id()))
}

fn fallback_mouse_name(product_id: u16) -> String {
    format!("Glorious Mouse 0x{product_id:04X}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(status: u8, marker: u8, battery: u8) -> [u8; REPORT_LEN] {
        let mut report = [0_u8; REPORT_LEN];
        report[1] = status;
        report[6] = marker;
        report[8] = battery;
        report
    }

    fn model(product_id: u16) -> MouseModel {
        MouseModel::from_product_id(product_id)
    }

    fn detected_device(key: &str, product_id: u16, name: &str, is_wired: bool) -> DetectedDevice {
        DetectedDevice {
            key: key.to_string(),
            product_id,
            name: name.to_string(),
            is_wired,
            known: MouseConfig::from_product_id(product_id).is_some(),
        }
    }

    #[test]
    fn parses_normal_wireless_battery() {
        assert_eq!(
            parse_battery_report(&report(0xA1, 0x83, 68), model(0x2034)),
            BatteryStatus::Normal {
                percentage: 68,
                mouse_model: model(0x2034)
            }
        );
    }

    #[test]
    fn parses_charging_wired_battery() {
        assert_eq!(
            parse_battery_report(&report(0xA1, 0x83, 44), model(0x2011)),
            BatteryStatus::Charging {
                percentage: 44,
                mouse_model: model(0x2011)
            }
        );
    }

    #[test]
    fn parses_fully_charged_wired_device() {
        assert_eq!(
            parse_battery_report(&report(0xA1, 0x83, 100), model(0x2011)),
            BatteryStatus::FullyCharged {
                mouse_model: model(0x2011)
            }
        );
    }

    #[test]
    fn parses_asleep_state() {
        assert_eq!(
            parse_battery_report(&report(0xA4, 0x83, 72), model(0x2034)),
            BatteryStatus::Asleep {
                mouse_model: model(0x2034)
            }
        );
    }

    #[test]
    fn parses_waking_state() {
        assert_eq!(
            parse_battery_report(&report(0xA0, 0x83, 72), model(0x2034)),
            BatteryStatus::WakingUp {
                mouse_model: model(0x2034)
            }
        );
    }

    #[test]
    fn parses_unknown_status() {
        assert_eq!(
            parse_battery_report(&report(0xFE, 0x83, 72), model(0x2034)),
            BatteryStatus::Unknown {
                raw_status: 0xFE,
                raw_battery: 72,
                mouse_model: model(0x2034)
            }
        );
    }

    #[test]
    fn invalid_response_marker_is_unknown() {
        assert_eq!(
            parse_battery_report(&report(0xA1, 0x00, 72), model(0x2034)),
            BatteryStatus::Unknown {
                raw_status: 0xA1,
                raw_battery: 72,
                mouse_model: model(0x2034)
            }
        );
    }

    #[test]
    fn unknown_glorious_product_ids_get_fallback_metadata() {
        let model = model(0x2040);

        assert_eq!(model.name(), "Glorious Mouse 0x2040");
        assert!(!model.is_wired());
    }

    #[test]
    fn selection_prefers_wired_then_lowest_product_id() {
        let devices = vec![
            detected_device("wireless", 0x2034, "Model D 2 PRO Wireless", false),
            detected_device("wired", 0x2011, "Model O Wired", true),
        ];

        assert_eq!(select_device(&devices, None).unwrap().key, "wired");
    }

    #[test]
    fn selection_uses_preferred_key_when_present() {
        let devices = vec![
            detected_device("a", 0x2011, "Model O Wired", true),
            detected_device("b", 0x2034, "Model D 2 PRO Wireless", false),
        ];

        assert_eq!(select_device(&devices, Some("b")).unwrap().key, "b");
    }

    #[test]
    fn selection_falls_back_when_preferred_key_missing() {
        let devices = vec![detected_device(
            "a",
            0x2034,
            "Model D 2 PRO Wireless",
            false,
        )];

        assert_eq!(select_device(&devices, Some("missing")).unwrap().key, "a");
    }

    #[test]
    fn selection_handles_empty_devices() {
        assert!(select_device(&[], None).is_none());
    }
}
