use gaudium_core::device::Usage;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::ptr;
use winapi::shared::{hidpi, hidusage, minwindef, ntdef, windef};
use winapi::um::winuser;

use crate::OpaqueBuffer;

pub trait TryFromDeviceInfo: Sized {
    fn try_from_device_info(info: &winuser::RID_DEVICE_INFO) -> Option<Self>;
}

impl TryFromDeviceInfo for Usage {
    fn try_from_device_info(info: &winuser::RID_DEVICE_INFO) -> Option<Self> {
        match info.dwType {
            winuser::RIM_TYPEMOUSE => Some(Usage::Mouse),
            winuser::RIM_TYPEKEYBOARD => Some(Usage::Keyboard),
            // TODO: Usage pages should be specified in only one place. See
            //       the `register` function.
            winuser::RIM_TYPEHID => unsafe {
                let hid = info.u.hid();
                if hid.usUsagePage == hidusage::HID_USAGE_PAGE_GENERIC {
                    match hid.usUsage {
                        hidusage::HID_USAGE_GENERIC_GAMEPAD
                        | hidusage::HID_USAGE_GENERIC_JOYSTICK => Some(Usage::GameController),
                        _ => None,
                    }
                }
                else {
                    None
                }
            },
            _ => None,
        }
    }
}

pub enum RawInput {
    Unboxed(winuser::RAWINPUT),
    Boxed(Box<winuser::RAWINPUT>),
}

impl Deref for RawInput {
    type Target = winuser::RAWINPUT;

    fn deref(&self) -> &Self::Target {
        match *self {
            RawInput::Unboxed(ref input) => input,
            RawInput::Boxed(ref input) => input,
        }
    }
}

impl DerefMut for RawInput {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match *self {
            RawInput::Unboxed(ref mut input) => input,
            RawInput::Boxed(ref mut input) => input,
        }
    }
}

pub fn register(window: windef::HWND) -> Result<(), ()> {
    // `RIDEV_DEVNOTIFY` enables `WM_INPUT_DEVICE_CHANGE` events. It seems
    // that `RIDEV_INPUTSINK` would be good to use as well, but from some
    // minimal testing it seems that these events are dispatched regardless of
    // window focus.
    let rids = [
        winuser::RAWINPUTDEVICE {
            usUsagePage: hidusage::HID_USAGE_PAGE_GENERIC,
            usUsage: hidusage::HID_USAGE_GENERIC_KEYBOARD,
            dwFlags: winuser::RIDEV_DEVNOTIFY,
            hwndTarget: window,
        },
        winuser::RAWINPUTDEVICE {
            usUsagePage: hidusage::HID_USAGE_PAGE_GENERIC,
            usUsage: hidusage::HID_USAGE_GENERIC_MOUSE,
            dwFlags: winuser::RIDEV_DEVNOTIFY,
            hwndTarget: window,
        },
        winuser::RAWINPUTDEVICE {
            usUsagePage: hidusage::HID_USAGE_PAGE_GENERIC,
            usUsage: hidusage::HID_USAGE_GENERIC_GAMEPAD,
            dwFlags: winuser::RIDEV_DEVNOTIFY,
            hwndTarget: window,
        },
        winuser::RAWINPUTDEVICE {
            usUsagePage: hidusage::HID_USAGE_PAGE_GENERIC,
            usUsage: hidusage::HID_USAGE_GENERIC_JOYSTICK,
            dwFlags: winuser::RIDEV_DEVNOTIFY,
            hwndTarget: window,
        },
    ];
    unsafe {
        if winuser::RegisterRawInputDevices(
            mem::transmute(&rids),
            rids.len() as u32,
            mem::size_of::<winuser::RAWINPUTDEVICE>() as u32,
        ) == 0
        {
            Err(())
        }
        else {
            Ok(())
        }
    }
}

pub fn raw_input_header(device: winuser::HRAWINPUT) -> Result<winuser::RAWINPUTHEADER, ()> {
    unsafe {
        let mut header = mem::uninitialized::<winuser::RAWINPUTHEADER>();
        let mut size = mem::size_of::<winuser::RAWINPUTHEADER>() as u32;
        if winuser::GetRawInputData(
            device,
            winuser::RID_HEADER,
            mem::transmute(&mut header),
            &mut size,
            mem::size_of::<winuser::RAWINPUTHEADER>() as u32,
        ) != size
        {
            Err(())
        }
        else {
            Ok(header)
        }
    }
}

