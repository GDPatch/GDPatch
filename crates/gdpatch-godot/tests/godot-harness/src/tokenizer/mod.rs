use crate::SharedInputs;
use crate::godot_comms::run_script;
use gdpatch_godot::gdscript::tokenizer::Tokenizer;
use gdpatch_godot::gdscript::{Token, TokenType};
use libtest_mimic::Failed;
use pretty_assertions::Comparison;
use serde::Deserialize;
use serde_json::Value;

pub mod buffer;
pub mod text;

#[derive(Debug, Deserialize, Clone)]
struct OutputToken<'s> {
    pub name: &'s str,
    pub lit: Value,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
struct Output<'s>(#[serde(borrow)] pub Vec<OutputToken<'s>>);

pub fn run_tokenizer_and_compare<'s, T>(
    shared_inputs: &SharedInputs,
    runner_script: &str,
    target_contents: &[u8],
    mut tokenizer: T,
) -> Result<(), Failed>
where
    T: Tokenizer,
{
    let output = run_script(shared_inputs, runner_script, target_contents)?;

    let gd_output = serde_json::from_slice::<Output<'_>>(&output).map_err(|err| {
        let stdout = String::from_utf8_lossy(&output);
        format!("reading godot output: {err}\n\n{}", stdout)
    })?;
    let gd_tokens = gd_output.0;

    // run rust tokenizer and compare
    let mut rs_tokens = Vec::new();

    loop {
        let Some((token, _)) = tokenizer.next() else {
            panic!("tokenizer returned None without returning EOF")
        };

        let token_type = token.typ();

        if !matches!(token, Token::Newline { continuation: true }) {
            rs_tokens.push(token);
        }

        if token_type == TokenType::Eof {
            break;
        }
    }

    // compare output to rust version
    let rs_token_names = rs_tokens
        .into_iter()
        .map(|tok| {
            let lit = match &tok {
                Token::Annotation(name) | Token::Identifier(name) => Some(format!("\"{name}\"")),
                Token::Literal(value) | Token::Error(value) => Some(value.to_string()),
                _ => None,
            };

            if let Some(lit) = lit {
                format!("{}({})", tok.token_name(), lit)
            } else {
                tok.token_name().to_owned()
            }
        })
        .collect::<Vec<_>>();

    let gd_token_names = gd_tokens
        .into_iter()
        .map(|tok| {
            let has_lit = tok.name == "Annotation"
                || tok.name == "Identifier"
                || tok.name == "Literal"
                || tok.name == "Error";

            if has_lit {
                let lit = match tok.lit {
                    Value::Null => "null".to_string(),
                    Value::Bool(b) => b.to_string(),
                    Value::Number(v) => v.to_string(),
                    Value::String(s) => format!("\"{}\"", s.escape_debug()),
                    Value::Array(_) => todo!(),
                    Value::Object(_) => todo!(),
                };

                format!("{}({})", tok.name, lit)
            } else {
                tok.name.to_owned()
            }
        })
        .collect::<Vec<_>>();

    if rs_token_names != gd_token_names {
        let msg = Comparison::new(&rs_token_names, &gd_token_names).to_string();
        return Err(msg.into());
    }

    Ok(())
}
