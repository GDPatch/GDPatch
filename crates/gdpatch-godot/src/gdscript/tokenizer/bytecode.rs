use crate::Error;
use crate::Error::{BadData, UnknownVersion};
use crate::build::GDScriptBuild;
use crate::gdscript::tokenizer::Tokenizer;
use crate::gdscript::{Position, Span, Spanned, Token, TokenType};
use crate::marshalling::{ReadableMarshalBuffer, WritableMarshalBuffer};
use crate::private::Sealed;
use crate::variant::Variant;
use indexmap::IndexSet;
use ruzstd::decoding::StreamingDecoder;
use ruzstd::encoding::CompressionLevel;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::Read;

const TOKEN_BYTE_MASK: u8 = 0x80;
const TOKEN_BITS: u32 = 8;
const TOKEN_MASK: u32 = (1 << (TOKEN_BITS - 1)) - 1;

/// For some reason identifiers are XOR'd with this constant key.
const INEXPLICABLE_XOR_KEY: u32 = 0xb6b6b6b6;

/// FOURCC magic for compiled GDScript.
pub const MAGIC: u32 = u32::from_le_bytes(*b"GDSC");

#[derive(Debug, Clone)]
pub struct TokenizerBytecode<'v> {
    /// Engine version we're parsing for.
    version: &'v GDScriptBuild,

    token_lines: HashMap<usize, usize>,
    token_columns: HashMap<usize, usize>,
    tokens: Vec<Spanned<Token>>,
    current: usize,
    current_line: usize,
    multiline_mode: bool,
    indent_stack: Vec<usize>,
    indent_stack_stack: Vec<Vec<usize>>,
    pending_indents: isize,
    last_token_was_newline: bool,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum CompressMode {
    None,
    Zstd,
}

