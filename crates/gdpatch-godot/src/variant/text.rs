use crate::string::{to_float, to_int};
use crate::util::{escape_string, escape_string_multiline};
use crate::variant::{
    Aabb, Array, Basis, Callable, Color, ContainerType, Dictionary, Nil, NodePath, Object,
    ObjectKind, Plane, Projection, Quaternion, Rect2, Rect2i, Rid, Signal, StringName, Transform2d,
    Transform3d, Variant, VariantType, Vector2, Vector2i, Vector3, Vector3i, Vector4, Vector4i,
};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::collections::HashMap;
use std::str::{Chars, FromStr};

#[derive(Debug)]
pub struct VariantParser<'a> {
    line: usize,
    input: Chars<'a>,
    saved_char: Option<Option<char>>,
}

#[derive(Debug)]
enum Token {
    Eof,
    BraceOpen,
    BraceClose,
    BracketOpen,
    BracketClose,
    ParenthesisOpen,
    ParenthesisClose,
    Colon,
    Comma,
    Period,
    Equal,
    Identifier(String),
    Float(f64),
    Integer(i64),
    Color(Color),
    String(String),
    StringName(StringName),
}

impl Token {
    pub fn name(&self) -> &str {
        match self {
            Token::BraceOpen => "'{'",
            Token::BraceClose => "'}'",
            Token::BracketOpen => "'['",
            Token::BracketClose => "']'",
            Token::ParenthesisOpen => "'['",
            Token::ParenthesisClose => "']'",
            Token::Identifier(_) => "identifier",
            Token::String(_) => "string",
            Token::StringName(_) => "string_name",
            Token::Float(_) | Token::Integer(_) => "number",
            Token::Color(_) => "color",
            Token::Colon => "':'",
            Token::Comma => "','",
            Token::Period => "'.'",
            Token::Equal => "'='",
            Token::Eof => "EOF",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Tag {
    pub name: String,
    pub fields: HashMap<String, Variant>,
}

#[derive(Debug, Clone)]
pub enum TagAssign {
    Tag(Tag),
    Variant { assign: String, value: Variant },
}

#[derive(Debug)]
pub struct ParseError(String);

impl From<ParseError> for crate::Error {
    fn from(value: ParseError) -> Self {
        Self::Parse(value.0)
    }
}

pub type ParseResult<T, E = ParseError> = Result<T, E>;

fn stor_fix(s: &str) -> Option<f64> {
    Some(match s {
        "inf" => f64::INFINITY,
        "-inf" | "inf_neg" => f64::NEG_INFINITY,
        "nan" => f64::NAN,
        _ => return None,
    })
}

fn rtos_fix(value: f64, compat: bool) -> String {
    if value == 0.0 {
        "0".to_string() // Avoid negative zero (-0) being written, which may annoy git, svn, etc. for changes when they don't exist.    } else if compat {
    } else if compat && value.is_infinite() && value < 0.0 {
        "inf_neg".to_string()
    } else {
        if value.is_nan() {
            "nan".to_string()
        } else if value.is_infinite() {
            if value.is_sign_negative() {
                "-inf".to_string()
            } else {
                "inf".to_string()
            }
        } else {
            let mut buffer = zmij::Buffer::new();
            buffer.format(value).to_string()
        }
    }
}

impl<'a> VariantParser<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            line: 0,
            input: s.chars(),
            saved_char: None,
        }
    }

    fn get_char(&mut self) -> Option<char> {
        if let Some(c) = self.saved_char.take() {
            c
        } else {
            self.input.next()
        }
    }

    fn parse_string(&mut self, is_string_name: bool) -> ParseResult<Token> {
        let mut s = String::new();
        let _prev = 0;

        loop {
            let Some(ch) = self.get_char() else {
                return Err(ParseError("Unterminated string".into()));
            };

            s.push(match ch {
                '"' => break,
                '\\' => {
                    // escaped character
                    let Some(next) = self.get_char() else {
                        return Err(ParseError("Unterminated string".into()));
                    };

                    match next {
                        'b' => '\x08',
                        't' => '\x09',
                        'n' => '\x0a',
                        'f' => '\x0c',
                        'r' => '\x0d',
                        'U' | 'u' => {
                            // Hexadecimal sequence.
                            let hex_len: usize = if next == 'U' { 6 } else { 4 };

                            let mut result = 0u32;

                            for _ in 0..hex_len {
                                let Some(ch) = self.get_char() else {
                                    return Err(ParseError("Unterminated string".into()));
                                };

                                let v = if ch.is_ascii_digit() {
                                    (ch as u32) - ('0' as u32)
                                } else if matches!(ch, 'a'..='f') {
                                    (ch as u32) - ('f' as u32) + 10
                                } else if matches!(ch, 'A'..='F') {
                                    (ch as u32) - ('F' as u32) + 10
                                } else {
                                    return Err(ParseError("Unterminated string".into()));
                                };

                                result <<= 4;
                                result |= v;
                            }

                            let Some(ch) = char::from_u32(result) else {
                                // XXX: Godot doesn't have this error case
                                return Err(ParseError(
                                    "Invalid character in unicode escape".into(),
                                ));
                            };

                            ch
                        }
                        other => other,
                    }
                }
                other => {
                    if other == '\n' {
                        self.line += 1;
                    }

                    other
                }
            })
        }

        // TODO: Godot does some shit with removing UTF-8 characters here?

        Ok(if is_string_name {
            Token::StringName(StringName(s.into()))
        } else {
            Token::String(s)
        })
    }

