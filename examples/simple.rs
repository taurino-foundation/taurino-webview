use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use taurino_webview::{
    Result,
    builder::WebViewBuilder,
    manager::{Manager, ManagerConfig},
};

fn main() -> Result<()> {
    // Beispiel 1: lokale HTML-Dateien aus dem dist-Ordner.
    // Zum Testen einfach oben auskommentieren und diese Zeile aktivieren:
    //  local_html_example()

    // Beispiel 2: externe Webseiten.
    // Zum Testen einfach oben auskommentieren und diese Zeile aktivieren:
    //
    external_url_example()
}

/* fn local_html_example() -> Result<()> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Taurino Local HTML Example")
        .build(&event_loop)
        .expect("failed to create tao window");








    let config = ManagerConfig::new()?.set_static_dir("dist");

    let mut manager = Manager::new()?.set_window_id(window.id()).set_manager_config(config);

    let pages = [
        ("webview-top-left", "top-left.html"),
        ("webview-top-right", "top-right.html"),
        ("webview-bottom-left", "bottom-left.html"),
        ("webview-bottom-right", "bottom-right.html"),
    ];

    let scale_factor = window.scale_factor();
    let window_size = window.inner_size().to_logical::<f32>(scale_factor);

    let half_width = window_size.width / 2.0;
    let half_height = window_size.height / 2.0;

    let layouts = [
        // oben links
        (0.0, 0.0, half_width, half_height),
        // oben rechts
        (half_width, 0.0, half_width, half_height),
        // unten links
        (0.0, half_height, half_width, half_height),
        // unten rechts
        (half_width, half_height, half_width, half_height),
    ];

    for ((label, page), (x, y, width, height)) in pages.into_iter().zip(layouts)
    {
        let pending = WebViewBuilder::app(label, page)
            .auto_resize()
            .bounds(x, y, width, height)
            .build();

        manager.create_webview(&window, pending)?;
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                manager.resize_webviews(&window, size);
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            _ => {}
        }
    })
} */

fn external_url_example() -> Result<()> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Taurino External URL Example")
        .build(&event_loop)
        .expect("failed to create tao window");

    let config = ManagerConfig::new()?.set_static_dir("dist");

    let mut manager = Manager::new()?
        .set_window_id(window.id())
        .set_manager_config(config);

    let urls = [
        ("webview-top-left", "https://tauri.app"),
        ("webview-top-right", "https://vite.dev"),
        ("webview-bottom-left", "https://nextjs.org"),
        ("webview-bottom-right", "https://nicegui.io"),
    ];

    let scale_factor = window.scale_factor();
    let window_size = window.inner_size().to_logical::<f32>(scale_factor);

    let half_width = window_size.width / 2.0;
    let half_height = window_size.height / 2.0;

    let layouts = [
        // oben links
        (0.0, 0.0, half_width, half_height),
        // oben rechts
        (half_width, 0.0, half_width, half_height),
        // unten links
        (0.0, half_height, half_width, half_height),
        // unten rechts
        (half_width, half_height, half_width, half_height),
    ];

    for ((label, url), (x, y, width, height)) in urls.into_iter().zip(layouts) {
        let pending = WebViewBuilder::external(label, url)?
            .auto_resize()
            .bounds(x, y, width, height)
            .build();

        manager.create_webview(&window, pending)?;
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                manager.resize_webviews(&window, size);
            }

            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }

            _ => {}
        }
    })
}
