use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::ptr::{addr_of, addr_of_mut}; // Импортируем макросы для безопасной работы с адресами статики
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::Graphics::Gdi::{CreateSolidBrush, HBRUSH};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, ShowWindow,
    TranslateMessage, CW_USEDEFAULT, MSG, SW_SHOW, WM_DESTROY, WS_OVERLAPPEDWINDOW,
    PostQuitMessage, RegisterClassW, WNDCLASSW, LoadCursorW, IDC_ARROW,
    WS_CHILD, WS_VISIBLE, BS_PUSHBUTTON, BS_FLAT, WM_SETFONT, WS_BORDER,
    BS_AUTOCHECKBOX, WM_COMMAND, WM_CTLCOLORSTATIC,
    WM_SIZE, SetWindowPos, SWP_NOMOVE, SWP_NOZORDER, SendMessageW,
};

const SS_LEFT: u32 = 0x00000000;

static CLASS_REGISTERED: AtomicBool = AtomicBool::new(false);
static WINDOW_COUNT: AtomicI32 = AtomicI32::new(0);

static mut CLASS_NAME_W: Vec<u16> = Vec::new();

static mut GLOBAL_FONT: HFONT = ptr::null_mut();
type HFONT = *mut std::ffi::c_void;

const MAX_FUNCTIONS: usize = 100;
static mut GLOBAL_CALLBACKS: [Option<fn()>; MAX_FUNCTIONS] = [None; MAX_FUNCTIONS];
static FUNCTION_ID_COUNTER: AtomicI32 = AtomicI32::new(1);

static mut RECT_HWND: HWND = ptr::null_mut();
static mut RECT_BRUSH: HBRUSH = 0 as HBRUSH;

fn to_utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub unsafe fn create_function(handler: fn()) -> i32 {
    let id = FUNCTION_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    if (id as usize) < MAX_FUNCTIONS {
        unsafe {
            let callbacks_ptr = addr_of_mut!(GLOBAL_CALLBACKS);
            (*callbacks_ptr)[id as usize] = Some(handler);
        }
    }
    id
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam & 0xFFFF) as usize;
            if id < MAX_FUNCTIONS {
                let callback = unsafe { (*addr_of!(GLOBAL_CALLBACKS))[id] };
                if let Some(cb) = callback {
                    cb();
                }
            }
            0
        }
        WM_SIZE => {
            let new_width = (lparam & 0xFFFF) as i32;
            unsafe {
                if !(*addr_of!(RECT_HWND)).is_null() {
                    SetWindowPos(*addr_of!(RECT_HWND), ptr::null_mut(), 0, 0, new_width, 60, SWP_NOMOVE | SWP_NOZORDER);
                }
            }
            0
        }
        WM_CTLCOLORSTATIC => {
            let child_hwnd = lparam as HWND;
            unsafe {
                if child_hwnd == *addr_of!(RECT_HWND) && *addr_of!(RECT_BRUSH) != 0 as HBRUSH {
                    return *addr_of!(RECT_BRUSH) as LRESULT;
                }
            }
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
        WM_DESTROY => {
            if WINDOW_COUNT.fetch_sub(1, Ordering::Relaxed) == 1 {
                unsafe { PostQuitMessage(0); }
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

pub unsafe fn create_window(title: &str, width: i32, height: i32) -> HWND {
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    
    if !CLASS_REGISTERED.load(Ordering::Relaxed) {
        unsafe {
            *addr_of_mut!(CLASS_NAME_W) = to_utf16("WiscoWindowClass");
            
            let wnd_class = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(window_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance,
                hIcon: ptr::null_mut(),
                hCursor: LoadCursorW(ptr::null_mut(), IDC_ARROW as *const u16),
                hbrBackground: 6 as *mut std::ffi::c_void,
                lpszMenuName: ptr::null(),
                // Используем сырой указатель напрямую, защищая от ссылок
                lpszClassName: (*addr_of!(CLASS_NAME_W)).as_ptr(),
            };
            RegisterClassW(&wnd_class);
        }
        CLASS_REGISTERED.store(true, Ordering::Relaxed);
    }

    let title_w = to_utf16(title);

    let hwnd = unsafe {
        CreateWindowExW(
            0, 
            (*addr_of!(CLASS_NAME_W)).as_ptr(),
            title_w.as_ptr(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT, CW_USEDEFAULT,
            width, height,
            ptr::null_mut(), ptr::null_mut(),
            instance, ptr::null(),
        )
    };

    if !hwnd.is_null() {
        WINDOW_COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { ShowWindow(hwnd, SW_SHOW); }
    }

    hwnd
}

pub unsafe fn create_button(parent: HWND, text: &str, x: i32, y: i32, width: i32, height: i32, action_id: i32) -> HWND {
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let button_class = to_utf16("BUTTON");
    let text_w = to_utf16(text);

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            button_class.as_ptr(),
            text_w.as_ptr(),
            WS_CHILD | WS_VISIBLE | (BS_PUSHBUTTON as u32) | (BS_FLAT as u32),
            x, y, width, height,
            parent,
            action_id as *mut std::ffi::c_void,
            instance,
            ptr::null(),
        )
    };

    unsafe {
        if !hwnd.is_null() && !(*addr_of!(GLOBAL_FONT)).is_null() {
            SendMessageW(hwnd, WM_SETFONT, *addr_of!(GLOBAL_FONT) as usize, 1);
        }
    }

    hwnd
}

pub unsafe fn create_text(parent: HWND, text: &str, x: i32, y: i32, width: i32, height: i32) -> HWND {
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let static_class = to_utf16("STATIC");
    let text_w = to_utf16(text);

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            static_class.as_ptr(),
            text_w.as_ptr(),
            WS_CHILD | WS_VISIBLE | SS_LEFT,
            x, y, width, height,
            parent,
            ptr::null_mut(),
            instance,
            ptr::null(),
        )
    };

    unsafe {
        if !hwnd.is_null() && !(*addr_of!(GLOBAL_FONT)).is_null() {
            SendMessageW(hwnd, WM_SETFONT, *addr_of!(GLOBAL_FONT) as usize, 1);
        }
    }

    hwnd
}

