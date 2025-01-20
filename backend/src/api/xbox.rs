use crate::controller::Status;

use super::bluetooth::{get_battery_percentage, get_bluetooth_address};
use anyhow::Result;
use hidapi::{DeviceInfo, HidApi};
use log::error;
use zbus::Connection;

use super::Controller;

pub const MS_VENDOR_ID: u16 = 0x045e;

// Xbox One S controller
pub const XBOX_ONE_S_CONTROLLER_USB_PRODUCT_ID: u16 = 0x02ea; // 746
pub const XBOX_ONE_S_CONTROLLER_BT_PRODUCT_ID: u16 = 0x02df; // 765

// after upgrade to the latest firmware (same as Series X/S),
// the One S controller changed product ID!
pub const XBOX_ONE_S_LATEST_FW_PRODUCT_ID: u16 = 0x0b20; // 2848

// Xbox Wireless Controller (model 1914)
pub const XBOX_WIRELESS_CONTROLLER_USB_PRODUCT_ID: u16 = 0x0b12; // 2834
pub const XBOX_WIRELESS_CONTROLLER_BT_PRODUCT_ID: u16 = 0x0b13; // 2835

// Xbox Elite Wireless Controller Series 2
pub const XBOX_WIRELESS_ELITE_CONTROLLER_USB_PRODUCT_ID: u16 = 0x0b00;
pub const XBOX_WIRELESS_ELITE_CONTROLLER_BT_PRODUCT_ID: u16 = 0x0b05;
pub const XBOX_WIRELESS_ELITE_CONTROLLER_BTLE_PRODUCT_ID: u16 = 0x0b22;

// Xbox Accessory
pub const XBOX_ACCESSORY_PID: u16 = 0x02fe; // New accessory PID

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
    controller.capacity = if bluetooth { 0 } else { 100 }; // for now for USB, "fake" it and set capacity to 100 as charging
    controller.status = if bluetooth {
        Status::Unknown
    } else {
        // for now for USB, "fake" it and set status to charging since it's plugged in
        Status::Charging
    };
}

fn get_battery_percentage_from_upower(device_model: &str) -> Option<u8> {
    // Connect to the system bus
    if let Ok(connection) = Connection::system() {
        // Query the UPower service
        if let Ok(proxy) = zbus::Proxy::new(
            &connection,
            "org.freedesktop.UPower",
            "/org/freedesktop/UPower",
            "org.freedesktop.UPower",
        ) {
            // Get all devices
            if let Ok(devices) = proxy.call_method::<Vec<String>>("EnumerateDevices", &()) {
                for device_path in devices {
                    // Create a proxy for the specific device
                    if let Ok(device_proxy) = zbus::Proxy::new(
                        &connection,
                        "org.freedesktop.UPower",
                        &device_path,
                        "org.freedesktop.UPower.Device",
                    ) {
                        // Match the device model
                        if let Ok(model) = device_proxy.get_property::<String>("Model") {
                            if model == device_model {
                                // Get the battery percentage
                                if let Ok(percentage) = device_proxy.get_property::<f64>("Percentage") {
                                    return Some(percentage as u8);
                                }
                            }
                        }
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
    let capacity = match get_bluetooth_address(device_info) {
        Ok(address) => match get_battery_percentage(address) {
            Ok(percentage) => percentage,
            Err(err) => {
                error!("get_battery_percentage via Bluetooth failed because {}", err);
                // Attempt to fetch battery level via UPower
                get_battery_percentage_from_upower("Microsoft Xbox Controller").unwrap_or(0)
            }
        },
        Err(err) => {
            error!("get_bluetooth_address failed because {}", err);
            // Attempt to fetch battery level via UPower
            get_battery_percentage_from_upower("Microsoft Xbox Controller").unwrap_or(0)
        }
    };

    let name = get_xbox_controller_name(device_info.product_id());

    let controller = Controller::from_hidapi(device_info, name, capacity, Status::Unknown);
    Ok(controller)
}
