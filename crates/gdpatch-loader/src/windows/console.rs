use color_eyre::eyre::{Context, ContextCompat};
use retour::static_detour;
use windows::core::HSTRING;
use windows::{
    Win32::{
        Foundation::HANDLE,
        System::{
            Console::{
                AllocConsole, CONSOLE_MODE, ENABLE_PROCESSED_OUTPUT,
                ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetStdHandle, STD_ERROR_HANDLE,
                STD_OUTPUT_HANDLE, SetConsoleMode,
            },
            LibraryLoader::{GetProcAddress, LoadLibraryW},
        },
        UI::WindowsAndMessaging::{MB_ICONERROR, MB_SYSTEMMODAL, MessageBoxW},
    },
    core::{BOOL, PCSTR, PCWSTR, s, w},
};

/// Setup a panic hook that creates a message box.
pub fn setup_panic_hook() {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .theme(color_eyre::config::Theme::new())
        .into_hooks();
    color_eyre::eyre::set_hook(eyre_hook.into_eyre_hook()).ok();

    std::panic::set_hook(Box::new(move |panic_info| {
        let report = panic_hook.panic_report(panic_info);
        tracing::error!("{}", report);

        unsafe {
            let payload = panic_info.payload();
            let payload = if let Some(s) = payload.downcast_ref::<&str>() {
                Some(s.to_string())
            } else {
                payload.downcast_ref::<String>().cloned()
            };
            let payload = if let Some(s) = payload {
                format!("The error was:\n{s}")
            } else {
                Default::default()
            };

            let msg = format!(
                r#"GDPatch has crashed! Sorry. :(

Check the log file for more debugging info. This may have been caused by a mod, or possibly GDPatch itself.

{payload}
                "#,
            );
            let msg = msg.trim();

            MessageBoxW(
                None,
                &HSTRING::from(msg),
                w!("GDPatch"),
                MB_ICONERROR | MB_SYSTEMMODAL,
            );

            std::process::exit(1);
        }
    }));
}

/// Setup console output and ANSI.
pub fn setup_console() -> color_eyre::Result<()> {
    unsafe {
        // allocate a console to fix stdout and stderr
        AllocConsole().context("failed to allocate console")?;

        // enable ANSI so our logs display properly
        {
            for id in [STD_OUTPUT_HANDLE, STD_ERROR_HANDLE] {
                let handle = GetStdHandle(id).context("failed to open std handle")?;

                let mut mode = CONSOLE_MODE::default();
                GetConsoleMode(handle, &mut mode).context("failed to get console mode")?;

                mode.0 |= ENABLE_VIRTUAL_TERMINAL_PROCESSING.0;
                mode.0 |= ENABLE_PROCESSED_OUTPUT.0;
                SetConsoleMode(handle, mode).context("failed to set console mode")?;
            }
        }

        // create a hook so the engine can't overwrite ANSI
        {
            let export = get_export(w!("kernel32.dll"), s!("SetConsoleMode"))
                .context("failed to get SetConsoleMode")?;
            let addr: SetConsoleModeFn = std::mem::transmute_copy(&export);
            SetConsoleModeHook
                .initialize(addr, set_console_mode_detour)
                .context("failed to initialize SetConsoleMode hook")?;
            SetConsoleModeHook
                .enable()
                .context("failed to enable SetConsoleMode hook")?;
        }

        Ok(())
    }
}

type SetConsoleModeFn = fn(HANDLE, u32) -> BOOL;

static_detour! {
    static SetConsoleModeHook: fn(HANDLE, u32) -> BOOL;
}

fn set_console_mode_detour(handle: HANDLE, mut mode: u32) -> BOOL {
    mode |= ENABLE_VIRTUAL_TERMINAL_PROCESSING.0;
    mode |= ENABLE_PROCESSED_OUTPUT.0;
    SetConsoleModeHook.call(handle, mode)
}

// copied from filesilly
fn get_export(module: PCWSTR, export: PCSTR) -> color_eyre::Result<*const std::ffi::c_void> {
    unsafe {
        let module = LoadLibraryW(module)?;
        GetProcAddress(module, export)
            .map(|p| p as *const std::ffi::c_void)
            .wrap_err("failed to get export")
    }
}
