#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    chatgpt_subscription_lens_lib::run()
}
