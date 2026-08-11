use crate::SharedInputs;
use crate::tokenizer::run_tokenizer_and_compare;
use gdpatch_godot::gdscript::tokenizer::TokenizerText;
use libtest_mimic::Failed;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fs, str};

pub fn run_text_tokenizer_test(
    shared_inputs: Arc<SharedInputs>,
    script_path: PathBuf,
) -> Result<(), Failed> {
    let contents = fs::read(script_path)?;
    let source = str::from_utf8(&contents)
        .map_err(|err| format!("source file isn't valid utf-8: {}", err))?;

    let gdscript_build = &shared_inputs.build.gdscript;
    let tokenizer = TokenizerText::new(gdscript_build, &source);

    run_tokenizer_and_compare(&shared_inputs, "tokenizer_text.gd", &contents, tokenizer)
}
