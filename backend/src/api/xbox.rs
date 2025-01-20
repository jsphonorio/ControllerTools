use crate::controller::Status;
use super::bluetooth::{get_battery_percentage, get_bluetooth_address};
use anyhow::Result;
use hidapi::{DeviceInfo, HidApi};
use log::{error, warn};
use std::process::Command;

use super::Controller;

pub const MS_VENDOR_ID: u16 = 0x045e;

// Xbox One S controller
pub const XBOX_ONE_S_CONTROLLER_USB_PRODUCT_ID: u16 = 0x02ea;
pub const XBOX_ONE_S_CONTROLLER_BT_PRODUCT_ID: u16 = 0x02df;
pub const XBOX_ONE_S_LATEST_FW_PRODUCT_ID: u16 = 0x0b20;

// Xbox Wireless Controller (model 1914)
pub const XBOX_WIRELESS_CONTROLLER_USB_PRODUCT_ID: u16 = 0x0b12;
pub const XBOX_WIRELESS_CONTROLLER_BT_PRODUCT_ID: u16 = 0x0b13;

// Xbox Elite Wireless Controller Series 2
pub const XBOX_WIRELESS_ELITE_CONTROLLER_USB_PRODUCT_ID: u16 = 0x0b00;
pub const XBOX_WIRELESS_ELITE_CONTROLLER_BT_PRODUCT_ID: u16 = 0x0b05;
pub const XBOX_WIRELESS_ELITE_CONTROLLER_BTLE_PRODUCT_ID: u16 = 0x0b22;

// Xbox Accessory (e.g., wireless adapter)
pub const XBOX_ACCESSORY_PID: u16 = 0x02fe;

fn get_xbox_controller_name(product_id: u16) -> &'static str {
    match product_id {
        XBOX_ONE_S_CONTROLLER_USB_PRODUCT_ID => "Xbox One S",
        XBOX_ONE_S_CONTROLLER_BT_PRODUCT_ID => "Xbox One S",
        XBOX_ONE_S_LATEST_FW_PRODUCT_ID => "Xbox One S",
        XBOX_WIRELESS_CONTROLLER_USB_PRODUCT_ID => "Xbox Series X/S",
        XBOX_WIRELESS_CONTROLLER_BT_PRODUCT_ID => "Xbox Series X/S",
        XBOX_WIRELESS_ELITE_CONTROLLER_USB_PRODUCT_ID => "Xbox Elite 2",
        XBOX_WIRELESS_ELITE_CONTROLLER_BT_PRODUCT_ID => "Xbox Elite 2",
        XBOX_WIRELESS_ELITE_CONTROLLER_BTLE_PRODUCT_ID => "Xbox Elite 2",
        XBOX_ACCESSORY_PID => "Xbox Accessory",
        _ => "Xbox Unknown",
    }
}

pub fn is_xbox_controller(vendor_id: u16) -> bool {
    vendor_id == MS_VENDOR_ID
}

pub fn update_xbox_controller(controller: &mut Controller, bluetooth: bool) {
    controller.name = get_xbox_controller_name(controller.product_id).to_string();
    controller.capacity = if bluetooth { 0 } else { 100 }; // For USB, "fake" it as fully charged
    controller.status = if bluetooth {
        Status::Unknown
    } else {
        Status::Charging // For USB, assume charging
    };
}

/// Queries UPower for the battery percentage of an Xbox controller
fn get_battery_from_upower(native_path: &str) -> Option<u8> {
    let output = Command::new("upower")
        .args(["-i", native_path])
        .output();

    if let Ok(output) = output {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            for line in output_str.lines() {
                if line.trim_start().starts_with("percentage:") {
                    if let Some(percent_str) = line.split(':').nth(1) {
                        return percent_str.trim().trim_end_matches('%').parse().ok();
                    }
                }
            }
        }
    }
    None
}

pub fn parse_xbox_controller_data(
    device_info: &DeviceInfo,
    _hidapi: &HidApi,
) -> Result<Controller> {
    let capacity: u8 = match get_bluetooth_address(device_info) {
        Ok(address) => match get_battery_percentage(address) {
            Ok(percentage) => percentage,
            Err(_) => {
                // Try fetching from UPower
                warn!("Bluetooth battery check failed; falling back to UPower");
                get_battery_from_upower(&device_info.path().to_string_lossy()).unwrap_or(0)
            }
        },
        Err(_) => {
            // Fallback to UPower directly
            warn!("Bluetooth address not found; falling back to UPower");
            get_battery_from_upower(&device_info.path().to_string_lossy()).unwrap_or(0)
        }
    };

    let name = get_xbox_controller_name(device_info.product_id());
    let controller = Controller::from_hidapi(device_info, name, capacity, Status::Unknown);

    Ok(controller)
}
