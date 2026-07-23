//! GDScript types and format parsing.

pub mod parser;
pub mod tokenizer;

use crate::variant::Variant;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TokenType {
    Empty,
    // Basic
    Annotation,
    Identifier,
    Literal,
    // Comparison
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    EqualEqual,
    BangEqual,
    // Logical
    And,
    Or,
    Not,
    AmpersandAmpersand,
    PipePipe,
    Bang,
    // Bitwise
    Ampersand,
    Pipe,
    Tilde,
    Caret,
    LessLess,
    GreaterGreater,
    // Math
    Plus,
    Minus,
    Star,
    StarStar,
    Slash,
    Percent,
    // Assignment
    Equal,
    PlusEqual,
    MinusEqual,
    StarEqual,
    StarStarEqual,
    SlashEqual,
    PercentEqual,
    LessLessEqual,
    GreaterGreaterEqual,
    AmpersandEqual,
    PipeEqual,
    CaretEqual,
    // Control flow
    If,
    Elif,
    Else,
    For,
    While,
    Break,
    Continue,
    Pass,
    Return,
    Match,
    When,
    // Keywords
    As,
    Assert,
    Await,
    Breakpoint,
    Class,
    ClassName,
    Const,
    Enum,
    Extends,
    Func,
    In,
    Is,
    Namespace,
    Preload,
    #[serde(rename = "Self")]
    _Self,
    Signal,
    Static,
    Super,
    Trait,
    Var,
    Void,
    Yield,
    // Punctuation
    BracketOpen,
    BracketClose,
    BraceOpen,
    BraceClose,
    ParenthesisOpen,
    ParenthesisClose,
    Comma,
    Semicolon,
    Period,
    PeriodPeriod,
    PeriodPeriodPeriod,
    Colon,
    Dollar,
    ForwardArrow,
    Underscore,
    // Whitespace
    Newline,
    Indent,
    Dedent,
    // Constants
    ConstPi,
    ConstTau,
    ConstInf,
    ConstNan,
    // Error message improvement
    VcsConflictMarker,
    Backtick,
    QuestionMark,
    // Special
    Error,
    Eof,

    // Unused keywords removed in Godot 3.1
    // https://github.com/godotengine/godot/commit/d35003d92ae97c515b6fd2c319df2d7a8f14e28d
    Do,
    Case,
    Switch,

    // Renamed. Deprecated in Godot 3.1, removed in Godot 3.2
    // https://github.com/godotengine/godot/commit/d6b31daec61286dc5ebf953e0f2e70817deaf5ef
    Slave,
    SlaveSync,

    // Removed in the GDScript 2.0 rewrite
    // https://github.com/godotengine/godot/commit/5d6e8538065050d5f5579ec03cfa9e241811e062

    // Basic
    BuiltInType,
    BuiltInFunc,

    // Keywords
    OnReady,
    Tool,
    Export,
    SetGet,
    Remote,
    Sync,
    Master,
    Puppet,
    RemoteSync,
    MasterSync,
    PuppetSync,

    Wildcard,
    Cursor,

    // Converted to an annotation in Godot 4.5
    // https://github.com/godotengine/godot/commit/1085200f51716608a59b055e0447b3924c62b7d8
    Abstract,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum Token {
    Empty,
    // Basic
    Annotation(String),
    Identifier(String),
    Literal(Variant),
    // Comparison
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    EqualEqual,
    BangEqual,
    // Logical
    And,
    Or,
    Not,
    AmpersandAmpersand,
    PipePipe,
    Bang,
    // Bitwise
    Ampersand,
    Pipe,
    Tilde,
    Caret,
    LessLess,
    GreaterGreater,
    // Math
    Plus,
    Minus,
    Star,
    StarStar,
    Slash,
    Percent,
    // Assignment
    Equal,
    PlusEqual,
    MinusEqual,
    StarEqual,
    StarStarEqual,
    SlashEqual,
    PercentEqual,
    LessLessEqual,
    GreaterGreaterEqual,
    AmpersandEqual,
    PipeEqual,
    CaretEqual,
    // Control flow
    If,
    Elif,
    Else,
    For,
    While,
    Break,
    Continue,
    Pass,
    Return,
    Match,
    When,
    // Keywords
    As,
    Assert,
    Await,
    Breakpoint,
    Class,
    ClassName,
    Const,
    Enum,
    Extends,
    Func,
    In,
    Is,
    Namespace,
    Preload,
    _Self,
    Signal,
    Static,
    Super,
    Trait,
    Var,
    Void,
    Yield,
    // Punctuation
    BracketOpen,
    BracketClose,
    BraceOpen,
    BraceClose,
    ParenthesisOpen,
    ParenthesisClose,
    Comma,
    Semicolon,
    Period,
    PeriodPeriod,
    PeriodPeriodPeriod,
    Colon,
    Dollar,
    ForwardArrow,
    Underscore,
    // Whitespace
    Newline {
        /// Whether this newline represents a line continuation (ending a line with a backslash).
        continuation: bool,
    },
    Indent,
    Dedent,
    // Constants
    ConstPi,
    ConstTau,
    ConstInf,
    ConstNan,
    // Error message improvement
    VcsConflictMarker,
    Backtick,
    QuestionMark,
    // Special
    Error(Variant),
    Eof,

    // Unused keywords removed in Godot 3.1
    // https://github.com/godotengine/godot/commit/d35003d92ae97c515b6fd2c319df2d7a8f14e28d
    Do,
    Case,
    Switch,

    // Renamed. Deprecated in Godot 3.1, removed in Godot 3.2
    // https://github.com/godotengine/godot/commit/d6b31daec61286dc5ebf953e0f2e70817deaf5ef
    Slave,
    SlaveSync,

    // Removed in the GDScript 2.0 rewrite
    // https://github.com/godotengine/godot/commit/5d6e8538065050d5f5579ec03cfa9e241811e062

    // Basic
    BuiltInType,
    BuiltInFunc,

    // Keywords
    OnReady,
    Tool,
    Export,
    SetGet,
    Remote,
    Sync,
    Master,
    Puppet,
    RemoteSync,
    MasterSync,
    PuppetSync,

    Wildcard,
    Cursor,

    // Converted to an annotation in Godot 4.5
    // https://github.com/godotengine/godot/commit/1085200f51716608a59b055e0447b3924c62b7d8
    Abstract,
}

impl Token {
    pub fn typ(&self) -> TokenType {
        match self {
            Self::Empty => TokenType::Empty,
            Self::Annotation(_) => TokenType::Annotation,
            Self::Identifier(_) => TokenType::Identifier,
            Self::Literal(_) => TokenType::Literal,
            Self::Less => TokenType::Less,
            Self::LessEqual => TokenType::LessEqual,
            Self::Greater => TokenType::Greater,
            Self::GreaterEqual => TokenType::GreaterEqual,
            Self::EqualEqual => TokenType::EqualEqual,
            Self::BangEqual => TokenType::BangEqual,
            Self::And => TokenType::And,
            Self::Or => TokenType::Or,
            Self::Not => TokenType::Not,
            Self::AmpersandAmpersand => TokenType::AmpersandAmpersand,
            Self::PipePipe => TokenType::PipePipe,
            Self::Bang => TokenType::Bang,
            Self::Ampersand => TokenType::Ampersand,
            Self::Pipe => TokenType::Pipe,
            Self::Tilde => TokenType::Tilde,
            Self::Caret => TokenType::Caret,
            Self::LessLess => TokenType::LessLess,
            Self::GreaterGreater => TokenType::GreaterGreater,
            Self::Plus => TokenType::Plus,
            Self::Minus => TokenType::Minus,
            Self::Star => TokenType::Star,
            Self::StarStar => TokenType::StarStar,
            Self::Slash => TokenType::Slash,
            Self::Percent => TokenType::Percent,
            Self::Equal => TokenType::Equal,
            Self::PlusEqual => TokenType::PlusEqual,
            Self::MinusEqual => TokenType::MinusEqual,
            Self::StarEqual => TokenType::StarEqual,
            Self::StarStarEqual => TokenType::StarStarEqual,
            Self::SlashEqual => TokenType::SlashEqual,
            Self::PercentEqual => TokenType::PercentEqual,
            Self::LessLessEqual => TokenType::LessLessEqual,
            Self::GreaterGreaterEqual => TokenType::GreaterGreaterEqual,
            Self::AmpersandEqual => TokenType::AmpersandEqual,
            Self::PipeEqual => TokenType::PipeEqual,
            Self::CaretEqual => TokenType::CaretEqual,
            Self::If => TokenType::If,
            Self::Elif => TokenType::Elif,
            Self::Else => TokenType::Else,
            Self::For => TokenType::For,
            Self::While => TokenType::While,
            Self::Break => TokenType::Break,
            Self::Continue => TokenType::Continue,
            Self::Pass => TokenType::Pass,
            Self::Return => TokenType::Return,
            Self::Match => TokenType::Match,
            Self::When => TokenType::When,
            Self::As => TokenType::As,
            Self::Assert => TokenType::Assert,
            Self::Await => TokenType::Await,
            Self::Breakpoint => TokenType::Breakpoint,
            Self::Class => TokenType::Class,
            Self::ClassName => TokenType::ClassName,
            Self::Const => TokenType::Const,
            Self::Enum => TokenType::Enum,
            Self::Extends => TokenType::Extends,
            Self::Func => TokenType::Func,
            Self::In => TokenType::In,
            Self::Is => TokenType::Is,
            Self::Namespace => TokenType::Namespace,
            Self::Preload => TokenType::Preload,
            Self::_Self => TokenType::_Self,
            Self::Signal => TokenType::Signal,
            Self::Static => TokenType::Static,
            Self::Super => TokenType::Super,
            Self::Trait => TokenType::Trait,
            Self::Var => TokenType::Var,
            Self::Void => TokenType::Void,
            Self::Yield => TokenType::Yield,
            Self::BracketOpen => TokenType::BracketOpen,
            Self::BracketClose => TokenType::BracketClose,
            Self::BraceOpen => TokenType::BraceOpen,
            Self::BraceClose => TokenType::BraceClose,
            Self::ParenthesisOpen => TokenType::ParenthesisOpen,
            Self::ParenthesisClose => TokenType::ParenthesisClose,
            Self::Comma => TokenType::Comma,
            Self::Semicolon => TokenType::Semicolon,
            Self::Period => TokenType::Period,
            Self::PeriodPeriod => TokenType::PeriodPeriod,
            Self::PeriodPeriodPeriod => TokenType::PeriodPeriodPeriod,
            Self::Colon => TokenType::Colon,
            Self::Dollar => TokenType::Dollar,
            Self::ForwardArrow => TokenType::ForwardArrow,
            Self::Underscore => TokenType::Underscore,
            Self::Newline { .. } => TokenType::Newline,
            Self::Indent { .. } => TokenType::Indent,
            Self::Dedent { .. } => TokenType::Dedent,
            Self::ConstPi => TokenType::ConstPi,
            Self::ConstTau => TokenType::ConstTau,
            Self::ConstInf => TokenType::ConstInf,
            Self::ConstNan => TokenType::ConstNan,
            Self::VcsConflictMarker => TokenType::VcsConflictMarker,
            Self::Backtick => TokenType::Backtick,
            Self::QuestionMark => TokenType::QuestionMark,
            Self::Error(_) => TokenType::Error,
            Self::Eof => TokenType::Eof,
            Self::Do => TokenType::Do,
            Self::Case => TokenType::Case,
            Self::Switch => TokenType::Switch,
            Self::Slave => TokenType::Slave,
            Self::SlaveSync => TokenType::SlaveSync,
            Self::BuiltInType => TokenType::BuiltInType,
            Self::BuiltInFunc => TokenType::BuiltInFunc,
            Self::OnReady => TokenType::OnReady,
            Self::Tool => TokenType::Tool,
            Self::Export => TokenType::Export,
            Self::SetGet => TokenType::SetGet,
            Self::Remote => TokenType::Remote,
            Self::Sync => TokenType::Sync,
            Self::Master => TokenType::Master,
            Self::Puppet => TokenType::Puppet,
            Self::RemoteSync => TokenType::RemoteSync,
            Self::MasterSync => TokenType::MasterSync,
            Self::PuppetSync => TokenType::PuppetSync,
            Self::Wildcard => TokenType::Wildcard,
            Self::Cursor => TokenType::Cursor,
            Self::Abstract => TokenType::Abstract,
        }
    }

    /// Whether this token can come before a binary operator.
    pub fn can_precede_bin_op(&self) -> bool {
        matches!(
            self,
            Self::Identifier(_)
                | Self::Literal(_)
                | Self::_Self
                | Self::BracketClose
                | Self::BraceClose
                | Self::ParenthesisClose
                | Self::ConstPi
                | Self::ConstTau
                | Self::ConstInf
                | Self::ConstNan
        )
    }

    /// Returns if a token is a valid identifier.
    pub fn is_identifier(&self) -> bool {
        match self {
            Self::Identifier(_) => true,
            Self::Match => true, // Used in String.match()
            Self::When => true,  // New keyword, avoid breaking existing code

            // Allow constants to be treated as regular identifiers
            Self::ConstPi => true,
            Self::ConstInf => true,
            Self::ConstTau => true,
            Self::ConstNan => true,

            _ => false,
        }
    }

    pub fn get_identifier(&self) -> &str {
        match self {
            Token::Identifier(inner) => inner,
            Token::ConstNan => "NAN", // XXX: bro
            other => other.token_name(),
        }
    }

    /// This is meant to allow keywords with the $ notation, but not as general identifiers.
    pub fn is_node_name(&self) -> bool {
        matches!(
            self,
            Self::Identifier(_)
                | Self::And
                | Self::As
                | Self::Assert
                | Self::Await
                | Self::Break
                | Self::Breakpoint
                | Self::ClassName
                | Self::Class
                | Self::Const
                | Self::ConstPi
                | Self::ConstInf
                | Self::ConstNan
                | Self::ConstTau
                | Self::Continue
                | Self::Elif
                | Self::Else
                | Self::Enum
                | Self::Extends
                | Self::For
                | Self::Func
                | Self::If
                | Self::In
                | Self::Is
                | Self::Match
                | Self::Namespace
                | Self::Not
                | Self::Or
                | Self::Pass
                | Self::Preload
                | Self::Return
                | Self::_Self
                | Self::Signal
                | Self::Static
                | Self::Super
                | Self::Trait
                | Self::Underscore
                | Self::Var
                | Self::Void
                | Self::While
                | Self::When
        )
    }

    pub fn token_name(&self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::Annotation(_) => "Annotation",
            Self::Identifier(_) => "Identifier",
            Self::Literal(_) => "Literal",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::EqualEqual => "==",
            Self::BangEqual => "!=",
            Self::And => "and",
            Self::Or => "or",
            Self::Not => "not",
            Self::AmpersandAmpersand => "&&",
            Self::PipePipe => "||",
            Self::Bang => "!",
            Self::Ampersand => "&",
            Self::Pipe => "|",
            Self::Tilde => "~",
            Self::Caret => "^",
            Self::LessLess => "<<",
            Self::GreaterGreater => ">>",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::StarStar => "**",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::Equal => "=",
            Self::PlusEqual => "+=",
            Self::MinusEqual => "-=",
            Self::StarEqual => "*=",
            Self::StarStarEqual => "**=",
            Self::SlashEqual => "/=",
            Self::PercentEqual => "%=",
            Self::LessLessEqual => "<<=",
            Self::GreaterGreaterEqual => ">>=",
            Self::AmpersandEqual => "&=",
            Self::PipeEqual => "|=",
            Self::CaretEqual => "^=",
            Self::If => "if",
            Self::Elif => "elif",
            Self::Else => "else",
            Self::For => "for",
            Self::While => "while",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Pass => "pass",
            Self::Return => "return",
            Self::Match => "match",
            Self::When => "when",
            Self::As => "as",
            Self::Assert => "assert",
            Self::Await => "await",
            Self::Breakpoint => "breakpoint",
            Self::Class => "class",
            Self::ClassName => "class_name",
            Self::Const => "const",
            Self::Enum => "enum",
            Self::Extends => "extends",
            Self::Func => "func",
            Self::In => "in",
            Self::Is => "is",
            Self::Namespace => "namespace",
            Self::Preload => "preload",
            Self::_Self => "self",
            Self::Signal => "signal",
            Self::Static => "static",
            Self::Super => "super",
            Self::Trait => "trait",
            Self::Var => "var",
            Self::Void => "void",
            Self::Yield => "yield",
            Self::BracketOpen => "[",
            Self::BracketClose => "]",
            Self::BraceOpen => "{",
            Self::BraceClose => "}",
            Self::ParenthesisOpen => "(",
            Self::ParenthesisClose => ")",
            Self::Comma => ",",
            Self::Semicolon => ";",
            Self::Period => ".",
            Self::PeriodPeriod => "..",
            Self::PeriodPeriodPeriod => "...",
            Self::Colon => ":",
            Self::Dollar => "$",
            Self::ForwardArrow => "->",
            Self::Underscore => "_",
            Self::Newline { .. } => "Newline",
            Self::Indent { .. } => "Indent",
            Self::Dedent { .. } => "Dedent",
            Self::ConstPi => "PI",
            Self::ConstTau => "TAU",
            Self::ConstInf => "INF",
            Self::ConstNan => "NaN",
            Self::VcsConflictMarker => "VCS conflict marker",
            Self::Backtick => "`",
            Self::QuestionMark => "?",
            Self::Error(_) => "Error",
            Self::Eof => "End of file",
            Self::Do => "do",
            Self::Case => "switch (reserved)",
            Self::Switch => "case (reserved)",
            Self::Slave => "slave",
            Self::SlaveSync => "slavesync",
            Self::BuiltInType => "Built-In Type",
            Self::BuiltInFunc => "Built-in Func",
            Self::OnReady => "onready",
            Self::Tool => "tool",
            Self::Export => "export",
            Self::SetGet => "setget",
            Self::Remote => "rpc",
            Self::Sync => "sync",
            Self::Master => "master",
            Self::Puppet => "puppet",
            Self::RemoteSync => "remotesync",
            Self::MasterSync => "mastersync",
            Self::PuppetSync => "puppetsync",
            Self::Wildcard => "_",
            Self::Cursor => "Cursor",
            Self::Abstract => "abstract",
        }
    }
}

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct Position {
    pub line: usize,

    /// Column (measured in characters).
    pub column: usize,
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    /// Creates a span between two positions.
    pub(crate) const fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Creates a span at the zero position.
    pub(crate) const fn zero() -> Span {
        let position = Position { line: 0, column: 0 };

        Self {
            start: position,
            end: position,
        }
    }
}

pub type Spanned<T> = (T, Span);
