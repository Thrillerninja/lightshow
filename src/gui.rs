use eframe::egui;
use image::GenericImageView;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use tray_icon::{Icon, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{SetWindowLongW, SetWindowPos, ShowWindow, GWL_STYLE, SW_HIDE, SW_SHOWDEFAULT, WS_POPUP, HWND_TOPMOST};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use crate::backend::main_program_start;
use crate::{logger, SharedState};

static VISIBLE: Mutex<bool> = Mutex::new(false);

pub fn start_ui(shared_state: Arc<Mutex<SharedState>>) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    logger::init_logger()?;

    let _tray_icon = gen_tray_icon()?;

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([100.0, 100.0]).with_position([-1000.0, -1000.0]),
        vsync: true,
        multisampling: 1,
        depth_buffer: 0,
        stencil_buffer: 0,
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Lightshow",
        options,
        Box::new(move |cc| {
            let RawWindowHandle::Win32(handle) = cc.window_handle().unwrap().as_raw() else {
                panic!("Unsupported platform");
            };

            let window_handle = HWND(handle.hwnd.into());

            TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
                match event {
                    TrayIconEvent::Click {
                        button_state: MouseButtonState::Down,
                        position,
                        ..
                    } => {
                        let mut visible = VISIBLE.lock().unwrap();

                        if *visible {
                            unsafe {
                                ShowWindow(window_handle, SW_HIDE);
                            }
                            *visible = false;
                        } else {
                            unsafe {
                                SetWindowLongW(window_handle, GWL_STYLE, WS_POPUP.0 as i32);
                                let _ = SetWindowPos(
                                    window_handle,
                                    HWND_TOPMOST,
                                    position.x as i32 - 60,
                                    position.y as i32 - 10 - 100,
                                    80,
                                    100,
                                    windows::Win32::UI::WindowsAndMessaging::SET_WINDOW_POS_FLAGS(0),
                                );
                                ShowWindow(window_handle, SW_SHOWDEFAULT);
                            }
                            *visible = true;
                        }
                    }
                    TrayIconEvent::Leave { .. } => {
                        let mut visible = VISIBLE.lock().unwrap();
                        if *visible {
                            unsafe {
                                ShowWindow(window_handle, SW_HIDE);
                            }
                            *visible = false;
                        }
                    }
                    _ => {}
                }
            }));

            let start_button_handler = {
                let shared_state = Arc::clone(&shared_state);
                move || {
                    println!("Start button clicked");
                    let mut state = shared_state.lock().unwrap();
                    state.is_active = true; // Activate
                    drop(state); // Release the lock promptly
                }
            };
            let stop_button_handler = {
                let shared_state = Arc::clone(&shared_state);
                move || {
                    println!("Stop button clicked");
                    let mut state = shared_state.lock().unwrap();
                    state.is_active = false; // Deactivate
                    drop(state); // Release the lock promptly
                }
            };
            Box::new(MyApp {
                start_button_handler: Box::new(start_button_handler),
                stop_button_handler: Box::new(stop_button_handler),
            })
        }),
    );

    Ok(())
}

struct MyApp {
    start_button_handler: Box<dyn Fn() + Send>,
    stop_button_handler: Box<dyn Fn() + Send>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                if ui.add_sized([80.0, 30.0], egui::Button::new("Start")).clicked() {
                    (self.start_button_handler)();
                }

                if ui.add_sized([80.0, 30.0], egui::Button::new("Stop")).clicked() {
                    (self.stop_button_handler)();
                }

                if ui.add_sized([80.0, 30.0], egui::Button::new("Quit")).clicked() {
                    std::process::exit(0);
                }
            });
        });
    }
}

fn gen_tray_icon() -> Result<TrayIcon, Box<dyn std::error::Error>> {
    let icon = if let Ok(img) = image::open("res/icon_16x16.png") {
        let (width, height) = img.dimensions();
        let rgba = img.to_rgba8().into_raw();
        Icon::from_rgba(rgba, width, height)?
    } else {
        let mut icon_data: Vec<u8> = Vec::with_capacity(16 * 16 * 4);
        for _ in 0..256 {
            icon_data.extend_from_slice(&[255, 0, 0, 255]);
        }
        Icon::from_rgba(icon_data, 16, 16)?
    };

    let tray_icon = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("My App")
        .build()?;

    Ok(tray_icon)
}
