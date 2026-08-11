//! Rust port of the GDScript text tokenizer.

use crate::build::GDScriptBuild;
use crate::gdscript::tokenizer::Tokenizer;
use crate::gdscript::{Position, Span, Spanned, Token};
use crate::private::Sealed;
use crate::string::{
    GodotUnicodeXID, bin_to_int, hex_to_int, is_godot_whitespace, to_float, to_int,
};
use crate::util::NPeekable;
use crate::variant::{Nil, NodePath, Variant};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::mem::take;
use std::str::{Chars, FromStr};
use unicode_xid::UnicodeXID;

#[derive(Debug)]
pub struct TokenizerText<'v, 's> {
    /// Engine version we're parsing for.
    version: &'v GDScriptBuild,

    /// Full input source.
    source: &'s str,

    /// Peekable iterator of characters to consume.
    chars: NPeekable<Chars<'s>>,

    /// Offset of next character from start of string (in bytes).
    offset: usize,

    /// Offset of the start of the currently-being-parsed token.
    start_offset: usize,

    /// Position at the start of the token currently being parsed.
    start_position: Position,

    /// Current position in the source.
    position: Position,

    /// Whether this line is a continuation of the previous, like when using '\'.
    line_continuation: bool,
    multiline_mode: bool,
    error_stack: VecDeque<Spanned<Token>>,
    last_token: Option<Spanned<Token>>,
    last_newline: Option<Spanned<Token>>,
    pending_indents: isize,
    indent_stack: VecDeque<usize>,
    indent_stack_stack: Vec<VecDeque<usize>>,
    paren_stack: VecDeque<char>,
    indent_char: Option<char>,
}

const TAB_SIZE: usize = 4;

fn get_indent_char_name(ch: char) -> Cow<'static, str> {
    match ch {
        ' ' => Cow::Borrowed("space"),
        '\t' => Cow::Borrowed("tab"),
        other => Cow::Owned(other.escape_default().to_string()),
    }
}

impl<'v, 's> TokenizerText<'v, 's> {
    pub fn new(version: &'v GDScriptBuild, source: &'s str) -> Self {
        Self {
            version,
            source,
            chars: NPeekable::new(source.chars()),
            start_offset: 0,
            offset: 0,
            start_position: Position::default(),
            position: Position { line: 1, column: 1 },
            line_continuation: false,
            multiline_mode: false,
            error_stack: Default::default(),
            last_token: None,
            last_newline: None,
            pending_indents: 0,
            indent_stack: Default::default(),
            indent_stack_stack: Default::default(),
            paren_stack: Default::default(),
            indent_char: None,
        }
    }

    // Input manipulation
    #[must_use]
    fn peek(&mut self, offset: usize) -> char {
        self.chars.peek(offset).unwrap_or('\0')
    }

    fn advance(&mut self) -> char {
        let Some(ch) = self.chars.next() else {
            panic!("tried to advance past the end of the source buffer");
        };

        self.offset += ch.len_utf8();
        self.position.column += 1;

        if self.is_at_end() {
            self.newline(true);
            self.check_indent();
        }

        ch
    }

    fn is_at_end(&self) -> bool {
        self.offset >= self.source.len()
    }

    fn push_paren(&mut self, ch: char) {
        self.paren_stack.push_back(ch);
    }

    // Error handling
    fn push_error(&mut self, message: Cow<'s, str>) {
        let token = Token::Error(message.into_owned().into());
        let error = self.span_token_and_set_last(token);
        self.error_stack.push_back(error);
    }

    fn push_single_char_error(&mut self, message: Cow<'s, str>) {
        let error = Token::Error(message.into_owned().into());
        let mut spanned = self.span_token_and_set_last(error);
        spanned.1.start.column = self.position.column;
        spanned.1.end.column = self.position.column + 1;
        self.error_stack.push_back(spanned);
    }

    /// Spans a token with the current span. Also sets it as the last token emitted.
    #[must_use]
    fn span_token_and_set_last(&mut self, token: Token) -> Spanned<Token> {
        let span = Span::new(self.start_position, self.position);
        let spanned = (token, span);

        self.last_token = Some(spanned.clone());
        spanned
    }

    fn pop_paren_or_make_error(
        &mut self,
        closing: char,
        wanted: char,
        token: Token,
    ) -> Spanned<Token> {
        // XXX: Godot's handling of this error case is bugged! They pop two parentheses instead of
        // one if they don't match.
        let got = self.paren_stack.pop_back();

        if got == Some(wanted) {
            return self.span_token_and_set_last(token);
        }

        let message = if let Some(ch) = self.paren_stack.pop_back() {
            format!(
                "Closing \"{}\" doesn't match the opening \"{}\".",
                closing, ch
            )
        } else {
            format!(
                "Closing \"{}\" doesn't have an opening counterpart.",
                closing
            )
        };

        let token = Token::Error(message.into());
        self.span_token_and_set_last(token)
    }

    fn check_vcs_marker(&mut self, test: char, double_token: Token) -> Spanned<Token> {
        let mut chars = 2;
        let mut next = 1;

        while self.peek(next) == test {
            chars += 1;
            next += 1;
        }

        if chars >= 7 {
            // It is a VCS conflict marker.
            while chars > 1 {
                // Consume all characters (first was already consumed by scan()).
                self.advance();
                chars -= 1;
            }

            self.span_token_and_set_last(Token::VcsConflictMarker)
        } else {
            // It's a regular double character token, consume the 2nd character.
            self.advance();
            self.span_token_and_set_last(double_token)
        }
    }

