use errors::ErrorMetadata;

pub fn syscall_name_for_error(name: &str) -> &'static str {
    match name {
        "count" | "get" | "insert" | "update" | "replace" | "queryStreamNext" | "queryPage"
        | "remove" => "Db",
        _ => "Syscall",
    }
}

pub fn syscall_description_for_error(name: &str) -> String {
    match name {
        "count" | "get" | "insert" | "update" | "replace" | "queryStreamNext" | "queryPage"
        | "remove" => "Database".to_string(),
        _ => format!("Syscall {name}"),
    }
}

// NB: This function loses the backtrace for `e` and creates a new backtrace.
pub fn clone_error_for_batch(e: &anyhow::Error) -> anyhow::Error {
    match e.downcast_ref::<ErrorMetadata>() {
        Some(error_metadata) => error_metadata.clone().into(),
        None => anyhow::anyhow!("{e}"),
    }
}
