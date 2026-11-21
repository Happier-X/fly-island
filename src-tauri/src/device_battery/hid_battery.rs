//! Standard HID Battery Strength (Generic Device Controls 0x06 / usage 0x20).

use windows::Win32::Devices::HumanInterfaceDevice::{HidP_Feature, HidP_Input};

use super::report::scale_hid_level;
use super::{classify, PeripheralBatteryDevice, PeripheralKind};
use crate::win32::hid::{HidDevice, BATTERY_USAGE, BATTERY_USAGE_PAGE};

pub(crate) fn read() -> Vec<PeripheralBatteryDevice> {
    let mut readings = Vec::new();
    for device in HidDevice::all() {
        if ignored(&device) {
            continue;
        }
        let feature_caps: Vec<_> = device
            .value_caps(HidP_Feature)
            .into_iter()
            .filter(is_battery_value)
            .collect();
        let input_caps: Vec<_> = device
            .value_caps(HidP_Input)
            .into_iter()
            .filter(is_battery_value)
            .collect();
        if feature_caps.is_empty() && input_caps.is_empty() {
            continue;
        }

        let mut reading = None;
        for cap in &feature_caps {
            let Some(report) = device.feature_report(cap.report_id) else {
                continue;
            };
            if cap.report_id != 0 && report.first().copied() != Some(cap.report_id) {
                continue;
            }
            let value = device.usage_value(
                HidP_Feature,
                &report,
                BATTERY_USAGE_PAGE,
                BATTERY_USAGE,
            );
            if let Some(level) = value.and_then(|value| scale_hid_level(value, cap.logical_max)) {
                reading = Some(level);
                break;
            }
        }

        // Some devices only publish battery in their input reports.
        if reading.is_none() && !input_caps.is_empty() {
            if let Some(report) = device.read_report(150) {
                for cap in &input_caps {
                    if cap.report_id != 0 && report.first().copied() != Some(cap.report_id) {
                        continue;
                    }
                    let value = device.usage_value(
                        HidP_Input,
                        &report,
                        BATTERY_USAGE_PAGE,
                        BATTERY_USAGE,
                    );
                    if let Some(level) =
                        value.and_then(|value| scale_hid_level(value, cap.logical_max))
                    {
                        reading = Some(level);
                        break;
                    }
                }
            }
        }

        let Some(level) = reading else {
            continue;
        };
        let kind = classify(&device.product, device.usage_page, device.usage, None);
        // A battery collection on a controller or an internal sensor is outside this module.
        if kind == PeripheralKind::Other {
            continue;
        }
        readings.push(PeripheralBatteryDevice {
            id: format!("hid:{}", device.path),
            name: display_name(&device),
            kind,
            level,
            charging: false,
        });
    }
    readings
}

fn is_battery_value(cap: &crate::win32::hid::HidValueCap) -> bool {
    cap.usage_page == BATTERY_USAGE_PAGE && cap.usage == BATTERY_USAGE
}

fn ignored(device: &HidDevice) -> bool {
    let name = device.product.to_lowercase();
    ["touchpad", "trackpad", "触摸板", "elan", "synaptics"]
        .iter()
        .any(|word| name.contains(word))
}

fn display_name(device: &HidDevice) -> String {
    if device.product.trim().is_empty() {
        format!("HID {:04X}:{:04X}", device.vendor_id, device.product_id)
    } else {
        device.product.clone()
    }
}
