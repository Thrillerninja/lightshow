use image::{ImageBuffer, RgbaImage};
use winapi::um::winuser::{GetDC, ReleaseDC};
use winapi::um::wingdi::{CreateCompatibleDC, CreateCompatibleBitmap, SelectObject, BitBlt, GetDIBits, DeleteObject, DeleteDC, SRCCOPY, BI_RGB, BITMAPINFO, BITMAPINFOHEADER};
use winapi::shared::windef::HDC;
use winapi::shared::minwindef::{LPARAM, BOOL, BYTE};
use winapi::um::winuser::{EnumDisplayMonitors, MONITORINFOEXW, GetMonitorInfoW};
use winapi::shared::windef::{HMONITOR, LPRECT};
use std::ptr::null_mut;
use std::mem::zeroed;

// Struct to hold monitor information
pub struct MonitorInfo {
    width: i32,
    height: i32,
    pos_x: i32,
    pos_y: i32,
}

// Callback function for EnumDisplayMonitors
unsafe extern "system" fn monitor_enum_proc(hmonitor: HMONITOR, _: HDC, _: LPRECT, lparam: LPARAM) -> BOOL {
    let monitors = &mut *(lparam as *mut Vec<MonitorInfo>);
    let mut mi: MONITORINFOEXW = zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    GetMonitorInfoW(hmonitor, &mut mi as *mut _ as *mut _);

    monitors.push(MonitorInfo {
        width: mi.rcMonitor.right - mi.rcMonitor.left,
        height: mi.rcMonitor.bottom - mi.rcMonitor.top,
        pos_x: mi.rcMonitor.left,
        pos_y: mi.rcMonitor.top,        
    });

    1 // Continue enumeration
}

pub fn get_monitor_info() -> Vec<MonitorInfo> {
    let mut monitors: Vec<MonitorInfo> = Vec::new();
    unsafe {
        EnumDisplayMonitors(null_mut(), null_mut(), Some(monitor_enum_proc), &mut monitors as *mut _ as LPARAM);
    }
    monitors
}

pub fn capture_screenshot() -> Result<(RgbaImage, i32, i32, i32, i32), Box<dyn std::error::Error>> {
    let hdc_screen = unsafe { GetDC(null_mut()) };
    let hdc_mem = unsafe { CreateCompatibleDC(hdc_screen) };

    let monitor_info = get_monitor_info();

    let min_x = monitor_info.iter().map(|mi| mi.pos_x).min().unwrap_or(0);
    let min_y = monitor_info.iter().map(|mi| mi.pos_y).min().unwrap_or(0);
    let max_x = monitor_info.iter().map(|mi| mi.pos_x + mi.width).max().unwrap_or(0);
    let max_y = monitor_info.iter().map(|mi| mi.pos_y + mi.height).max().unwrap_or(0);

    log::info!("Monitor configuration: min_x: {}, min_y: {}, max_x: {}, max_y: {}", min_x, min_y, max_x, max_y);

    let total_width: i32 = monitor_info.iter().map(|mi| mi.width + (mi.pos_x - min_x)).max().unwrap_or(1);
    let total_height: i32 = monitor_info.iter().map(|mi| mi.height + (mi.pos_y - min_y)).max().unwrap_or(1);

    let hbitmap = unsafe { CreateCompatibleBitmap(hdc_screen, total_width, total_height) };
    unsafe { SelectObject(hdc_mem, hbitmap as _) };

    for monitor in &monitor_info {
        unsafe {
            BitBlt(
                hdc_mem,
                monitor.pos_x - min_x,
                monitor.pos_y - min_y,
                monitor.width,
                monitor.height,
                hdc_screen,
                monitor.pos_x,
                monitor.pos_y,
                SRCCOPY,
            );
        }
    }

    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: total_width,
            biHeight: -total_height, // top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        },
        bmiColors: [unsafe { zeroed() }; 1],
    };

    let mut pixels: Vec<BYTE> = vec![0; (total_width * total_height * 4) as usize];
    unsafe {
        GetDIBits(
            hdc_mem,
            hbitmap,
            0,
            total_height as u32,
            pixels.as_mut_ptr() as *mut _,
            &mut bitmap_info,
            0,
        );
    }

    // Convert from BGRA to RGBA
    for chunk in pixels.chunks_exact_mut(4) {
        let b = chunk[0];
        let r = chunk[2];
        chunk[0] = r; // swap B and R
        chunk[2] = b;
    }

    let image = ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        total_width as u32,
        total_height as u32,
        pixels,
    )
    .ok_or("Failed to create image buffer")?;

    unsafe {
        DeleteObject(hbitmap as _);
        DeleteDC(hdc_mem);
        ReleaseDC(null_mut(), hdc_screen);
    }

    Ok((image, min_x, min_y, total_width, total_height))
}