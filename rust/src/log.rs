//! Where the lines go. The CLI prints them (stdout / stderr, as before); the
//! desktop app has no console, so the bridge collects the same lines and the
//! LOG page shows them.

use std::cell::RefCell;
use std::fmt::Arguments;

thread_local! {
    static SINK: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

/// True when the line was taken by a collector on this thread.
fn collected(text: &str) -> bool {
    SINK.with(|s| match s.borrow_mut().as_mut() {
        Some(lines) => {
            lines.extend(text.lines().map(str::to_owned));
            true
        }
        None => false,
    })
}

/// A stdout line (`say!`).
pub fn out(args: Arguments) {
    let line = args.to_string();
    if !collected(&line) {
        println!("{line}");
    }
}

/// A stderr line (`note!`).
pub fn err(args: Arguments) {
    let line = args.to_string();
    if !collected(&line) {
        eprintln!("{line}");
    }
}

/// Stdout text that carries its own newlines (a unified diff).
pub fn out_raw(text: &str) {
    if !collected(text) {
        print!("{text}");
    }
}

/// Run `f` with every `say!` / `note!` line on this thread collected instead
/// of printed; returns them in order.
pub fn collect<T>(f: impl FnOnce() -> T) -> (T, Vec<String>) {
    SINK.with(|s| *s.borrow_mut() = Some(Vec::new()));
    let result = f();
    let lines = SINK.with(|s| s.borrow_mut().take()).unwrap_or_default();
    (result, lines)
}

#[macro_export]
macro_rules! say {
    ($($t:tt)*) => { $crate::log::out(format_args!($($t)*)) };
}

#[macro_export]
macro_rules! note {
    ($($t:tt)*) => { $crate::log::err(format_args!($($t)*)) };
}

#[cfg(test)]
mod tests {
    #[test]
    fn collect_takes_the_lines_in_order_and_releases_the_sink() {
        let (n, lines) = super::collect(|| {
            say!("wrote {}", "a");
            note!("notice: {}", 1);
            super::out_raw("--- a\n+++ b\n");
            7
        });
        assert_eq!(n, 7);
        assert_eq!(lines, ["wrote a", "notice: 1", "--- a", "+++ b"]);
        assert!(!super::collected("printed, not collected"));
    }
}
