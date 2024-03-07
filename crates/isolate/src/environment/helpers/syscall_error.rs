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
