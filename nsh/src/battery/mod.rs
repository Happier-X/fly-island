//! Peripheral battery levels: Bluetooth LE Battery Service, standard HID Battery Strength,
//! and Logitech HID++ receivers. Devices that do not report a percentage are simply left out;
//! the laptop's own battery is never read here.

#![allow(dead_code)]

mod bluetooth;
mod com;
mod hid;
mod hid_battery;
mod hidpp;
mod report;
#[cfg(test)]
mod tests;

use serde::Serialize;
use std::collections::HashSet;

use self::com::WinRtApartment;

/// What kind of device a battery reading belongs to. `Other` is still listed, with a generic icon.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PeripheralKind {
    Mouse,
    Keyboard,
    Headset,
    Receiver,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PeripheralBatteryDevice {
    pub id: String,
    pub name: String,
    pub kind: PeripheralKind,
    pub level: u8,
    pub charging: bool,
}

/// Reads every peripheral battery that is readable right now. A failed source yields no
/// devices instead of an error, so a missing Bluetooth stack or HID permission never blocks
/// the page or shows a dialog.
pub fn read_batteries() -> Vec<PeripheralBatteryDevice> {
    let mut devices = Vec::new();
    // BLE GATT needs WinRT on this thread; HID does not, so a failure only drops Bluetooth.
    if let Ok(_winrt) = WinRtApartment::multithreaded() {
        devices.extend(bluetooth::read());
    }
    devices.extend(hid_battery::read());
    devices.extend(hidpp::read());
    normalize(devices)
}

/// Sorts by device type and name, then keeps one row per name. Bluetooth is read first,
/// so a device that also exposes a HID battery report keeps the richer BLE name.
fn normalize(mut devices: Vec<PeripheralBatteryDevice>) -> Vec<PeripheralBatteryDevice> {
    devices.retain(|device| device.level <= 100);
    for device in &mut devices {
        device.name = device.name.trim().to_string();
        if device.name.is_empty() {
            device.name = "HID device".to_string();
        }
    }
    devices.sort_by(|left, right| {
        kind_rank(left.kind)
            .cmp(&kind_rank(right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    let mut seen = HashSet::new();
    devices.retain(|device| {
        seen.insert(device.id.clone()) && seen.insert(format!("name:{}", device.name.to_lowercase()))
    });
    devices.truncate(16);
    devices
}


fn kind_rank(kind: PeripheralKind) -> u8 {
    match kind {
        PeripheralKind::Mouse => 0,
        PeripheralKind::Keyboard => 1,
        PeripheralKind::Headset => 2,
        PeripheralKind::Receiver => 3,
        PeripheralKind::Other => 4,
    }
}

/// Classifies a device from its HID collection, BLE appearance, or name. Usage and
/// appearance are more reliable than product strings, which are often generic.
pub(crate) fn classify(
    name: &str,
    usage_page: u16,
    usage: u16,
    appearance: Option<(u16, u16)>,
) -> PeripheralKind {
    if usage_page == 0x01 && usage == 0x02 {
        return PeripheralKind::Mouse;
    }
    if usage_page == 0x01 && usage == 0x06 {
        return PeripheralKind::Keyboard;
    }
    // Bluetooth LE HID appearance: category 0x0F, subcategory 1 keyboard / 2 mouse.
    if let Some((0x0F, subcategory)) = appearance {
        match subcategory {
            0x01 => return PeripheralKind::Keyboard,
            0x02 => return PeripheralKind::Mouse,
            _ => {}
        }
    }

    let lower = name.to_lowercase();
    if ["headset", "headphone", "earbud", "airpod", "buds", "wh-", "wf-", "耳机", "耳麦", "speaker"]
        .iter()
        .any(|word| lower.contains(word))
    {
        return PeripheralKind::Headset;
    }
    if ["keyboard", "键盘"].iter().any(|word| lower.contains(word)) {
        return PeripheralKind::Keyboard;
    }
    if ["mouse", "鼠标"].iter().any(|word| lower.contains(word)) {
        return PeripheralKind::Mouse;
    }
    if ["receiver", "dongle", "接收器"].iter().any(|word| lower.contains(word)) {
        return PeripheralKind::Receiver;
    }
    PeripheralKind::Other
}
