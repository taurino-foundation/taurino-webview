use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use taurino_webview::{
    Result,
    builder::WebViewBuilder,
    layout::FixedLayout,
    manager::{Manager, ManagerConfig},
};

fn main() -> Result<()> {
    // Example 1: local HTML files from the dist folder.
    // To test this example, comment out the external URL example below
    // and enable this line:
    //
    local_html_example()

    // Example 2: external websites.
    // To test this example, keep this line enabled:
    /* external_url_example() */
}


fn local_html_example() -> Result<()> {
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("Taurino Local HTML Example")
        .build(&event_loop)
        .expect("failed to create tao window");

    let config = ManagerConfig::new()?.set_static_dir("dist");

    let mut manager = Manager::new()?
        .set_window_id(window.id())
        .set_manager_config(config);

    let pages = [
        ("webview-top-left", "top-left.html"),
        ("webview-top-right", "top-right.html"),
        ("webview-bottom-left", "bottom-left.html"),
        ("webview-bottom-right", "bottom-right.html"),
    ];

    let scale_factor = window.scale_factor();
    let window_size = window.inner_size().to_logical::<f32>(scale_factor);

    let layout = FixedLayout::Grid { rows: 2, cols: 2 };

    let bounds = layout.resolve(
        pages.len(),
        window_size.width,
        window_size.height,
    );

    for ((label, page), bounds) in pages.into_iter().zip(bounds.into_iter()) {
        let pending = WebViewBuilder::app(label, page)
            .bounds_rect(bounds)
            .devtools(true)
            .build();

        manager.create_webview(&window, pending)?;
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                manager.resize_webviews_with_layout(&window, &layout);
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

/* 

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

    let layout = FixedLayout::Grid { rows: 2, cols: 2 };

    let bounds =
        layout.resolve(urls.len(), window_size.width, window_size.height);

    for ((label, url), bounds) in urls.into_iter().zip(bounds.into_iter()) {
        let pending = WebViewBuilder::external(label, url)?
            .bounds_rect(bounds)
            .devtools(true)
            .build();

        manager.create_webview(&window, pending)?;
    }
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                manager.resize_webviews_with_layout(&window, &layout);
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
 */