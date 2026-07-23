use indexmap::IndexSet;
use std::path::Path;
use std::{env, fs};

fn generate_exports_file() -> color_eyre::Result<()> {
    let mut exports = IndexSet::new();

    for line in fs::read_to_string("windows_exports.txt")?.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("#") {
            continue;
        }

        exports.insert(line.to_string());
    }

    let mut output_src = "\
    // GENERATED FILE, DO NOT EDIT (see build.rs)\n\
    use windows::Win32::Foundation::{FARPROC, HMODULE};\n\
    use windows::Win32::System::LibraryLoader::GetProcAddress;\n\
    use windows::core::PCSTR;\n\
    use std::arch::naked_asm;\n\
    \n\
    "
    .to_owned();

    let mut statics_src = String::new();
    let mut exports_src = String::new();
    let mut imports_src = String::new();

    for export in exports {
        let addr_static = format!("addr_{export}");

        statics_src += "#[allow(non_upper_case_globals)]\n";
        statics_src += &format!("static mut {addr_static}: FARPROC = None;\n");
        statics_src += "\n";

        exports_src += "#[allow(non_snake_case)]\n";
        exports_src += "#[unsafe(naked)]\n";
        exports_src += &format!("#[unsafe(export_name = \"{export}\")]\n");
        exports_src += &format!("unsafe extern \"C\" fn {export}() -> ! {{\n");
        exports_src +=
            &format!("    naked_asm!(\"jmp qword ptr [rip + {{}}]\", sym {addr_static});\n");
        exports_src += "}\n";
        exports_src += "\n";

        imports_src += &format!(
            "        {addr_static} = GetProcAddress(module, PCSTR(c\"{export}\".as_ptr().cast::<u8>()));\n"
        );
    }

    output_src += &statics_src;
    output_src += &exports_src;

    output_src += "pub unsafe fn find_exports(module: HMODULE) {\n";
    output_src += "    unsafe {\n";
    output_src += &imports_src;
    output_src += "    }\n";
    output_src += "}";

    let exported_path = env::var_os("OUT_DIR").expect("missing OUT_DIR environment variable");
    let exported_path = Path::new(&exported_path).join("proxy_generated.rs");
    fs::write(exported_path, output_src)?;

    Ok(())
}

fn main() -> color_eyre::Result<()> {
    println!("cargo:rerun-if-changed=windows_exports.txt");

    if env::var_os("CARGO_CFG_WINDOWS").is_some() {
        generate_exports_file()?;
    }

    Ok(())
}
