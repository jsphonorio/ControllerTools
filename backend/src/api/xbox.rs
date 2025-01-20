use crate::controller::Status;
use super::bluetooth::{get_battery_percentage, get_bluetooth_address};
use hidapi::{DeviceInfo, HidApi};
use log::error;
use dbus::{blocking::Connection, Message};
use dbus::blocking::BlockingSender; // Import the BlockingSender trait
use anyhow::{Result, Context, Error}; // Add Error to use anyhow::Error
use std::time::Duration; // Import the Duration struct

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
fn get_battery_percentage_upower() -> Result<u8> {
    let conn = Connection::new_session().context("Failed to connect to D-Bus")?;

    let msg = Message::new_method_call(
        "org.freedesktop.UPower",
        "/org/freedesktop/UPower/devices/battery_gip1x0",
        "org.freedesktop.UPower.Device",
        "GetPercentage",
    )
    .map_err(|e| Error::msg(format!("Failed to create DBus method call: {}", e)))?; // Wrap the String error in anyhow::Error

    let timeout = Duration::from_millis(2000); // Convert the integer to Duration

    let response = conn
    .send_with_reply_and_block(msg, timeout) // Pass the Duration object
    .context("Failed to get battery percentage from D-Bus")?;

    let percentage: u8 = response.read1().context("Failed to read battery percentage")?;

    Ok(percentage)
}
pub fn update_xbox_controller(controller: &mut Controller, bluetooth: bool) {
    controller.name = get_xbox_controller_name(controller.product_id).to_string();

    // Check if it's an Xbox Accessory and fetch battery percentage accordingly
    if controller.product_id == XBOX_ACCESSORY_PID {
        match get_battery_percentage_upower() {
            Ok(percentage) => {
                controller.capacity = 100;// percentage; // Set the battery percentage
                controller.status = Status::Charging; // Assuming charging status
            }
            Err(err) => {
                error!("Failed to get battery percentage for Xbox Accessory: {}", err);
                controller.capacity = 0; // Set capacity to 0 if failed to fetch
                controller.status = Status::Charging;
            }
        }
    } else {
        // For other controllers, use the Bluetooth method
        controller.capacity = if bluetooth { 0 } else { 100 }; // For USB controllers, "fake" it to 100 as charging
        controller.status = if bluetooth {
            Status::Unknown
        } else {
            // For USB, "fake" charging status
            Status::Charging
        };
    }
}

pub fn parse_xbox_controller_data(
    device_info: &DeviceInfo,
    _hidapi: &HidApi,
) -> Result<Controller> {
    let capacity: u8 = if device_info.product_id() == XBOX_ACCESSORY_PID {
        // If Xbox Accessory, get battery percentage from UPower
        match get_battery_percentage_upower() {
            Ok(percentage) => percentage,
            Err(err) => {
                error!("get_battery_percentage_upower failed because {}", err);
                0 // Return 0 if failed
            }
        }
    } else {
        // For other devices, use Bluetooth method
        match get_bluetooth_address(device_info) {
            Ok(address) => match get_battery_percentage(address) {
                Ok(percentage) => percentage,
                Err(err) => {
                    error!("get_battery_percentage failed because {}", err);
                    0
                }
            },
            Err(err) => {
                error!("get_bluetooth_address failed because {}", err);
                0
            }
        }
    };

    let name = get_xbox_controller_name(device_info.product_id());

    let controller = Controller::from_hidapi(device_info, name, capacity, Status::Unknown);
    Ok(controller)
}