    fn annotation(&mut self) -> Spanned<Token> {
        if self.peek(0).is_godot_xid_start() {
            self.advance(); // Consume start character.
        } else {
            self.push_error("Expected annotation identifier after \"@\".".into());
        }

        while self.peek(0).is_xid_continue() {
            // Consume all identifier characters.
            self.advance();
        }

        let identifier = &self.source[self.start_offset..self.offset];
        let token = Token::Annotation(identifier.into());
        self.span_token_and_set_last(token)
    }

    fn potential_identifier(&mut self, start_char: char) -> Spanned<Token> {
        let mut only_ascii = start_char.is_ascii();

        while self.peek(0).is_xid_continue() {
            let ch = self.advance();
            only_ascii &= ch.is_ascii();
        }

        let len = self.offset - self.start_offset;

        if len == 1 && start_char == '_' {
            // Lone underscore.
            return self.span_token_and_set_last(Token::Underscore);
        }

        let ident = &self.source[self.start_offset..self.offset];

        // XXX: This is in a different position prior to f68beeb7faf060c74550e93dccaf27115c60a8ee
        // (4.2). I don't think it matters much.
        if !only_ascii {
            // Cannot be a keyword since keywords are ASCII-only.
            let token = Token::Identifier(ident.into());
            return self.span_token_and_set_last(token);
        }

        let matched_token = match ident {
            "as" => Token::As,
            "and" => Token::And,
            "assert" => Token::Assert,
            "await" => Token::Await,
            "break" => Token::Break,
            "breakpoint" => Token::Breakpoint,
            "class" => Token::Class,
            "class_name" => Token::ClassName,
            "const" => Token::Const,
            "continue" => Token::Continue,
            "elif" => Token::Elif,
            "else" => Token::Else,
            "enum" => Token::Enum,
            "extends" => Token::Extends,
            "for" => Token::For,
            "func" => Token::Func,
            "if" => Token::If,
            "in" => Token::In,
            "is" => Token::Is,
            "match" => Token::Match,
            "namespace" => Token::Namespace,
            "not" => Token::Not,
            "or" => Token::Or,
            "pass" => Token::Pass,
            "preload" => Token::Preload,
            "return" => Token::Return,
            "self" => Token::_Self,
            "signal" => Token::Signal,
            "static" => Token::Static,
            "super" => Token::Super,
            "trait" => Token::Trait,
            "var" => Token::Var,
            "void" => Token::Void,
            "while" => Token::While,
            // When was added in 54a1414500ee2f8f87647fc0ffe921498332446f (4.2)
            "when" if self.version.has_when => Token::When,
            "yield" => Token::Yield,
            "INF" => Token::ConstInf,
            "NAN" => Token::ConstNan,
            "PI" => Token::ConstPi,
            "TAU" => Token::ConstTau,
            "true" => Token::Literal(true.into()),
            "false" => Token::Literal(false.into()),
            "null" => Token::Literal(Nil.into()),

            // Not a keyword, make it an identifier.
            _ => Token::Identifier(ident.into()),
        };

        self.span_token_and_set_last(matched_token)
    }

    fn newline(&mut self, make_token: bool) {
        if make_token && self.last_newline.is_none() {
            let start = Position {
                line: self.position.line,
                column: self.position.column - 1,
            };

            let end = Position {
                line: self.position.line,
                column: self.position.column,
            };

            let token = (
                Token::Newline {
                    continuation: self.line_continuation,
                },
                Span::new(start, end),
            );

            self.last_token = Some(token.clone());
            self.last_newline = Some(token);
        }

        self.position.line += 1;
        self.position.column = 1;
    }

