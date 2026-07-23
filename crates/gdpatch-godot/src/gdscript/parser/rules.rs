use crate::gdscript::parser::node::NodeRef;
use crate::gdscript::{
    TokenType,
    parser::{Parser, Precedence, node::ExpressionNode},
};

type PrefixFunction = fn(&mut Parser<'_>, can_assign: bool) -> Option<NodeRef<ExpressionNode>>;

type InfixFunction = fn(
    &mut Parser<'_>,
    previous_operand: NodeRef<ExpressionNode>,
    can_assign: bool,
) -> Option<NodeRef<ExpressionNode>>;

#[derive(Debug, Copy, Clone)]
pub struct InfixRule {
    pub parser: InfixFunction,
    pub precedence: Precedence,
}

impl InfixRule {
    pub fn new(parser: InfixFunction, precedence: Precedence) -> Self {
        Self { parser, precedence }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ParseRule {
    pub prefix: Option<PrefixFunction>,
    pub infix: Option<InfixRule>,
}

impl ParseRule {
    fn new(prefix: Option<PrefixFunction>, infix: Option<InfixRule>) -> Self {
        ParseRule { prefix, infix }
    }

    pub fn precedence(&self) -> Precedence {
        let Some(infix) = &self.infix else {
            return Precedence::None;
        };
        infix.precedence
    }
}

pub fn get_rule(token: TokenType) -> ParseRule {
    match token {
        TokenType::Empty => ParseRule::new(None, None),
        TokenType::Annotation => ParseRule::new(None, None),
        TokenType::Identifier => ParseRule::new(
            Some(|parser, _| Some(parser.parse_identifier().upcast())),
            None,
        ),
        TokenType::Literal => ParseRule::new(
            Some(|parser, can_assign| parser.parse_literal(can_assign)),
            None,
        ),
        TokenType::Less => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::Comparison,
            )),
        ),
        TokenType::LessEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::Comparison,
            )),
        ),
        TokenType::Greater => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::Comparison,
            )),
        ),
        TokenType::GreaterEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::Comparison,
            )),
        ),
        TokenType::EqualEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::Comparison,
            )),
        ),
        TokenType::BangEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::Comparison,
            )),
        ),
        TokenType::And => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::LogicAnd,
            )),
        ),
        TokenType::Or => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::LogicOr,
            )),
        ),
        TokenType::Not => ParseRule::new(
            Some(|parser, can_assign| parser.parse_unary_operator(can_assign)),
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_not_in_operator(prev, can_assign),
                Precedence::ContentTest,
            )),
        ),
        TokenType::AmpersandAmpersand => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::LogicAnd,
            )),
        ),
        TokenType::PipePipe => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::LogicOr,
            )),
        ),
        TokenType::Bang => ParseRule::new(
            Some(|parser, can_assign| parser.parse_unary_operator(can_assign)),
            None,
        ),
        TokenType::Ampersand => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::BitAnd,
            )),
        ),
        TokenType::Pipe => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::BitOr,
            )),
        ),
        TokenType::Tilde => ParseRule::new(
            Some(|parser, can_assign| parser.parse_unary_operator(can_assign)),
            None,
        ),
        TokenType::Caret => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::BitXor,
            )),
        ),
        TokenType::LessLess => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::BitShift,
            )),
        ),
        TokenType::GreaterGreater => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::BitShift,
            )),
        ),
        TokenType::Plus => ParseRule::new(
            Some(|parser, can_assign| parser.parse_unary_operator(can_assign)),
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::AdditionSubtraction,
            )),
        ),
        TokenType::Minus => ParseRule::new(
            Some(|parser, can_assign| parser.parse_unary_operator(can_assign)),
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::AdditionSubtraction,
            )),
        ),
        TokenType::Star => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::Factor,
            )),
        ),
        TokenType::StarStar => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::Power,
            )),
        ),
        TokenType::Slash => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::Factor,
            )),
        ),
        TokenType::Percent => ParseRule::new(
            Some(|parser, can_assign| parser.parse_get_node(can_assign)),
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::Factor,
            )),
        ),
        TokenType::Equal => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::PlusEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::MinusEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::StarEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::StarStarEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::SlashEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::PercentEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::LessLessEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::GreaterGreaterEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::AmpersandEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::PipeEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::CaretEqual => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_assignment(prev, can_assign),
                Precedence::Assignment,
            )),
        ),
        TokenType::If => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_ternary_operator(prev, can_assign),
                Precedence::Ternary,
            )),
        ),
        TokenType::Elif => ParseRule::new(None, None),
        TokenType::Else => ParseRule::new(None, None),
        TokenType::For => ParseRule::new(None, None),
        TokenType::While => ParseRule::new(None, None),
        TokenType::Break => ParseRule::new(None, None),
        TokenType::Continue => ParseRule::new(None, None),
        TokenType::Pass => ParseRule::new(None, None),
        TokenType::Return => ParseRule::new(None, None),
        TokenType::Match => ParseRule::new(None, None),
        TokenType::When => ParseRule::new(None, None),
        TokenType::As => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_cast(prev, can_assign),
                Precedence::Cast,
            )),
        ),
        TokenType::Assert => ParseRule::new(None, None),
        TokenType::Await => ParseRule::new(
            Some(|parser, can_assign| parser.parse_await(can_assign)),
            None,
        ),
        TokenType::Breakpoint => ParseRule::new(None, None),
        TokenType::Class => ParseRule::new(None, None),
        TokenType::ClassName => ParseRule::new(None, None),
        TokenType::Const => ParseRule::new(None, None),
        TokenType::Enum => ParseRule::new(None, None),
        TokenType::Extends => ParseRule::new(None, None),
        TokenType::Func => ParseRule::new(
            Some(|parser, can_assign| parser.parse_lambda(can_assign)),
            None,
        ),
        TokenType::In => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_binary_operator(prev, can_assign),
                Precedence::ContentTest,
            )),
        ),
        TokenType::Is => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_type_test(prev, can_assign),
                Precedence::TypeTest,
            )),
        ),
        TokenType::Namespace => ParseRule::new(None, None),
        TokenType::Preload => ParseRule::new(
            Some(|parser, can_assign| parser.parse_preload(can_assign)),
            None,
        ),
        TokenType::_Self => ParseRule::new(
            Some(|parser, can_assign| parser.parse_self(can_assign)),
            None,
        ),
        TokenType::Signal => ParseRule::new(None, None),
        TokenType::Static => ParseRule::new(None, None),
        TokenType::Super => ParseRule::new(
            Some(|parser, can_assign| parser.parse_call_prefix(can_assign)),
            None,
        ),
        TokenType::Trait => ParseRule::new(None, None),
        TokenType::Var => ParseRule::new(None, None),
        TokenType::Void => ParseRule::new(None, None),
        TokenType::Yield => ParseRule::new(
            Some(|parser, can_assign| parser.parse_yield(can_assign)),
            None,
        ),
        TokenType::BracketOpen => ParseRule::new(
            Some(|parser, can_assign| parser.parse_array(can_assign)),
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_subscript(prev, can_assign),
                Precedence::Subscript,
            )),
        ),
        TokenType::BracketClose => ParseRule::new(None, None),
        TokenType::BraceOpen => ParseRule::new(
            Some(|parser, can_assign| parser.parse_dictionary(can_assign)),
            None,
        ),
        TokenType::BraceClose => ParseRule::new(None, None),
        TokenType::ParenthesisOpen => ParseRule::new(
            Some(|parser, can_assign| parser.parse_grouping(can_assign)),
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_call_infix(prev, can_assign),
                Precedence::Call,
            )),
        ),
        TokenType::ParenthesisClose => ParseRule::new(None, None),
        TokenType::Comma => ParseRule::new(None, None),
        TokenType::Semicolon => ParseRule::new(None, None),
        TokenType::Period => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_attribute(prev, can_assign),
                Precedence::Attribute,
            )),
        ),
        TokenType::PeriodPeriod => ParseRule::new(None, None),
        TokenType::PeriodPeriodPeriod => ParseRule::new(None, None),
        TokenType::Colon => ParseRule::new(None, None),
        TokenType::Dollar => ParseRule::new(
            Some(|parser, can_assign| parser.parse_get_node(can_assign)),
            None,
        ),
        TokenType::ForwardArrow => ParseRule::new(None, None),
        TokenType::Underscore => ParseRule::new(None, None),
        TokenType::Newline => ParseRule::new(None, None),
        TokenType::Indent => ParseRule::new(None, None),
        TokenType::Dedent => ParseRule::new(None, None),
        TokenType::ConstPi => ParseRule::new(
            Some(|parser, can_assign| parser.parse_builtin_constant(can_assign)),
            None,
        ),
        TokenType::ConstTau => ParseRule::new(
            Some(|parser, can_assign| parser.parse_builtin_constant(can_assign)),
            None,
        ),
        TokenType::ConstInf => ParseRule::new(
            Some(|parser, can_assign| parser.parse_builtin_constant(can_assign)),
            None,
        ),
        TokenType::ConstNan => ParseRule::new(
            Some(|parser, can_assign| parser.parse_builtin_constant(can_assign)),
            None,
        ),
        TokenType::VcsConflictMarker => ParseRule::new(None, None),
        TokenType::Backtick => ParseRule::new(None, None),
        TokenType::QuestionMark => ParseRule::new(
            None,
            Some(InfixRule::new(
                |parser, prev, can_assign| parser.parse_invalid_token(prev, can_assign),
                Precedence::Cast,
            )),
        ),
        TokenType::Error => ParseRule::new(None, None),
        TokenType::Eof => ParseRule::new(None, None),

        _ => todo!("GDScript V1"),
    }
}
