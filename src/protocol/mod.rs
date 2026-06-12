pub mod ipc;
pub(crate) mod isolation;
pub(crate) mod pattern;
pub(crate) mod taurino;


/* 

let manager = Manager::new()?
    .on_ipc_message(|request| {
        user_command_system_handle(request)
    });
fn user_command_system_handle(request: IpcRequest) -> IpcResponse {
    // Der User entscheidet selbst:
    // - welche Commands existieren
    // - wie JSON geparst wird
    // - ob async, stateful, plugin-basiert, Python-basiert usw.
    // - welche Antwort zurückgeht

    IpcResponse::reject_json(serde_json::json!({
        "error": format!("Command not implemented by user: {}", request.command)
    }))
}
*/