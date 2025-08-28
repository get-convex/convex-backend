use std::{
    process::Stdio,
    sync::{
        atomic::AtomicUsize,
        Arc,
    },
};

use colored::Colorize;
use futures::StreamExt;
use parking_lot::Mutex;
use tokio::{
    io::{
        AsyncBufReadExt,
        BufReader,
    },
    process::Command,
};

#[derive(Clone)]
pub struct LogInterleaver {
    inner: Arc<Mutex<LogInterleaverInner>>,
}

impl LogInterleaver {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LogInterleaverInner::new())),
        }
    }

    /// Spawns c, outputting its stdout/stderr line-buffered and prefixed by
    /// `prefix`.
    pub fn spawn_with_prefixed_logs(
        &self,
        prefix: String,
        c: &mut Command,
    ) -> anyhow::Result<tokio::process::Child> {
        self.inner.lock().spawn_with_prefixed_logs(prefix, c)
    }
}

struct LogInterleaverInner {
    next_color: colored::Color,
    // Align prefixes by keeping track of the longest we've seen so far, and pad out to that width.
    prefix_width: Arc<AtomicUsize>,
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl Drop for LogInterleaverInner {
    fn drop(&mut self) {
        while let Some(handle) = self.handles.pop() {
            handle.abort();
        }
    }
}

impl LogInterleaverInner {
    fn new() -> Self {
        Self {
            next_color: colored::Color::Yellow,
            prefix_width: Arc::new(AtomicUsize::new(0)),
            handles: vec![],
        }
    }

    fn spawn_with_prefixed_logs(
        &mut self,
        prefix: String,
        c: &mut Command,
    ) -> anyhow::Result<tokio::process::Child> {
        let prefix_width = self.prefix_width.clone();
        prefix_width.fetch_max(prefix.len(), std::sync::atomic::Ordering::SeqCst);
        let color = self.next_color;
        self.next_color = Self::next_color(self.next_color);

        let mut child = c.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
        let stdout = tokio_stream::wrappers::LinesStream::new(
            BufReader::new(child.stdout.take().unwrap()).lines(),
        );
        let stderr = tokio_stream::wrappers::LinesStream::new(
            BufReader::new(child.stderr.take().unwrap()).lines(),
        );
        let mut combined = futures::stream::select(stdout, stderr);

        self.handles.push(tokio::spawn(async move {
            while let Some(maybe_line) = combined.next().await {
                let prefix_width = prefix_width.load(std::sync::atomic::Ordering::SeqCst);
                tracing::info!(
                    "{} | {}",
                    pad_right(&prefix, prefix_width).color(color),
                    maybe_line
                        .unwrap_or_else(|e| format!("error reading from stdout/stderr: {e:?}"))
                        .trim_end(),
                );
            }
        }));

        Ok(child)
    }

    fn next_color(c: colored::Color) -> colored::Color {
        match c {
            // Red looks like errors and black is hard to read. Use everything else.
            colored::Color::Green => colored::Color::Yellow,
            colored::Color::Yellow => colored::Color::Blue,
            colored::Color::Blue => colored::Color::Magenta,
            colored::Color::Magenta => colored::Color::Cyan,
            colored::Color::Cyan => colored::Color::BrightGreen,
            colored::Color::BrightGreen => colored::Color::BrightYellow,
            colored::Color::BrightYellow => colored::Color::BrightBlue,
            colored::Color::BrightBlue => colored::Color::BrightMagenta,
            colored::Color::BrightMagenta => colored::Color::BrightCyan,
            colored::Color::BrightCyan => colored::Color::BrightWhite,
            colored::Color::BrightWhite => colored::Color::Green,
            _ => colored::Color::Yellow,
        }
    }
}

/// Adds spaces to the right side of s so it's l characters long, or returns s
/// if s.len()<l.
///
/// Only behaves reasonably with ASCII.
fn pad_right(s: &str, l: usize) -> String {
    if s.len() >= l {
        return s.to_string();
    }
    s.to_string() + &" ".repeat(l - s.len())
}
