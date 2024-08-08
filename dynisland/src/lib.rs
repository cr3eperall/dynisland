//! This crate manages the dynisland window(s),
//! brings together the modules,
//! handles the configuration file
//! and manages the app lifecycle
//!

pub mod app;
pub mod config;
pub mod layout_manager;
pub mod module_loading;

// fn start_fps_counter(window: &gtk::ApplicationWindow, log_level: log::Level, update_interval: Duration) {
//     let window=window.clone();
//     let tick: Box<(dyn FnMut() -> glib::ControlFlow + 'static)> = match log_level {
//         log::Level::Debug => {
//             Box::new(move || {
//                 let fps=window.frame_clock().unwrap().fps();
//                 log::debug!("fps: {}", fps);
//                 glib::ControlFlow::Continue
//             })
//         }
//         log::Level::Info => {
//             Box::new(move || {
//                 let fps=window.frame_clock().unwrap().fps();
//                 log::info!("fps: {}", fps);
//                 glib::ControlFlow::Continue
//             })
//         }
//         log::Level::Warn => {
//             Box::new(move || {
//                 let fps=window.frame_clock().unwrap().fps();
//                 log::warn!("fps: {}", fps);
//                 glib::ControlFlow::Continue
//             })
//         }
//         log::Level::Error => {
//             Box::new(move || {
//                 let fps=window.frame_clock().unwrap().fps();
//                 log::error!("fps: {}", fps);
//                 glib::ControlFlow::Continue
//             })
//         }
//         log::Level::Trace => {
//             Box::new(move || {
//                 let fps=window.frame_clock().unwrap().fps();
//                 log::trace!("fps: {}", fps);
//                 glib::ControlFlow::Continue
//             })
//         }
//     };
//     glib::timeout_add_local(update_interval, tick);
// }
