#![windows_subsystem = "windows"]

use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::ptr;

mod wisco_compose {
    pub mod wisco;
}

use wisco_compose::wisco;

fn main() {
    unsafe {
        wisco::set_global_font("Segoe UI", 18);

        let mut window_hwnd = ptr::null_mut();
        let mut current_y = 20;

        if let Ok(lines) = read_lines("main.wisco") {
            for line in lines.flatten() {
                let trimmed = line.trim();

                if trimmed.starts_with("<body>") {
                    window_hwnd = wisco::create_window("Wisco Layout Engine", 800, 600);
                    let _header = wisco::create_rect(window_hwnd, 0, 0, 800, 60);
                    current_y = 80;
                } else if trimmed.starts_with("<p>") && trimmed.ends_with("</p>") {
                    if !window_hwnd.is_null() {
                        let text_content = trimmed.strip_prefix("<p>").unwrap().strip_suffix("</p>").unwrap();
                        let _text = wisco::create_text(window_hwnd, text_content, 50, current_y, 400, 30);
                        current_y += 40;
                    }
                } else if trimmed.starts_with("<button>") && trimmed.ends_with("</button>") {
                    if !window_hwnd.is_null() {
                        let btn_content = trimmed.strip_prefix("<button>").unwrap().strip_suffix("</button>").unwrap();

                        let function_id = wisco::create_function(|| {
                            windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxA(
                                ptr::null_mut(),
                                b"Clicked on HTML Button!\0".as_ptr(),
                                b"HTML Event\0".as_ptr(),
                                0,
                            );
                        });

                        let _button = wisco::create_button(window_hwnd, btn_content, 50, current_y, 150, 40, function_id);
                        current_y += 50;
                    }
                }
            }
        }

        if !window_hwnd.is_null() {
            wisco::run_app();
        }
    }
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
