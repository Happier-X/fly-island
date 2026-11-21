//! Safe wrappers around HID enumeration and report I/O.
//!
//! Peripheral battery readings use HID value caps (the standard Battery Strength usage)
//! and, for Logitech receivers, HID++ reports sent over the vendor collection. All unsafe
//! Win32 calls stay in this module; callers only see owned values and safe methods.

use std::ffi::c_void;
use std::mem::size_of;

use windows::core::PCWSTR;
use windows::Win32::Devices::DeviceAndDriverInstallation::{
    SetupDiEnumDeviceInterfaces, SetupDiGetClassDevsW, SetupDiGetDeviceInterfaceDetailW,
    DIGCF_DEVICEINTERFACE, DIGCF_PRESENT, SP_DEVICE_INTERFACE_DATA,
    SP_DEVICE_INTERFACE_DETAIL_DATA_W,
};
use windows::Win32::Devices::HumanInterfaceDevice::{
    HidD_FreePreparsedData, HidD_GetAttributes, HidD_GetFeature, HidD_GetPreparsedData,
    HidD_GetProductString, HidP_Feature, HidP_GetCaps, HidP_GetUsageValue, HidP_GetValueCaps,
    HidP_Input, GUID_DEVINTERFACE_HID, HIDD_ATTRIBUTES, HIDP_CAPS, HIDP_REPORT_TYPE,
    HIDP_STATUS_SUCCESS, HIDP_VALUE_CAPS,
};
use windows::Win32::Foundation::{
    CloseHandle, ERROR_IO_PENDING, GENERIC_READ, GENERIC_WRITE, HANDLE, WAIT_OBJECT_0,
};
use windows::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE,
    OPEN_EXISTING,
};
use windows::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

/// A HID value cap reduced to what battery readings need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HidValueCap {
    pub(crate) usage_page: u16,
    pub(crate) usage: u16,
    pub(crate) report_id: u8,
    pub(crate) logical_max: i32,
}

/// One opened HID interface. The handle, preparsed data and event are released on drop.
pub(crate) struct HidDevice {
    handle: HidHandle,
    preparsed: PreparsedData,
    pub(crate) path: String,
    pub(crate) vendor_id: u16,
    pub(crate) product_id: u16,
    pub(crate) product: String,
    pub(crate) usage_page: u16,
    pub(crate) usage: u16,
    pub(crate) input_report_len: usize,
    pub(crate) output_report_len: usize,
    pub(crate) feature_report_len: usize,
    input_value_caps: u16,
    feature_value_caps: u16,
    writable: bool,
}

impl HidDevice {
    /// Every HID interface currently present. Interfaces that cannot be opened are skipped.
    pub(crate) fn all() -> Vec<Self> {
        let mut devices = Vec::new();
        // SAFETY: the class GUID is the constant HID interface class and no window or
        // enumerator string is supplied; SetupAPI owns the returned device info set.
        let info = match unsafe {
            SetupDiGetClassDevsW(
                Some(&GUID_DEVINTERFACE_HID),
                PCWSTR::null(),
                None,
                DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
            )
        } {
            Ok(info) => info,
            Err(_) => return devices,
        };

        for index in 0.. {
            let mut interface = SP_DEVICE_INTERFACE_DATA {
                cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };
            // SAFETY: `info` comes from SetupDiGetClassDevsW, the interface is initialized
            // with its size, and the class GUID is the same constant passed above.
            if unsafe {
                SetupDiEnumDeviceInterfaces(
                    info,
                    None,
                    &GUID_DEVINTERFACE_HID,
                    index,
                    &mut interface,
                )
            }
            .is_err()
            {
                break;
            }
            let Some(path) = interface_path(info, &interface) else {
                continue;
            };
            if let Some(device) = Self::open(&path) {
                devices.push(device);
            }
        }
        devices
    }

