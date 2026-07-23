#![cfg(windows)]
use color_eyre::eyre::Context;
use gdpatch::GDPatch;
use retour::static_detour;
use std::ffi::{OsString, c_void};
use std::os::windows::ffi::OsStringExt;
use std::path::Path;
use tracing::warn;
use windows::Win32::Foundation::{HINSTANCE, HMODULE};
use windows::Win32::System::LibraryLoader::{DisableThreadLibraryCalls, GetModuleFileNameW};
use windows::Win32::System::SystemServices::{DLL_PROCESS_ATTACH, DLL_PROCESS_DETACH};
use windows::{
    Win32::System::{
        LibraryLoader::GetModuleHandleW,
        ProcessStatus::{GetModuleInformation, MODULEINFO},
        Threading::GetCurrentProcess,
    },
    core::PCWSTR,
};

mod console;
mod proxy;

// This is the PE entrypoint, not WinMain
// https://devblogs.microsoft.com/oldnewthing/20110525-00/?p=10573
static_detour! {
    static EntryPointHook: fn() -> u32;
}

fn get_console_env_var() -> bool {
    std::env::var("GDPATCH_CONSOLE")
        .map(|e| e.parse::<bool>().unwrap_or_default() || e.parse::<u8>().unwrap_or_default() != 0)
        .unwrap_or_default()
}

// Avoid Windows loader lock by hooking the entrypoint
// https://learn.microsoft.com/en-us/windows/win32/dlls/dynamic-link-library-best-practices#general-best-practices
fn entrypoint_detour() -> u32 {
    if let Err(err) = GDPatch::setup_instance_logging_etc() {
        panic!("GDPatch early initialization failed: {err:?}");
    }

    let instance = GDPatch::instance();

    // Only enable console if we didn't already do that
    if instance.config.log.console
        && !get_console_env_var()
        && let Err(err) = console::setup_console()
    {
        warn!(?err, "failed to initialize console");
    }

    // Finish setup.
    if let Err(err) = instance.finish_setup() {
        panic!("GDPatch initialization failed: {err:?}");
    }

    EntryPointHook.call()
}

fn setup_entrypoint_hook() -> color_eyre::Result<()> {
    unsafe {
        let process = GetCurrentProcess();
        let module = GetModuleHandleW(PCWSTR::null()).context("couldn't get module handle")?;

        let mut module_info = MODULEINFO::default();
        GetModuleInformation(
            process,
            module,
            &mut module_info,
            size_of::<MODULEINFO>() as u32,
        )
        .context("couldn't get module information")?;

        let entry = module_info.EntryPoint;
        EntryPointHook
            .initialize(
                std::mem::transmute::<*mut c_void, fn() -> u32>(entry),
                entrypoint_detour,
            )
            .context("couldn't initialize hook")?;
        EntryPointHook.enable().context("couldn't enable hook")?;

        Ok(())
    }
}

fn get_module_path(dll_handle: HMODULE) -> windows::core::Result<OsString> {
    let mut buffer = vec![0u16; 64];

    loop {
        let length = unsafe { GetModuleFileNameW(Some(dll_handle), &mut buffer) } as usize;

        if length == 0 {
            return Err(windows::core::Error::from_thread());
        } else if length < buffer.len() {
            buffer.truncate(length);
            return Ok(OsString::from_wide(&buffer));
        } else {
            buffer.resize(buffer.len() * 2, 0);
        }
    }
}

fn setup_proxy(dll_handle: HMODULE) -> color_eyre::Result<()> {
    let module_path = get_module_path(dll_handle)?;
    let module_path = Path::new(&module_path);
    proxy::load(module_path)?;

    Ok(())
}

#[unsafe(export_name = "DllMain")]
extern "C" fn dll_main(dll_handle: HINSTANCE, reason: u32, _reserved: *const ()) -> bool {
    unsafe {
        let _ = DisableThreadLibraryCalls(dll_handle.into());
    }

    if reason == DLL_PROCESS_DETACH {
        unsafe {
            proxy::unload();
        }
    }

    if reason != DLL_PROCESS_ATTACH {
        return true;
    }

    // Set a panic hook so logs show up easier
    // Attach console (using a non-figment env var!)
    console::setup_panic_hook();

    if get_console_env_var()
        && let Err(err) = console::setup_console()
    {
        // good luck seeing this error message
        eprintln!("Failed to initialize console: {err:?}");
    }

    // Load the DLL we're proxying.
    if let Err(err) = setup_proxy(dll_handle.into()) {
        eprintln!("Failed to setup DLL proxying - game may crash: {err:?}")
    }

    if let Err(e) = setup_entrypoint_hook() {
        eprintln!("Failed to pre-initialize: {e:?}");
        false
    } else {
        true
    }
}
