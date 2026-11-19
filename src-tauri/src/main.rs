// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--codex-mcp") {
        std::process::exit(agent_desk_lib::codex_delivery::start_stdio_connector(
            &args[1..],
        ));
    }
    agent_desk_lib::run();
}