    /// Opens one HID interface by its device interface path.
    pub(crate) fn open(path: &str) -> Option<Self> {
        let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `wide` is a NUL-terminated UTF-16 path that stays alive for the call,
        // sharing read/write access keeps the device available to its normal driver.
        let (handle, writable) = match unsafe {
            CreateFileW(
                PCWSTR(wide.as_ptr()),
                (GENERIC_READ | GENERIC_WRITE).0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                None,
            )
        } {
            Ok(handle) => (handle, true),
            Err(_) => {
                // Battery feature reports only need read access.
                // SAFETY: same path and sharing as above, with read-only access.
                let handle = unsafe {
                    CreateFileW(
                        PCWSTR(wide.as_ptr()),
                        GENERIC_READ.0,
                        FILE_SHARE_READ | FILE_SHARE_WRITE,
                        None,
                        OPEN_EXISTING,
                        FILE_FLAG_OVERLAPPED,
                        None,
                    )
                }
                .ok()?;
                (handle, false)
            }
        };
        let handle = HidHandle(handle);

        let mut attributes = HIDD_ATTRIBUTES {
            Size: size_of::<HIDD_ATTRIBUTES>() as u32,
            ..Default::default()
        };
        // SAFETY: the handle is open and `attributes` has its Size field set.
        if !unsafe { HidD_GetAttributes(handle.0, &mut attributes) } {
            return None;
        }

        let mut product_buffer = [0u16; 128];
        // SAFETY: the buffer is valid for its full byte length and the handle is open.
        let has_product = unsafe {
            HidD_GetProductString(
                handle.0,
                product_buffer.as_mut_ptr() as *mut c_void,
                size_of::<u16>() as u32 * product_buffer.len() as u32,
            )
        };
        let product = if has_product {
            let end = product_buffer
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(product_buffer.len());
            String::from_utf16_lossy(&product_buffer[..end])
        } else {
            String::new()
        };

        let mut preparsed_raw =
            windows::Win32::Devices::HumanInterfaceDevice::PHIDP_PREPARSED_DATA::default();
        // SAFETY: the handle is open; HidD_GetPreparsedData fills an opaque allocation
        // that is freed by PreparsedData's Drop.
        if !unsafe { HidD_GetPreparsedData(handle.0, &mut preparsed_raw) } {
            return None;
        }
        let preparsed = PreparsedData(preparsed_raw);
        let mut caps = HIDP_CAPS::default();
        // SAFETY: `preparsed` is the valid allocation returned above.
        if unsafe { HidP_GetCaps(preparsed.0, &mut caps) } != HIDP_STATUS_SUCCESS {
            return None;
        }

        Some(Self {
            handle,
            preparsed,
            path: path.to_string(),
            vendor_id: attributes.VendorID,
            product_id: attributes.ProductID,
            product,
            usage_page: caps.UsagePage,
            usage: caps.Usage,
            input_report_len: caps.InputReportByteLength as usize,
            output_report_len: caps.OutputReportByteLength as usize,
            feature_report_len: caps.FeatureReportByteLength as usize,
            input_value_caps: caps.NumberInputValueCaps,
            feature_value_caps: caps.NumberFeatureValueCaps,
            writable,
        })
    }

    pub(crate) fn is_writable(&self) -> bool {
        self.writable
    }

    /// Value caps for input or feature reports, reduced to the fields battery logic reads.
    pub(crate) fn value_caps(&self, report_type: HIDP_REPORT_TYPE) -> Vec<HidValueCap> {
        let count = if report_type == HidP_Input {
            self.input_value_caps
        } else if report_type == HidP_Feature {
            self.feature_value_caps
        } else {
            0
        } as usize;
        if count == 0 {
            return Vec::new();
        }
        let mut caps = vec![HIDP_VALUE_CAPS::default(); count];
        let mut length = count as u16;
        // SAFETY: the vector has `count` entries and `length` says how many it can hold;
        // the preparsed data belongs to this open device.
        let status = unsafe {
            HidP_GetValueCaps(
                report_type,
                caps.as_mut_ptr(),
                &mut length,
                self.preparsed.0,
            )
        };
        if status != HIDP_STATUS_SUCCESS {
            return Vec::new();
        }
        caps.truncate(length as usize);
        caps.iter()
            .filter(|cap| !cap.IsRange)
            .map(|cap| HidValueCap {
                usage_page: cap.UsagePage,
                // SAFETY: NotRange is the union arm for a single usage and IsRange was
                // checked above, so reading this arm is valid.
                usage: unsafe { cap.Anonymous.NotRange.Usage },
                report_id: cap.ReportID,
                logical_max: cap.LogicalMax,
            })
            .collect()
    }

