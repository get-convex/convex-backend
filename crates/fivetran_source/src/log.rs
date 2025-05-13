use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct LogLine<'a> {
    level: &'a str,
    message: &'a str,
    message_origin: &'a str,
}

pub fn log(message: &str) {
    log_with_level(message, "INFO");
}

#[allow(dead_code)]
pub fn warn(message: &str) {
    log_with_level(message, "WARNING");
}

#[allow(dead_code)]
pub fn error(message: &str) {
    log_with_level(message, "SEVERE");
}

fn log_with_level(message: &str, level: &str) {
    let result = serde_json::to_string(&LogLine {
        level,
        message,
        message_origin: "sdk_connector",
    });
    match result {
        Ok(msg) => println!("{msg}"),
        Err(e) => println!("Unable to serialize to json: {message}: {e}"),
    }
}