pub fn raw_input(device: winuser::HRAWINPUT) -> Result<RawInput, ()> {
    // Avoid allocations by using the static size of `RAWINPUT` (48 bytes) when
    // reading generic keyboard or mouse input.
    //
    // Read the header and then determine the type of device.
    match raw_input_header(device)?.dwType {
        winuser::RIM_TYPEKEYBOARD | winuser::RIM_TYPEMOUSE => unsafe {
            let mut input = mem::uninitialized::<winuser::RAWINPUT>();
            let mut size = mem::size_of::<winuser::RAWINPUT>() as u32;
            if winuser::GetRawInputData(
                device,
                winuser::RID_INPUT,
                mem::transmute(&mut input),
                &mut size,
                mem::size_of::<winuser::RAWINPUTHEADER>() as u32,
            ) > size
            {
                Err(())
            }
            else {
                Ok(RawInput::Unboxed(input))
            }
        },
        _ => unsafe {
            let mut size = 0;
            if winuser::GetRawInputData(
                device,
                winuser::RID_INPUT,
                ptr::null_mut(),
                &mut size,
                mem::size_of::<winuser::RAWINPUTHEADER>() as u32,
            ) != 0
            {
                return Err(());
            }
            let mut buffer = OpaqueBuffer::with_size(size as usize);
            if winuser::GetRawInputData(
                device,
                winuser::RID_INPUT,
                buffer.as_mut_ptr(),
                &mut size,
                mem::size_of::<winuser::RAWINPUTHEADER>() as u32,
            ) != size
            {
                return Err(());
            }
            Ok(RawInput::Boxed(buffer.into_box()))
        },
    }
}

pub fn preparsed_data(device: ntdef::HANDLE) -> Result<Box<hidpi::HIDP_PREPARSED_DATA>, ()> {
    unsafe {
        let mut size = 0;
        if winuser::GetRawInputDeviceInfoW(
            device,
            winuser::RIDI_PREPARSEDDATA,
            ptr::null_mut(),
            &mut size,
        ) == 0
        {
            if size != 0 {
                let mut buffer = OpaqueBuffer::with_size(size as usize);
                if winuser::GetRawInputDeviceInfoW(
                    device,
                    winuser::RIDI_PREPARSEDDATA,
                    buffer.as_mut_ptr(),
                    &mut size,
                ) == size
                {
                    Ok(buffer.into_box())
                }
                else {
                    Err(())
                }
            }
            else {
                Err(())
            }
        }
        else {
            Err(())
        }
    }
}

pub fn device_info(device: ntdef::HANDLE) -> Result<Box<winuser::RID_DEVICE_INFO>, ()> {
    unsafe {
        let mut size = 0;
        if winuser::GetRawInputDeviceInfoW(
            device,
            winuser::RIDI_DEVICEINFO,
            ptr::null_mut(),
            &mut size,
        ) == 0
        {
            if size != 0 {
                let mut buffer = OpaqueBuffer::with_size(size as usize);
                if winuser::GetRawInputDeviceInfoW(
                    device,
                    winuser::RIDI_DEVICEINFO,
                    buffer.as_mut_ptr(),
                    &mut size,
                ) == size
                {
                    Ok(buffer.into_box())
                }
                else {
                    Err(())
                }
            }
            else {
                Err(())
            }
        }
        else {
            Err(())
        }
    }
}

pub fn device_name(device: ntdef::HANDLE) -> Result<String, ()> {
    unsafe {
        let mut n = 0;
        if winuser::GetRawInputDeviceInfoW(
            device,
            winuser::RIDI_DEVICENAME,
            ptr::null_mut(),
            &mut n,
        ) == 0
        {
            if n != 0 {
                let mut buffer = Vec::<u16>::with_capacity(n as usize);
                if winuser::GetRawInputDeviceInfoW(
                    device,
                    winuser::RIDI_DEVICENAME,
                    mem::transmute(buffer.as_mut_ptr()),
                    &mut n,
                ) == n
                {
                    buffer.set_len(n as usize);
                    Ok(String::from_utf16_lossy(buffer.as_slice()))
                }
                else {
                    Err(())
                }
            }
            else {
                Err(())
            }
        }
        else {
            Err(())
        }
    }
}

pub fn devices() -> Result<Vec<winuser::RAWINPUTDEVICELIST>, ()> {
    unsafe {
        let mut n = 0;
        if winuser::GetRawInputDeviceList(
            ptr::null_mut(),
            &mut n,
            mem::size_of::<winuser::RAWINPUTDEVICELIST>() as u32,
        ) == 0
        {
            let mut devices = Vec::with_capacity(n as usize);
            if winuser::GetRawInputDeviceList(
                devices.as_mut_ptr(),
                &mut n,
                mem::size_of::<winuser::RAWINPUTDEVICELIST>() as u32,
            ) == n
            {
                devices.set_len(n as usize);
                Ok(devices)
            }
            else {
                Err(())
            }
        }
        else {
            Err(())
        }
    }
}