    fn parse_number(&mut self, mut ch: char) -> Token {
        enum Reading {
            Int,
            Dec,
            Exp,
            Done,
        }

        let mut reading = Reading::Int;

        let mut exp_sign = false;
        let mut exp_beg = false;
        let mut is_float = false;

        let mut token_text = String::new();

        loop {
            reading = match reading {
                Reading::Int => {
                    if ch.is_ascii_digit() {
                        // pass
                        Reading::Int
                    } else if ch == '.' {
                        is_float = true;
                        Reading::Dec
                    } else if ch == 'e' || ch == 'E' {
                        is_float = true;
                        Reading::Exp
                    } else {
                        Reading::Done
                    }
                }

                Reading::Dec => {
                    if ch.is_ascii_digit() {
                        // pass
                        Reading::Int
                    } else if ch == 'e' || ch == 'E' {
                        Reading::Exp
                    } else {
                        Reading::Done
                    }
                }

                Reading::Exp => {
                    if ch.is_ascii_digit() {
                        exp_beg = true;
                        Reading::Exp
                    } else if (ch == '-' || ch == '+') && !exp_sign && !exp_beg {
                        exp_sign = true;
                        Reading::Exp
                    } else {
                        Reading::Done
                    }
                }

                Reading::Done => break,
            };

            token_text.push(ch);

            let next_ch = self.get_char();
            self.saved_char = Some(next_ch);

            if let Some(next_ch) = next_ch {
                ch = next_ch;
            } else {
                break;
            }
        }

        if is_float {
            Token::Float(to_float(&token_text))
        } else {
            Token::Integer(to_int(&token_text))
        }
    }