// Заглушаем предупреждения о неиспользуемом коде (dead_code) для функций про запас
#[allow(dead_code)]
pub unsafe fn create_edit(parent: HWND, default_text: &str, x: i32, y: i32, width: i32, height: i32) -> HWND {
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let edit_class = to_utf16("EDIT");
    let text_w = to_utf16(default_text);

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            edit_class.as_ptr(),
            text_w.as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_BORDER,
            x, y, width, height,
            parent,
            ptr::null_mut(),
            instance,
            ptr::null(),
        )
    };

    unsafe {
        if !hwnd.is_null() && !(*addr_of!(GLOBAL_FONT)).is_null() {
            SendMessageW(hwnd, WM_SETFONT, *addr_of!(GLOBAL_FONT) as usize, 1);
        }
    }

    hwnd
}

#[allow(dead_code)]
pub unsafe fn create_checkbox(parent: HWND, text: &str, x: i32, y: i32, width: i32, height: i32) -> HWND {
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let button_class = to_utf16("BUTTON");
    let text_w = to_utf16(text);

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            button_class.as_ptr(),
            text_w.as_ptr(),
            WS_CHILD | WS_VISIBLE | (BS_AUTOCHECKBOX as u32),
            x, y, width, height,
            parent,
            ptr::null_mut(),
            instance,
            ptr::null(),
        )
    };

    unsafe {
        if !hwnd.is_null() && !(*addr_of!(GLOBAL_FONT)).is_null() {
            SendMessageW(hwnd, WM_SETFONT, *addr_of!(GLOBAL_FONT) as usize, 1);
        }
    }

    hwnd
}

pub unsafe fn create_rect(parent: HWND, x: i32, i1: i32, width: i32, height: i32) -> HWND {
    let instance = unsafe { GetModuleHandleW(ptr::null()) };
    let static_class = to_utf16("STATIC");

    unsafe {
        if *addr_of!(RECT_BRUSH) == 0 as HBRUSH {
            *addr_of_mut!(RECT_BRUSH) = CreateSolidBrush(rgb(0, 120, 215));
        }
    }

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            static_class.as_ptr(),
            ptr::null(),
            WS_CHILD | WS_VISIBLE,
            x, i1, width, height,
            parent,
            ptr::null_mut(),
            instance,
            ptr::null(),
        )
    };

    unsafe {
        *addr_of_mut!(RECT_HWND) = hwnd;
    }

    hwnd
}

pub unsafe fn set_global_font(name: &str, size: i32) {
    let name_w = to_utf16(name);
    unsafe {
        *addr_of_mut!(GLOBAL_FONT) = windows_sys::Win32::Graphics::Gdi::CreateFontW(
            size, 0, 0, 0,
            windows_sys::Win32::Graphics::Gdi::FW_NORMAL as i32,
            0, 0, 0,
            windows_sys::Win32::Graphics::Gdi::DEFAULT_CHARSET as u32,
            windows_sys::Win32::Graphics::Gdi::OUT_DEFAULT_PRECIS as u32,
            windows_sys::Win32::Graphics::Gdi::CLIP_DEFAULT_PRECIS as u32,
            windows_sys::Win32::Graphics::Gdi::DEFAULT_QUALITY as u32,
            (windows_sys::Win32::Graphics::Gdi::DEFAULT_PITCH | windows_sys::Win32::Graphics::Gdi::FF_DONTCARE) as u32,
            name_w.as_ptr(),
        );
    }
}

pub unsafe fn run_app() {
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    while unsafe { GetMessageW(&mut msg, ptr::null_mut(), 0, 0) } > 0 {
        unsafe {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
