//! A braille dot-matrix wait spinner for slow steps (a remote clone, a scan).
//!
//! It animates on stderr so it never pollutes the report on stdout, and only
//! when stderr is a terminal — piped or redirected, it is a silent no-op. The
//! line is cleared when the spinner stops, whether that is an explicit stop or a
//! drop on an early `?` return.

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub(super) struct Spinner {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Spinner {
    /// Start spinning with `message`. On a non-terminal stderr this is a no-op.
    pub(super) fn start(message: impl Into<String>) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        if !std::io::stderr().is_terminal() {
            return Spinner { stop, handle: None };
        }
        let message = message.into();
        let color = std::env::var_os("NO_COLOR").is_none();
        let flag = stop.clone();
        let handle = thread::spawn(move || {
            let mut err = std::io::stderr();
            let mut frame = 0usize;
            while !flag.load(Ordering::Relaxed) {
                let dot = FRAMES[frame % FRAMES.len()];
                let dot = if color {
                    format!("\u{1b}[36m{dot}\u{1b}[0m")
                } else {
                    dot.to_owned()
                };
                let _ = write!(err, "\r{dot} {message}");
                let _ = err.flush();
                frame += 1;
                thread::sleep(Duration::from_millis(80));
            }
            // Erase the spinner line so the report starts clean.
            let _ = write!(err, "\r\u{1b}[2K");
            let _ = err.flush();
        });
        Spinner {
            stop,
            handle: Some(handle),
        }
    }

    /// Stop spinning and clear the line.
    pub(super) fn stop(mut self) {
        self.finish();
    }

    fn finish(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.finish();
    }
}