    fn get_token(&mut self) -> ParseResult<Token> {
        loop {
            let Some(mut ch) = self.get_char() else {
                return Ok(Token::Eof);
            };

            return Ok(match ch {
                '\n' => {
                    self.line += 1;
                    continue;
                }
                '{' => Token::BraceOpen,
                '}' => Token::BraceClose,
                '[' => Token::BracketOpen,
                ']' => Token::BracketClose,
                '(' => Token::ParenthesisOpen,
                ')' => Token::ParenthesisClose,
                ':' => Token::Colon,
                ';' => {
                    // Comment
                    loop {
                        let Some(c) = self.get_char() else {
                            return Ok(Token::Eof);
                        };

                        if c == '\n' {
                            self.line += 1;
                            break;
                        }
                    }

                    continue;
                }
                ',' => Token::Comma,
                '.' => Token::Period,
                '=' => Token::Equal,
                '#' => {
                    let mut color_str = "#".to_owned();

                    loop {
                        let Some(ch) = self.get_char() else {
                            // This is weird because it ignores previous characters if you hit EOF.
                            return Ok(Token::Eof);
                        };

                        if ch.is_ascii_hexdigit() {
                            color_str.push(ch);
                        } else {
                            self.saved_char = Some(Some(ch));
                            break;
                        }
                    }

                    let color = Color::from_str(&color_str).unwrap_or_else(|_| Color::default());
                    Token::Color(color)
                }
                // '@' => for 3.x?
                '&' => {
                    let ch = self.get_char();

                    if ch != Some('"') {
                        return Err(ParseError(r#"Expected '"' after '&'"#.into()));
                    }

                    self.parse_string(true)?
                }
                '"' => self.parse_string(false)?,
                _ => {
                    if (ch as i32) <= 32 {
                        continue;
                    }

                    let mut token_text = String::new();

                    if ch == '-' {
                        token_text.push(ch);

                        if let Some(next_ch) = self.get_char() {
                            ch = next_ch;
                        } else {
                            // Godot code has an unhandled error case here that falls into the
                            // "Unexpected character" condition.
                            return Err(ParseError("Unexpected character".into()));
                        }
                    }

                    if ch.is_ascii_digit() {
                        self.parse_number(ch)
                    } else if ch.is_ascii_alphabetic() || ch == '_' {
                        let mut first = true;
                        let mut maybe_ch = Some(ch);

                        while let Some(ch) = maybe_ch
                            && (ch.is_ascii_alphabetic()
                                || ch == '_'
                                || (!first && ch.is_ascii_digit()))
                        {
                            token_text.push(ch);
                            maybe_ch = self.get_char();
                            first = false;
                        }

                        self.saved_char = Some(maybe_ch);
                        Token::Identifier(token_text)
                    } else {
                        return Err(ParseError("Unexpected character".into()));
                    }
                }
            });
        }
    }

    fn parse_construct(&mut self) -> ParseResult<Vec<Token>> {
        let token = self.get_token().ok();

        if !matches!(token, Some(Token::ParenthesisOpen)) {
            return Err(ParseError("Expected '(' in constructor.".into()));
        }

        let mut first = true;
        let mut arguments = Vec::new();

        loop {
            if !first {
                let token = self.get_token().ok();

                match token {
                    Some(Token::Comma) => {}
                    Some(Token::ParenthesisClose) => break,
                    _ => return Err(ParseError("Expected ',' or ')' in constructor".into())),
                }
            }

            let mut token = self.get_token().ok();

            if first && matches!(token, Some(Token::ParenthesisClose)) {
                break;
            } else if !matches!(token, Some(Token::Float(_) | Token::Integer(_))) {
                if let Some(Token::Identifier(inner)) = &token
                    && let Some(real) = stor_fix(inner)
                {
                    token = Some(Token::Float(real));
                } else {
                    return Err(ParseError("Expected float in constructor".into()));
                }
            }

            arguments.push(token.unwrap());
            first = false;
        }

        Ok(arguments)
    }

    fn parse_construct_integer(&mut self) -> ParseResult<Vec<i64>> {
        Ok(self
            .parse_construct()?
            .into_iter()
            .map(|token| match token {
                Token::Float(v) => v as i64,
                Token::Integer(v) => v,
                _ => unreachable!(),
            })
            .collect::<Vec<_>>())
    }

    fn parse_construct_float(&mut self) -> ParseResult<Vec<f64>> {
        Ok(self
            .parse_construct()?
            .into_iter()
            .map(|token| match token {
                Token::Float(v) => v,
                Token::Integer(v) => v as f64,
                _ => unreachable!(),
            })
            .collect::<Vec<_>>())
    }

    fn parse_byte_array(&mut self) -> ParseResult<Vec<u8>> {
        let token = self.get_token().ok();
        if !matches!(token, Some(Token::ParenthesisOpen)) {
            return Err(ParseError("Expected '(' in constructor".into()));
        }

        let token = self.get_token().ok();

        match token {
            Some(Token::String(str)) => {
                // Base64 encoded array.

                let data = STANDARD
                    .decode(str)
                    .map_err(|_| ParseError("Invalid base64-encoded string".into()))?;

                let token = self.get_token().ok();
                if !matches!(token, Some(Token::ParenthesisClose)) {
                    return Err(ParseError("Expected ')' in constructor".into()));
                }

                Ok(data)
            }

            Some(Token::Integer(_) | Token::Float(_) | Token::Identifier(_)) => {
                // Individual elements.
                let mut token = token;
                let mut construct = Vec::new();

                loop {
                    if !matches!(token, Some(Token::Integer(_) | Token::Float(_))) {
                        let mut valid = false;

                        if let Some(Token::Identifier(identifier)) = token.as_ref()
                            && let Some(real) = stor_fix(identifier)
                        {
                            token = Some(Token::Float(real));
                            valid = true;
                        }

                        if !valid {
                            return Err(ParseError("Expected number in constructor".into()));
                        }
                    }

                    let value = match token {
                        Some(Token::Float(v)) => v as u8,
                        Some(Token::Integer(v)) => v as u8,
                        _ => unreachable!(),
                    };
                    construct.push(value);

                    token = self.get_token().ok();
                    if matches!(token, Some(Token::Comma)) {
                        //do none
                    } else if matches!(token, Some(Token::ParenthesisClose)) {
                        return Ok(construct);
                    } else {
                        return Err(ParseError("Expected ',' or ')' in constructor".into()));
                    }

                    token = self.get_token().ok();
                }
            }

            Some(Token::ParenthesisClose) => {
                // Empty array.
                Ok(Vec::new())
            }

            _ => Err(ParseError(
                "Expected base64 string, or list of numbers in constructor".into(),
            )),
        }
    }

    fn parse_dictionary(&mut self) -> ParseResult<Dictionary> {
        let mut at_key = true;
        let mut key: Option<Variant> = None;
        let mut need_comma = false;
        let mut object = Dictionary::default();

        loop {
            let token = self.get_token()?;
            if matches!(token, Token::Eof) {
                return Err(ParseError("Unexpected EOF while parsing dictionary".into()));
            }

            if at_key {
                if matches!(token, Token::BraceClose) {
                    return Ok(object);
                }

                if need_comma {
                    if !matches!(token, Token::Comma) {
                        return Err(ParseError("Expected '}' or ','".into()));
                    } else {
                        need_comma = false;
                        continue;
                    }
                }

                key = Some(self.parse_value(token)?);

                let token = self.get_token()?;
                if !matches!(token, Token::Colon) {
                    return Err(ParseError("Expected ':'".into()));
                }

                at_key = false;
            } else {
                let value = self.parse_value(token)?;
                let Some(key) = key.as_ref() else {
                    return Err(ParseError("Expected key for dictionary".into()));
                };

                object.inner.insert(key.clone(), value);

                need_comma = true;
                at_key = true;
            }
        }
    }

    fn parse_array(&mut self) -> ParseResult<Array> {
        let mut need_comma = false;
        let mut array = Array::default();

        loop {
            let token = self.get_token()?;
            if matches!(token, Token::Eof) {
                return Err(ParseError("Unexpected EOF while parsing array".into()));
            }

            if matches!(token, Token::BracketClose) {
                return Ok(array);
            }

            if need_comma {
                if !matches!(token, Token::Comma) {
                    return Err(ParseError("Expected ','".into()));
                } else {
                    need_comma = false;
                    continue;
                }
            }

            let value = self.parse_value(token)?;
            array.inner.push(value);
            need_comma = true;
        }
    }

    fn parse_value(&mut self, token: Token) -> ParseResult<Variant> {
        Ok(match token {
            Token::BraceOpen => self.parse_dictionary()?.into(),
            Token::BracketOpen => self.parse_array()?.into(),
            Token::Identifier(identifier) => match identifier.as_str() {
                "true" => true.into(),
                "false" => false.into(),
                "null" | "nil" => Nil.into(),
                "inf" => f64::INFINITY.into(),
                "-inf" | "inf_neg" => f64::NEG_INFINITY.into(),
                "nan" => f64::NAN.into(),
                "Vector2" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 2 {
                        return Err(ParseError("Expected 2 arguments for constructor".into()));
                    }

                    Vector2::new(args[0], args[1]).into()
                }
                "Vector2i" => {
                    let args = self.parse_construct_integer()?;

                    if args.len() != 2 {
                        return Err(ParseError("Expected 2 arguments for constructor".into()));
                    }

                    Vector2i::new(args[0] as i32, args[1] as i32).into()
                }
                "Rect2" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 4 {
                        return Err(ParseError("Expected 4 arguments for constructor".into()));
                    }

                    Rect2::new(args[0], args[1], args[2], args[3]).into()
                }
                "Rect2i" => {
                    let args = self.parse_construct_integer()?;

                    if args.len() != 4 {
                        return Err(ParseError("Expected 4 arguments for constructor".into()));
                    }

                    Rect2i::new(
                        args[0] as i32,
                        args[1] as i32,
                        args[2] as i32,
                        args[3] as i32,
                    )
                    .into()
                }
                "Vector3" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 3 {
                        return Err(ParseError("Expected 3 arguments for constructor".into()));
                    }

                    Vector3::new(args[0], args[1], args[2]).into()
                }
                "Vector3i" => {
                    let args = self.parse_construct_integer()?;

                    if args.len() != 3 {
                        return Err(ParseError("Expected 3 arguments for constructor".into()));
                    }

                    Vector3i::new(args[0] as i32, args[1] as i32, args[2] as i32).into()
                }
                "Vector4" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 4 {
                        return Err(ParseError("Expected 4 arguments for constructor".into()));
                    }

                    Vector4::new(args[0], args[1], args[2], args[3]).into()
                }
                "Vector4i" => {
                    let args = self.parse_construct_integer()?;

                    if args.len() != 4 {
                        return Err(ParseError("Expected 4 arguments for constructor".into()));
                    }

                    Vector4i::new(
                        args[0] as i32,
                        args[1] as i32,
                        args[2] as i32,
                        args[3] as i32,
                    )
                    .into()
                }
                "Transform2D" | "Matrix32" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 6 {
                        return Err(ParseError("Expected 6 arguments for constructor".into()));
                    }

                    Transform2d::new(
                        Vector2::new(args[0], args[1]),
                        Vector2::new(args[2], args[3]),
                        Vector2::new(args[4], args[5]),
                    )
                    .into()
                }
                "Plane" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 4 {
                        return Err(ParseError("Expected 4 arguments for constructor".into()));
                    }

                    Plane::new(args[0], args[1], args[2], args[3]).into()
                }
                "Quaternion" | "Quat" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 4 {
                        return Err(ParseError("Expected 4 arguments for constructor".into()));
                    }