/// Converts a list of tokens into the representation Godot uses for compiled GDScripts.
pub fn reconstruct_script_binary(
    version: &GDScriptBuild,
    tokens: &[Spanned<Token>],
    compression: CompressMode,
    real_t_is_double: bool,
) -> Result<Vec<u8>, Error> {
    // The Godot binary token format doesn't contain any indent, dedent or newline tokens (due to
    // generating the token stream with the multiline mode set on text tokenizers, which disables
    // generation of newlines). Instead, they are synthesized based on the line/column information
    // within the buffer (token_lines and token_columns in the Godot code) - for a nicer API we
    // want to generate a normal token stream with whitespace tokens, so we have to generate span
    // information that recreates those tokens.

    let mut buffer = WritableMarshalBuffer::new(real_t_is_double);
    let mut identifiers = IndexSet::new();
    let mut constants = IndexSet::new();
    let mut token_lines = HashMap::new();
    let mut token_columns = HashMap::new();
    let mut token_counter = 0;

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    enum NewlineState {
        /// Emitting normal tokens.
        None,

        /// We saw a continuation-type newline, don't emit `token_lines` but allow indent changes.
        Continuation,

        /// We saw a normal newline, emit `token_lines` on the next indent change.
        Normal,
    }

    // Information on the most recently seen newline token.
    let mut newline_state = NewlineState::None;

    // Indentation level for the current line.
    let mut current_indentation = 0u32;

    // Whether we've seen an `Indent` token since the last token that generated a newline - Godot
    // will only ever emit one indent token for each newline.
    let mut saw_indent_since_last_token = false;

    // TODO: error returns here aren't descriptive

    for (token, span) in tokens.iter() {
        match token {
            Token::Eof => break,
            Token::Indent => {
                if newline_state == NewlineState::None {
                    // Indent/dedent tokens must come directly after a newline, as Godot only
                    // changes the indentation level after a newline.
                    return Err(BadData);
                }

                if saw_indent_since_last_token {
                    // Godot only ever adds one indent per newline.
                    return Err(BadData);
                }

                current_indentation += 1;
                saw_indent_since_last_token = true;
                continue;
            }

            Token::Dedent => {
                if newline_state == NewlineState::None {
                    return Err(BadData);
                }

                if current_indentation == 0 {
                    // error in the input token stream (more dedents than indents)
                    return Err(BadData);
                }

                current_indentation -= 1;
                continue;
            }

            Token::Newline { continuation: true } => {
                // Godot doesn't emit anything for line continuations.
                if newline_state == NewlineState::Normal {
                    // both normal newlines and line continuations
                    return Err(BadData);
                }

                newline_state = NewlineState::Continuation;
                continue;
            }

            Token::Newline {
                continuation: false,
            } => {
                if newline_state == NewlineState::Continuation {
                    // both normal newlines and line continuations
                    return Err(BadData);
                }

                newline_state = NewlineState::Normal;
                continue;

                // The tokenizer generates a newline token if `token_lines` contains a value for
                // the current token, and then emits indentation tokens based on the `token_columns`
                // value.

                // If the current token index is in `token_lines` and the previous token was a real
                // token (not an indent, dedent or newline) then the buffer tokenizer emits a
                // newline and updates the current indentation level based on the `token_column`
                // value. The span applied to tokens is based on

                // Godot stores line continuations in bytecode form by removing the `token_lines`
                // entry for the last token in a line.
            }

            _ => {
                if newline_state == NewlineState::Normal {
                    // Emit token line / column to generate whitespace.
                    // NOTE: this line number is used as the line number for synthetic tokens
                    token_lines.insert(token_counter, span.start.line as u32);
                    token_columns.insert(token_counter, 1 + current_indentation);
                    saw_indent_since_last_token = false;
                }

                newline_state = NewlineState::None;

                let (token_id, _) = version
                    .tokens
                    .iter()
                    .enumerate()
                    .find(|(_, typ)| **typ == token.typ())
                    .ok_or(BadData)?;

                let token_id = match token {
                    Token::Annotation(identifier) | Token::Identifier(identifier) => {
                        let (pos, _) = identifiers.insert_full(identifier.clone());
                        (token_id as u32) | ((pos as u32) << TOKEN_BITS)
                    }

                    Token::Error(lit) | Token::Literal(lit) => {
                        let (pos, _) = constants.insert_full(lit.clone());
                        (token_id as u32) | ((pos as u32) << TOKEN_BITS)
                    }

                    _ => token_id as u32,
                };

                if (token_id & TOKEN_MASK) != 0 {
                    buffer.encode_uint32(token_id | (TOKEN_BYTE_MASK as u32));
                } else {
                    buffer.push(token_id as u8);
                }

                // This line number is used as the line number for non-synthetic tokens.
                buffer.encode_uint32(span.start.line as u32);
            }
        }

        token_counter += 1;
    }

    let mut contents = WritableMarshalBuffer::new(real_t_is_double);
    contents.encode_uint32(identifiers.len() as u32);
    contents.encode_uint32(constants.len() as u32);
    contents.encode_uint32(token_lines.len() as u32);

    if version.has_extra_word_in_binary_script_header {
        contents.encode_uint32(1337);
    }

    contents.encode_uint32(token_counter);

    // Save identifiers.
    for ident in identifiers {
        let chars = ident.chars().collect::<Vec<_>>();
        contents.encode_uint32(chars.len() as u32);

        for ch in chars {
            contents.encode_uint32((ch as u32) ^ INEXPLICABLE_XOR_KEY);
        }
    }

    // Save constants.
    for constant in constants {
        constant.encode(&mut contents, false)?;
    }

    // Save lines and columns.
    for (counter, line) in token_lines {
        contents.encode_uint32(counter);
        contents.encode_uint32(line);
    }

    for (counter, column) in token_columns {
        contents.encode_uint32(counter);
        contents.encode_uint32(column);
    }

    // Store tokens.
    contents.buffer().extend_from_slice(&buffer);

    let mut buf = WritableMarshalBuffer::new(real_t_is_double);
    buf.encode_uint32(MAGIC);
    buf.encode_uint32(
        version
            .tokenizer_version
            .expect("tried to encode tokens for a Godot build without a tokenizer version"),
    );

    match compression {
        CompressMode::None => {
            buf.encode_uint32(0);
            buf.buffer().extend_from_slice(&contents);
        }

        CompressMode::Zstd => {
            buf.encode_uint32(contents.len() as u32);

            let compressed =
                ruzstd::encoding::compress_to_vec(&*contents, CompressionLevel::Default);
            buf.buffer().extend_from_slice(&compressed);
        }
    }

    Ok(buf.into_inner())
}