pub fn hid_capabilities(data: &mut hidpi::HIDP_PREPARSED_DATA) -> Result<hidpi::HIDP_CAPS, ()> {
    unsafe {
        let mut capabilities = mem::uninitialized();
        if hidpi::HidP_GetCaps(data, &mut capabilities) == hidpi::HIDP_STATUS_SUCCESS {
            Ok(capabilities)
        }
        else {
            Err(())
        }
    }
}

pub fn hid_button_capabilities(
    capabilities: &hidpi::HIDP_CAPS,
    data: &mut hidpi::HIDP_PREPARSED_DATA,
) -> Result<Vec<hidpi::HIDP_BUTTON_CAPS>, ()> {
    unsafe {
        let mut n = capabilities.NumberInputButtonCaps;
        let mut buttons = Vec::with_capacity(n as usize);
        if hidpi::HidP_GetButtonCaps(hidpi::HidP_Input, buttons.as_mut_ptr(), &mut n, data)
            == hidpi::HIDP_STATUS_SUCCESS
        {
            buttons.set_len(n as usize);
            Ok(buttons)
        }
        else {
            Err(())
        }
    }
}

pub fn hid_button_count(capabilities: &hidpi::HIDP_BUTTON_CAPS) -> Result<u16, ()> {
    if capabilities.IsRange != 0 {
        unsafe {
            let range = capabilities.u.Range();
            Ok((range.UsageMax - range.UsageMin) + 1)
        }
    }
    else {
        Err(())
    }
}

pub fn read_hid_buttons(
    capabilities: &hidpi::HIDP_BUTTON_CAPS,
    input: &mut RawInput,
    data: &mut hidpi::HIDP_PREPARSED_DATA,
) -> Result<Vec<hidusage::USAGE>, ()> {
    hid_button_count(capabilities).and_then(|n| unsafe {
        let mut n = n as minwindef::ULONG;
        let mut usages = Vec::with_capacity(n as usize);
        if input.header.dwType == winuser::RIM_TYPEHID {
            if hidpi::HidP_GetUsages(
                hidpi::HidP_Input,
                capabilities.UsagePage,
                0,
                usages.as_mut_ptr(),
                &mut n,
                data,
                mem::transmute(&mut input.data.hid_mut().bRawData),
                input.data.hid().dwSizeHid,
            ) == hidpi::HIDP_STATUS_SUCCESS
            {
                usages.set_len(n as usize);
                Ok(usages)
            }
            else {
                Err(())
            }
        }
        else {
            Err(())
        }
    })
}

pub fn hid_value_capabilities(
    capabilities: &hidpi::HIDP_CAPS,
    data: &mut hidpi::HIDP_PREPARSED_DATA,
) -> Result<Vec<hidpi::HIDP_VALUE_CAPS>, ()> {
    unsafe {
        let mut n = capabilities.NumberInputValueCaps;
        let mut values = Vec::with_capacity(n as usize);
        if hidpi::HidP_GetValueCaps(hidpi::HidP_Input, values.as_mut_ptr(), &mut n, data)
            == hidpi::HIDP_STATUS_SUCCESS
        {
            values.set_len(n as usize);
            Ok(values)
        }
        else {
            Err(())
        }
    }
}

pub fn read_hid_value(
    capabilities: &hidpi::HIDP_VALUE_CAPS,
    input: &mut RawInput,
    data: &mut hidpi::HIDP_PREPARSED_DATA,
) -> Result<minwindef::ULONG, ()> {
    if capabilities.IsRange != 0 {
        unsafe {
            let mut value = 0;
            if input.header.dwType == winuser::RIM_TYPEHID {
                if hidpi::HidP_GetUsageValue(
                    hidpi::HidP_Input,
                    capabilities.UsagePage,
                    0,
                    capabilities.u.Range().UsageMin,
                    &mut value,
                    data,
                    mem::transmute(&mut input.data.hid_mut().bRawData),
                    input.data.hid().dwSizeHid,
                ) == hidpi::HIDP_STATUS_SUCCESS
                {
                    Ok(value)
                }
                else {
                    Err(())
                }
            }
            else {
                Err(())
            }
        }
    }
    else {
        Err(())
    }
}