                    Quaternion::new(args[0], args[1], args[2], args[3]).into()
                }
                "AABB" | "Rect3" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 6 {
                        return Err(ParseError("Expected 6 arguments for constructor".into()));
                    }

                    Aabb::new(
                        Vector3::new(args[0], args[1], args[2]),
                        Vector3::new(args[3], args[4], args[5]),
                    )
                    .into()
                }
                "Basis" | "Matrix3" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 9 {
                        return Err(ParseError("Expected 9 arguments for constructor".into()));
                    }

                    Basis::new(
                        Vector3::new(args[0], args[1], args[2]),
                        Vector3::new(args[3], args[4], args[5]),
                        Vector3::new(args[6], args[7], args[8]),
                    )
                    .into()
                }
                "Transform3D" | "Transform" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 12 {
                        return Err(ParseError("Expected 12 arguments for constructor".into()));
                    }

                    Transform3d::new(
                        Basis::new(
                            Vector3::new(args[0], args[1], args[2]),
                            Vector3::new(args[3], args[4], args[5]),
                            Vector3::new(args[6], args[7], args[8]),
                        ),
                        Vector3::new(args[9], args[10], args[11]),
                    )
                    .into()
                }
                "Projection" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 16 {
                        return Err(ParseError("Expected 16 arguments for constructor".into()));
                    }

                    Projection::new(
                        Vector4::new(args[0], args[1], args[2], args[3]),
                        Vector4::new(args[4], args[5], args[6], args[7]),
                        Vector4::new(args[8], args[9], args[10], args[11]),
                        Vector4::new(args[12], args[13], args[14], args[15]),
                    )
                    .into()
                }
                "Color" => {
                    let args = self.parse_construct_float()?;

                    if args.len() != 4 {
                        return Err(ParseError("Expected 4 arguments for constructor".into()));
                    }

                    Color::new(
                        args[0] as f32,
                        args[1] as f32,
                        args[2] as f32,
                        args[3] as f32,
                    )
                    .into()
                }
                "NodePath" => {
                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let Some(Token::String(s)) = self.get_token().ok() else {
                        return Err(ParseError(
                            "Expected string as an argument for NodePath()".into(),
                        ));
                    };

                    let path = NodePath::from_str(&s).unwrap_or_default();

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisClose)) {
                        return Err(ParseError("Expected ')'".into()));
                    }

                    path.into()
                }
                "RID" => {
                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let token = self.get_token().ok();

                    let rid = match token {
                        Some(Token::ParenthesisClose) => return Ok(Rid::default().into()),
                        Some(Token::Integer(v)) => Rid(v as u64),
                        Some(Token::Float(v)) => Rid(v as u64),
                        _ => {
                            return Err(ParseError("Expected number as argument or ')'".into()));
                        }
                    };

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisClose)) {
                        return Err(ParseError("Expected ')'".into()));
                    }

                    rid.into()
                }
                "Signal" => {
                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisClose)) {
                        return Err(ParseError("Expected ')'".into()));
                    }

                    Signal::default().into()
                }
                "Callable" => {
                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisClose)) {
                        return Err(ParseError("Expected ')'".into()));
                    }

                    Callable.into()
                }
                "Object" => {
                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let Some(Token::Identifier(class)) = self.get_token().ok() else {
                        return Err(ParseError("Expected identifier with type of object".into()));
                    };

                    let mut obj = Object {
                        class,
                        properties: Default::default(),
                    };

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::Comma)) {
                        return Err(ParseError("Expected ',' after object type".into()));
                    }

                    let mut at_key = true;
                    let mut key: Option<String> = None;
                    let mut need_comma = false;

                    loop {
                        let token = self.get_token()?;
                        if matches!(token, Token::Eof) {
                            return Err(ParseError("Unexpected EOF while parsing Object()".into()));
                        }

                        if at_key {
                            if matches!(token, Token::ParenthesisClose) {
                                return Ok(obj.into());
                            }

                            if need_comma {
                                if !matches!(token, Token::Comma) {
                                    return Err(ParseError("Expected '}' or ','".into()));
                                } else {
                                    need_comma = false;
                                    continue;
                                }
                            }

                            let Token::String(str) = token else {
                                return Err(ParseError("Expected property name as string".into()));
                            };

                            key = Some(str);

                            let token = self.get_token()?;
                            if !matches!(token, Token::Colon) {
                                return Err(ParseError("Expected ':'".into()));
                            }

                            at_key = false;
                        } else {
                            let value = self.parse_value(token)?;
                            let Some(key) = key.as_ref() else {
                                return Err(ParseError("Expected key for object".into()));
                            };
                            obj.properties.insert(key.clone(), value);

                            need_comma = true;
                            at_key = true;
                        }
                    }
                }
                "Resource" | "SubResource" | "ExtResource" => {
                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    // TODO: calling custom resource parsers?

                    let Ok(Token::String(_path)) = self.get_token() else {
                        return Err(ParseError(
                            "Expected string as argument for Resource()".into(),
                        ));
                    };

                    unimplemented!("Resource()")
                }
                "Dictionary" => {
                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::BracketOpen)) {
                        return Err(ParseError("Expected '['".into()));
                    }

                    let Ok(Token::Identifier(key)) = self.get_token() else {
                        return Err(ParseError("Expected type identifier for key".into()));
                    };

                    let key_type = if let Ok(typ) = VariantType::from_str(&key) {
                        ContainerType::Builtin(typ)
                    } else if key == "Resource" || key == "SubResource" || key == "ExtResource" {
                        unimplemented!("Resource() as Dictionary key")
                    } else {
                        ContainerType::ClassName(key)
                    };

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::Comma)) {
                        return Err(ParseError("Expected ',' after key type".into()));
                    }

                    let Ok(Token::Identifier(key)) = self.get_token() else {
                        return Err(ParseError("Expected type identifier for value".into()));
                    };

                    let value_type = if let Ok(typ) = VariantType::from_str(&key) {
                        ContainerType::Builtin(typ)
                    } else if key == "Resource" || key == "SubResource" || key == "ExtResource" {
                        unimplemented!("Resource() as Dictionary key")
                    } else {
                        ContainerType::ClassName(key)
                    };

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::BracketClose)) {
                        return Err(ParseError("Expected ']'".into()));
                    }

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::BracketOpen)) {
                        return Err(ParseError("Expected '{'".into()));
                    }

                    let mut dict = self.parse_dictionary()?;
                    dict.key_type = key_type;
                    dict.value_type = value_type;

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisClose)) {
                        return Err(ParseError("Expected ')'".into()));
                    }

                    Variant::Dictionary(dict)
                }
                "Array" => {
                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::BracketOpen)) {
                        return Err(ParseError("Expected '['".into()));
                    }

                    let Ok(Token::Identifier(key)) = self.get_token() else {
                        return Err(ParseError("Expected type identifier".into()));
                    };

                    let r#type = if let Ok(typ) = VariantType::from_str(&key) {
                        ContainerType::Builtin(typ)
                    } else if key == "Resource" || key == "SubResource" || key == "ExtResource" {
                        unimplemented!("Resource() as Array type")
                    } else {
                        ContainerType::ClassName(key)
                    };

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::BracketClose)) {
                        return Err(ParseError("Expected ']'".into()));
                    }

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::BracketOpen)) {
                        return Err(ParseError("Expected '{'".into()));
                    }

                    let mut array = self.parse_array()?;
                    array.element_type = r#type;

                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisClose)) {
                        return Err(ParseError("Expected ')'".into()));
                    }

                    Variant::Array(array)
                }
                "PackedByteArray" | "PoolByteArray" | "ByteArray" => {
                    let args = self.parse_byte_array()?;
                    Variant::PackedByteArray(args)
                }
                "PackedInt32Array" | "PackedIntArray" | "PoolIntArray" | "IntArray" => {
                    let args = self.parse_construct_integer()?;
                    Variant::PackedInt32Array(args.iter().map(|p| *p as i32).collect())
                }
                "PackedInt64Array" => {
                    let args = self.parse_construct_integer()?;
                    Variant::PackedInt64Array(args)
                }
                "PackedFloat32Array" | "PackedRealArray" | "PoolRealArray" | "FloatArray" => {
                    let args = self.parse_construct_float()?;
                    Variant::PackedFloat32Array(args.iter().map(|p| (*p as f32).into()).collect())
                }
                "PackedFloat64Array" => {
                    let args = self.parse_construct_float()?;
                    Variant::PackedFloat64Array(args.iter().map(|p| (*p).into()).collect())
                }
                "PackedStringArray" | "PoolStringArray" | "StringArray" => {
                    let token = self.get_token().ok();
                    if !matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let mut args = Vec::new();

                    let mut first = true;
                    loop {
                        if !first {
                            let token = self.get_token().ok();

                            if matches!(token, Some(Token::Comma)) {
                                // do none
                            } else if matches!(token, Some(Token::ParenthesisClose)) {
                                break;
                            } else {
                                return Err(ParseError("Expected ',' or ')'".into()));
                            }
                        }

                        let token = self.get_token().ok();
                        if matches!(token, Some(Token::ParenthesisClose)) {
                            break;
                        } else if let Some(Token::String(str)) = token {
                            first = false;
                            args.push(str);
                        } else {
                            return Err(ParseError("Expected string".into()));
                        }
                    }

                    Variant::PackedStringArray(args)
                }
                "PackedVector2Array" | "PoolVector2Array" | "Vector2Array" => {
                    let args = self.parse_construct_float()?;
                    let args = args
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|v| Vector2::new(v[0], v[1]))
                        .collect();
                    Variant::PackedVector2Array(args)
                }
                "PackedVector3Array" | "PoolVector3Array" | "Vector3Array" => {
                    let args = self.parse_construct_float()?;
                    let args = args
                        .as_chunks::<3>()
                        .0
                        .iter()
                        .map(|v| Vector3::new(v[0], v[1], v[2]))
                        .collect();
                    Variant::PackedVector3Array(args)
                }
                "PackedVector4Array" | "PoolVector4Array" | "Vector4Array" => {
                    let args = self.parse_construct_float()?;
                    let args = args
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|v| Vector4::new(v[0], v[1], v[2], v[3]))
                        .collect();
                    Variant::PackedVector4Array(args)
                }
                "PackedColorArray" | "PoolColorArray" | "ColorArray" => {
                    let args = self.parse_construct_float()?;
                    let args = args
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|v| Color::new(v[0] as f32, v[1] as f32, v[2] as f32, v[3] as f32))
                        .collect();
                    Variant::PackedColorArray(args)
                }
                _ => {
                    return Err(ParseError(format!("Unexpected identifier '{identifier}'")));
                }
            },
            Token::Integer(value) => value.into(),
            Token::Float(value) => value.into(),
            Token::String(value) => value.into(),
            Token::StringName(value) => value.into(),
            Token::Color(value) => value.into(),
            _ => {
                return Err(ParseError(format!(
                    "Expected value, got '{}'",
                    token.name()
                )));
            }
        })
    }

    pub fn parse(&mut self) -> ParseResult<Variant> {
        let token = self.get_token()?;

        if matches!(token, Token::Eof) {
            return Err(ParseError("Expected token, got EOF".into()));
        }

        self.parse_value(token)
    }

    fn parse_tag(&mut self, simple_tag: bool) -> ParseResult<Tag> {
        let token = self.get_token().ok();

        if matches!(token, Some(Token::Eof)) {
            return Err(ParseError("Expected token, got EOF".into()));
        }

        if !matches!(token, Some(Token::BracketOpen)) {
            return Err(ParseError("Expected '['".into()));
        }

        let mut tag = Tag::default();
        if simple_tag {
            let mut name = String::new();
            let mut escaping = false;

            loop {
                let Some(ch) = self.get_char() else {
                    return Err(ParseError("Unexpected EOF while parsing simple tag".into()));
                };

                if ch == ']' {
                    if escaping {
                        escaping = false;
                    } else {
                        break;
                    }
                } else {
                    escaping = ch == '\\';
                }

                name.push(ch);
            }

            tag.name = name.trim().to_string();
            return Ok(tag);
        }

        let Ok(Token::Identifier(name)) = self.get_token() else {
            return Err(ParseError("Expected identifier (tag name)".into()));
        };
        tag.name = name;

        let mut parsing_tag = true;
        loop {
            let mut token = self.get_token().ok();
            if matches!(token, Some(Token::Eof)) {
                return Err(ParseError(format!(
                    "Unexpected EOF while parsing tag '{}'",
                    tag.name
                )));
            }

            if matches!(token, Some(Token::BracketClose)) {
                break;
            }

            if parsing_tag && matches!(token, Some(Token::Period)) {
                tag.name.push('.'); // support tags such as [someprop.Android] for specific platforms
                token = self.get_token().ok();
            } else if parsing_tag && matches!(token, Some(Token::Colon)) {
                tag.name.push(':'); // support tags such as [someprop.Android] for specific platforms

                token = self.get_token().ok();
            } else {
                parsing_tag = false;
            }

            let Some(Token::Identifier(id)) = token else {
                return Err(ParseError("Expected identifier".into()));
            };

            if parsing_tag {
                tag.name.push_str(&id);
                continue;
            }

            let token = self.get_token().ok();
            if !matches!(token, Some(Token::Equal)) {
                return Err(ParseError("Expected '=' after identifier".into()));
            }

            let token = self.get_token()?;
            let value = self.parse_value(token)?;
            tag.fields.insert(id, value);
        }

        Ok(tag)
    }

    pub fn parse_tag_assign_eof(&mut self, simple_tag: bool) -> ParseResult<Option<TagAssign>> {
        let mut what = String::new();

        loop {
            let Some(ch) = self.get_char() else {
                return Ok(None);
            };

            if ch == ';' {
                // comment
                loop {
                    let Some(ch) = self.get_char() else {
                        return Ok(None);
                    };

                    if ch == '\n' {
                        self.line += 1;
                        break;
                    }
                }

                continue;
            }

            if ch == '[' && what.is_empty() {
                // it's a tag!
                self.saved_char = Some(Some('['));

                let tag = self.parse_tag(simple_tag)?;
                return Ok(Some(TagAssign::Tag(tag)));
            }

            if (ch as u32) > 32 {
                if ch == '"' {
                    // quoted
                    self.saved_char = Some(Some('"'));

                    let Token::String(value) = self.get_token()? else {
                        return Err(ParseError("Error reading quoted string".into()));
                    };

                    what = value;
                } else if ch != '=' {
                    what.push(ch);
                } else {
                    let token = self.get_token()?;
                    let value = self.parse_value(token)?;
                    return Ok(Some(TagAssign::Variant {
                        assign: what,
                        value,
                    }));
                }
            } else if ch == '\n' {
                self.line += 1;
            }
        }
    }
}

