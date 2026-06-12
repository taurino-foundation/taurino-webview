use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use taurino_webview::{
    Result,
    webview::{Manager, ManagerConfig, builder::WebViewBuilder},
    window::builder::WindowBuilder,
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
        .title("Taurino Local HTML Example")
        .build(&event_loop)
        .expect("failed to create tao window");

    let config = ManagerConfig::new()?.set_static_dir("dist");

    let mut manager = Manager::new()?
        .set_window_id(window.get_inner().id())
        .set_manager_config(config);

    let pending = WebViewBuilder::app("main", "index.html")
        .devtools(true)
        .use_https_scheme(false)
        .build();

    manager.create_webview(&window, pending)?;
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
