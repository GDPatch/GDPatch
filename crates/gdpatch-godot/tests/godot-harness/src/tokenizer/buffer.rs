use crate::SharedInputs;
use crate::godot_comms::run_script;
use crate::tokenizer::run_tokenizer_and_compare;
use gdpatch_godot::gdscript::tokenizer::TokenizerBytecode;
use libtest_mimic::Completion::Completed;
use libtest_mimic::{Completion, Failed};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

pub fn convert_and_run_buffer_tokenizer_test(
    shared_inputs: Arc<SharedInputs>,
    script_path: PathBuf,
) -> Result<Completion, Failed> {
    let gdscript_build = &shared_inputs.build.gdscript;
    if gdscript_build.tokenizer_version.is_none() {
        return Ok(Completion::ignored_with("bytecode scripts unsupported"));
    }

    let contents = fs::read(script_path)?;
    let converted = run_script(
        &shared_inputs,
        "convert_text_script_to_binary.gd",
        &contents,
    )?;

    let converted = str::from_utf8(&converted)
        .map_err(|err| format!("non-utf8 output from buffer conversion script: {err}"))?
        .trim();

    let mut buffer = Vec::with_capacity(converted.len() / 2);

    for i in (0..converted.len()).step_by(2) {
        let b = u8::from_str_radix(&converted[i..i + 2], 16)
            .map_err(|err| format!("non-hex output from buffer conversion script: {err}"))?;

        buffer.push(b);
    }

    let tokenizer = TokenizerBytecode::new(gdscript_build, &buffer)
        .map_err(|err| format!("failed to parse binary gdscript: {err}"))?;

    run_tokenizer_and_compare(&shared_inputs, "tokenizer_buffer.gd", &buffer, tokenizer)?;

    Ok(Completed)
}

pub fn run_buffer_tokenizer_test(
    shared_inputs: Arc<SharedInputs>,
    script_path: PathBuf,
) -> Result<Completion, Failed> {
    let gdscript_build = &shared_inputs.build.gdscript;
    if gdscript_build.tokenizer_version.is_none() {
        return Ok(Completion::ignored_with("bytecode scripts unsupported"));
    }

    let contents = fs::read(script_path)?;

    let tokenizer = TokenizerBytecode::new(gdscript_build, &contents)
        .map_err(|e| format!("failed to parse binary gdscript: {}", e))?;

    run_tokenizer_and_compare(&shared_inputs, "tokenizer_buffer.gd", &contents, tokenizer)?;

    Ok(Completed)
}
