//! Animated terminal feedback while synchronous work is running.

use std::io::{IsTerminal, Write};
use std::sync::mpsc;
use std::time::Duration;

pub(crate) fn with_loading<T>(message: &str, operation: impl FnOnce() -> T) -> T {
    eprintln!("  ▸ {message}");
    if !std::io::stderr().is_terminal() {
        return operation();
    }
    std::thread::scope(|scope| {
        let (stop, receiver) = mpsc::channel();
        let worker = scope.spawn(move || {
            let mut index = 0;
            while matches!(
                receiver.recv_timeout(Duration::from_millis(100)),
                Err(mpsc::RecvTimeoutError::Timeout)
            ) {
                let marker = ['|', '/', '-', '\\'][index % 4];
                eprint!("\r  {marker} {message}\x1b[K");
                let _ = std::io::stderr().flush();
                index += 1;
            }
            eprint!("\r\x1b[K");
        });
        let result = operation();
        let _ = stop.send(());
        let _ = worker.join();
        result
    })
}
