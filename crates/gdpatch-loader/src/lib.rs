#[cfg(windows)]
mod windows;

#[cfg(unix)]
mod unix;

#[cfg(not(any(windows, unix)))]
compile_error!("Unsupported platform");
