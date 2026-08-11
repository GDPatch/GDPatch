//! GDScript tokenizers. Converts an input (in
mod bytecode;
mod reconstitute_text;
mod text;

use std::fmt::Debug;

use crate::build::GDScriptBuild;
use crate::gdscript::{Spanned, Token};
use crate::private::Sealed;

pub use self::bytecode::{CompressMode, TokenizerBytecode, reconstruct_script_binary};
pub use self::reconstitute_text::reconstruct_script_text;
pub use self::text::TokenizerText;

pub trait Tokenizer: Sealed + Iterator<Item = Spanned<Token>> + Debug {
    fn version(&self) -> &GDScriptBuild;
    fn set_multiline_mode(&mut self, multiline: bool);
    fn push_expression_indented_block(&mut self);
    fn pop_expression_indented_block(&mut self) -> isize;
}