/// Reads a single token from a buffer.
fn binary_to_token(
    identifiers: &[String],
    constants: &[Variant],
    tokens: &[TokenType],
    buf: &mut ReadableMarshalBuffer<'_>,
) -> Result<Spanned<Token>, Error> {
    buf.ensure_remaining(1)?;
    let first_byte = buf.buffer()[0];
    let is_4_byte_token = first_byte & TOKEN_BYTE_MASK != 0;

    let raw_id = if is_4_byte_token {
        buf.decode_uint32()?
    } else {
        buf.advance(1);
        first_byte as u32
    };

    buf.mark();

    let token_type = tokens.get((raw_id & TOKEN_MASK) as usize).ok_or(BadData)?;

    let start_line = buf.decode_uint32()?;
    buf.mark();

    let pos = Position {
        line: start_line as usize,
        column: 0,
    };
    let span = Span::new(pos, pos);

    let token = match token_type {
        TokenType::Annotation | TokenType::Identifier => {
            // Get name from map.
            let identifier_pos = (raw_id >> TOKEN_BITS) as usize;

            if identifier_pos >= identifiers.len() {
                let error = Token::Error("Identifier index out of bounds.".into());
                return Ok((error, Span::zero()));
            }

            if *token_type == TokenType::Annotation {
                Token::Annotation(identifiers[identifier_pos].clone())
            } else {
                Token::Identifier(identifiers[identifier_pos].clone())
            }
        }

        TokenType::Error | TokenType::Literal => {
            // Get literal from map.
            let constant_pos = (raw_id >> TOKEN_BITS) as usize;

            if constant_pos >= constants.len() {
                let error = Token::Error("Constant index out of bounds.".into());
                return Ok((error, Span::zero()));
            }

            let constant = constants[constant_pos].clone();

            if *token_type == TokenType::Error {
                Token::Error(constant)
            } else {
                Token::Literal(constant)
            }
        }

        // Whitespace tokens should never appear in bytecode.
        TokenType::Newline | TokenType::Indent | TokenType::Dedent => return Err(BadData),

        TokenType::Empty => Token::Empty,
        TokenType::Less => Token::Less,
        TokenType::LessEqual => Token::LessEqual,
        TokenType::Greater => Token::Greater,
        TokenType::GreaterEqual => Token::GreaterEqual,
        TokenType::EqualEqual => Token::EqualEqual,
        TokenType::BangEqual => Token::BangEqual,
        TokenType::And => Token::And,
        TokenType::Or => Token::Or,
        TokenType::Not => Token::Not,
        TokenType::AmpersandAmpersand => Token::AmpersandAmpersand,
        TokenType::PipePipe => Token::PipePipe,
        TokenType::Bang => Token::Bang,
        TokenType::Ampersand => Token::Ampersand,
        TokenType::Pipe => Token::Pipe,
        TokenType::Tilde => Token::Tilde,
        TokenType::Caret => Token::Caret,
        TokenType::LessLess => Token::LessLess,
        TokenType::GreaterGreater => Token::GreaterGreater,
        TokenType::Plus => Token::Plus,
        TokenType::Minus => Token::Minus,
        TokenType::Star => Token::Star,
        TokenType::StarStar => Token::StarStar,
        TokenType::Slash => Token::Slash,
        TokenType::Percent => Token::Percent,
        TokenType::Equal => Token::Equal,
        TokenType::PlusEqual => Token::PlusEqual,
        TokenType::MinusEqual => Token::MinusEqual,
        TokenType::StarEqual => Token::StarEqual,
        TokenType::StarStarEqual => Token::StarStarEqual,
        TokenType::SlashEqual => Token::SlashEqual,
        TokenType::PercentEqual => Token::PercentEqual,
        TokenType::LessLessEqual => Token::LessLessEqual,
        TokenType::GreaterGreaterEqual => Token::GreaterGreaterEqual,
        TokenType::AmpersandEqual => Token::AmpersandEqual,
        TokenType::PipeEqual => Token::PipeEqual,
        TokenType::CaretEqual => Token::CaretEqual,
        TokenType::If => Token::If,
        TokenType::Elif => Token::Elif,
        TokenType::Else => Token::Else,
        TokenType::For => Token::For,
        TokenType::While => Token::While,
        TokenType::Break => Token::Break,
        TokenType::Continue => Token::Continue,
        TokenType::Pass => Token::Pass,
        TokenType::Return => Token::Return,
        TokenType::Match => Token::Match,
        TokenType::When => Token::When,
        TokenType::As => Token::As,
        TokenType::Assert => Token::Assert,
        TokenType::Await => Token::Await,
        TokenType::Breakpoint => Token::Breakpoint,
        TokenType::Class => Token::Class,
        TokenType::ClassName => Token::ClassName,
        TokenType::Const => Token::Const,
        TokenType::Enum => Token::Enum,
        TokenType::Extends => Token::Extends,
        TokenType::Func => Token::Func,
        TokenType::In => Token::In,
        TokenType::Is => Token::Is,
        TokenType::Namespace => Token::Namespace,
        TokenType::Preload => Token::Preload,
        TokenType::_Self => Token::_Self,
        TokenType::Signal => Token::Signal,
        TokenType::Static => Token::Static,
        TokenType::Super => Token::Super,
        TokenType::Trait => Token::Trait,
        TokenType::Var => Token::Var,
        TokenType::Void => Token::Void,
        TokenType::Yield => Token::Yield,
        TokenType::BracketOpen => Token::BracketOpen,
        TokenType::BracketClose => Token::BracketClose,
        TokenType::BraceOpen => Token::BraceOpen,
        TokenType::BraceClose => Token::BraceClose,
        TokenType::ParenthesisOpen => Token::ParenthesisOpen,
        TokenType::ParenthesisClose => Token::ParenthesisClose,
        TokenType::Comma => Token::Comma,
        TokenType::Semicolon => Token::Semicolon,
        TokenType::Period => Token::Period,
        TokenType::PeriodPeriod => Token::PeriodPeriod,
        TokenType::PeriodPeriodPeriod => Token::PeriodPeriodPeriod,
        TokenType::Colon => Token::Colon,
        TokenType::Dollar => Token::Dollar,
        TokenType::ForwardArrow => Token::ForwardArrow,
        TokenType::Underscore => Token::Underscore,
        TokenType::ConstPi => Token::ConstPi,
        TokenType::ConstTau => Token::ConstTau,
        TokenType::ConstInf => Token::ConstInf,
        TokenType::ConstNan => Token::ConstNan,
        TokenType::VcsConflictMarker => Token::VcsConflictMarker,
        TokenType::Backtick => Token::Backtick,
        TokenType::QuestionMark => Token::QuestionMark,
        TokenType::Eof => Token::Eof,
        TokenType::Do => Token::Do,
        TokenType::Case => Token::Case,
        TokenType::Switch => Token::Switch,
        TokenType::Slave => Token::Slave,
        TokenType::SlaveSync => Token::SlaveSync,
        TokenType::BuiltInType => Token::BuiltInType,
        TokenType::BuiltInFunc => Token::BuiltInFunc,
        TokenType::OnReady => Token::OnReady,
        TokenType::Tool => Token::Tool,
        TokenType::Export => Token::Export,
        TokenType::SetGet => Token::SetGet,
        TokenType::Remote => Token::Remote,
        TokenType::Sync => Token::Sync,
        TokenType::Master => Token::Master,
        TokenType::Puppet => Token::Puppet,
        TokenType::RemoteSync => Token::RemoteSync,
        TokenType::MasterSync => Token::MasterSync,
        TokenType::PuppetSync => Token::PuppetSync,
        TokenType::Wildcard => Token::Wildcard,
        TokenType::Cursor => Token::Cursor,
        TokenType::Abstract => Token::Abstract,
    };

    Ok((token, span))
}

