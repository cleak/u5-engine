//! Windowed launcher: the default launch (setup wizard if needed, then the
//! windowed game) with no console window on Windows. Startup errors go to a
//! native message box and the log file, since there is no stdout.

#![cfg_attr(windows, windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    u5_tui::run_gui_main()
}
