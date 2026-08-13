//! Converts a series of tokens back into a text format that parses to approximately the same
//! input (not including token spans).
// This code is mostly ported from GDRETools/gdsdecomp, an MIT licensed project.
//
// MIT License
//
// Copyright (c) 2019 bruvzg
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

use crate::gdscript::{Span, Spanned, Token, TokenType};
use crate::util::escape_string;
use crate::variant::Variant;

fn is_token_newline_or_indent(token: &Token) -> bool {
    matches!(token, Token::Newline { .. } | Token::Indent | Token::Dedent)
}

#[derive(Debug)]
struct TextReconstructor {
    script_text: String,
    line: String,
    indent: usize,
    first_line: bool,
    prev_token: TokenType,
    prev_line: usize,
}

impl TextReconstructor {
    fn reconstitute_variant(&mut self, v: &Variant) {
        match v {
            Variant::Nil(_) => self.line.push_str("null"),
            Variant::Bool(b) => self.line.push_str(if *b { "true" } else { "false" }),
            Variant::Int(i) => self.line.push_str(&i.to_string()),
            Variant::Float(f) => {
                let mut buffer = zmij::Buffer::new();
                self.line.push_str(buffer.format(f.0));
            }
            Variant::String(s) => {
                self.line.push_str(&format!("\"{}\"", escape_string(s)));
            }
            Variant::StringName(s) => {
                self.line.push_str(&format!("&\"{}\"", escape_string(&s.0)));
            }
            Variant::NodePath(s) => {
                self.line
                    .push_str(&format!("^\"{}\"", escape_string(&s.to_string())));
            }

            _ => unimplemented!(),
        }
    }

    fn write_current_line(&mut self) {
        for _ in 0..self.indent {
            // TODO: support indentation with spaces?
            self.script_text.push('\t');
        }

        self.script_text.push_str(&self.line);
    }

    fn handle_newline(&mut self, _span: &Span) {
        // let mut curr_line = span.end.line;
        self.write_current_line();
        self.script_text.push('\n');

        // if curr_line <= self.prev_line {
        //     curr_line = self.prev_line + 1; // force new line
        // }
        //
        // while curr_line > self.prev_line {
        //     // TODO: missing support for <2.0
        //
        //     if span.start.line != span.end.line {
        //         if !self.first_line
        //             || !self
        //                 .line
        //                 .trim_matches(|c| c == ' ' || c == '\t' || c == '\n' || c == '\r')
        //                 .is_empty()
        //         {
        //             self.script_text.push_str("\\");
        //         }
        //     }
        //
        //     self.script_text.push_str("\n");
        //     self.prev_line += 1;
        // }

        self.first_line = false;
        self.line.clear();
        self.prev_token = TokenType::Newline;
    }

    fn check_new_line(&self, _span: &Span) -> bool {
        // let ln = span.start.line;
        // if ln > self.prev_line && ln != 0 {
        //     return true;
        // }
        //
        // let ln = span.end.line;
        // if ln != self.prev_line && ln != 0 {
        //     return true;
        // }

        false
    }

    fn maybe_emit_multiline_string(&mut self, token: &Token, span: &Span) -> bool {
        let Token::Literal(Variant::String(content)) = token else {
            return false;
        };

        let content = content.replace("\\", "\\\\").replace("\"", "\\\"");
        let num_newlines = content.chars().filter(|c| *c == '\n').count();
        let num_lines = span.end.line - span.start.line;

        if num_newlines == num_lines {
            self.line.push_str("\"\"\"");
            self.line.push_str(&content);
            self.line.push_str("\"\"\"");
            self.prev_line = span.end.line;
            true
        } else {
            false
        }
    }

