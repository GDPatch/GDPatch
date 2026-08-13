use crate::string::{to_float, to_int};
use crate::variant::{
    Aabb, Array, Basis, Callable, Color, Dictionary, Nil, NodePath, Plane, Projection, Quaternion,
    Rect2, Rect2i, Rid, Signal, StringName, Transform2d, Transform3d, Variant, Vector2, Vector2i,
    Vector3, Vector3i, Vector4, Vector4i,
};
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

#[derive(Debug)]
pub struct ParseError(String);

pub type ParseResult<T, E = ParseError> = Result<T, E>;

fn stor_fix(s: &str) -> Option<f64> {
    Some(match s {
        "inf" => f64::INFINITY,
        "-inf" | "inf_neg" => f64::NEG_INFINITY,
        "nan" => f64::NAN,
        _ => return None,
    })
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

    fn _parse_construct(&mut self) -> ParseResult<Vec<Token>> {
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
            ._parse_construct()?
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
            ._parse_construct()?
            .into_iter()
            .map(|token| match token {
                Token::Float(v) => v,
                Token::Integer(v) => v as f64,
                _ => unreachable!(),
            })
            .collect::<Vec<_>>())
    }

    fn parse_dictionary(&mut self) -> ParseResult<Dictionary> {
        todo!()
    }

    fn parse_array(&mut self) -> ParseResult<Array> {
        todo!()
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
                    if matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let Some(Token::String(s)) = self.get_token().ok() else {
                        return Err(ParseError(
                            "Expected string as an argument for NodePath()".into(),
                        ));
                    };

                    let path = NodePath::from_str(&s).unwrap_or_default();

                    let token = self.get_token().ok();
                    if matches!(token, Some(Token::ParenthesisClose)) {
                        return Err(ParseError("Expected ')'".into()));
                    }

                    path.into()
                }
                "RID" => {
                    let token = self.get_token().ok();
                    if matches!(token, Some(Token::ParenthesisOpen)) {
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
                    if matches!(token, Some(Token::ParenthesisClose)) {
                        return Err(ParseError("Expected ')'".into()));
                    }

                    rid.into()
                }
                "Signal" => {
                    let token = self.get_token().ok();
                    if matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let token = self.get_token().ok();
                    if matches!(token, Some(Token::ParenthesisClose)) {
                        return Err(ParseError("Expected ')'".into()));
                    }

                    Signal::default().into()
                }
                "Callable" => {
                    let token = self.get_token().ok();
                    if matches!(token, Some(Token::ParenthesisOpen)) {
                        return Err(ParseError("Expected '('".into()));
                    }

                    let token = self.get_token().ok();
                    if matches!(token, Some(Token::ParenthesisClose)) {
                        return Err(ParseError("Expected ')'".into()));
                    }

                    Callable.into()
                }
                "Object" => {
                    todo!()
                }
                "Resource" | "SubResource" | "ExtResource" => {
                    todo!()
                }
                "Dictionary" => {
                    todo!()
                }
                "Array" => {
                    todo!()
                }
                "PackedByteArray" | "PoolByteArray" | "ByteArray" => {
                    todo!()
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
                    todo!()
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
            todo!()
        }

        self.parse_value(token)
    }
}
