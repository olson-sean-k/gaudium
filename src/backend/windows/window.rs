use lazy_static::lazy_static;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::mem;
use std::panic;
use std::ptr;
use winapi::shared::{basetsd, minwindef, ntdef, windef};
use winapi::um::{commctrl, libloaderapi, winuser};

use crate::backend::windows;
use crate::backend::windows::reactor::ThreadContext;
use crate::backend::windows::WideNullTerminated;
use crate::backend::windows::{input, keyboard, mouse, reactor};
use crate::backend::{FromRawHandle, IntoHandle, IntoRawHandle, RawHandle};
use crate::device::Usage;
use crate::display::{IntoLogical, IntoPhysical, LogicalUnit};
use crate::event::*;

const WINDOW_SUBCLASS_ID: basetsd::UINT_PTR = 0;

lazy_static! {
    static ref WM_DROP: minwindef::UINT =
        unsafe { winuser::RegisterWindowMessageA("WM_DROP".as_ptr() as ntdef::LPCSTR) };
    static ref WINDOW_CLASS_NAME: Vec<ntdef::WCHAR> = {
        let name = "GAUDIUM_WINDOW_CLASS".wide_null_terminated();
        unsafe {
            let class = winuser::WNDCLASSEXW {
                cbSize: mem::size_of::<winuser::WNDCLASSEXW>() as minwindef::UINT,
                style: winuser::CS_HREDRAW | winuser::CS_VREDRAW | winuser::CS_OWNDC,
                lpfnWndProc: Some(winuser::DefWindowProcW),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: libloaderapi::GetModuleHandleW(ptr::null()),
                hIcon: ptr::null_mut(),
                hCursor: ptr::null_mut(),
                hbrBackground: ptr::null_mut(),
                lpszMenuName: ptr::null(),
                lpszClassName: name.as_ptr(),
                hIconSm: ptr::null_mut(),
            };
            winuser::RegisterClassExW(&class);
        }
        name
    };
}

impl RawHandle for windef::HWND {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WindowHandle(windef::HWND);

impl FromRawHandle<windef::HWND> for WindowHandle {
    fn from_raw_handle(handle: windef::HWND) -> Self {
        WindowHandle(handle)
    }
}

impl IntoRawHandle<windef::HWND> for WindowHandle {
    fn into_raw_handle(self) -> windef::HWND {
        self.0
    }
}

unsafe impl Send for WindowHandle {}
unsafe impl Sync for WindowHandle {}

// TODO: This will typically leak given the current structure of window
//       destruction.
#[derive(Debug, Default)]
pub struct WindowState;

pub struct WindowBuilder {
    title: String,
    dimensions: (u32, u32),
    // TODO: This should be more than a simple `bool`. A display must be
    //       targeted, for example. Improve this once it is possible to query
    //       and target displays.
    exclusive: bool,
    parent: Option<WindowHandle>,
}

impl WindowBuilder {
    pub fn build(self, context: &ThreadContext) -> Result<Window, ()> {
        Window::new(self, context)
    }

    pub fn with_title(&mut self, title: &str) -> &mut Self {
        self.title = title.to_owned();
        self
    }

    pub fn with_dimensions<T>(&mut self, dimensions: (T, T)) -> &mut Self
    where
        T: Into<LogicalUnit>,
    {
        let dpi = 1.0; // TODO: Get the DPI factor.
        let (width, height) = dimensions.into_physical(dpi);
        self.dimensions = (width.into(), height.into());
        self
    }

    fn with_parent_window(&mut self, parent: &Window) -> &mut Self {
        self.parent = Some(parent.handle());
        self
    }
}

impl Default for WindowBuilder {
    fn default() -> Self {
        WindowBuilder {
            title: String::new(),
            dimensions: (640, 480),
            exclusive: false,
            parent: None,
        }
    }
}

pub struct Window {
    handle: WindowHandle,
    children: HashSet<Window>,
}

impl Window {
    fn new(builder: WindowBuilder, _: &ThreadContext) -> Result<Self, ()> {
        let WindowBuilder {
            ref title,
            dimensions,
            exclusive: _,
            mut parent,
        } = builder;
        let (parent, style, extended_style) = if let Some(parent) = parent.take() {
            (
                parent.into_raw_handle(),
                winuser::WS_CAPTION | winuser::WS_CHILD | winuser::WS_VISIBLE,
                winuser::WS_EX_WINDOWEDGE,
            )
        }
        else {
            (
                ptr::null_mut(),
                winuser::WS_CLIPCHILDREN
                    | winuser::WS_CLIPSIBLINGS
                    | winuser::WS_OVERLAPPEDWINDOW
                    | winuser::WS_VISIBLE,
                winuser::WS_EX_APPWINDOW | winuser::WS_EX_WINDOWEDGE,
            )
        };
        let rectangle = unsafe {
            let mut rectangle = windef::RECT {
                left: 0,
                top: 0,
                right: dimensions.0 as ntdef::LONG,
                bottom: dimensions.1 as ntdef::LONG,
            };
            winuser::AdjustWindowRectEx(&mut rectangle, style, 0, extended_style);
            rectangle
        };
        let handle = unsafe {
            let handle = winuser::CreateWindowExW(
                extended_style,
                WINDOW_CLASS_NAME.as_ptr(),
                title.wide_null_terminated().as_ptr() as ntdef::LPCWSTR,
                style,
                winuser::CW_USEDEFAULT,
                winuser::CW_USEDEFAULT,
                rectangle.right - rectangle.left,
                rectangle.bottom - rectangle.top,
                parent,
                ptr::null_mut(),
                libloaderapi::GetModuleHandleW(ptr::null()),
                ptr::null_mut(),
            );
            let state = Box::into_raw(Box::new(WindowState::default()));
            if commctrl::SetWindowSubclass(
                handle,
                Some(procedure),
                WINDOW_SUBCLASS_ID,
                state as basetsd::DWORD_PTR,
            ) == 0
            {
                return Err(());
            }
            handle.into_handle()
        };
        input::register(handle).unwrap();
        Ok(Window {
            handle,
            children: HashSet::new(),
        })
    }

