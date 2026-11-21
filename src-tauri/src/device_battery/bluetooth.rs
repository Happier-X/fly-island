//! Bluetooth LE Battery Service (0x180F / 0x2A19).

use std::future::IntoFuture;

use windows::core::GUID;
use windows::Devices::Bluetooth::BluetoothLEDevice;
use windows::Devices::Bluetooth::GenericAttributeProfile::{
    GattCommunicationStatus, GattDeviceService,
};
use windows::Devices::Enumeration::{DeviceAccessStatus, DeviceInformation};
use windows::Storage::Streams::DataReader;

use super::{classify, PeripheralBatteryDevice};

const BATTERY_SERVICE: GUID = GUID::from_u128(0x0000180F_0000_1000_8000_00805F9B34FB);
const BATTERY_LEVEL: GUID = GUID::from_u128(0x00002A19_0000_1000_8000_00805F9B34FB);

pub(crate) fn read() -> Vec<PeripheralBatteryDevice> {
    let mut readings = Vec::new();
    let Ok(selector) = GattDeviceService::GetDeviceSelectorFromUuid(BATTERY_SERVICE) else {
        return readings;
    };
    let Ok(found) = DeviceInformation::FindAllAsyncAqsFilter(&selector)
        .and_then(|operation| pollster::block_on(operation.into_future()))
    else {
        return readings;
    };
    let Ok(count) = found.Size() else {
        return readings;
    };

    for index in 0..count {
        let Ok(info) = found.GetAt(index) else {
            continue;
        };
        let Ok(id) = info.Id() else {
            continue;
        };
        let Ok(service) = GattDeviceService::FromIdAsync(&id)
            .and_then(|operation| pollster::block_on(operation.into_future()))
        else {
            continue;
        };
        let name = info
            .Name()
            .map(|name| name.to_string())
            .unwrap_or_default();
        let appearance = appearance(&service);
        let Some(level) = battery_level(&service) else {
            continue;
        };
        let name = if name.trim().is_empty() {
            "Bluetooth device".to_string()
        } else {
            name
        };
        readings.push(PeripheralBatteryDevice {
            id: format!("ble:{}", id.to_string_lossy()),
            name: name.clone(),
            kind: classify(&name, 0, 0, appearance),
            level,
            charging: false,
        });
    }
    readings
}

fn battery_level(service: &GattDeviceService) -> Option<u8> {
    let mut result = read_characteristic(service)?;
    if result.Status().ok()? != GattCommunicationStatus::Success {
        // Some systems deny GATT reads until the device access is requested explicitly.
        let access = pollster::block_on(service.RequestAccessAsync().ok()?.into_future()).ok()?;
        if access != DeviceAccessStatus::Allowed {
            return None;
        }
        result = read_characteristic(service)?;
    }
    let value = result.Value().ok()?;
    let reader = DataReader::FromBuffer(&value).ok()?;
    let level = reader.ReadByte().ok()?;
    (level <= 100).then_some(level)
}

fn read_characteristic(
    service: &GattDeviceService,
) -> Option<windows::Devices::Bluetooth::GenericAttributeProfile::GattReadResult> {
    let characteristics = pollster::block_on(
        service
            .GetCharacteristicsForUuidAsync(BATTERY_LEVEL)
            .ok()?
            .into_future(),
    )
    .ok()?;
    if characteristics.Status().ok()? != GattCommunicationStatus::Success {
        return None;
    }
    let list = characteristics.Characteristics().ok()?;
    let count = list.Size().ok()?;
    for index in 0..count {
        let Ok(characteristic) = list.GetAt(index) else {
            continue;
        };
        let Ok(result) = characteristic
            .ReadValueAsync()
            .and_then(|operation| pollster::block_on(operation.into_future()))
        else {
            continue;
        };
        if result.Status().ok()? == GattCommunicationStatus::Success {
            return Some(result);
        }
    }
    None
}

fn appearance(service: &GattDeviceService) -> Option<(u16, u16)> {
    let id = service.DeviceId().ok()?;
    let device = pollster::block_on(BluetoothLEDevice::FromIdAsync(&id).ok()?.into_future()).ok()?;
    let appearance = device.Appearance().ok()?;
    Some((appearance.Category().ok()?, appearance.SubCategory().ok()?))
}