impl<'v> TokenizerBytecode<'v> {
    fn read_token_lines(
        count: usize,
        buf: &mut ReadableMarshalBuffer,
    ) -> crate::Result<HashMap<usize, usize>> {
        let mut token_lines = HashMap::with_capacity(count);

        for _ in 0..count {
            let token_index = buf.decode_uint32()?;
            let line = buf.decode_uint32()?;
            buf.mark();

            token_lines.insert(token_index as usize, line as usize);
        }

        Ok(token_lines)
    }

    fn create_current_position_span(&self) -> Span {
        let pos = Position {
            line: self.current_line,
            column: 0,
        };
        Span::new(pos, pos)
    }

    pub fn new(version: &'v GDScriptBuild, buf: &[u8]) -> crate::Result<TokenizerBytecode<'v>> {
        let mut buf = ReadableMarshalBuffer::new(buf, false);

        if buf.decode_uint32()? != MAGIC {
            return Err(BadData);
        }

        let wanted_version = buf.decode_uint32()?;

        if version.tokenizer_version != Some(wanted_version) {
            return Err(UnknownVersion(wanted_version));
        }

        let decompressed_size = buf.decode_uint32()?;

        let buf = if decompressed_size == 0 {
            Cow::Borrowed(buf.buffer())
        } else {
            let mut result = Vec::with_capacity(decompressed_size as usize);

            let mut decoder = StreamingDecoder::new(buf.buffer()).map_err(|_| BadData)?;
            let amount_decoded = decoder.read_to_end(&mut result).map_err(|_| BadData)?;

            if amount_decoded != decompressed_size as usize {
                return Err(BadData);
            }

            Cow::Owned(result)
        };

        let mut buf = ReadableMarshalBuffer::new(&buf, false);

        let identifier_count = buf.decode_uint32()? as usize;
        let constant_count = buf.decode_uint32()? as usize;
        let token_line_count = buf.decode_uint32()? as usize;

        if version.has_extra_word_in_binary_script_header {
            buf.decode_uint32()?;
        }

        let token_count = buf.decode_uint32()? as usize;
        buf.mark();

        let mut identifiers = Vec::with_capacity(identifier_count);
        for _ in 0..identifier_count {
            let len = buf.decode_uint32()? as usize;
            buf.mark();

            let Some(len_bytes) = len.checked_mul(4) else {
                return Err(BadData);
            };

            let mut identifier = String::with_capacity(len_bytes);
            buf.ensure_remaining(len_bytes)?;

            for _ in 0..len {
                // XXX: wtf? why is this XOR ciphered?
                let ch = buf.decode_uint32()?;
                let ch = ch ^ INEXPLICABLE_XOR_KEY;
                let ch = char::try_from(ch).map_err(|_| BadData)?;
                identifier.push(ch);

                buf.mark();
            }

            identifiers.push(identifier);
        }

        let mut constants = Vec::with_capacity(constant_count);
        for _ in 0..constant_count {
            let variant = Variant::decode(&mut buf, false)?;
            constants.push(variant);
        }

        let token_lines = Self::read_token_lines(token_line_count, &mut buf)?;
        let token_columns = Self::read_token_lines(token_line_count, &mut buf)?;

        let mut tokens = Vec::with_capacity(token_count);
        for _ in 0..token_count {
            let token = binary_to_token(&identifiers, &constants, &version.tokens, &mut buf)?;

            tokens.push(token);
        }

        if buf.remaining() != 0 {
            return Err(BadData);
        }

        Ok(Self {
            version,
            token_lines,
            token_columns,
            tokens,
            current: 0,
            current_line: 0,
            multiline_mode: false,
            indent_stack: vec![],
            indent_stack_stack: vec![],
            pending_indents: 0,
            last_token_was_newline: false,
        })
    }
}

impl<'v> Iterator for TokenizerBytecode<'v> {
    type Item = Spanned<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        // Add final newline.
        if self.current >= self.tokens.len() && !self.last_token_was_newline {
            self.last_token_was_newline = true;

            let span = self.create_current_position_span();
            return Some((
                Token::Newline {
                    continuation: false,
                },
                span,
            ));
        }