    pub fn insert(
        &mut self,
        mut builder: WindowBuilder,
        context: &ThreadContext,
    ) -> Result<(), ()> {
        builder.with_parent_window(self);
        builder
            .build(context)
            .map(|window| self.children.insert(window))
            .map(|_| ())
    }

    pub fn handle(&self) -> WindowHandle {
        self.handle
    }

    pub fn transform<T>(&self, position: (T, T)) -> Result<(LogicalUnit, LogicalUnit), ()>
    where
        T: Into<LogicalUnit>,
    {
        let dpi = 1.0; // TODO: Get the DPI factor.
        let (x, y) = position.into_physical(dpi);
        let mut point = windef::POINT {
            x: x.into(),
            y: y.into(),
        };
        unsafe {
            if winuser::ScreenToClient(self.handle.into_raw_handle(), &mut point) != 0 {
                Ok((point.x as i32, point.y as i32).into_logical(dpi))
            }
            else {
                Err(())
            }
        }
    }

    pub fn class_name(&self) -> &[ntdef::WCHAR] {
        WINDOW_CLASS_NAME.as_slice()
    }

    pub fn raw_handle(&self) -> windef::HWND {
        self.handle.into_raw_handle()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            winuser::PostMessageW(self.handle.into_raw_handle(), *WM_DROP, 0, 0);
        }
    }
}

impl Eq for Window {}

impl Hash for Window {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.handle.hash(state);
    }
}

impl PartialEq for Window {
    fn eq(&self, other: &Self) -> bool {
        self.handle.eq(&other.handle)
    }
}

unsafe impl Send for Window {}
unsafe impl Sync for Window {}

extern "system" fn procedure(
    window: windef::HWND,
    message: minwindef::UINT,
    wparam: minwindef::WPARAM,
    lparam: minwindef::LPARAM,
    _: basetsd::UINT_PTR,
    state: basetsd::DWORD_PTR,
) -> minwindef::LRESULT {
    // TODO: Is there some way to avoid this overhead? Perhaps an optional and
    //       unsafe `NoPanic` trait for reactors?
    // TODO: Depending on how this should terminate, this may not require
    //       `catch_unwind` after https://github.com/rust-lang/rust/pull/55982
    //       lands.
    match panic::catch_unwind(move || unsafe {
        let state = &mut *(state as *mut WindowState);
        match message {
            winuser::WM_CLOSE => {
                let _ = reactor::react(Event::Window {
                    window: window.into_handle(),
                    event: WindowEvent::Closed(WindowCloseState::Requested),
                });
                return 0; // Do NOT destroy the window yet.
            }
            // TODO: This will typically not execute (for the last window)
            //       given the current structure of window destruction.
            winuser::WM_DESTROY => {
                let _ = Box::from_raw(state);
                let _ = reactor::react(Event::Window {
                    window: window.into_handle(),
                    event: WindowEvent::Closed(WindowCloseState::Committed),
                });
            }
            winuser::WM_INPUT => {
                if let Ok(mut input) = input::raw_input(lparam as winuser::HRAWINPUT) {
                    let device = input.header.hDevice.into_handle();
                    match input.header.dwType {
                        winuser::RIM_TYPEKEYBOARD => {
                            if let Ok(event) = keyboard::parse_raw_input(input.data.keyboard()) {
                                let _ = reactor::react(Event::Input {
                                    device,
                                    window: None,
                                    event,
                                });
                            }
                        }
                        winuser::RIM_TYPEMOUSE => {
                            if let Ok(events) = mouse::parse_raw_input(window, input.data.mouse()) {
                                let _ = reactor::enqueue(events.into_iter().map(|event| {
                                    Event::Input {
                                        device,
                                        window: None,
                                        event,
                                    }
                                }));
                            }
                        }
                        // TODO: Enqueue events for game controllers.
                        // TODO: Marshal game controller data.
                        winuser::RIM_TYPEHID => {
                            if let Ok(mut data) = input::preparsed_data(device) {
                                let _ = input::hid_capabilities(&mut data)
                                    .and_then(|capabilities| {
                                        input::hid_button_capabilities(&capabilities, &mut data)
                                    })
                                    .map(|capabilities| {
                                        for capability in capabilities {
                                            let _ = input::read_hid_buttons(
                                                &capability,
                                                &mut input,
                                                &mut data,
                                            );
                                        }
                                    });
                            }
                        }
                        _ => {}
                    }
                }
            }
            winuser::WM_INPUT_DEVICE_CHANGE => {
                let device = (lparam as ntdef::HANDLE).into_handle();
                let _ = reactor::react(Event::Input {
                    device,
                    window: Some(window.into_handle()),
                    event: if (wparam as minwindef::DWORD) == winuser::GIDC_ARRIVAL {
                        InputEvent::Connected {
                            usage: input::device_info(device)
                                .ok()
                                .and_then(|info| Usage::from_device_info(&info)),
                        }
                    }
                    else {
                        InputEvent::Disconnected
                    },
                });
            }
            // Handle application-specific messages.
            _ => {
                if message == *WM_DROP {
                    winuser::DestroyWindow(window);
                }
            }
        }
        commctrl::DefSubclassProc(window, message, wparam, lparam)
    }) {
        Ok(result) => result,
        Err(_) => {
            // All bets are off. Kill the process.
            windows::exit_process(1)
        }
    }
}
