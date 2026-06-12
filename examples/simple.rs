use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
};

use taurino_webview::{
    IpcBody, IpcResponse, Result,
    webview::{Manager, ManagerConfig, builder::WebViewBuilder},
    window::builder::WindowBuilder,
};

enum Message {
    StartDragging,
    InternalToggleMaximize,
}

fn main() -> Result<()> {
    let event_loop = EventLoopBuilder::<Message>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .label("main")
        .title("Taurino App")
        .inner_size(1000.0, 800.0)
        .center()
        .resizable(true)
        .decorations(false)
        .visible(true)
        .build(&event_loop)?;

    let config = ManagerConfig::new()?
        .set_static_dir("dist")
        .set_allow_dragging(true);

    let mut manager = Manager::new()?
        .set_window_id(window.get_inner().id())
        .on_ipc_message(move |req| match req.command.as_str() {
            "ping" => IpcResponse::resolve_json(serde_json::json!({
                "ok": true,
                "command": req.command,
                "message": "pong",
                "windowLabel": req.window_label,
                "webviewId": req.webview_id,
                "received": ipc_body_to_json(req.body),
            })),

            "plugin:window|start_dragging" => match proxy.send_event(Message::StartDragging) {
                Ok(_) => IpcResponse::resolve_json(serde_json::json!({
                    "ok": true,
                    "command": "start_dragging"
                })),
                Err(error) => IpcResponse::reject_json(serde_json::json!({
                    "ok": false,
                    "command": "start_dragging",
                    "error": error.to_string()
                })),
            },

            "plugin:window|internal_toggle_maximize" => {
                match proxy.send_event(Message::InternalToggleMaximize) {
                    Ok(_) => IpcResponse::resolve_json(serde_json::json!({
                        "ok": true,
                        "command": "internal_toggle_maximize"
                    })),
                    Err(error) => IpcResponse::reject_json(serde_json::json!({
                        "ok": false,
                        "command": "internal_toggle_maximize",
                        "error": error.to_string()
                    })),
                }
            }
            "add" => {
                let body = ipc_body_to_json(req.body);

                let a = body.get("a").and_then(|value| value.as_i64()).unwrap_or(0);

                let b = body.get("b").and_then(|value| value.as_i64()).unwrap_or(0);

                IpcResponse::resolve_json(serde_json::json!({
                    "ok": true,
                    "command": req.command,
                    "result": a + b,
                    "a": a,
                    "b": b,
                }))
            }

            "echo" => IpcResponse::resolve_json(serde_json::json!({
                "ok": true,
                "command": req.command,
                "echo": ipc_body_to_json(req.body),
            })),

            _ => IpcResponse::reject_json(serde_json::json!({
                "ok": false,
                "error": format!("Unknown command: {}", req.command),
                "hint": "Implement this command inside on_ipc_message(...)",
            })),
        })
        .set_manager_config(config);

    let pending = WebViewBuilder::app("main", "index.html")
        .devtools(true)
        .scroll_bar_style(taurino_webview::utils::types::ScrollBarStyle::FluentOverlay)
        .use_https_scheme(true)
        .build();

    manager.create_webview(&window, pending)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(Message::StartDragging) => {
                if let Err(error) = window.start_dragging() {
                    eprintln!("failed to start window dragging: {error}");
                }
            }

            Event::UserEvent(Message::InternalToggleMaximize) => {
                if window.is_resizable().unwrap() {
                    match window.is_maximized().unwrap() {
                        true => window.unmaximize().unwrap(),
                        false => window.maximize().unwrap(),
                    }
                }
            }
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

fn ipc_body_to_json(body: IpcBody) -> serde_json::Value {
    match body {
        IpcBody::Json(value) => value,

        IpcBody::Raw(bytes) => {
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap_or_else(|_| {
                serde_json::json!({
                    "raw": bytes,
                })
            })
        }
    }
}
