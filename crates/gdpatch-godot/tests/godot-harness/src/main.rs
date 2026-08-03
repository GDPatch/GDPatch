extern crate core;

mod godot_comms;
mod tokenizer;

use crate::tokenizer::buffer::convert_and_run_buffer_tokenizer_test;
use color_eyre::eyre::{OptionExt, WrapErr, bail, eyre};
use gdpatch_godot::build::{EngineBuild, VersionSpecifier, resolve_bundled_builds};
use libtest_mimic::{Arguments, Trial};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::sync::Arc;
use std::{env, fs};
use tokenizer::buffer::run_buffer_tokenizer_test;
use tokenizer::text::run_text_tokenizer_test;
use walkdir::WalkDir;

fn try_parse_version_string(version: &str) -> Option<VersionSpecifier> {
    let parts = version.trim().split(".").collect::<Vec<_>>();

    // expect major.minor at least
    let major = u32::from_str(parts.get(0)?).ok()?;
    let minor = u32::from_str(parts.get(1)?).ok()?;

    let potential_patch = *parts.get(2)?;

    let (patch, branch) = if potential_patch.chars().all(|c| c.is_ascii_digit()) {
        let patch = u32::from_str(potential_patch).ok()?;
        let branch = *parts.get(3)?;
        (patch, branch)
    } else {
        (0, potential_patch)
    };

    Some(VersionSpecifier::new(
        major,
        minor,
        patch,
        0,
        branch.to_owned(),
    ))
}

fn determine_godot_build(godot_binary: &Path) -> color_eyre::Result<EngineBuild> {
    let process = Command::new(godot_binary)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .arg("--version")
        .spawn()
        .map_err(|err| eyre!("failed to spawn Godot process: {}", err))?;

    let gd_output = process.wait_with_output()?;

    if gd_output.stdout.is_empty() {
        bail!("--version didn't output anything");
    }

    let version_string = String::from_utf8(gd_output.stdout)?;
    let version = try_parse_version_string(&version_string)
        .ok_or_else(|| eyre!("failed to parse version string {version_string}"))?;

    let builds = resolve_bundled_builds(None).unwrap();
    let build = builds
        .find_approximate_build(&version)
        .ok_or_else(|| eyre!("failed to find build for {version}"))?;

    Ok(build.clone())
}

#[derive(Debug)]
struct SharedInputs {
    pub build: EngineBuild,
    pub project_path: PathBuf,
    pub godot_binary: PathBuf,
}

fn main() -> color_eyre::Result<()> {
    let args = Arguments::from_args();
    let cwd = env::current_dir().context("failed to get current directory")?;

    let project_path = {
        let path = cwd.join("tests").join("test-project");

        if !fs::exists(&path).unwrap_or(false) {
            bail!("Couldn't find test project. Are you running in the right directory?");
        }

        path.canonicalize()?
    };

    let godot_binary = {
        let path =
            env::var_os("GDPATCH_TEST_GODOT").ok_or_eyre("missing GDPATCH_TEST_GODOT variable")?;
        let path = PathBuf::from(path);

        if !fs::exists(&path).unwrap_or(false) {
            bail!("Couldn't find the Godot binary set by GDPATCH_TEST_GODOT");
        }

        path.canonicalize()?
    };

    let corpus_path = {
        if let Some(path) = env::var_os("GDPATCH_TEST_CORPUS") {
            if !fs::exists(&path).unwrap_or(false) {
                bail!("Couldn't find the corpus set by GDPATCH_TEST_CORPUS");
            }

            Path::new(&path).canonicalize()?
        } else {
            let path = cwd.join("tests").join("scripts");

            if !fs::exists(&path).unwrap_or(false) {
                bail!("Couldn't find default test corpus. Are you running in the right directory?");
            }

            path.canonicalize()?
        }
    };

    let build = determine_godot_build(&godot_binary)?;

    let shared_inputs = Arc::new(SharedInputs {
        build,
        project_path,
        godot_binary,
    });

    // generate all our test cases
    let mut tests = Vec::new();

    for file in WalkDir::new(&corpus_path) {
        let file = file.wrap_err("error walking corpus path")?;

        if file
            .metadata()
            .context("getting corpus file metadata")?
            .is_dir()
        {
            continue;
        }

        let Some(extension) = file.path().extension() else {
            continue;
        };

        let Some(extension) = extension.to_str() else {
            continue;
        };

        let relative_path = file
            .path()
            .strip_prefix(&corpus_path)
            .unwrap_or(file.path());

        match extension {
            "gd" => {
                {
                    let shared_inputs = shared_inputs.clone();
                    let path = file.path().to_owned();
                    let test_name = relative_path.to_str().unwrap().replace('\\', "/");

                    let test = Trial::test(test_name, move || {
                        run_text_tokenizer_test(shared_inputs, path)
                    })
                    .with_kind("text-tokenizer");

                    tests.push(test);
                }

                {
                    let shared_inputs = shared_inputs.clone();
                    let path = file.path().to_owned();
                    let test_name = relative_path
                        .with_added_extension("gdc")
                        .to_str()
                        .unwrap()
                        .replace('\\', "/");

                    let test = Trial::ignorable_test(test_name, move || {
                        convert_and_run_buffer_tokenizer_test(shared_inputs, path)
                    })
                    .with_kind("converted-buffer-tokenizer");

                    tests.push(test);
                }
            }

            "gdc" => {
                let shared_inputs = shared_inputs.clone();
                let path = file.path().to_owned();
                let test_name = relative_path.to_str().unwrap().replace('\\', "/");

                let test = Trial::ignorable_test(&test_name, move || {
                    run_buffer_tokenizer_test(shared_inputs, path)
                })
                .with_kind("buffer-tokenizer");
                tests.push(test);
            }

            _ => {}
        }
    }

    libtest_mimic::run(&args, tests).exit();
}
