#![forbid(unsafe_code)]

//! Tauri v2 desktop shell entry point.
//!
//! Thin client only: all business logic lives in the headless engine
//! crates. See `lib.rs` for the builder wiring.

fn main() {
    autocompare_desktop_lib::run();
}