pub fn write_variant(variant: &Variant, compat: bool) -> String {
    let mut str = String::new();

    match variant {
        Variant::Nil(_) => str.push_str("null"),
        Variant::Bool(value) => str.push_str(if *value { "true" } else { "false" }),
        Variant::Int(value) => str.push_str(&value.to_string()),
        Variant::Float(value) => {
            let mut s = rtos_fix((*value).into(), compat);

            // Append ".0" to floats to ensure they are float literals.
            if s != "inf"
                && s != "-inf"
                && s != "nan"
                && !s.contains('.')
                && !s.contains('e')
                && !s.contains('E')
            {
                s += ".0";
            }

            str.push_str(&s);
        }
        Variant::String(value) => {
            str.push_str(&format!("\"{}\"", escape_string_multiline(value)));
        }

        Variant::Vector2(value) => {
            str.push_str(&format!(
                "Vector2({}, {})",
                rtos_fix(value.x.into(), compat),
                rtos_fix(value.y.into(), compat),
            ));
        }
        Variant::Vector2i(value) => {
            str.push_str(&format!(
                "Vector2i({}, {})",
                rtos_fix(value.x.into(), compat),
                rtos_fix(value.y.into(), compat),
            ));
        }
        Variant::Rect2(value) => {
            str.push_str(&format!(
                "Rect2({}, {}, {}, {})",
                rtos_fix(value.position.x.into(), compat),
                rtos_fix(value.position.y.into(), compat),
                rtos_fix(value.size.x.into(), compat),
                rtos_fix(value.size.y.into(), compat),
            ));
        }
        Variant::Rect2i(value) => {
            str.push_str(&format!(
                "Rect2i({}, {}, {}, {})",
                rtos_fix(value.position.x.into(), compat),
                rtos_fix(value.position.y.into(), compat),
                rtos_fix(value.size.x.into(), compat),
                rtos_fix(value.size.y.into(), compat),
            ));
        }
        Variant::Vector3(value) => {
            str.push_str(&format!(
                "Vector3({}, {}, {})",
                rtos_fix(value.x.into(), compat),
                rtos_fix(value.y.into(), compat),
                rtos_fix(value.z.into(), compat),
            ));
        }
        Variant::Vector3i(value) => {
            str.push_str(&format!(
                "Vector3i({}, {}, {})",
                rtos_fix(value.x.into(), compat),
                rtos_fix(value.y.into(), compat),
                rtos_fix(value.z.into(), compat),
            ));
        }
        Variant::Vector4(value) => {
            str.push_str(&format!(
                "Vector4({}, {}, {}, {})",
                rtos_fix(value.x.into(), compat),
                rtos_fix(value.y.into(), compat),
                rtos_fix(value.z.into(), compat),
                rtos_fix(value.w.into(), compat),
            ));
        }
        Variant::Vector4i(value) => {
            str.push_str(&format!(
                "Vector4i({}, {}, {}, {})",
                rtos_fix(value.x.into(), compat),
                rtos_fix(value.y.into(), compat),
                rtos_fix(value.z.into(), compat),
                rtos_fix(value.w.into(), compat),
            ));
        }
        Variant::Plane(value) => {
            str.push_str(&format!(
                "Plane({}, {}, {}, {})",
                rtos_fix(value.normal.x.into(), compat),
                rtos_fix(value.normal.y.into(), compat),
                rtos_fix(value.normal.z.into(), compat),
                rtos_fix(value.d.into(), compat),
            ));
        }
        Variant::Aabb(value) => {
            str.push_str(&format!(
                "AABB({}, {}, {}, {}, {}, {})",
                rtos_fix(value.position.x.into(), compat),
                rtos_fix(value.position.y.into(), compat),
                rtos_fix(value.position.z.into(), compat),
                rtos_fix(value.size.x.into(), compat),
                rtos_fix(value.size.y.into(), compat),
                rtos_fix(value.size.z.into(), compat),
            ));
        }
        Variant::Quaternion(value) => {
            str.push_str(&format!(
                "Quaternion({}, {}, {}, {})",
                rtos_fix(value.x.into(), compat),
                rtos_fix(value.y.into(), compat),
                rtos_fix(value.z.into(), compat),
                rtos_fix(value.w.into(), compat),
            ));
        }
        Variant::Transform2d(value) => {
            str.push_str(&format!(
                "Transform2D({}, {}, {}, {}, {}, {})",
                rtos_fix(value.x.x.into(), compat),
                rtos_fix(value.x.y.into(), compat),
                rtos_fix(value.y.x.into(), compat),
                rtos_fix(value.y.y.into(), compat),
                rtos_fix(value.origin.x.into(), compat),
                rtos_fix(value.origin.y.into(), compat),
            ));
        }
        Variant::Basis(value) => {
            str.push_str(&format!(
                "Basis({}, {}, {}, {}, {}, {}, {}, {}, {})",
                rtos_fix(value.x.x.into(), compat),
                rtos_fix(value.x.y.into(), compat),
                rtos_fix(value.x.z.into(), compat),
                rtos_fix(value.y.x.into(), compat),
                rtos_fix(value.y.y.into(), compat),
                rtos_fix(value.y.z.into(), compat),
                rtos_fix(value.z.x.into(), compat),
                rtos_fix(value.z.y.into(), compat),
                rtos_fix(value.z.z.into(), compat),
            ));
        }
        Variant::Transform3d(value) => {
            str.push_str(&format!(
                "Transform3D({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
                rtos_fix(value.basis.x.x.into(), compat),
                rtos_fix(value.basis.x.y.into(), compat),
                rtos_fix(value.basis.x.z.into(), compat),
                rtos_fix(value.basis.y.x.into(), compat),
                rtos_fix(value.basis.y.y.into(), compat),
                rtos_fix(value.basis.y.z.into(), compat),
                rtos_fix(value.basis.z.x.into(), compat),
                rtos_fix(value.basis.z.y.into(), compat),
                rtos_fix(value.basis.z.z.into(), compat),
                rtos_fix(value.origin.x.into(), compat),
                rtos_fix(value.origin.y.into(), compat),
                rtos_fix(value.origin.z.into(), compat),
            ));
        }
        Variant::Projection(value) => {
            str.push_str(&format!(
                "Projection({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
                rtos_fix(value.x.x.into(), compat),
                rtos_fix(value.x.y.into(), compat),
                rtos_fix(value.x.z.into(), compat),
                rtos_fix(value.x.w.into(), compat),
                rtos_fix(value.y.x.into(), compat),
                rtos_fix(value.y.y.into(), compat),
                rtos_fix(value.y.z.into(), compat),
                rtos_fix(value.y.w.into(), compat),
                rtos_fix(value.z.x.into(), compat),
                rtos_fix(value.z.y.into(), compat),
                rtos_fix(value.z.z.into(), compat),
                rtos_fix(value.z.w.into(), compat),
                rtos_fix(value.w.x.into(), compat),
                rtos_fix(value.w.y.into(), compat),
                rtos_fix(value.w.z.into(), compat),
                rtos_fix(value.w.w.into(), compat),
            ));
        }

        Variant::Color(value) => {
            str.push_str(&format!(
                "Color({}, {}, {}, {})",
                rtos_fix(value.r.0.into(), compat),
                rtos_fix(value.g.0.into(), compat),
                rtos_fix(value.b.0.into(), compat),
                rtos_fix(value.a.0.into(), compat),
            ));
        }
        Variant::StringName(value) => {
            str.push_str(&format!("&\"{}\"", escape_string(&value.0)));
        }
        Variant::NodePath(value) => {
            str.push_str(&format!("^\"{}\"", escape_string(&value.to_string())));
        }
        Variant::Rid(value) => {
            if value.0 == 0 {
                str.push_str("RID()");
            } else {
                str.push_str(&format!("RID({})", value.0));
            }
        }

        Variant::Signal(_) => {
            str.push_str("Signal()");
        }
        Variant::Callable(_) => {
            str.push_str("Callable()");
        }

        Variant::Object(value) => {
            let obj = match value {
                ObjectKind::Object(obj) => obj,
                _ => unreachable!(),
            };

            str.push_str(&format!("Object({},", obj.class));

            let mut first = true;
            for (key, value) in &obj.properties {
                if first {
                    first = false;
                } else {
                    str.push('.');
                }

                str.push_str(&format!("\"{}\"", key));
                str.push_str(&write_variant(value, compat));
            }

            str.push_str(")\n");
        }
        Variant::Dictionary(dict) => {
            let is_typed =
                dict.key_type != ContainerType::None || dict.value_type != ContainerType::None;

            if is_typed {
                str.push_str("Dictionary[");

                match &dict.key_type {
                    ContainerType::ClassName(class_name) => str.push_str(class_name),
                    ContainerType::Script(script) => str.push_str(script),
                    ContainerType::Builtin(variant) => {
                        if *variant == VariantType::Nil {
                            str.push_str("Variant");
                        } else {
                            str.push_str(variant.name());
                        }
                    }
                    ContainerType::None => unreachable!(),
                }

                str.push_str(", ");

                match &dict.value_type {
                    ContainerType::ClassName(class_name) => str.push_str(class_name),
                    ContainerType::Script(script) => str.push_str(script),
                    ContainerType::Builtin(variant) => {
                        if *variant == VariantType::Nil {
                            str.push_str("Variant");
                        } else {
                            str.push_str(variant.name());
                        }
                    }
                    ContainerType::None => unreachable!(),
                }

                str.push_str("](");
            }

            if dict.inner.is_empty() {
                // Avoid unnecessary line break.
                str.push_str("{}");
            } else {
                str.push_str("{\n");

                for (i, (key, value)) in dict.inner.iter().enumerate() {
                    str.push_str(&write_variant(key, compat));
                    str.push_str(": ");
                    str.push_str(&write_variant(value, compat));

                    if i + 1 < dict.inner.len() {
                        str.push_str(",\n");
                    } else {
                        str.push('\n');
                    }
                }

                str.push('}');
            }

            if is_typed {
                str.push(')');
            }
        }
        Variant::Array(array) => {
            let is_typed = array.element_type != ContainerType::None;
            if is_typed {
                str.push_str("Array[");

                match &array.element_type {
                    ContainerType::ClassName(class_name) => str.push_str(class_name),
                    ContainerType::Script(script) => str.push_str(script),
                    ContainerType::Builtin(variant) => str.push_str(variant.name()),
                    ContainerType::None => unreachable!(),
                }

                str.push_str("](");
            }

            str.push('[');

            let mut first = true;
            for value in &array.inner {
                if first {
                    first = false;
                } else {
                    str.push_str(", ");
                }

                str.push_str(&write_variant(value, compat));
            }

            str.push(']');

            if is_typed {
                str.push(')');
            }
        }

        Variant::PackedByteArray(value) => {
            str.push_str(&format!(
                "PackedByteArray({})",
                if compat {
                    value
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                } else {
                    format!("\"{}\"", STANDARD.encode(value))
                }
            ));
        }
        Variant::PackedInt32Array(value) => {
            str.push_str(&format!(
                "PackedInt32Array({})",
                value
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Variant::PackedInt64Array(value) => {
            str.push_str(&format!(
                "PackedInt64Array({})",
                value
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Variant::PackedFloat32Array(value) => {
            str.push_str(&format!(
                "PackedFloat32Array({})",
                value
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Variant::PackedFloat64Array(value) => {
            str.push_str(&format!(
                "PackedFloat64Array({})",
                value
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Variant::PackedStringArray(value) => {
            str.push_str(&format!(
                "PackedStringArray({})",
                value
                    .iter()
                    .map(|c| format!("\"{}\"", escape_string(c)))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Variant::PackedVector2Array(value) => {
            str.push_str(&format!(
                "PackedVector2Array({})",
                value
                    .iter()
                    .map(|c| format!(
                        "{}, {}",
                        rtos_fix(c.x.into(), compat),
                        rtos_fix(c.y.into(), compat)
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Variant::PackedVector3Array(value) => {
            str.push_str(&format!(
                "PackedVector3Array({})",
                value
                    .iter()
                    .map(|c| format!(
                        "{}, {}, {}",
                        rtos_fix(c.x.into(), compat),
                        rtos_fix(c.y.into(), compat),
                        rtos_fix(c.z.into(), compat)
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Variant::PackedColorArray(value) => {
            str.push_str(&format!(
                "PackedColorArray({})",
                value
                    .iter()
                    .map(|c| format!(
                        "{}, {}, {}, {}",
                        rtos_fix(c.r.0.into(), compat),
                        rtos_fix(c.g.0.into(), compat),
                        rtos_fix(c.b.0.into(), compat),
                        rtos_fix(c.a.0.into(), compat)
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        Variant::PackedVector4Array(value) => {
            str.push_str(&format!(
                "PackedVector4Array({})",
                value
                    .iter()
                    .map(|c| format!(
                        "{}, {}, {}, {}",
                        rtos_fix(c.x.into(), compat),
                        rtos_fix(c.y.into(), compat),
                        rtos_fix(c.z.into(), compat),
                        rtos_fix(c.w.into(), compat)
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }

    str
}