    fn reconstruct(&mut self, tokens: &[Spanned<Token>]) {
        #[derive(Debug, Copy, Clone)]
        enum Padding {
            Empty,
            Before,
            After(Option<TokenType>),
            Both(Option<TokenType>),
        }

        use Padding::*;

        let mut iterator = tokens.iter().peekable();

        while let Some((token, span)) = iterator.next() {
            if !is_token_newline_or_indent(token) && self.check_new_line(span) {
                if self.maybe_emit_multiline_string(token, span) {
                    continue;
                }

                self.handle_newline(span);
            }

            let (text, padding) = match token {
                Token::Empty => ("", Empty),
                Token::Annotation(ident) | Token::Identifier(ident) => (ident.as_ref(), Empty),
                Token::Literal(lit) => {
                    if let Some((next_tk, next_span)) = iterator.peek()
                        && self.check_new_line(next_span)
                        && !matches!(next_tk, Token::Newline { .. })
                        && span.start.line == next_span.start.line
                        && self.maybe_emit_multiline_string(token, span)
                    {
                        continue;
                    }

                    self.reconstitute_variant(lit);
                    self.prev_token = TokenType::Literal;
                    continue;
                }
                Token::_Self => ("self", Empty),
                Token::BuiltInType | Token::BuiltInFunc => unimplemented!("GDScript V1"),
                Token::In => ("in ", Before),
                Token::EqualEqual => ("== ", Before),
                Token::BangEqual => ("!= ", Before),
                Token::Less => ("< ", Before),
                Token::LessEqual => ("<= ", Before),
                Token::Greater => ("> ", Before),
                Token::GreaterEqual => (">= ", Before),
                Token::And => ("and ", Before),
                Token::Or => ("or ", Before),
                Token::Not => ("not", Both(None)),
                Token::Plus => ("+ ", Before),
                Token::Minus => ("- ", Before), // FIXME: don't add space for unary minus
                Token::Star => ("* ", Before),
                Token::Slash => ("/ ", Before),
                Token::Percent => {
                    let padding = if self.prev_token == TokenType::Literal
                        || self.prev_token == TokenType::Identifier
                        || self.prev_token == TokenType::ParenthesisClose
                    {
                        Both(None)
                    } else {
                        Before
                    };

                    ("%", padding)
                }
                Token::LessLess => ("<< ", Before),
                Token::GreaterGreater => (">> ", Before),
                Token::Equal => ("= ", Before),
                Token::PlusEqual => ("+= ", Before),
                Token::MinusEqual => ("-= ", Before),
                Token::StarEqual => ("*= ", Before),
                Token::SlashEqual => ("/= ", Before),
                Token::PercentEqual => ("%= ", Before),
                Token::LessLessEqual => ("<<= ", Before),
                Token::GreaterGreaterEqual => (">>= ", Before),
                Token::AmpersandEqual => ("&= ", Before),
                Token::PipeEqual => ("|= ", Before),
                Token::CaretEqual => ("^= ", Before),
                Token::Ampersand => ("& ", Before),
                Token::Pipe => ("| ", Before),
                Token::Caret => ("^ ", Before),
                Token::Tilde => ("~ ", Before),
                Token::If => ("if ", Before),
                Token::Elif => ("elif ", Empty),
                Token::Else => ("else", Both(Some(TokenType::Colon))),
                Token::For => ("for ", Empty),
                Token::While => ("while ", Empty),
                Token::Break => ("break", Empty),
                Token::Continue => ("continue", Empty),
                Token::Pass => ("pass", Empty),
                Token::Return => ("return", After(None)),
                Token::Match => ("match", After(None)),
                Token::Func => ("func", Both(Some(TokenType::ParenthesisOpen))),
                Token::Class => ("class ", Before),
                Token::ClassName => ("class_name ", Before),
                Token::Extends => ("extends ", Before),
                Token::Is => ("is ", Before),
                Token::OnReady => ("onready ", Empty),
                Token::Tool => ("tool ", Empty),
                Token::Static => ("static ", Empty),
                Token::Export => ("export ", Empty),
                Token::SetGet => (" setget ", Empty),
                Token::Const => ("const ", Empty),
                Token::Var => ("var ", Before),
                Token::As => ("as ", Before),
                Token::Void => ("void ", Empty),
                Token::Enum => ("enum ", Empty),
                Token::Preload => ("preload", Empty),
                Token::Assert => ("assert ", Empty),
                Token::Yield => ("yield", After(Some(TokenType::ParenthesisOpen))),
                Token::Signal => ("signal ", Empty),
                Token::Breakpoint => ("breakpoint", After(None)),
                Token::Remote => ("remote ", Empty),
                Token::Sync => ("sync ", Empty),
                Token::Master => ("master ", Empty),
                Token::Slave => ("slave ", Empty),
                Token::Puppet => ("puppet ", Empty),
                Token::RemoteSync => ("remotesync ", Empty),
                Token::MasterSync => ("mastersync ", Empty),
                Token::PuppetSync => ("puppetsync ", Empty),
                Token::BracketOpen => ("[", Empty),
                Token::BracketClose => ("]", Empty),
                Token::BraceOpen => ("{", Empty),
                Token::BraceClose => ("}", Empty),
                Token::ParenthesisOpen => ("(", Empty),
                Token::ParenthesisClose => (")", Empty),
                Token::Comma => (", ", Empty),
                Token::Semicolon => (";", Empty),
                Token::Period => (".", Empty),
                Token::QuestionMark => ("?", Empty),
                Token::Colon => (":", After(None)),
                Token::Dollar => ("$", Empty),
                Token::ForwardArrow => ("->", Both(None)),
                Token::Indent => {
                    self.indent += 1;
                    ("", Empty)
                }
                Token::Dedent => {
                    self.indent -= 1;
                    ("", Empty)
                }
                Token::Newline { continuation: _ } => {
                    self.handle_newline(span);
                    ("", Empty)
                }
                Token::ConstPi => ("PI", Empty),
                Token::ConstTau => ("TAU", Empty),
                Token::Wildcard => ("_", Empty),
                Token::ConstInf => ("INF", Empty),
                Token::ConstNan => ("NAN", Empty),
                Token::SlaveSync => ("slavesync ", Empty),
                Token::Do => ("do ", Empty),
                Token::Case => ("case ", Empty),
                Token::Switch => ("switch ", Empty),
                Token::AmpersandAmpersand => ("&& ", Before),
                Token::PipePipe => ("|| ", Before),
                Token::Bang => ("!", Before),
                Token::StarStar => ("** ", Before),
                Token::StarStarEqual => ("**= ", Before),
                Token::When => ("when ", Before),
                Token::Await => ("await ", Before),
                Token::Namespace => ("namespace ", Before),
                Token::Super => ("super ", Both(Some(TokenType::Period))),
                Token::Trait => ("trait ", Before),
                Token::PeriodPeriod => ("..", Empty),
                Token::PeriodPeriodPeriod => ("...", Empty),
                Token::Underscore => ("_", Empty),
                Token::Backtick => ("`", Empty),
                Token::Abstract => ("abstract ", Empty),
                Token::Error(_) => ("", Empty),
                Token::Eof => ("", Empty),
                Token::Cursor => ("", Empty),
                Token::VcsConflictMarker => ("", Empty),
            };

            if matches!(padding, Before | Both(_))
                && !self.line.ends_with(" ")
                && !matches!(
                    self.prev_token,
                    TokenType::Newline | TokenType::Indent | TokenType::Dedent,
                )
            {
                self.line.push(' ');
            }

            self.line.push_str(text);

            if let Both(check_tk) | After(check_tk) = padding
                && let Some(next_tk) = iterator.peek()
            {
                let next_token_isnt_check_token =
                    check_tk.is_none() || Some(next_tk.0.typ()) != check_tk;
                let next_token_is_not_newline =
                    !is_token_newline_or_indent(&next_tk.0) && !self.check_new_line(&next_tk.1);

                if !self.line.ends_with(" ")
                    && next_token_is_not_newline
                    && next_token_isnt_check_token
                {
                    self.line += " ";
                }
            }

            self.prev_token = token.typ();
        }

        if !self.line.is_empty() {
            self.write_current_line();
        }
    }
}

/// Reconstructs GDScript text from a series of tokens.
pub fn reconstruct_script_text(tokens: &[Spanned<Token>]) -> String {
    let mut reconstructor = TextReconstructor {
        script_text: String::new(),
        line: String::new(),
        indent: 0,
        first_line: true,
        prev_token: TokenType::Newline,
        prev_line: 1,
    };

    reconstructor.reconstruct(tokens);
    reconstructor.script_text
}
