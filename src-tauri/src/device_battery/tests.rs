use super::*;

fn device(id: &str, name: &str, kind: PeripheralKind, level: u8) -> PeripheralBatteryDevice {
    PeripheralBatteryDevice {
        id: id.to_string(),
        name: name.to_string(),
        kind,
        level,
        charging: false,
    }
}

#[test]
fn devices_are_classified_from_usage_appearance_and_name() {
    assert_eq!(classify("USB Receiver", 0x01, 0x02, None), PeripheralKind::Mouse);
    assert_eq!(classify("键盘", 0, 0, None), PeripheralKind::Keyboard);
    assert_eq!(classify("keycool 5.0", 0, 0, Some((0x0F, 0x01))), PeripheralKind::Keyboard);
    assert_eq!(classify("MCHOSE G3 A", 0, 0, Some((0x0F, 0x02))), PeripheralKind::Mouse);
    assert_eq!(classify("WH-1000XM5", 0, 0, None), PeripheralKind::Headset);
    assert_eq!(classify("LIGHTSPEED Receiver", 0, 0, None), PeripheralKind::Receiver);
    assert_eq!(classify("Some HID", 0, 0, None), PeripheralKind::Other);
}

#[test]
fn readings_are_sorted_and_deduplicated_by_name() {
    let readings = normalize(vec![
        device("ble:1", "MCHOSE G3 A", PeripheralKind::Mouse, 80),
        device("ble:2", "keycool 5.0", PeripheralKind::Keyboard, 60),
        device("hid:3", "MCHOSE G3 A", PeripheralKind::Mouse, 81),
        device("hid:4", "  ", PeripheralKind::Other, 30),
        device("hid:5", "too high", PeripheralKind::Mouse, 101),
    ]);
    assert_eq!(readings.len(), 3);
    assert_eq!(readings[0].name, "MCHOSE G3 A");
    assert_eq!(readings[0].level, 80);
    assert_eq!(readings[1].name, "keycool 5.0");
    assert_eq!(readings[2].name, "HID device");
}

#[test]
#[ignore = "reads the peripherals connected to this machine"]
fn probe_connected_peripherals() {
    let readings = read_batteries();
    println!("{readings:#?}");
    assert!(readings.iter().all(|device| device.level <= 100));
}