        // Resolve pending indentation change.
        if self.pending_indents > 0 {
            self.pending_indents -= 1;
            let span = self.create_current_position_span();
            return Some((Token::Indent, span));
        } else if self.pending_indents < 0 {
            self.pending_indents += 1;
            let span = self.create_current_position_span();
            return Some((Token::Dedent, span));
        }

        if self.current >= self.tokens.len() {
            if !self.indent_stack.is_empty() {
                self.pending_indents -= self.indent_stack.len() as isize;
                self.indent_stack.clear();
                return self.next();
            }

            return if self.current == usize::MAX {
                None
            } else {
                self.current = usize::MAX;

                let span = self.create_current_position_span();
                Some((Token::Eof, span))
            };
        };

        let mut emit_newline = false;

        if !self.last_token_was_newline && self.token_lines.contains_key(&self.current) {
            self.current_line = self.token_lines[&self.current];
            let current_column = self.token_columns[&self.current];

            // Check if there's a need to indent/dedent.
            if !self.multiline_mode {
                let mut previous_indent = self.indent_stack.last().copied().unwrap_or(0);

                if current_column - 1 > previous_indent {
                    self.pending_indents += 1;
                    self.indent_stack.push(current_column - 1);
                } else {
                    while current_column - 1 < previous_indent {
                        self.pending_indents -= 1;
                        self.indent_stack.pop();

                        if let Some(last) = self.indent_stack.last() {
                            previous_indent = *last;
                        } else {
                            break;
                        }
                    }
                }

                emit_newline = true;
            }
        }

        Some(if emit_newline {
            self.last_token_was_newline = true;

            (
                Token::Newline {
                    continuation: false,
                },
                self.create_current_position_span(),
            )
        } else {
            self.last_token_was_newline = false;

            let token = self.tokens[self.current].clone();
            self.current += 1;
            token
        })
    }
}

impl<'v> Sealed for TokenizerBytecode<'v> {}
impl<'v> Tokenizer for TokenizerBytecode<'v> {
    fn version(&self) -> &GDScriptBuild {
        self.version
    }

    fn set_multiline_mode(&mut self, multiline: bool) {
        self.multiline_mode = multiline;
    }

    fn push_expression_indented_block(&mut self) {
        self.indent_stack_stack.push(self.indent_stack.clone());
    }

    fn pop_expression_indented_block(&mut self) -> isize {
        let current = self.indent_stack.len() as isize;
        self.indent_stack = self
            .indent_stack_stack
            .pop()
            .expect("mismatched pop_expression_indented_block call");
        let previous = self.indent_stack.len() as isize;
        previous - current
    }
}