    fn number(&mut self, start_char: char) -> Spanned<Token> {
        let mut base = 10;
        let mut has_decimal = false;
        let mut has_exponent = false;
        let mut has_error = false;
        let mut need_digits = false;

        // Sign before hexadecimal or binary.
        // Added in d15511725acdfe90f9d5967119294b591becd8fa (4.1)
        if self.version.has_literal_sign_handling
            && (start_char == '+' || start_char == '-')
            && self.peek(0) == '0'
        {
            self.advance();
        }

        if start_char == '.' {
            has_decimal = true;
        } else if start_char == '0' {
            let next = self.peek(0);

            // Uppercase X/B support was added to 4.x in 3be46a69c431519fbe4b6a5d39374585fd994802
            // (4.4)
            if next == 'x' || (self.version.has_uppercase_number_types && next == 'X') {
                // Hexadecimal.
                base = 16;
                need_digits = true;
                self.advance();

                // Disallow `0x_` - since fba8cbe6dbf17399e06ac9141a862734187dfb65 (4.1)
                if self.version.has_new_number_underscore_parsing && self.peek(0) == '_' {
                    self.push_single_char_error(
                        format!("Unexpected underscore after \"0{}\".", next).into(),
                    );
                    has_error = true;
                }
            } else if next == 'b' || (self.version.has_uppercase_number_types && next == 'B') {
                // Binary.
                base = 2;
                need_digits = true;
                self.advance();

                // Disallow `0b_` - since fba8cbe6dbf17399e06ac9141a862734187dfb65 (4.1)
                if self.version.has_new_number_underscore_parsing && self.peek(0) == '_' {
                    self.push_single_char_error(
                        format!("Unexpected underscore after \"0{}\".", next).into(),
                    );
                    has_error = true;
                }
            }
        }

        // Allow `_` to be used in a number, for readability.
        let mut previous_was_underscore = false;

        while self.peek(0).is_digit(base) || self.peek(0) == '_' {
            if self.peek(0) == '_' {
                if previous_was_underscore {
                    if self.version.has_new_number_underscore_parsing {
                        self.push_single_char_error(
                            "Multiple underscores cannot be adjacent in a numeric literal.".into(),
                        );
                    } else {
                        self.push_single_char_error(
                            "Only one underscore can be used as a numeric separator.".into(),
                        )
                    }
                }

                previous_was_underscore = true;
            } else {
                need_digits = false;
                previous_was_underscore = false;
            }

            self.advance();
        }

        // It might be a ".." token (instead of decimal point) so we check if it's not.
        if self.peek(0) == '.' && self.peek(1) != '.' {
            if base == 10 && !has_decimal {
                has_decimal = true;
            } else if base == 10 {
                self.push_single_char_error("Cannot use a decimal point twice in a number.".into());
                has_error = true;
            } else if base == 16 {
                self.push_single_char_error(
                    "Cannot use a decimal point in a hexadecimal number.".into(),
                );
                has_error = true;
            } else if base == 2 {
                self.push_single_char_error(
                    "Cannot use a decimal point in a binary number.".into(),
                );
                has_error = true;
            }

            if !has_error {
                self.advance();

                // Consume decimal digits.
                if self.version.has_new_number_underscore_parsing && self.peek(0) == '_' {
                    // Disallow `10._`, but allow `10.`.
                    self.push_single_char_error(
                        "Unexpected underscore after decimal point.".into(),
                    );
                    has_error = true;
                }

                previous_was_underscore = false;

                while self.peek(0).is_ascii_digit() || self.peek(0) == '_' {
                    if self.version.has_new_number_underscore_parsing {
                        if self.peek(0) == '_' {
                            if previous_was_underscore {
                                self.push_single_char_error(
                                    "Multiple underscores cannot be adjacent in a numeric literal."
                                        .into(),
                                );
                            }

                            previous_was_underscore = true;
                        } else {
                            previous_was_underscore = false;
                        }
                    }

                    self.advance();
                }
            }
        }

        if base == 10 && (self.peek(0) == 'e' || self.peek(0) == 'E') {
            has_exponent = true;
            self.advance();

            if self.peek(0) == '+' || self.peek(0) == '-' {
                // Exponent sign.
                self.advance();
            }

            // Consume exponent digits.
            if !self.peek(0).is_ascii_digit() {
                self.push_single_char_error("Expected exponent value after \"e\".".into());
            }

            previous_was_underscore = false;

            while self.peek(0).is_ascii_digit() || self.peek(0) == '_' {
                if self.peek(0) == '_' {
                    if previous_was_underscore {
                        if self.version.has_new_number_underscore_parsing {
                            self.push_single_char_error(
                                "Multiple underscores cannot be adjacent in a numeric literal."
                                    .into(),
                            );
                        } else {
                            self.push_single_char_error(
                                "Only one underscore can be used as a numeric separator.".into(),
                            );
                        }
                    }

                    previous_was_underscore = true;
                } else {
                    previous_was_underscore = false;
                }

                self.advance();
            }
        }

        // Cherry-picked into 4.2.2 in 4e5b545c0465c8c007440e21b72c6d0ac35feb4e
        if self.version.need_digits_in_hex_and_binary && need_digits {
            assert!(base == 16 || base == 2);
            let error = Token::Error(
                format!(
                    "Expected {} digit after \"0{}\".",
                    if base == 16 { "hexadecimal" } else { "binary" },
                    if base == 16 { "x" } else { "b" }
                )
                .into(),
            );
            let mut spanned = self.span_token_and_set_last(error);
            spanned.1.start.column = self.position.column;
            spanned.1.end.column = self.position.column + 1;
            return spanned;
        }

        // Detect extra decimal point.
        if !has_error && has_decimal && self.peek(0) == '.' && self.peek(1) != '.' {
            self.push_single_char_error("Cannot use a decimal point twice in a number.".into());
        } else if self.peek(0).is_godot_xid_start() || self.peek(0).is_xid_continue() {
            self.push_error("Invalid numeric notation.".into());
        }

        // Create a string with the whole number.
        let number = self.source[self.start_offset..self.offset].replace('_', "");

        let value = if base == 16 {
            Variant::Int(hex_to_int(&number))
        } else if base == 2 {
            Variant::Int(bin_to_int(&number))
        } else if has_decimal || has_exponent {
            Variant::Float(to_float(&number).into())
        } else {
            Variant::Int(to_int(&number))
        };

        self.span_token_and_set_last(Token::Literal(value))
    }

