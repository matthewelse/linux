// SPDX-License-Identifier: GPL-2.0

//! HID devices and drivers.
//!
//! C header: [`include/linux/hid.h`](../../../../include/linux/hid.h)

use core::slice::from_raw_parts;

use bindings::hid_report;

use crate::{
    bindings, device, driver,
    error::{from_kernel_result, Result},
    str::CStr,
    to_result, ThisModule,
};

/// An adapter for registration of HID devices.
pub struct Adapter<T: Driver>(T);

impl<T: Driver> driver::DriverOps for Adapter<T> {
    type RegType = bindings::hid_driver;

    unsafe fn register(
        reg: *mut bindings::hid_driver,
        name: &'static CStr,
        module: &'static ThisModule,
    ) -> Result {
        // SAFETY: By the safety requirements of this function (defined in the trait definition),
        // `reg` is non-null and valid.
        let hid = unsafe { &mut *reg };

        hid.name = name.as_char_ptr();
        hid.probe = Some(Self::probe_callback);
        hid.remove = Some(Self::remove_callback);
        hid.raw_event = Some(Self::raw_event_callback);
        hid.id_table = T::ID_TABLE.as_ptr();

        // SAFETY:
        //   - `reg` lives at least until the call to `hid_unregister_driver()` returns.
        //   - `name` pointer has static lifetime.
        //   - `module.0` lives at least as long as the module.
        //   - `probe()`, `remove()`, and `raw_event`  are static functions.
        //   - `id_table` is a raw pointer with static lifetime ,as guaranteed by the type of [`driver::ID_TABLE`]
        to_result(unsafe { bindings::__hid_register_driver(reg, module.0, name.as_char_ptr()) })
    }

    unsafe fn unregister(reg: *mut bindings::hid_driver) {
        // SAFETY: By the safety requirements of this function (defined in the trait definition),
        // `reg` was passed (and updated) by a previous successful call to
        // `__hid_register_driver`.
        unsafe { bindings::hid_unregister_driver(reg) };
    }
}

impl<T: Driver> Adapter<T> {
    extern "C" fn probe_callback(
        hid: *mut bindings::hid_device,
        raw_id: *const bindings::hid_device_id,
    ) -> core::ffi::c_int {
        from_kernel_result! {
                // SAFETY: `hid` is valid by the contract with the C code. `dev`
                // is alive only for the duration of this call, so it is
                // guaranteed to remain alive for the lifetime of `hid`.
                let mut dev = unsafe { Device::from_ptr(hid) };

                // SAFETY: `raw_id` is valid by the contract  with the c code.
                // `id` only lives for the duration of this function call.
                let id = unsafe { raw_id.as_ref().unwrap() };

                T::probe(&mut dev, id)?;
                Ok(0)
        }
    }

    extern "C" fn remove_callback(hid: *mut bindings::hid_device) {
        // SAFETY: `hid` is valid by the contract with the C code. `dev` is
        // alive only for the duration of this call, so it is guaranteed to
        // remain alive for the lifetime of `hid`.
        let mut dev = unsafe { Device::from_ptr(hid) };
        T::remove(&mut dev);
    }

    extern "C" fn raw_event_callback(
        hid: *mut bindings::hid_device,
        hid_report: *mut hid_report,
        raw_data: *mut u8,
        size: i32,
    ) -> core::ffi::c_int {
        from_kernel_result! {
            // SAFETY: `hid` is valid by the contract with the C code. `dev` is
            // alive only for the duration of this call, so it is guaranteed to
            // remain alive for the lifetime of `hid`.
            let mut dev = unsafe { Device::from_ptr(hid) };

            // SAFETY: `data` and `size` are valid from the contract with the C
            // code. `data` is alive only for the duration of this call.
            let data: &[u8] = unsafe { from_raw_parts(raw_data, size as usize) };

            // SAFETY: `hid_report` is valid from the contract with the C code,
            // and this reference only lives for the duration of this function
            // call.
            let hid_report = unsafe { hid_report.as_ref().unwrap() };

            T::raw_event(&mut dev, hid_report, data)?;

            Ok(0)
        }
    }
}

/// An HID device
/// 
/// # Invariants
/// 
/// The field `ptr` is non-null and valid for the lifetime of the object.
pub struct Device {
    ptr: *mut bindings::hid_device,
}

impl Device {
    /// Creates a new device from the given pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be non-null and valid. It must remain valid for the lifetime of the returned
    /// instance.
    unsafe fn from_ptr(ptr: *mut bindings::hid_device) -> Self {
        // INVARIANT: The safety requirements of the function ensure the lifetime invariant.
        Self { ptr }
    }

    /// Returns the name of this HID device.
    pub fn name(&self) -> &CStr {
        // SAFETY: self.ptr has the same lifetime as self.
        unsafe { CStr::from_char_ptr((*self.ptr).name.as_ptr()) }
    }
}

// SAFETY: The device returned by `raw_device` is the raw platform deggice.
unsafe impl device::RawDevice for Device {
    fn raw_device(&self) -> *mut bindings::device {
        // SAFETY: By the type invariants, we know that `self.ptr` is non-null and valid.
        unsafe { &mut (*self.ptr).dev }
    }
}

/// A kind of HID device (i.e. a bus that might connect us to an HID device)
#[derive(Copy, Clone, Debug)]
pub enum DeviceKind {
    /// An HID device connected via Bluetooth.
    Bluetooth = 5,
    /// An HID device connected via USB.
    USB = 3,
}

impl DeviceKind {
    const fn bus_id(self) -> u16 {
        (match self {
            Self::Bluetooth => bindings::BUS_BLUETOOTH,
            Self::USB => bindings::BUS_USB,
        }) as u16
    }
}

/// An HID device ID.
#[derive(Copy, Clone, Debug)]
pub struct HidDeviceId {
    /// Indicates the bus used to connect to this HID device.
    pub kind: DeviceKind,
    /// The USB vendor ID of this HID device.
    pub vendor: u16,
    /// The USB product ID of this HID device.
    pub product: u16,
}

impl HidDeviceId {
    /// The "null" HID device. Indicates the end of a list of HID devices.
    pub const ZERO: bindings::hid_device_id = bindings::hid_device_id {
        bus: 0,
        group: 0,
        vendor: 0,
        product: 0,
        driver_data: 0,
    };

    /// Converts this HID device ID to the internal representation used in the kernel.
    pub const fn to_rawid(self) -> bindings::hid_device_id {
        let HidDeviceId {
            kind,
            vendor,
            product,
        } = self;

        bindings::hid_device_id {
            bus: kind.bus_id(),
            vendor: vendor as u32,
            product: product as u32,
            ..Self::ZERO
        }
    }
}

/// Indicates how to implement a HID device driver.
pub trait Driver {
    /// The name of this HID driver.
    const NAME: &'static CStr;

    /// A table of HID devices that may be handled by this driver. When devices
    /// matching the table provided are connected, `probe` will be called.
    const ID_TABLE: &'static [bindings::hid_device_id];

    /// Called when a new device is inserted.
    fn probe(dev: &mut Device, id: &bindings::hid_device_id) -> Result;

    /// Called when a device is removed.
    fn remove(dev: &mut Device);

    /// Called when an HID report arrives.
    fn raw_event(dev: &mut Device, hid_report: &bindings::hid_report, data: &[u8]) -> Result;
}

/// Define an HID driver module.
/// 
/// TODO: example
#[macro_export]
macro_rules! module_hid_driver {
    ($($f:tt)*) => {
        $crate::module_driver!(<T>, $crate::hid::Adapter<T>, { $($f)* });
    };
}
