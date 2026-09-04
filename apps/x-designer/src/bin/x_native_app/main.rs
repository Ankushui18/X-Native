mod chrome;
mod theme;

use chrome::XNativeApp;
use std::sync::Arc;
use winit::{event_loop::EventLoop, window::Window};

fn main() {
    let event_loop = EventLoop::new().expect("event loop");
    let window = event_loop
        .create_window(
            Window::default_attributes()
                .with_title("Untitled — X-Native")
                .with_inner_size(winit::dpi::LogicalSize::new(1440.0, 900.0))
                .with_min_inner_size(winit::dpi::LogicalSize::new(980.0, 680.0)),
        )
        .expect("window");

    let mut app = pollster::block_on(XNativeApp::new(Arc::new(window)));
    event_loop
        .run(move |event, elwt| {
            app.handle_event(event, elwt);
        })
        .expect("event loop run");
}