    fn string(&mut self, start_char: char) -> Spanned<Token> {
        #[derive(Debug, Copy, Clone, Eq, PartialEq)]
        enum StringType {
            Regular,
            RegularRaw,
            Name,
            NodePath,
        }

        let mut is_multiline = false;
        let mut typ = StringType::Regular;

        let quote_char = match start_char {
            // Raw strings were added in 2964c7d51cbdaa616841c23d03f4a2f9966554b5 (4.2)
            'r' if self.version.has_raw_strings => {
                typ = StringType::RegularRaw;
                self.advance()
            }
            '&' => {
                typ = StringType::Name;
                self.advance()
            }
            '^' => {
                typ = StringType::NodePath;
                self.advance()
            }
            _ => start_char,
        };

        if self.peek(0) == quote_char && self.peek(1) == quote_char {
            is_multiline = true;
            // Consume all quotes.
            self.advance();
            self.advance();
        }

        let mut result = String::new();
        let mut prev = None;
        let mut prev_pos = 0;

        loop {
            // Consume actual string.
            if self.is_at_end() {
                let error = Token::Error("Unterminated string.".into());
                return self.span_token_and_set_last(error);
            }

            let ch = self.peek(0);
            let codepoint = ch as u32;

            if codepoint == 0x200E
                || codepoint == 0x200F
                || (0x202A..=0x202E).contains(&codepoint)
                || (0x2066..=0x2069).contains(&codepoint)
            {
                if typ == StringType::RegularRaw {
                    self.push_single_char_error("Invisible text direction control character present in the string, use regular string literal instead of r-string.".into());
                } else {
                    self.push_single_char_error(format!("Invisible text direction control character present in the string, escape it (\"\\u{:x}\") to avoid confusion.", codepoint).into());
                }
            }

            if ch == '\\' {
                // Escape pattern.
                self.advance();

                if self.is_at_end() {
                    let error = Token::Error("Unterminated string.".into());
                    return self.span_token_and_set_last(error);
                }

                if typ == StringType::RegularRaw {
                    if self.peek(0) == quote_char {
                        self.advance();

                        if self.is_at_end() {
                            let error = Token::Error("Unterminated string.".into());
                            return self.span_token_and_set_last(error);
                        }

                        result.reserve(2);
                        result.push('\\');
                        result.push(quote_char);
                    } else if self.peek(0) == '\\' {
                        // For `\\\"`.
                        self.advance();

                        if self.is_at_end() {
                            let error = Token::Error("Unterminated string.".into());
                            return self.span_token_and_set_last(error);
                        }

                        result.reserve(2);
                        result.push('\\');
                        result.push('\\');
                    } else {
                        result.push('\\');
                    }
                } else {
                    // Grab escape character.
                    let ch = self.advance();

                    if self.is_at_end() {
                        let error = Token::Error("Unterminated string.".into());
                        return self.span_token_and_set_last(error);
                    }

                    let escaped = match ch {
                        'a' => Some('\x07'),
                        'b' => Some('\x08'),
                        'f' => Some('\x0C'),
                        'n' => Some('\n'),
                        'r' => Some('\r'),
                        't' => Some('\t'),
                        'v' => Some('\x0B'),
                        '\'' => Some('\''),
                        '\"' => Some('\"'),
                        '\\' => Some('\\'),
                        'U' | 'u' => {
                            // Hexadecimal sequence.
                            let hex_len = if ch == 'U' { 6 } else { 4 };
                            let mut result = 0;

                            let mut i = 0;

                            loop {
                                if i >= hex_len {
                                    // XXX: This check doesn't exist in Godot but it should be fine
                                    break Some(char::try_from(result).unwrap());
                                }

                                if self.is_at_end() {
                                    let error = Token::Error("Unterminated string.".into());
                                    return self.span_token_and_set_last(error);
                                }

                                let digit = self.peek(0);

                                let value = if digit.is_ascii_digit() {
                                    digit as u32 - '0' as u32
                                } else if ('a'..='f').contains(&digit) {
                                    digit as u32 - 'a' as u32 + 10
                                } else if ('A'..='F').contains(&digit) {
                                    digit as u32 - 'A' as u32 + 10
                                } else {
                                    // Make error, but keep parsing the string.
                                    self.push_single_char_error(
                                        "Invalid hexadecimal digit in unicode escape sequence."
                                            .into(),
                                    );
                                    break None;
                                };

                                result <<= 4;
                                result |= value;
                                self.advance();

                                i += 1;
                            }
                        }
                        '\r' => {
                            if self.peek(0) != '\n' {
                                // XXX: The Godot code has TWO bugs.
                                // 1. they append `ch` not `code`. `ch` in the original is the
                                //    backslash, not the carriage return
                                // 2. they don't set `valid_escape` to `false`, so the code below
                                //    appends a null byte variable to the output as a character.
                                //
                                // The diff tests just opt to not exercise this code path since the
                                // original implementation is clearly broken.

                                // Carriage return without newline in string. (???)
                                // Just add it to the string and keep going.
                                result.push(ch);
                                self.advance();
                            } else {
                                // Escaping newline.
                                self.newline(false);
                            }

                            None
                        }
                        '\n' => {
                            self.newline(false);
                            None
                        }
                        _ => {
                            let error = Token::Error("Invalid escape in string.".into());
                            let mut spanned = self.span_token_and_set_last(error);
                            spanned.1.start.column = self.position.column - 2;
                            self.error_stack.push_back(spanned);

                            None
                        }
                    };

                    // Parse UTF-16 pair.
                    let escaped = if let Some(escaped) = escaped {
                        let codepoint = escaped as u32;

                        // TODO: Rust doesn't allow unpaired UTF-16 surrogates in strings. This
                        //  code doesn't ever run unless we change our input format to something
                        //  like WTF-8.

                        let escaped = if (codepoint & 0xfffffc00) == 0xd800 {
                            if prev.is_none() {
                                prev = Some(escaped);
                                prev_pos = self.position.column - 2;
                                continue;
                            } else {
                                let error = Token::Error(
                                    "Invalid UTF-16 sequence in string, unpaired lead surrogate."
                                        .into(),
                                );
                                let mut spanned = self.span_token_and_set_last(error);
                                spanned.1.start.column = self.position.column - 2;
                                self.error_stack.push_back(spanned);

                                prev = None;
                                None
                            }
                        } else if (codepoint & 0xfffffc00) == 0xdc00 {
                            if let Some(cur_prev) = prev {
                                let escaped_codepoint = ((cur_prev as u32) << 10)
                                    + (escaped as u32)
                                    - ((0xd800 << 10) + 0xdc00 - 0x10000);
                                prev = None;

                                Some(char::from_u32(escaped_codepoint).unwrap())
                            } else {
                                let error = Token::Error(
                                    "Invalid UTF-16 sequence in string, unpaired lead surrogate."
                                        .into(),
                                );
                                let mut spanned = self.span_token_and_set_last(error);
                                spanned.1.start.column = self.position.column - 2;
                                self.error_stack.push_back(spanned);

                                None
                            }
                        } else {
                            Some(escaped)
                        };

                        if prev.is_some() {
                            let error = Token::Error(
                                "Invalid UTF-16 sequence in string, unpaired lead surrogate."
                                    .into(),
                            );
                            let mut spanned = self.span_token_and_set_last(error);
                            spanned.1.start.column = prev_pos;
                            self.error_stack.push_back(spanned);

                            prev = None;
                        }

                        escaped
                    } else {
                        escaped
                    };

                    if let Some(ch) = escaped {
                        result.push(ch);
                    }
                }
            } else if ch == quote_char {
                if prev.is_some() {
                    let error = Token::Error(
                        "Invalid UTF-16 sequence in string, unpaired lead surrogate.".into(),
                    );
                    let mut spanned = self.span_token_and_set_last(error);
                    spanned.1.start.column = prev_pos;
                    self.error_stack.push_back(spanned);

                    prev = None;
                }

                self.advance();

                if is_multiline {
                    if self.peek(0) == quote_char && self.peek(1) == quote_char {
                        // Ended the multiline string. Consume all quotes.
                        self.advance();
                        self.advance();
                        break;
                    } else {
                        // Not a multiline string termination, add consumed quote.
                        result.push(quote_char);
                    }
                } else {
                    // Ended single-line string.
                    break;
                }
            } else {
                if prev.is_some() {
                    let error = Token::Error(
                        "Invalid UTF-16 sequence in string, unpaired lead surrogate.".into(),
                    );
                    let mut spanned = self.span_token_and_set_last(error);
                    spanned.1.start.column = prev_pos;
                    self.error_stack.push_back(spanned);

                    prev = None;
                }

                result.push(ch);
                self.advance();

                if ch == '\n' {
                    self.newline(false);
                }
            }
        }

        if prev.is_some() {
            let error =
                Token::Error("Invalid UTF-16 sequence in string, unpaired lead surrogate.".into());
            let mut spanned = self.span_token_and_set_last(error);
            spanned.1.start.column = prev_pos;
            self.error_stack.push_back(spanned);
        }

        // Make the literal.
        let value = match typ {
            StringType::Regular | StringType::RegularRaw => Variant::String(result.into()),
            StringType::Name => Variant::StringName(result.into()),
            StringType::NodePath => {
                let path = NodePath::from_str(&result).unwrap_or_else(|_| NodePath::default());
                Variant::NodePath(path)
            }
        };

        let token = Token::Literal(value);
        self.span_token_and_set_last(token)
    }

