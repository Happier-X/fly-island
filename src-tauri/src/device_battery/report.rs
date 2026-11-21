//! Parsing rules for battery values. Kept free of Win32 calls so they can be unit tested.

/// Scales a HID usage value to a percentage. A report whose logical maximum is not a
/// percentage (some receivers use 0-255) is scaled; anything outside 0-100 is rejected.
pub(crate) fn scale_hid_level(value: u32, logical_max: i32) -> Option<u8> {
    if logical_max <= 0 {
        return None;
    }
    let level = if logical_max == 100 {
        i64::from(value)
    } else {
        (i64::from(value) * 100 + i64::from(logical_max) / 2) / i64::from(logical_max)
    };
    u8::try_from(level).ok().filter(|level| *level <= 100)
}

/// HID++ replies put an error code in the fifth byte when the 0x80 bit is set.
pub(crate) fn hidpp_error(report: &[u8]) -> Option<u8> {
    if report.len() < 4 || report[3] & 0x80 == 0 {
        return None;
    }
    Some(report.get(4).copied().unwrap_or(0xFF))
}

/// The feature index returned by a root-feature lookup. Short and long replies put it in
/// different positions, so probe the three slots devices are known to use.
pub(crate) fn hidpp_feature_index(report: &[u8]) -> Option<u8> {
    [6, 4, 5]
        .into_iter()
        .filter_map(|position| report.get(position).copied())
        .find(|index| *index != 0)
}

/// Battery percentage from a HID++ 2.0 unified battery (0x1004) status reply.
pub(crate) fn hidpp_unified_level(report: &[u8]) -> Option<u8> {
    if hidpp_error(report).is_some() {
        return None;
    }
    let level = *report.get(4)?;
    (level <= 100).then_some(level)
}

/// Battery percentage from a HID++ 2.0 battery status (0x1000) reply.
pub(crate) fn hidpp_status_level(report: &[u8]) -> Option<u8> {
    if hidpp_error(report).is_some() {
        return None;
    }
    let level = *report.get(4)?;
    (level <= 100).then_some(level)
}

/// Battery percentage (and charging bit) from a HID++ 1.0 register 0x0D reply.
pub(crate) fn hidpp_register_level(report: &[u8]) -> Option<(u8, bool)> {
    let value = *report.get(4)?;
    let level = value & 0x7F;
    (level <= 100).then_some((level, value & 0x80 != 0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hid_values_are_scaled_to_percentages() {
        assert_eq!(scale_hid_level(85, 100), Some(85));
        assert_eq!(scale_hid_level(255, 255), Some(100));
        assert_eq!(scale_hid_level(128, 255), Some(50));
        assert_eq!(scale_hid_level(100, 0), None);
        assert_eq!(scale_hid_level(200, 100), None);
    }

    #[test]
    fn hidpp_error_and_feature_index_are_read_from_replies() {
        assert_eq!(hidpp_error(&[0x10, 1, 0x00, 0x8F, 0x09]), Some(0x09));
        assert_eq!(hidpp_error(&[0x10, 1, 0x00, 0x00]), None);
        assert_eq!(hidpp_feature_index(&[0x10, 1, 0x00, 0x00, 0, 0, 0x05]), Some(0x05));
        assert_eq!(hidpp_feature_index(&[0x11, 1, 0x00, 0x00, 0x05]), Some(0x05));
        assert_eq!(hidpp_feature_index(&[0x10, 1, 0x00, 0x00]), None);
    }

    #[test]
    fn hidpp_battery_levels_stay_in_range() {
        assert_eq!(hidpp_unified_level(&[0x10, 1, 0x04, 0x00, 72]), Some(72));
        assert_eq!(hidpp_unified_level(&[0x10, 1, 0x04, 0x00, 120]), None);
        assert_eq!(hidpp_unified_level(&[0x10, 1, 0x04, 0x8F, 0x01]), None);
        assert_eq!(hidpp_status_level(&[0x10, 1, 0x00, 0x00, 64, 0]), Some(64));
        assert_eq!(hidpp_register_level(&[0x10, 1, 0x81, 0x0D, 0xE1]), Some((97, true)));
        assert_eq!(hidpp_register_level(&[0x10, 1, 0x81, 0x0D, 0xFF]), None);
    }
}