    /// Reads the Battery Strength value from a report buffer, if that usage is present.
    pub(crate) fn usage_value(
        &self,
        report_type: HIDP_REPORT_TYPE,
        report: &[u8],
        usage_page: u16,
        usage: u16,
    ) -> Option<u32> {
        let mut value = 0u32;
        // SAFETY: the device is open, the report buffer is valid for its length, and
        // HidP_GetUsageValue only writes the out parameter.
        let status = unsafe {
            HidP_GetUsageValue(
                report_type,
                usage_page,
                None,
                usage,
                &mut value,
                self.preparsed.0,
                report,
            )
        };
        (status == HIDP_STATUS_SUCCESS).then_some(value)
    }

    /// Reads a feature report by id. The returned buffer is the full report, report id first.
    pub(crate) fn feature_report(&self, report_id: u8) -> Option<Vec<u8>> {
        if self.feature_report_len == 0 {
            return None;
        }
        let mut buffer = vec![0u8; self.feature_report_len];
        buffer[0] = report_id;
        // SAFETY: the buffer is exactly the feature report byte length and the handle is open.
        if unsafe {
            HidD_GetFeature(
                self.handle.0,
                buffer.as_mut_ptr() as *mut c_void,
                buffer.len() as u32,
            )
        } {
            Some(buffer)
        } else {
            None
        }
    }

    /// Reads one input report, waiting up to `timeout_ms`.
    pub(crate) fn read_report(&self, timeout_ms: u32) -> Option<Vec<u8>> {
        if self.input_report_len == 0 {
            return None;
        }
        let mut buffer = vec![0u8; self.input_report_len];
        let transferred = self.overlapped_read(&mut buffer, timeout_ms)?;
        if transferred == 0 {
            return None;
        }
        Some(buffer)
    }

    /// Writes one output report. `report` must have the output report byte length.
    pub(crate) fn write_report(&self, report: &[u8]) -> bool {
        if !self.writable
            || self.output_report_len == 0
            || report.len() != self.output_report_len
        {
            return false;
        }
        self.overlapped_write(report, 250).is_some()
    }

    fn overlapped_read(&self, buffer: &mut [u8], timeout_ms: u32) -> Option<u32> {
        let _event = EventHandle::create()?;
        let mut overlapped = OVERLAPPED {
            hEvent: _event.0,
            ..Default::default()
        };
        // SAFETY: the handle is open for overlapped I/O, `buffer` is valid for its length,
        // and `overlapped` lives until the operation is complete.
        let started = unsafe {
            ReadFile(
                self.handle.0,
                Some(buffer),
                None,
                Some(&mut overlapped),
            )
        };
        self.finish_overlapped(started, &mut overlapped, timeout_ms)
    }

    fn overlapped_write(&self, buffer: &[u8], timeout_ms: u32) -> Option<u32> {
        let _event = EventHandle::create()?;
        let mut overlapped = OVERLAPPED {
            hEvent: _event.0,
            ..Default::default()
        };
        // SAFETY: the handle is open for overlapped I/O, `buffer` is valid for its length,
        // and `overlapped` lives until the operation is complete.
        let started = unsafe {
            WriteFile(
                self.handle.0,
                Some(buffer),
                None,
                Some(&mut overlapped),
            )
        };
        self.finish_overlapped(started, &mut overlapped, timeout_ms)
    }