    fn check_mixed_indentation(&mut self, mixed: bool) {
        // Cherry-picked into 4.x in 4d38529284120562abec62425b21c9b90b56faa7 (4.0.3)
        if mixed
            && (!self.version.allow_mixed_indentation_when_multiline
                || (!self.line_continuation && !self.multiline_mode))
        {
            let error = Token::Error("Mixed use of tabs and spaces for indentation.".into());
            let mut spanned = self.span_token_and_set_last(error);
            spanned.1.start.line = self.position.line;
            spanned.1.start.column = 1;
            self.error_stack.push_back(spanned);
        }
    }

    fn check_indent(&mut self) {
        assert_eq!(
            self.position.column, 1,
            "Checking tokenizer identation in the middle of a line."
        );

        if self.is_at_end() {
            self.pending_indents -= self.indent_stack.len() as isize;
            self.indent_stack.clear();
            return;
        }

        loop {
            let current_indent_char = self.peek(0);
            let mut indent_count = 0;

            if current_indent_char != ' '
                && current_indent_char != '\t'
                && current_indent_char != '\r'
                && current_indent_char != '\n'
                && current_indent_char != '#'
            {
                // First character of the line is not whitespace, so we clear all indentation levels.
                // Unless we are in a continuation or in multiline mode (inside expression).
                if self.line_continuation || self.multiline_mode {
                    return;
                }

                self.pending_indents -= self.indent_stack.len() as isize;
                self.indent_stack.clear();
                return;
            }

            if self.peek(0) == '\r' {
                self.advance();

                if self.peek(0) != '\n' {
                    self.push_error("Stray carriage return character in source code.".into());
                }
            }

            if self.peek(0) == '\n' {
                // Empty line, keep going.
                self.advance();
                self.newline(false);
                continue;
            }

            // Check indent level.
            let mut mixed = false;
            while !self.is_at_end() {
                let space = self.peek(0);
                if space == '\t' {
                    if !self.version.expands_tabs_in_span_column {
                        // Consider individual tab columns.
                        self.position.column += TAB_SIZE - 1;
                    }

                    indent_count += TAB_SIZE;
                } else if space == ' ' {
                    indent_count += 1;
                } else {
                    break;
                }

                mixed = mixed || space != current_indent_char;
                self.advance();
            }

            if !self.version.allow_mixed_indentation_on_blank_lines {
                self.check_mixed_indentation(mixed);
            }

            if self.is_at_end() {
                // Reached the end with an empty line, so just dedent as much as needed.
                self.pending_indents -= self.indent_stack.len() as isize;
                self.indent_stack.clear();
                return;
            }

            if self.peek(0) == '\r' {
                self.advance();

                if self.peek(0) != '\n' {
                    self.push_error("Stray carriage return character in source code.".into());
                }
            }

            if self.peek(0) == '\n' {
                // Empty line, keep going.
                self.advance();
                self.newline(false);
                continue;
            }

            if self.peek(0) == '#' {
                // Comment. Advance to the next line.
                while self.peek(0) != '\n' && !self.is_at_end() {
                    self.advance();
                }

                if self.is_at_end() {
                    // Reached the end with an empty line, so just dedent as much as needed.
                    self.pending_indents -= self.indent_stack.len() as isize;
                    self.indent_stack.clear();
                    return;
                }

                self.advance(); // Consume '\n'.
                self.newline(false);
                continue;
            }

            if self.version.allow_mixed_indentation_on_blank_lines {
                self.check_mixed_indentation(mixed);
            }

            if self.line_continuation || self.multiline_mode {
                // We cleared up all the whitespace at the beginning of the line.
                // If this is a line continuation or we're in multiline mode then we don't want any indentation changes.
                return;
            }

            // Check if indentation character is consistent.
            // XXX: The Godot implementation allows this code to execute with weird whitespace
            // characters like CR.
            if let Some(indent_char) = self.indent_char {
                if current_indent_char != indent_char {
                    let error = Token::Error(format!("Used {} character for indentation instead of {} as used before in the file.",
                                                     get_indent_char_name(current_indent_char), get_indent_char_name(indent_char)).into());
                    let mut spanned = self.span_token_and_set_last(error);
                    spanned.1.start.line = self.position.line;
                    spanned.1.start.column = 1;
                    self.error_stack.push_back(spanned);
                }
            } else {
                // First time indenting, choose character now.
                self.indent_char = Some(current_indent_char);
            }

            // Now we can do actual indentation changes.

            // Check if indent or dedent.
            let mut previous_indent = 0;
            if !self.indent_stack.is_empty() {
                previous_indent = *self.indent_stack.back().unwrap();
            }

            if indent_count == previous_indent {
                // No change in indentation.
                return;
            }

            if indent_count > previous_indent {
                // Indentation increased.
                self.indent_stack.push_back(indent_count);
                self.pending_indents += 1;
            } else {
                // Indentation decreased (dedent).
                assert!(
                    !self.indent_stack.is_empty(),
                    "trying to dedent without previous indent"
                );

                while !self.indent_stack.is_empty()
                    && *self.indent_stack.back().unwrap() > indent_count
                {
                    self.indent_stack.pop_back();
                    self.pending_indents -= 1;
                }

                if (!self.indent_stack.is_empty()
                    && *self.indent_stack.back().unwrap() != indent_count)
                    || (self.indent_stack.is_empty() && indent_count != 0)
                {
                    // Mismatched indentation alignment.
                    let error = Token::Error(
                        "Unindent doesn't match the previous indentation level.".into(),
                    );
                    let mut spanned = self.span_token_and_set_last(error);
                    spanned.1.start.line = self.position.line;
                    spanned.1.start.column = 1;
                    spanned.1.end.column = self.position.column + 1;
                    self.error_stack.push_back(spanned);

                    // Still, we'll be lenient and keep going, so keep this level in the stack.
                    self.indent_stack.push_back(indent_count);
                }
            }

            break; // Get out of the loop in any case.
        }
    }

