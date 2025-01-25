use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::{self, Sender, Receiver};

mod gui;
mod backend;
mod screen_capture;
mod arduino;
mod logger;
mod config;
mod hardware_interaction;

struct SharedState {
    value: i32,
    is_active: bool,
}

fn main() {
    // Initialize the shared state
    let shared_state = Arc::new(Mutex::new(SharedState { value: 0, is_active: true }));

    // Clone the shared state for the backend
    let backend_state = Arc::clone(&shared_state);
    let backend_thread = thread::spawn(move || {
        // Start the backend
        backend::main_program_start(backend_state).unwrap();
    });

    // Initialize the UI on the main thread
    gui::start_ui(shared_state).unwrap();

    // Wait for the backend thread to finish
    backend_thread.join().unwrap();
}