    fn finish_overlapped(
        &self,
        started: windows::core::Result<()>,
        overlapped: &mut OVERLAPPED,
        timeout_ms: u32,
    ) -> Option<u32> {
        match started {
            Ok(()) => {}
            Err(error) if (error.code().0 & 0xFFFF) as u32 == ERROR_IO_PENDING.0 => {
                // SAFETY: OVERLAPPED.hEvent is the manual-reset event created above.
                if unsafe { WaitForSingleObject(overlapped.hEvent, timeout_ms) } != WAIT_OBJECT_0 {
                    // SAFETY: the operation belongs to this handle and OVERLAPPED. Cancel
                    // it and wait for the cancellation, so no completion can touch the
                    // caller's buffer after this returns.
                    let mut transferred = 0u32;
                    // SAFETY: both calls use this handle and the still-live OVERLAPPED;
                    // the blocking wait ends before the buffer is returned.
                    unsafe {
                        let _ = CancelIoEx(self.handle.0, Some(overlapped));
                        let _ = GetOverlappedResult(
                            self.handle.0,
                            overlapped,
                            &mut transferred,
                            true,
                        );
                    }
                    return None;
                }
            }
            Err(_) => return None,
        }
        let mut transferred = 0u32;
        // SAFETY: the operation has completed (immediately or after the wait) and the
        // OVERLAPPED is still alive; no blocking wait is needed.
        unsafe { GetOverlappedResult(self.handle.0, overlapped, &mut transferred, false) }
            .ok()?;
        Some(transferred)
    }
}

fn interface_path(
    info: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    interface: &SP_DEVICE_INTERFACE_DATA,
) -> Option<String> {
    let mut required = 0u32;
    // SAFETY: `info` and `interface` come from SetupAPI; a null detail buffer asks for
    // the required size only and does not dereference the out pointer.
    let _ = unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            info,
            interface,
            None,
            0,
            Some(&mut required),
            None,
        )
    };
    if required == 0 {
        return None;
    }
    // The detail struct starts with a u32, so keep the buffer 4-byte aligned.
    let mut buffer = vec![0u32; (required as usize).div_ceil(4)];
    let detail = buffer.as_mut_ptr() as *mut SP_DEVICE_INTERFACE_DETAIL_DATA_W;
    // SAFETY: the buffer is at least the required size reported above; cbSize must be
    // set before SetupAPI writes the variable-length path at the end of the struct.
    unsafe {
        (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
    }
    // SAFETY: the detail buffer is large enough for the required size and the interface
    // descriptor is initialized; the path is read only after this call succeeds.
    if unsafe {
        SetupDiGetDeviceInterfaceDetailW(
            info,
            interface,
            Some(detail),
            required,
            None,
            None,
        )
    }
    .is_err()
    {
        return None;
    }
    // SAFETY: SetupAPI wrote a NUL-terminated UTF-16 path into the buffer; reading it
    // while the detail buffer is alive is valid.
    unsafe {
        let path = PCWSTR((*detail).DevicePath.as_ptr());
        path.to_string().ok()
    }
}

struct HidHandle(HANDLE);

impl Drop for HidHandle {
    fn drop(&mut self) {
        // SAFETY: this handle came from CreateFileW and is closed exactly once here.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

struct EventHandle(HANDLE);

impl EventHandle {
    fn create() -> Option<Self> {
        // SAFETY: no event attributes or name are supplied; the returned handle is
        // owned by the EventHandle and closed in Drop.
        unsafe { CreateEventW(None, true, false, PCWSTR::null()) }
            .ok()
            .map(Self)
    }
}

impl Drop for EventHandle {
    fn drop(&mut self) {
        // SAFETY: this handle came from CreateEventW and is closed exactly once here.
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}

struct PreparsedData(windows::Win32::Devices::HumanInterfaceDevice::PHIDP_PREPARSED_DATA);

impl Drop for PreparsedData {
    fn drop(&mut self) {
        // SAFETY: this allocation came from HidD_GetPreparsedData on the same thread.
        unsafe {
            let _ = HidD_FreePreparsedData(self.0);
        }
    }
}

/// Battery usages we read: Generic Device Controls (0x06) Battery Strength (0x20).
pub(crate) const BATTERY_USAGE_PAGE: u16 = 0x06;
pub(crate) const BATTERY_USAGE: u16 = 0x20;

pub(crate) fn is_vendor_collection(device: &HidDevice) -> bool {
    device.usage_page == 0xFF00
}
