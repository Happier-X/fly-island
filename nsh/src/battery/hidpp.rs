//! Logitech HID++ battery readings for 2.4 GHz receivers.

use std::time::{Duration, Instant};

use super::report::{
    hidpp_error, hidpp_feature_index, hidpp_register_level, hidpp_status_level,
    hidpp_unified_level,
};
use super::{classify, PeripheralBatteryDevice, PeripheralKind};
use super::hid::{is_vendor_collection, HidDevice};

const FEATURE_BATTERY_STATUS: u16 = 0x1000;
const FEATURE_UNIFIED_BATTERY: u16 = 0x1004;
const HIDPP_INDICES: [u8; 7] = [0xFF, 1, 2, 3, 4, 5, 6];

pub(crate) fn read() -> Vec<PeripheralBatteryDevice> {
    let devices = HidDevice::all();
    let mut readings = Vec::new();
    for device in &devices {
        if device.vendor_id != 0x046D || !device.is_writable() || !is_vendor_collection(device) {
            continue;
        }
        let fallback_kind = receiver_kind(device, &devices);
        // Short reports (0x10) are HID++ 1.0 / short 2.0; long reports (0x11) are 2.0.
        for report_id in [0x11u8, 0x10] {
            let report_len = if report_id == 0x11 { 20 } else { 7 };
            if device.output_report_len != report_len || device.input_report_len != report_len {
                continue;
            }
            // A receiver that does not answer a feature query on its first paired slot has no
            // active device; probing the remaining slots would only wait out their timeouts.
            if call(device, report_id, 1, 0x00, 0x00, &[0x10, 0x04]).is_none() {
                continue;
            }
            for index in HIDPP_INDICES {
                let Some((level, charging)) = probe(device, report_id, index) else {
                    continue;
                };
                readings.push(PeripheralBatteryDevice {
                    id: format!(
                        "logitech:{:04X}:{:04X}:{index}",
                        device.vendor_id, device.product_id
                    ),
                    name: display_name(device, index),
                    kind: fallback_kind,
                    level,
                    charging,
                });
            }
        }
    }
    readings
}

fn probe(device: &HidDevice, report_id: u8, index: u8) -> Option<(u8, bool)> {
    if let Some(feature) = feature_index(device, report_id, index, FEATURE_UNIFIED_BATTERY) {
        let reply = call(device, report_id, index, feature, 0x00, &[])?;
        if let Some(level) = hidpp_unified_level(&reply) {
            return Some((level, false));
        }
    }
    if let Some(feature) = feature_index(device, report_id, index, FEATURE_BATTERY_STATUS) {
        let reply = call(device, report_id, index, feature, 0x00, &[])?;
        if let Some(level) = hidpp_status_level(&reply) {
            return Some((level, false));
        }
    }
    // Older receivers answer with the HID++ 1.0 register instead of feature tables.
    if report_id == 0x10 {
        if let Some(reply) = call_register(device, index, 0x0D) {
            if let Some((level, charging)) = hidpp_register_level(&reply) {
                return Some((level, charging));
            }
        }
    }
    None
}

fn feature_index(device: &HidDevice, report_id: u8, index: u8, feature: u16) -> Option<u8> {
    let reply = call(
        device,
        report_id,
        index,
        0x00,
        0x00,
        &[(feature >> 8) as u8, (feature & 0xFF) as u8],
    )?;
    if hidpp_error(&reply).is_some() {
        return None;
    }
    hidpp_feature_index(&reply)
}

fn call_register(device: &HidDevice, index: u8, register: u8) -> Option<Vec<u8>> {
    if device.output_report_len != 7 || device.input_report_len != 7 {
        return None;
    }
    let mut request = [0u8; 7];
    request[0] = 0x10;
    request[1] = index;
    request[2] = 0x81;
    request[3] = register;
    if !device.write_report(&request) {
        return None;
    }
    let deadline = Instant::now() + Duration::from_millis(80);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now()).as_millis() as u32;
        if remaining == 0 {
            return None;
        }
        let reply = device.read_report(remaining)?;
        if reply.len() < 5
            || reply[0] != 0x10
            || reply[1] != index
            || reply[2] != 0x81
            || reply[3] != register
        {
            continue;
        }
        return Some(reply);
    }
}

fn call(
    device: &HidDevice,
    report_id: u8,
    index: u8,
    feature: u8,
    function: u8,
    params: &[u8],
) -> Option<Vec<u8>> {
    let len = if report_id == 0x11 { 20 } else { 7 };
    if device.output_report_len != len || device.input_report_len != len {
        return None;
    }
    let mut request = vec![0u8; len];
    request[0] = report_id;
    request[1] = index;
    request[2] = feature;
    request[3] = (function << 4) | 0x01;
    let copy = params.len().min(len.saturating_sub(4));
    request[4..4 + copy].copy_from_slice(&params[..copy]);
    if !device.write_report(&request) {
        return None;
    }

    let deadline = Instant::now() + Duration::from_millis(80);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now()).as_millis() as u32;
        if remaining == 0 {
            return None;
        }
        let reply = device.read_report(remaining)?;
        if reply.len() < 4
            || reply[0] != report_id
            || reply[1] != index
            || reply[2] != feature
            || (reply[3] >> 4) != function
        {
            continue;
        }
        return Some(reply);
    }
}

/// A receiver with only one kind of HID child gives that child's kind to its battery row.
fn receiver_kind(device: &HidDevice, devices: &[HidDevice]) -> PeripheralKind {
    let same_receiver = |other: &&HidDevice| {
        other.vendor_id == device.vendor_id
            && other.product_id == device.product_id
            && other.usage_page == 0x01
    };
    let has_mouse = devices.iter().filter(same_receiver).any(|other| other.usage == 0x02);
    let has_keyboard = devices.iter().filter(same_receiver).any(|other| other.usage == 0x06);
    match (has_mouse, has_keyboard) {
        (true, false) => PeripheralKind::Mouse,
        (false, true) => PeripheralKind::Keyboard,
        _ => classify(&device.product, device.usage_page, device.usage, None),
    }
}

fn display_name(device: &HidDevice, index: u8) -> String {
    let product = if device.product.trim().is_empty() {
        format!("Logitech {:04X}:{:04X}", device.vendor_id, device.product_id)
    } else {
        device.product.clone()
    };
    if index == 0xFF {
        product
    } else {
        format!("{product} · {index}")
    }
}