    fn skip_whitespace(&mut self) {
        if self.pending_indents != 0 {
            // Still have some indent/dedent tokens to give.
            return;
        }

        let is_bol = self.position.column == 1; // Beginning of line.

        if is_bol {
            self.check_indent();
            return;
        }

        loop {
            let ch = self.peek(0);

            match ch {
                ' ' => {
                    self.advance();
                }
                '\t' => {
                    self.advance();

                    // Consider individual tab columns.
                    self.position.column += TAB_SIZE - 1;
                }

                '\r' => {
                    self.advance(); // Consume either way.

                    if self.peek(0) != '\n' {
                        self.push_error("Stray carriage return character in source code.".into());
                        return;
                    }
                }

                '\n' => {
                    self.advance();
                    self.newline(!is_bol); // Don't create new line token if line is empty.
                    self.check_indent();
                }

                '#' => {
                    // Comment.
                    while self.peek(0) != '\n' && !self.is_at_end() {
                        self.advance();
                    }

                    if self.is_at_end() {
                        return;
                    }

                    self.advance(); // Consume '\n'
                    self.newline(!is_bol);
                    self.check_indent();
                }

                _ => return,
            }
        }
    }
}

impl<'s> Iterator for TokenizerText<'_, 's> {
    type Item = Spanned<Token>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(error) = self.error_stack.pop_back() {
            return Some(error);
        }

        self.skip_whitespace();

        if let Some(last_newline) = take(&mut self.last_newline)
            && !self.multiline_mode
        {
            // Don't return newline tokens on multiline mode.
            return Some(last_newline);
        }

        // Check for potential errors after skipping whitespace().
        if let Some(error) = self.error_stack.pop_back() {
            return Some(error);
        }

        self.start_offset = self.offset;
        self.start_position = self.position;

        if self.pending_indents != 0 {
            // Adjust position for indent.
            // XXX: In some cases (notably when starting a file with a tab) this appears to
            // underflow in GDScript, causing the tokenizer to read before the start of the buffer.
            // I'm not convinced this code is ever correct, in fact.
            // self.start_offset -= self.start_position.column - 1;
            self.start_position.column = 1;

            return if self.pending_indents > 0 {
                // Indents.
                self.pending_indents -= 1;
                Some(self.span_token_and_set_last(Token::Indent))
            } else {
                // Dedents.
                self.pending_indents += 1;

                let mut dedent = self.span_token_and_set_last(Token::Dedent);
                dedent.1.end.column += 1;
                Some(dedent)
            };
        }

        if self.is_at_end() {
            return if self.offset == usize::MAX {
                None
            } else {
                self.offset = usize::MAX;
                let eof = self.span_token_and_set_last(Token::Eof);
                Some(eof)
            };
        }

        let ch = self.advance();

        if ch == '\\' {
            // Line continuation with backslash.
            if self.peek(0) == '\r' {
                if self.peek(1) != '\n' {
                    let error = Token::Error("Unexpected carriage return character.".into());
                    return Some(self.span_token_and_set_last(error));
                }

                self.advance();
            }

            if self.peek(0) != '\n' {
                let error = Token::Error("Expected new line after \"\\\".".into());
                return Some(self.span_token_and_set_last(error));
            }

            self.advance();
            self.newline(false);
            self.line_continuation = true;

            // Changed in 02253b6b91472e251418bd0545afb2b653b5385c (4.3)
            if self.version.has_fixed_continuation_lines {
                self.skip_whitespace(); // Skip whitespace/comment lines after `\`. See GH-89403.
            }

            return self.next(); // Recurse to get next token.
        }

        self.line_continuation = false;

        if ch.is_ascii_digit() {
            return Some(self.number(ch));
        } else if self.version.has_raw_strings
            && ch == 'r'
            && (self.peek(0) == '"' || self.peek(0) == '\'')
        {
            // Raw string literals.
            return Some(self.string(ch));
        } else if ch.is_godot_xid_start() {
            return Some(self.potential_identifier(ch));
        }

        let can_precede_bin_op = if let Some((last_token, _)) = &self.last_token {
            last_token.can_precede_bin_op()
        } else {
            false
        };

        let token = match ch {
            // String literals.
            '"' | '\'' => return Some(self.string(ch)),

            // Annotation.
            '@' => return Some(self.annotation()),

            // Single characters.
            '~' => Token::Tilde,
            ',' => Token::Comma,
            ':' => Token::Colon,
            ';' => Token::Semicolon,
            '$' => Token::Dollar,
            '?' => Token::QuestionMark,
            '`' => Token::Backtick,

            // Parens.
            '(' => {
                self.push_paren('(');
                Token::ParenthesisOpen
            }
            '[' => {
                self.push_paren('[');
                Token::BracketOpen
            }
            '{' => {
                self.push_paren('{');
                Token::BraceOpen
            }
            ')' => {
                return Some(self.pop_paren_or_make_error(ch, '(', Token::ParenthesisClose));
            }
            ']' => {
                return Some(self.pop_paren_or_make_error(ch, '[', Token::BracketClose));
            }
            '}' => {
                return Some(self.pop_paren_or_make_error(ch, '{', Token::BraceClose));
            }

            // Double characters.
            '!' => {
                if self.peek(0) == '=' {
                    self.advance();
                    Token::BangEqual
                } else {
                    Token::Bang
                }
            }
            '.' => {
                if self.peek(0) == '.' {
                    self.advance();

                    // "..." token (added in ee121ef80e36865ac9d5c55ab2ec419f48ef6954)
                    if self.version.has_variadic_functions && self.peek(0) == '.' {
                        self.advance();
                        Token::PeriodPeriodPeriod
                    } else {
                        Token::PeriodPeriod
                    }
                } else if self.peek(0).is_ascii_digit() {
                    // Number starting with '.'.
                    return Some(self.number(ch));
                } else {
                    Token::Period
                }
            }
            '+' => {
                if self.peek(0) == '=' {
                    self.advance();
                    Token::PlusEqual
                } else if self.version.has_literal_sign_handling
                    && self.peek(0).is_ascii_digit()
                    && !can_precede_bin_op
                {
                    // Number starting with '+'.
                    return Some(self.number(ch));
                } else {
                    Token::Plus
                }
            }
            '-' => {
                if self.peek(0) == '=' {
                    self.advance();
                    Token::MinusEqual
                } else if self.version.has_literal_sign_handling
                    && self.peek(0).is_ascii_digit()
                    && !can_precede_bin_op
                {
                    // Number starting with '-'.
                    return Some(self.number(ch));
                } else if self.peek(0) == '>' {
                    self.advance();
                    Token::ForwardArrow
                } else {
                    Token::Minus
                }
            }
            '*' => {
                if self.peek(0) == '=' {
                    self.advance();
                    Token::StarEqual
                } else if self.peek(0) == '*' {
                    if self.peek(1) == '=' {
                        self.advance();
                        self.advance(); // Advance both '*' and '='
                        Token::StarStarEqual
                    } else {
                        self.advance();
                        Token::StarStar
                    }
                } else {
                    Token::Star
                }
            }
            '/' => {
                if self.peek(0) == '=' {
                    self.advance();
                    Token::SlashEqual
                } else {
                    Token::Slash
                }
            }
            '%' => {
                if self.peek(0) == '=' {
                    self.advance();
                    Token::PercentEqual
                } else {
                    Token::Percent
                }
            }
            '^' => {
                if self.peek(0) == '=' {
                    self.advance();
                    Token::CaretEqual
                } else if self.peek(0) == '"' || self.peek(0) == '\'' {
                    // Node path
                    return Some(self.string(ch));
                } else {
                    Token::Caret
                }
            }
            '&' => {
                if self.peek(0) == '&' {
                    self.advance();
                    Token::AmpersandAmpersand
                } else if self.peek(0) == '=' {
                    self.advance();
                    Token::AmpersandEqual
                } else if self.peek(0) == '"' || self.peek(0) == '\'' {
                    // String Name
                    return Some(self.string(ch));
                } else {
                    Token::Ampersand
                }
            }
            '|' => {
                if self.peek(0) == '|' {
                    self.advance();
                    Token::PipePipe
                } else if self.peek(0) == '=' {
                    self.advance();
                    Token::PipeEqual
                } else {
                    Token::Pipe
                }
            }

            // Potential VCS conflict markers.
            '=' => {
                if self.peek(0) == '=' {
                    return Some(self.check_vcs_marker('=', Token::EqualEqual));
                } else {
                    Token::Equal
                }
            }
            '<' => {
                if self.peek(0) == '=' {
                    self.advance();
                    Token::LessEqual
                } else if self.peek(0) == '<' {
                    if self.peek(1) == '=' {
                        self.advance();
                        self.advance(); // Advance both '<' and '='
                        Token::LessLessEqual
                    } else {
                        return Some(self.check_vcs_marker('<', Token::LessLess));
                    }
                } else {
                    Token::Less
                }
            }
            '>' => {
                if self.peek(0) == '=' {
                    self.advance();
                    Token::GreaterEqual
                } else if self.peek(0) == '>' {
                    if self.peek(1) == '=' {
                        self.advance();
                        self.advance(); // Advance both '>' and '='
                        Token::GreaterGreaterEqual
                    } else {
                        return Some(self.check_vcs_marker('>', Token::GreaterGreater));
                    }
                } else {
                    Token::Greater
                }
            }
            _ => {
                // Error message was improved in 54770ba9c545bd1fd2f3c2b1be52228ab5728a85 (4.1)
                let msg = if is_godot_whitespace(ch, self.version.allow_zwsp_as_whitespace) {
                    if self.version.has_improved_invalid_character_error {
                        format!("Invalid white space character U+{:04X}.", ch as u32)
                    } else {
                        format!("Invalid white space character \"\\u{:X}\".", ch as u32)
                    }
                } else {
                    if self.version.has_improved_invalid_character_error {
                        format!("Invalid character \"{}\" (U+{:04X}).", ch, ch as u32)
                    } else {
                        format!("Unknown character \"{}\".", ch)
                    }
                };

                let error = Token::Error(msg.into());
                return Some(self.span_token_and_set_last(error));
            }
        };

        Some(self.span_token_and_set_last(token))
    }
}

impl Sealed for TokenizerText<'_, '_> {}
impl Tokenizer for TokenizerText<'_, '_> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::{VersionSpecifier, bundled_builds};
    use crate::gdscript::tokenizer::{CompressMode, TokenizerBytecode, reconstruct_script_binary};

    #[test]
    fn test_newline_jank() {
        let builds = bundled_builds().clone().resolve().unwrap();
        let build = builds
            .find_exact_build(&VersionSpecifier::from_str("4.5-stable").unwrap())
            .unwrap();
        let gdscript = &build.gdscript;

        let src = "
func meow() -> void:
    cat
        ";

        let tokenizer = TokenizerText::new(gdscript, src);
        let tokens = tokenizer.collect::<Vec<_>>();
        assert_eq!(tokens.len(), 13);
        dbg!(&tokens);

        let reconstituted = reconstruct_script_binary(gdscript, &tokens, CompressMode::None, false)
            .expect("failed to reconstitute tokens into bytecode");
        let tokenizer = TokenizerBytecode::new(gdscript, &reconstituted)
            .expect("failed to parse reconstituted bytecode");
        let tokens = tokenizer.collect::<Vec<_>>();
        dbg!(&tokens);
    }
}
