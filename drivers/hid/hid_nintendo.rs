// SPDX-License-Identifier: GPL-2.0

//! Nintendo Switch Controller Support

use kernel::bindings;
use kernel::c_str;
use kernel::hid::ConnectionRequest;
use kernel::hid::DeviceKind;
use kernel::hid::HidDeviceId;
use kernel::module_hid_driver;
use kernel::prelude::*;

struct Nintendo;

const PRODUCT_JOYCON: u32 = 0x2009;

enum ControllerState {
    Init,
}

struct Controller {
    state: ControllerState,
}

fn hid_send(dev: &kernel::hid::Device, data: &[u8]) -> Result {
    let copied_data = Vec::try_with_capacity(data.len())?;
    data.clone_into(&mut copied_data);
    dev.hw_output_report(&mut copied_data)
}

fn hid_send_sync(dev: &kernel::hid::Device, data: &[u8], timeout: u32) -> Result {
    let tries = 2;

    for _ in 0..tries {
        // if this fails, return eagerly from the function
        hid_send(dev, data)?;

        
    }
}

fn is_procon(dev: &kernel::hid::Device) -> bool {
    dev.product_id() == PRODUCT_JOYCON
}

impl kernel::hid::Driver for Nintendo {
    fn probe(dev: &mut kernel::hid::Device, id: &kernel::bindings::hid_device_id) -> Result {
        let name = dev.name();
        let state = Box::try_new(Controller {
            state: ControllerState::Init,
        })?;

        pr_info!("{name}: probe!\n");
        dev.parse()?;

        pr_info!("{name}: dev.hw_start!\n");
        dev.hw_start(ConnectionRequest::HidRaw.into())?;

        pr_info!("{name}: dev.hw_open!\n");
        dev.hw_open()?;

        pr_info!("{name}: dev.io_start\n");
        dev.io_start();

        // try handshake :)

        Ok(())
    }

    fn remove(dev: &mut kernel::hid::Device) {
        let name = dev.name();
        pr_info!("{name}: remove\n");
    }

    fn raw_event(
        dev: &mut kernel::hid::Device,
        hid_report: &kernel::bindings::hid_report,
        data: &[u8],
    ) -> Result {
        let name = dev.name();
        let len = data.len();
        pr_info!("{name}: raw_event! {len}\n");
        Ok(())
    }

    const NAME: &'static CStr = c_str!("nintendo");

    const ID_TABLE: &'static [bindings::hid_device_id] = &[
        HidDeviceId {
            kind: DeviceKind::USB,
            vendor: 0x057e,
            product: 0x2009,
        }
        .to_rawid(),
        HidDeviceId {
            kind: DeviceKind::Bluetooth,
            vendor: 0x057e,
            product: 0x2009,
        }
        .to_rawid(),
        HidDeviceId {
            kind: DeviceKind::USB,
            vendor: 0x057e,
            product: 0x200e,
        }
        .to_rawid(),
        HidDeviceId {
            kind: DeviceKind::Bluetooth,
            vendor: 0x057e,
            product: 0x2006,
        }
        .to_rawid(),
        HidDeviceId {
            kind: DeviceKind::Bluetooth,
            vendor: 0x057e,
            product: 0x2007,
        }
        .to_rawid(),
        HidDeviceId::ZERO,
    ];
}

module_hid_driver! {
    type : Nintendo,
    name : "nintendo",
    author : "Matthew Else",
    description : "Nintendo Switch Controller driver.",
    license : "GPL v2",
}
