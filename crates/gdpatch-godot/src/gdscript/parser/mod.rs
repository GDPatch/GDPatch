//! Rust port of the GDScript text parser.
//! This parser skips some behavior that the upstream parser performs, notably completion contexts and annotation validation.

use crate::build::GDScriptV2Build;
use crate::do_while;
use crate::gdscript::parser::node::{
    AnnotationNode, ArrayNode, ArrayPattern, AssertNode, AssignmentNode, AssignmentOperation,
    AwaitNode, BinaryOpNode, BinaryOperation, BreakNode, BreakpointNode, CallNode, CastNode,
    ClassNode, ConstantNode, ContinueNode, DictionaryNode, DictionaryPattern,
    DictionaryPatternEntry, DictionaryStyle, DowncastFromNode, EnumNode, EnumValue, ExpressionNode,
    ForNode, FunctionNode, GetNodeNode, IdentifierNode, IfNode, LambdaNode, LiteralNode,
    MatchBranchNode, MatchNode, Node, NodePool, NodeRef, ParameterNode, PassNode, PatternNode,
    PatternType, PreloadNode, Property, ReturnNode, SelfNode, SignalNode, SubscriptNode,
    SubscriptNodeInner, SuiteNode, TernaryOpNode, TypeNode, TypeTestNode, UnaryOpNode,
    UnaryOperation, VariableNode, WhileNode,
};
use crate::gdscript::parser::rules::get_rule;
use crate::gdscript::tokenizer::Tokenizer;
use crate::gdscript::{Position, TokenType};
use crate::variant::{Nil, StringName};
use crate::{
    gdscript::{Span, Spanned, Token},
    variant::{Variant, VariantType},
};
use color_eyre::eyre::bail;
use core::panic;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::f64;
use std::mem;
use std::rc::Rc;
use tracing::error;

mod node;
mod rules;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
enum Precedence {
    None,
    Assignment,
    Cast,
    Ternary,
    LogicOr,
    LogicAnd,
    LogicNot,
    ContentTest,
    Comparison,
    BitOr,
    BitXor,
    BitAnd,
    BitShift,
    AdditionSubtraction,
    Factor,
    Sign,
    BitNot,
    Power,
    TypeTest,
    Await,
    Call,
    Attribute,
    Subscript,
    Primary,
}

impl Precedence {
    pub fn next(self) -> Option<Precedence> {
        Some(match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Cast,
            Precedence::Cast => Precedence::Ternary,
            Precedence::Ternary => Precedence::LogicOr,
            Precedence::LogicOr => Precedence::LogicAnd,
            Precedence::LogicAnd => Precedence::LogicNot,
            Precedence::LogicNot => Precedence::ContentTest,
            Precedence::ContentTest => Precedence::Comparison,
            Precedence::Comparison => Precedence::BitOr,
            Precedence::BitOr => Precedence::BitXor,
            Precedence::BitXor => Precedence::BitAnd,
            Precedence::BitAnd => Precedence::BitShift,
            Precedence::BitShift => Precedence::AdditionSubtraction,
            Precedence::AdditionSubtraction => Precedence::Factor,
            Precedence::Factor => Precedence::Sign,
            Precedence::Sign => Precedence::BitNot,
            Precedence::BitNot => Precedence::Power,
            Precedence::Power => Precedence::TypeTest,
            Precedence::TypeTest => Precedence::Await,
            Precedence::Await => Precedence::Call,
            Precedence::Call => Precedence::Attribute,
            Precedence::Attribute => Precedence::Subscript,
            Precedence::Subscript => Precedence::Primary,
            Precedence::Primary => return None,
        })
    }
}

#[derive(Debug)]
struct ParseError {
    pub message: String,
    pub position: Span,
}

#[derive(Debug)]
pub struct Parser<'tokenizer> {
    tokenizer: &'tokenizer mut dyn Tokenizer,

    consumed_tokens: Vec<Spanned<Token>>,
    current: Spanned<Token>,
    previous: Option<Spanned<Token>>,

    lambda_ended: bool,
    in_lambda: bool,
    panic_mode: bool,

    head: NodeRef<ClassNode>,
    current_class: Option<NodeRef<ClassNode>>,
    current_function: Option<NodeRef<FunctionNode>>,
    current_lambda: Option<NodeRef<LambdaNode>>,
    current_suite: Option<NodeRef<SuiteNode>>,

    node_pool: NodePool,
    multiline_stack: VecDeque<bool>,
    errors: Vec<ParseError>,
    nodes_in_progress: Rc<RefCell<Vec<NodeRef<Node>>>>,

    can_break: bool,
    can_continue: bool,

    pending_indents_at_newline: isize,
}

#[derive(Debug, Eq, PartialEq)]
struct InProgressGuard {
    nodes_in_progress: Rc<RefCell<Vec<NodeRef<Node>>>>,
    idx: usize,
}

impl Drop for InProgressGuard {
    fn drop(&mut self) {
        let mut nodes_in_progress = self.nodes_in_progress.borrow_mut();

        if nodes_in_progress.len() != self.idx + 1 {
            panic!("finished a span in the wrong order");
        }

        nodes_in_progress.pop().unwrap();
    }
}

impl<'tokenizer> Parser<'tokenizer> {
    pub fn new(tokenizer: &'tokenizer mut dyn Tokenizer) -> Self {
        let mut consumed_tokens = Vec::new();

        let current = tokenizer
            .next()
            .expect("tokenizer should have at least one item");
        consumed_tokens.push(current.clone());

        let mut node_pool = NodePool::default();

        let head = node_pool.push((
            ClassNode::default(),
            Span::new(
                Position { line: 1, column: 0 },
                Position { line: 1, column: 0 },
            ),
        ));

        Self {
            tokenizer,
            consumed_tokens,
            current,
            previous: None,

            lambda_ended: false,
            in_lambda: false,
            panic_mode: false,

            head,
            current_class: None,
            current_function: None,
            current_lambda: None,
            current_suite: None,

            node_pool,
            multiline_stack: Default::default(),
            errors: Default::default(),
            nodes_in_progress: Default::default(),

            can_break: false,
            can_continue: false,

            pending_indents_at_newline: 0,
        }
    }
}

impl Parser<'_> {
    fn version(&self) -> &GDScriptV2Build {
        self.tokenizer.version()
    }

    fn alloc_node<T>(&mut self) -> (NodeRef<T>, InProgressGuard)
    where
        T: DowncastFromNode + Default,
    {
        let node = T::default();
        let span = self.previous().1;
        let r = self.node_pool.push((node, span));

        let mut guard = self.nodes_in_progress.borrow_mut();
        let idx = guard.len();
        guard.push(r.upcast::<Node>());
        drop(guard);

        let guard = InProgressGuard {
            nodes_in_progress: self.nodes_in_progress.clone(),
            idx,
        };

        (r, guard)
    }

    fn alloc_recovery_node<T>(&mut self) -> NodeRef<T>
    where
        T: DowncastFromNode + Default,
    {
        let node = T::default();
        let span = self.previous().1;

        self.node_pool.push((node, span))
    }

    fn alloc_recovery_suite(&mut self) -> NodeRef<SuiteNode> {
        let suite = self.alloc_recovery_node::<SuiteNode>();

        self.node_pool[suite].parent_block = self.current_suite;
        self.node_pool[suite].parent_function = self.current_function;
        self.node_pool[suite].is_in_loop = self.node_pool[self.current_suite.unwrap()].is_in_loop;

        suite
    }

    /// Updates the ending position of a span reference to match the previous token's end.
    fn update_span<T>(&mut self, node: NodeRef<T>)
    where
        T: DowncastFromNode,
    {
        self.node_pool.span_mut(node).end = self.previous().1.end;
    }

    /// Resets a span reference to match an existing span.
    fn reset_span<T>(&mut self, node: NodeRef<T>, to: Span)
    where
        T: DowncastFromNode,
    {
        let span = self.node_pool.span_mut(node);
        *span = to;
    }

    fn consume(&mut self, typ: TokenType, message: impl Into<String>) {
        if self.current.0.typ() == typ {
            self.advance();
        } else {
            // FIXME
            self.push_error(format!("{} {:#?}", message.into(), self.current.0));
        }
    }

    fn push_spanned_error(&mut self, message: impl ToString, span: Option<Span>) {
        let span = span.unwrap_or_else(|| self.previous().1);
        let error = ParseError {
            message: message.to_string(),
            position: span,
        };
        self.errors.push(error);
    }

    fn push_error(&mut self, message: impl ToString) {
        self.push_spanned_error(message, None);
    }

    /// Advances to the next token without updating the previous token.
    fn advance_inner(&mut self) -> (Token, Span) {
        if matches!(self.current.0, Token::Eof) {
            panic!("trying to advance past end of stream");
        }

        let Some(next_token) = self.tokenizer.next() else {
            panic!("tokenizer returned None without returning EOF")
        };

        self.consumed_tokens.push(next_token.clone());

        if matches!(
            next_token.0,
            Token::Newline {
                continuation: false
            }
        ) {
            while self.pending_indents_at_newline < 0 {
                self.consumed_tokens.push((Token::Dedent, Span::zero()));
                self.pending_indents_at_newline += 1;
            }

            while self.pending_indents_at_newline > 0 {
                self.consumed_tokens.push((Token::Indent, Span::zero()));
                self.pending_indents_at_newline -= 1;
            }
        }

        next_token
    }

    fn advance(&mut self) {
        self.lambda_ended = false;

        let next_token = self.advance_inner();
        let previous = mem::replace(&mut self.current, next_token);
        self.previous = Some(previous);

        loop {
            if let (Token::Error(message), span) = &self.current {
                self.push_spanned_error(message.clone(), Some(*span));
            } else if matches!(self.current.0, Token::Newline { continuation: true }) {
                // Skip line continuation tokens since they don't exist in Godot's token stream.
            } else {
                break;
            }

            self.current = self.advance_inner();
        }
    }

    fn previous(&self) -> &Spanned<Token> {
        let Some(previous) = &self.previous else {
            panic!("tried to get previous token before first `advance` call")
        };

        previous
    }

    fn push_multiline(&mut self, state: bool) {
        self.multiline_stack.push_back(state);
        self.tokenizer.set_multiline_mode(state);
        if state {
            // Consume potential whitespace tokens already waiting in line.
            while matches!(
                self.current.0,
                Token::Newline { .. } | Token::Indent | Token::Dedent
            ) {
                self.consumed_tokens.pop();
                self.advance();
            }
        }
    }

    fn pop_multiline(&mut self) {
        assert!(
            !self.multiline_stack.is_empty(),
            "trying to pop from multiline stack without available value"
        );

        self.multiline_stack.pop_back();
        let state = self.multiline_stack.back().copied().unwrap_or(false);
        self.tokenizer.set_multiline_mode(state);
    }

    fn is_statement_end_token(&self) -> bool {
        matches!(
            self.current.0,
            Token::Newline { .. } | Token::Semicolon | Token::Eof
        )
    }

    fn is_statement_end(&self) -> bool {
        self.lambda_ended || self.in_lambda || self.is_statement_end_token()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.current.0, Token::Eof)
    }

    /// Helper method that acts as [`Variant`]'s implicit conversion to [`StringName`].
    fn previous_as_string(&self) -> Option<StringName> {
        let token = self.previous.as_ref()?;

        let variant = match &token.0 {
            Token::Literal(literal) => literal,
            _ => return None,
        };

        Some(match variant {
            Variant::String(s) => StringName(s.clone()),
            Variant::StringName(s) => s.clone(),
            _ => return None,
        })
    }

    pub fn parse(&mut self) {
        // Avoid error or newline as the first token.
        // The latter can mess with the parser when opening files filled exclusively with comments and newlines.
        while matches!(self.current.0, Token::Error(_) | Token::Newline { .. }) {
            if let (Token::Error(error), span) = &self.current {
                self.push_spanned_error(error.to_string(), Some(*span));
            }

            self.advance();
        }

        self.push_multiline(false); // Keep one for the whole parsing.
        self.parse_program();
        self.pop_multiline();

        assert!(
            self.multiline_stack.is_empty(),
            "imbalanced multiline stack"
        );
    }

    fn end_statement(&mut self, context: &str) {
        let mut found = false;
        while self.is_statement_end() && !self.is_at_end() {
            // Remove sequential newlines/semicolons.
            if self.is_statement_end_token() {
                // Only consume if this is an actual token.
                self.advance();
            } else if self.lambda_ended {
                self.lambda_ended = false; // Consume this "token".
                found = true;
                break;
            } else {
                if !found {
                    self.lambda_ended = true; // Mark the lambda as done since we found something else to end the statement.
                    found = true;
                }
                break;
            }

            found = true;
        }

        if !found && !self.is_at_end() {
            self.push_error(format!(
                r#"Expected end of statement after {}, found "{:?}" instead."#,
                context,
                self.current.0.typ()
            ));
        }
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;

        while !self.is_at_end() {
            if matches!(
                self.previous,
                Some((Token::Newline { .. } | Token::Semicolon, _))
            ) {
                return;
            }

            if matches!(
                self.current.0,
                Token::Class
                    | Token::Func
                    | Token::Static
                    | Token::Var
                    | Token::Const
                    | Token::Signal
                    // | Token::If // Can also be inside expressions.
                    | Token::For
                    | Token::While
                    | Token::Match
                    | Token::Return
                    | Token::Annotation(_)
            ) {
                return;
            }

            self.advance();
        }
    }

    fn parse_program(&mut self) {
        self.head = self.node_pool.push((
            ClassNode::default(),
            Span::new(
                Position { line: 1, column: 0 },
                Position { line: 1, column: 0 },
            ),
        ));
        self.current_class = Some(self.head);

        let mut can_have_class_or_extends = true;

        while !matches!(self.current.0, Token::Eof) {
            if let Token::Annotation(_) = self.current.0 {
                self.advance();
                self.parse_annotation();
            } else if let Token::Literal(Variant::String(_)) = self.current.0 {
                // Allow strings in class body as multiline comments.
                self.advance();
                self.consume(TokenType::Newline, "Expected newline after comment string.");
            } else {
                break;
            }
        }

        if matches!(self.current.0, Token::ClassName | Token::Extends) {
            // Set range of the class to only start at extends or class_name if present.
            self.reset_span(self.head, self.current.1);
        }

        while can_have_class_or_extends {
            // Order here doesn't matter, but there should be only one of each at most.
            match &self.current.0 {
                Token::ClassName => {
                    //: PUSH_PENDING_ANNOTATIONS_TO_HEAD;
                    self.advance();

                    let head = self.node_pool.get(self.head);
                    if head.0.identifier.is_none() {
                        self.parse_class_name();
                    } else {
                        self.push_error(r#""class_name" can only be used once."#);
                    }
                }

                Token::Extends => {
                    //: PUSH_PENDING_ANNOTATIONS_TO_HEAD;
                    self.advance();

                    let head = self.node_pool.get(self.head);
                    if head.0.extends_used {
                        self.push_error(r#""extends" can only be used once."#);
                    } else {
                        self.parse_extends();
                        self.end_statement("superclass");
                    }
                }

                Token::Eof => {
                    //: PUSH_PENDING_ANNOTATIONS_TO_HEAD;
                    can_have_class_or_extends = false;
                }

                Token::Literal(literal) => {
                    if literal.typ() == VariantType::String {
                        // Allow strings in class body as multiline comments.
                        self.advance();
                        self.consume(TokenType::Newline, "Expected newline after comment string.");
                    }

                    // No tokens are allowed between script annotations and class/extends.
                    can_have_class_or_extends = false;
                }

                _ => {
                    // No tokens are allowed between script annotations and class/extends.
                    can_have_class_or_extends = false;
                }
            }
        }

        self.parse_class_body(true);

        self.node_pool.span_mut(self.head).end = self.current.1.end;

        if !matches!(self.current.0, Token::Eof) {
            let mut tokens = vec![self.current.0.clone()];

            for (token, _) in &mut *self.tokenizer {
                tokens.push(token);
            }

            self.push_error(format!(
                "Expected end of file. Remaining tokens: {:?}",
                tokens
            ));
        }

        //: clear_unused_annotations();
    }

    fn parse_class_name(&mut self) {
        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
            let identifier = self.parse_identifier();
            if let Some(current_class) = self.current_class {
                self.node_pool[current_class].identifier = Some(identifier);
            }
        } else {
            self.push_error(r#"Expected identifier for the global class name after "class_name"."#);
        }

        if matches!(self.current.0, Token::Extends) {
            self.advance();
            self.parse_extends();
            self.end_statement("superclass");
        } else {
            self.end_statement("class_name statement");
        }
    }

    fn parse_extends(&mut self) {
        if let Some(current_class) = self.current_class {
            self.node_pool[current_class].extends_used = true;
        }

        if let Token::Literal(literal) = &self.current.0 {
            let literal_type = literal.typ();
            self.advance();

            if literal_type != VariantType::String {
                self.push_error(
                    format!(
                        r#"Only strings or identifiers can be used after "extends", found "{:?}" instead."#,
                        literal_type
                    )
                );
            }

            let extends_path = self.previous_as_string();
            if let Some(current_class) = self.current_class {
                self.node_pool[current_class].extends_path = extends_path;
            }

            if matches!(self.current.0, Token::Period) {
                self.advance();
                return;
            } else {
                return;
            }
        }

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
        } else {
            self.push_error(r#"Expected superclass name after "extends"."#);
            return;
        }

        let extends = self.parse_identifier();
        if let Some(current_class) = self.current_class {
            self.node_pool[current_class].extends.push(extends);
        }

        while matches!(self.current.0, Token::Period) {
            self.advance();

            if matches!(self.current.0, Token::Identifier(_)) {
                self.advance();
            } else {
                self.push_error(r#"Expected superclass name after "."."#);
                return;
            }

            let extends = self.parse_identifier();
            if let Some(current_class) = self.current_class {
                self.node_pool
                    .get_mut(current_class)
                    .0
                    .extends
                    .push(extends);
            }
        }
    }

    fn parse_identifier(&mut self) -> NodeRef<IdentifierNode> {
        assert!(
            self.previous().0.is_identifier(),
            "parsing identifier node without identifier token"
        );

        let name = self.previous().0.get_identifier().to_string();

        let (identifier, guard) = self.alloc_node::<IdentifierNode>();
        drop(guard);
        self.node_pool[identifier].name = StringName(name.into());
        if self.node_pool[identifier].name.0.is_empty() {
            // XXX: This prints "Empty identifier found."? wtf?
        }

        /*:
        identifier->suite = current_suite;

        if (current_suite != None && current_suite->has_local(identifier->name)) {
            const SuiteNode::Local &declaration = current_suite->get_local(identifier->name);

            identifier->source_function = declaration.source_function;
            switch (declaration.type) {
                case SuiteNode::Local::CONSTANT:
                    identifier->source = IdentifierNode::LOCAL_CONSTANT;
                    identifier->constant_source = declaration.constant;
                    declaration.constant->usages++;
                    break;
                case SuiteNode::Local::VARIABLE:
                    identifier->source = IdentifierNode::LOCAL_VARIABLE;
                    identifier->variable_source = declaration.variable;
                    declaration.variable->usages++;
                    break;
                case SuiteNode::Local::PARAMETER:
                    identifier->source = IdentifierNode::FUNCTION_PARAMETER;
                    identifier->parameter_source = declaration.parameter;
                    declaration.parameter->usages++;
                    break;
                case SuiteNode::Local::FOR_VARIABLE:
                    identifier->source = IdentifierNode::LOCAL_ITERATOR;
                    identifier->bind_source = declaration.bind;
                    declaration.bind->usages++;
                    break;
                case SuiteNode::Local::PATTERN_BIND:
                    identifier->source = IdentifierNode::LOCAL_BIND;
                    identifier->bind_source = declaration.bind;
                    declaration.bind->usages++;
                    break;
                case SuiteNode::Local::UNDEFINED:
                    ERR_FAIL_V_MSG(None, "Undefined local found.");
            }
        }
        */

        identifier
    }

    fn parse_class_body(&mut self, is_multiline: bool) {
        let mut class_end = false;
        let mut next_is_static = false;

        while !class_end && !matches!(self.current.0, Token::Eof) {
            let token_was_static = matches!(self.current.0, Token::Static);

            match &self.current.0 {
                Token::Var => {
                    self.parse_class_member(
                        |p, is_static| p.parse_variable(is_static, true),
                        "variable",
                        next_is_static,
                    );

                    if next_is_static && let Some(current_class) = self.current_class {
                        self.node_pool[current_class].has_static_data = true;
                    }
                }
                Token::Const => {
                    self.parse_class_member(|p, _| p.parse_constant(), "constant", false);
                }
                Token::Signal => self.parse_class_member(|p, _| p.parse_signal(), "signal", false),
                Token::Func => {
                    self.parse_class_member(
                        |p, is_static| p.parse_function(is_static),
                        "function",
                        next_is_static,
                    );
                }
                Token::Class => {
                    self.parse_class_member(|p, _| p.parse_class(), "class", false);
                }
                Token::Enum => {
                    self.parse_class_member(|p, _| p.parse_enum(), "enum", false);
                }
                Token::Static if self.version().has_static_variables => {
                    self.advance();
                    next_is_static = true;

                    if !matches!(self.current.0, Token::Func | Token::Var) {
                        self.push_error(r#"Expected "func" or "var" after "static"."#);
                    }
                }
                Token::Annotation(_) => {
                    self.advance();
                    self.parse_annotation();
                }
                Token::Pass => {
                    self.advance();
                    self.end_statement("pass");
                }
                Token::Dedent => {
                    class_end = true;
                }
                Token::Literal(Variant::String(_)) => {
                    self.advance();
                    self.consume(TokenType::Newline, "Expected newline after comment string.");
                }
                _ => {
                    self.advance();
                    // TODO: proper formatting
                    self.push_error(format!("Unexpected {:?} in class body.", self.previous().0));
                }
            }

            if !token_was_static {
                next_is_static = false;
            }

            if self.panic_mode {
                self.synchronize();
            }

            if !is_multiline {
                class_end = true;
            }
        }
    }

    fn parse_class_member<F, T>(
        &mut self,
        parse_function: F,
        _member_kind: &'static str,
        is_static: bool,
    ) where
        F: FnOnce(&mut Self, bool) -> Option<NodeRef<T>>,
    {
        self.advance();

        /*:
        // Consume annotations.
        List<AnnotationNode *> annotations;
        while (!annotation_stack.is_empty()) {
            AnnotationNode *last_annotation = annotation_stack.back()->get();
            if (last_annotation->applies_to(p_target)) {
                annotations.push_front(last_annotation);
                annotation_stack.pop_back();
            } else {
                push_error(vformat(R"(Annotation "%s" cannot be applied to a %s.)", last_annotation->name, p_member_kind));
                clear_unused_annotations();
            }
        }
        */

        let Some(_member) = parse_function(self, is_static) else {
            return;
        };

        /*:
        for (AnnotationNode *&annotation : annotations) {
            member->annotations.push_back(annotation);
        }
        if (member->identifier != None) {
            if (!((String)member->identifier->name).is_empty()) { // Enums may be unnamed.
                if (current_class->members_indices.has(member->identifier->name)) {
                    push_error(vformat(R"(%s "%s" has the same name as a previously declared %s.)", p_member_kind.capitalize(), member->identifier->name, current_class->get_member(member->identifier->name).get_type_name()), member->identifier);
                } else {
                    current_class->add_member(member);
                }
            } else {
                current_class->add_member(member);
            }
        }
        */
    }

    fn parse_variable(
        &mut self,
        is_static: bool,
        allow_property: bool,
    ) -> Option<NodeRef<VariableNode>> {
        let (variable, guard) = self.alloc_node::<VariableNode>();

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
        } else {
            self.push_error(r#"Expected variable name after "var"."#);
            self.node_pool.try_free(variable);
            return None;
        }

        let identifier = self.parse_identifier();
        self.node_pool[variable].base.identifier = Some(identifier);
        //: variable->export_info.name = variable->identifier->name;
        self.node_pool[variable].is_static = is_static;

        if matches!(self.current.0, Token::Colon) {
            self.advance();

            if matches!(self.current.0, Token::Newline { .. }) {
                if allow_property {
                    self.advance();
                    return self.parse_property(variable, guard, true);
                } else {
                    self.push_error(r#"Expected type after \":\"."#);
                    return None;
                }
            } else if matches!(self.current.0, Token::Equal) {
                // Infer type.
                self.node_pool[variable].base.infer_datatype = true;
            } else {
                if allow_property && let Token::Identifier(identifier) = &self.current.0 {
                    // Check if get or set.
                    if identifier == "get" || identifier == "set" {
                        return self.parse_property(variable, guard, false);
                    }
                }

                // Parse type.
                let r#type = self.parse_type(false);
                self.node_pool[variable].base.datatype_specifier = r#type;
            }
        }

        if matches!(self.current.0, Token::Equal) {
            self.advance();

            // Initializer.
            let initializer = self.parse_expression(false, false);
            self.node_pool[variable].base.initializer = initializer;

            if self.node_pool[variable].base.initializer.is_none() {
                self.push_error(r#"Expected expression for variable initial value after "="."#);
            }

            //: variable->assignments++;
        }

        if allow_property && matches!(self.current.0, Token::Colon) {
            self.advance();

            if matches!(self.current.0, Token::Newline { .. }) {
                self.advance();
                return self.parse_property(variable, guard, true);
            } else {
                return self.parse_property(variable, guard, false);
            }
        }

        drop(guard);
        self.end_statement("variable declaration");

        Some(variable)
    }

    fn parse_constant(&mut self) -> Option<NodeRef<ConstantNode>> {
        let (constant, guard) = self.alloc_node::<ConstantNode>();

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
        } else {
            self.push_error(r#"Expected constant name after "const"."#);
            self.node_pool.try_free(constant);
            return None;
        }

        let identifier = self.parse_identifier();
        self.node_pool[constant].0.identifier = Some(identifier);

        if matches!(self.current.0, Token::Colon) {
            self.advance();

            if matches!(self.current.0, Token::Equal) {
                // Infer type.
                self.node_pool[constant].0.infer_datatype = true;
            } else {
                // Parse type.
                let r#type = self.parse_type(false);
                self.node_pool[constant].0.datatype_specifier = r#type;
            }
        }

        if matches!(self.current.0, Token::Equal) {
            self.advance();

            // Initializer.
            let initializer = self.parse_expression(false, false);
            self.node_pool[constant].0.initializer = initializer;

            if self.node_pool[constant].0.initializer.is_none() {
                self.push_error("Expected initializer expression for constant.");
                return None;
            }
        } else {
            self.push_error("Expected initializer after constant name.");
            return None;
        }

        drop(guard);
        self.end_statement("constant declaration");

        Some(constant)
    }

    fn parse_signal(&mut self) -> Option<NodeRef<SignalNode>> {
        let (signal, guard) = self.alloc_node::<SignalNode>();

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
        } else {
            self.push_error(r#"Expected signal name after "signal"."#);
            self.node_pool.try_free(signal);
            return None;
        }

        let identifier = self.parse_identifier();
        self.node_pool[signal].identifier = Some(identifier);

        if matches!(self.current.0, Token::ParenthesisOpen) {
            self.push_multiline(true);
            self.advance();

            do_while!({
                if matches!(self.current.0, Token::ParenthesisClose) {
                    // Allow for trailing comma.
                    break;
                }

                let Some(parameter) = self.parse_parameter() else {
                    self.push_error("Expected signal parameter name.");
                    break;
                };

                if self.node_pool[parameter].0.initializer.is_some() {
                    self.push_error("Signal parameters cannot have a default value.");
                }

                let parameter_identifier = self.node_pool[parameter].0.identifier.unwrap();
                let identifier_name = self.node_pool[parameter_identifier].name.clone();
                if self.node_pool[signal]
                    .parameters
                    .contains_key(&identifier_name)
                {
                    self.push_error(format!(
                        r#"Parameter with name "{}" was already declared for this signal."#,
                        identifier_name.0
                    ));
                } else {
                    self.node_pool[signal]
                        .parameters
                        .insert(identifier_name, parameter);
                }
            } while {
                if matches!(self.current.0, Token::Comma) {
                    self.advance();
                    !self.is_at_end()
                } else {
                    false
                }
            });

            self.pop_multiline();
            self.consume(
                TokenType::ParenthesisClose,
                r#"(Expected closing ")" after signal parameters."#,
            );
        }

        drop(guard);
        self.end_statement("signal declaration");

        Some(signal)
    }

    fn parse_function(&mut self, is_static: bool) -> Option<NodeRef<FunctionNode>> {
        let (function, _guard) = self.alloc_node::<FunctionNode>();

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
        } else {
            self.push_error(r#"Expected function name after "func"."#);
            self.node_pool.try_free(function);
            return None;
        }

        self.node_pool[function].is_static = is_static;

        let previous_function = self.current_function;
        self.current_function = Some(function);

        let identifier = self.parse_identifier();
        self.node_pool[function].identifier = Some(identifier);

        let (body, body_guard) = self.alloc_node::<SuiteNode>();

        let previous_suite = self.current_suite;
        self.current_suite = Some(body);

        self.push_multiline(true);
        self.consume(
            TokenType::ParenthesisOpen,
            r#"Expected opening "(" after function name."#,
        );

        let has_body = self.parse_function_signature(function, body, "function");

        self.current_suite = previous_suite;

        if !has_body {
            // Abstract functions do not have a body.
            self.end_statement("bodyless function declaration");
            self.reset_span(body, self.current.1);
            drop(body_guard);
            self.node_pool[function].body_suite = Some(body);
        } else {
            self.parse_suite("function declaration", body, body_guard, false);
            self.node_pool[function].body_suite = Some(body);
        }

        self.current_function = previous_function;

        Some(function)
    }

    fn parse_function_signature(
        &mut self,
        function: NodeRef<FunctionNode>,
        _body: NodeRef<SuiteNode>,
        r#type: &str,
    ) -> bool {
        if !matches!(self.current.0, Token::ParenthesisClose) && !self.is_at_end() {
            let mut default_used = false;

            do_while!({
                if matches!(self.current.0, Token::ParenthesisClose) {
                    // Allow for trailing comma.
                    break;
                }

                let mut is_rest = false;

                if self.version().has_variadic_functions
                    && matches!(self.current.0, Token::PeriodPeriodPeriod)
                {
                    self.advance();
                    is_rest = true;
                }

                let Some(parameter) = self.parse_parameter() else {
                    break;
                };

                if self.version().has_variadic_functions
                    && self.node_pool[function].rest_parameter.is_some()
                {
                    self.push_error("Cannot have parameters after the rest parameter.");
                    continue;
                }

                if self.node_pool[parameter].0.initializer.is_some() {
                    if is_rest {
                        self.push_error("The rest parameter cannot have a default value.");
                        continue;
                    }

                    default_used = true;
                } else {
                    if default_used && !is_rest {
                        self.push_error(
                            "Cannot have mandatory parameters after optional parameters.",
                        );
                        continue;
                    }
                }

                let identifier = self.node_pool[parameter].0.identifier.unwrap();
                let identifier_name = self.node_pool[identifier].name.clone();
                if self.node_pool[function]
                    .parameters
                    .contains_key(&identifier_name)
                {
                    self.push_error(format!(
                        r#"Parameter with name "{}" was already declared for this {}."#,
                        identifier_name.0, r#type
                    ));
                } else if is_rest {
                    self.node_pool[function].rest_parameter = Some(parameter);
                    //: p_body->add_local(parameter, current_function);
                } else {
                    self.node_pool[function]
                        .parameters
                        .insert(identifier_name, parameter);
                    //: p_body->add_local(parameter, current_function);
                }
            } while {
                if matches!(self.current.0, Token::Comma) {
                    self.advance();
                    true
                } else {
                    false
                }
            });
        }

        self.pop_multiline();
        if matches!(self.current.0, Token::ParenthesisClose) {
            self.advance();
        } else {
            self.push_error(format!(
                r#"Expected closing ")" after {} parameters."#,
                r#type
            ));
        }

        if matches!(self.current.0, Token::ForwardArrow) {
            self.advance();

            let r#type = self.parse_type(true);
            self.node_pool[function].return_type = r#type;
            if self.node_pool[function].return_type.is_none() {
                self.push_error(r#"Expected return type or "void" after "->"."#);
            }
        }

        /*:
        if (!p_function->source_lambda && p_function->identifier && p_function->identifier->name == GDScriptLanguage::get_singleton()->strings._static_init) {
            if (!p_function->is_static) {
                push_error(R"(Static constructor must be declared static.)");
            }
            if (!p_function->parameters.is_empty() || p_function->is_vararg()) {
                push_error(R"(Static constructor cannot have parameters.)");
            }
        }
        */

        if matches!(self.current.0, Token::Colon) {
            self.advance();
            true
        } else {
            if r#type == "lambda" {
                self.push_error(r#"Expected ":" after lambda declaration."#);
            }

            false
        }
    }

    fn parse_suite(
        &mut self,
        context: &str,
        suite: NodeRef<SuiteNode>,
        guard: InProgressGuard,
        for_lambda: bool,
    ) {
        self.node_pool[suite].parent_block = self.current_suite;
        self.node_pool[suite].parent_function = self.current_function;
        self.current_suite = Some(suite);

        if self.version().has_77744_suite_changes
            && !for_lambda
            && let Some(block) = self.node_pool[suite].parent_block
            && self.node_pool[block].is_in_loop
        {
            // Do not reset to false if true is set before calling parse_suite().
            self.node_pool[suite].is_in_loop = true;
        }

        let mut multiline = false;
        if matches!(self.current.0, Token::Newline { .. }) {
            self.advance();
            multiline = true;
        }

        if multiline {
            if matches!(self.current.0, Token::Indent) {
                self.advance();
            } else {
                self.push_error(format!(r#"Expected indented block after {}."#, context));

                self.current_suite = self.node_pool[suite].parent_block;
                return;
            }
        }
        self.reset_span(suite, self.current.1);

        let mut error_count = 0;
        do_while!({
            if self.is_at_end()
                || (!multiline
                    && self.previous().0.typ() == TokenType::Semicolon
                    && self.current.0.typ() == TokenType::Newline)
            {
                break;
            }

            let Some(statement) = self.parse_statement() else {
                error_count += 1;
                if error_count > 100 {
                    self.push_error("Too many statement errors.");
                    break;
                }
                continue;
            };

            self.node_pool[suite].statements.push(statement);

            /*:
            // Register locals.
            switch (statement->type) {
                case Node::VARIABLE: {
                    VariableNode *variable = static_cast<VariableNode *>(statement);
                    const SuiteNode::Local &local = current_suite->get_local(variable->identifier->name);
                    if (local.type != SuiteNode::Local::UNDEFINED) {
                        push_error(vformat(R"(There is already a %s named "%s" declared in this scope.)", local.get_name(), variable->identifier->name), variable->identifier);
                    }
                    current_suite->add_local(variable, current_function);
                    break;
                }
                case Node::CONSTANT: {
                    ConstantNode *constant = static_cast<ConstantNode *>(statement);
                    const SuiteNode::Local &local = current_suite->get_local(constant->identifier->name);
                    if (local.type != SuiteNode::Local::UNDEFINED) {
                        String name;
                        if (local.type == SuiteNode::Local::CONSTANT) {
                            name = "constant";
                        } else {
                            name = "variable";
                        }
                        push_error(vformat(R"(There is already a %s named "%s" declared in this scope.)", name, constant->identifier->name), constant->identifier);
                    }
                    current_suite->add_local(constant, current_function);
                    break;
                }
                default:
                    break;
            }
            */
        } while {
            (multiline || self.previous().0.typ() == TokenType::Semicolon)
                && self.current.0.typ() != TokenType::Dedent
                && !self.lambda_ended
                && !self.is_at_end()
        });

        drop(guard);

        if multiline {
            if !self.lambda_ended {
                if matches!(self.current.0, Token::Dedent) {
                    self.advance();
                } else {
                    self.push_error(format!(r#"Missing unindent at the end of {}."#, context));
                }
            } else {
                if matches!(self.current.0, Token::Dedent) {
                    self.advance();
                }
            }
        } else if self.previous().0.typ() == TokenType::Semicolon {
            if matches!(self.current.0, Token::Newline { .. }) {
                self.advance();
            } else {
                self.push_error(format!(
                    r#"Expected newline after ";" at the end of {}."#,
                    context
                ));
            }
        }

        if for_lambda {
            self.lambda_ended = true;
        }

        self.current_suite = self.node_pool[suite].parent_block;
    }

    fn parse_statement(&mut self) -> Option<NodeRef<Node>> {
        /*:
        List<AnnotationNode *> annotations;
        if (current.type != GDScriptTokenizer::Token::ANNOTATION) {
            while (!annotation_stack.is_empty()) {
                AnnotationNode *last_annotation = annotation_stack.back()->get();
                if (last_annotation->applies_to(AnnotationInfo::STATEMENT)) {
                    annotations.push_front(last_annotation);
                    annotation_stack.pop_back();
                } else {
                    push_error(vformat(R"(Annotation "%s" cannot be applied to a statement.)", last_annotation->name));
                    clear_unused_annotations();
                }
            }
        }
        */

        let result: Option<NodeRef<Node>> = match &self.current.0 {
            Token::Pass => {
                self.advance();
                let (node, guard) = self.alloc_node::<PassNode>();
                drop(guard);
                self.end_statement(r#""pass""#);
                Some(node.upcast())
            }

            Token::Var => {
                self.advance();
                self.parse_variable(false, false).map(|n| n.upcast())
            }

            Token::Const => {
                self.advance();
                self.parse_constant().map(|n| n.upcast())
            }

            Token::If => {
                self.advance();
                Some(self.parse_if("if").upcast())
            }

            Token::For => {
                self.advance();
                Some(self.parse_for().upcast())
            }

            Token::While => {
                self.advance();
                Some(self.parse_while().upcast())
            }

            Token::Match => {
                self.advance();
                Some(self.parse_match().upcast())
            }

            Token::Break => {
                self.advance();
                Some(self.parse_break().upcast())
            }

            Token::Continue => {
                self.advance();
                Some(self.parse_continue().upcast())
            }

            Token::Return => {
                self.advance();

                let (r#return, guard) = self.alloc_node::<ReturnNode>();

                if !self.is_statement_end() {
                    /*:
                    if (current_function && (current_function->identifier->name == GDScriptLanguage::get_singleton()->strings._init || current_function->identifier->name == GDScriptLanguage::get_singleton()->strings._static_init)) {
                        push_error(R"(Constructor cannot return a value.)");
                    }
                    */
                    self.node_pool[r#return].value = self.parse_expression(false, false);
                } else if self.in_lambda && !self.is_statement_end_token() {
                    // Try to parse it anyway as this might not be the statement end in a lambda.
                    // If this fails the expression will be nullptr, but that's the same as no return, so it's fine.
                    let return_value = self.parse_expression(false, false);
                    self.node_pool[r#return].value = return_value;
                }

                drop(guard);

                if let Some(current_suite) = self.current_suite {
                    self.node_pool[current_suite].has_return = true;
                }
                self.end_statement("return statement");

                Some(r#return.upcast())
            }

            Token::Breakpoint => {
                self.advance();
                let (node, guard) = self.alloc_node::<BreakpointNode>();
                drop(guard);
                self.end_statement(r#""breakpoint""#);
                Some(node.upcast())
            }

            Token::Assert => {
                self.advance();
                self.parse_assert().map(|n| n.upcast())
            }

            Token::Annotation(_) => {
                self.advance();
                let _annotation = self.parse_annotation();

                /*:
                if (annotation != nullptr) {
                    if (annotation->applies_to(AnnotationInfo::STANDALONE)) {
                        if (previous.type != GDScriptTokenizer::Token::NEWLINE) {
                            push_error(R"(Expected newline after a standalone annotation.)");
                        }
                        if (annotation->name == SNAME("@warning_ignore_start") || annotation->name == SNAME("@warning_ignore_restore")) {
                            // Some annotations need to be resolved and applied in the parser.
                            annotation->apply(this, nullptr, nullptr);
                        } else {
                            push_error(R"(Unexpected standalone annotation.)");
                        }
                    } else {
                        annotation_stack.push_back(annotation);
                    }
                }
                break;
                */

                None
            }

            _ => {
                // Expression statement.
                let expression = self.parse_expression(true, false); // Allow assignment here.

                let mut has_ended_lambda = false;
                if expression.is_none() {
                    if self.in_lambda {
                        // If it's not a valid expression beginning, it might be the continuation of the outer expression where this lambda is.
                        self.lambda_ended = true;
                        has_ended_lambda = true;
                    } else {
                        self.advance();

                        // TODO: proper formatting
                        self.push_error(format!(
                            r#"Expected statement, found "{:?}" instead."#,
                            self.previous().0
                        ));
                    }
                } else {
                    self.end_statement("expression");
                }

                self.lambda_ended = self.lambda_ended || has_ended_lambda;
                expression.map(|n| n.upcast())
            }
        };

        /*:
        if (result != nullptr && !annotations.is_empty()) {
            for (AnnotationNode *&annotation : annotations) {
                result->annotations.push_back(annotation);
            }
        }
        */

        if self.panic_mode {
            self.synchronize();
        }

        result
    }

    fn parse_if(&mut self, token: &str) -> NodeRef<IfNode> {
        let (r#if, _guard) = self.alloc_node::<IfNode>();

        let condition = self.parse_expression(false, false);
        self.node_pool[r#if].condition = condition;
        if self.node_pool[r#if].condition.is_none() {
            self.push_error(format!(
                r#"Expected conditional expression after "{}"."#,
                token
            ));
        }

        self.consume(
            TokenType::Colon,
            format!(r#"Expected ":" after "{}" condition."#, token),
        );

        let (true_block, true_block_guard) = self.alloc_node::<SuiteNode>();
        self.parse_suite(
            format!(r#""{}" block"#, token).as_str(),
            true_block,
            true_block_guard,
            false,
        );
        self.node_pool[r#if].true_block = Some(true_block);
        self.node_pool[true_block].parent_if = Some(r#if);

        if self.node_pool[true_block].has_continue
            && let Some(current_suite) = self.current_suite
        {
            self.node_pool[current_suite].has_continue = true;
        }

        if matches!(self.current.0, Token::Elif) {
            self.advance();

            let (else_block, _else_block_guard) = self.alloc_node::<SuiteNode>();

            self.node_pool[else_block].parent_function = self.current_function;
            self.node_pool[else_block].parent_block = self.current_suite;

            let previous_suite = self.current_suite;
            self.current_suite = Some(else_block);

            let elif = self.parse_if("elif");
            self.node_pool[else_block].statements.push(elif.upcast());
            self.node_pool[r#if].false_block = Some(else_block);

            self.current_suite = previous_suite;
        } else if matches!(self.current.0, Token::Else) {
            self.advance();

            self.consume(TokenType::Colon, r#"Expected ":" after "else"."#);

            let (false_block, false_block_guard) = self.alloc_node::<SuiteNode>();
            self.parse_suite(r#""else" block"#, false_block, false_block_guard, false);
            self.node_pool[r#if].false_block = Some(false_block);
        }

        if let Some(true_block) = self.node_pool[r#if].true_block
            && let Some(false_block) = self.node_pool[r#if].false_block
            && self.node_pool[false_block].has_return
            && self.node_pool[true_block].has_return
            && let Some(current_suite) = self.current_suite
        {
            self.node_pool[current_suite].has_return = true;
        }

        if let Some(false_block) = self.node_pool[r#if].false_block
            && self.node_pool[false_block].has_continue
            && let Some(current_suite) = self.current_suite
        {
            self.node_pool[current_suite].has_continue = true;
        }

        r#if
    }

    fn parse_for(&mut self) -> NodeRef<ForNode> {
        let (r#for, _guard) = self.alloc_node::<ForNode>();

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
            let variable = self.parse_identifier();
            self.node_pool[r#for].variable = Some(variable);
        } else {
            self.push_error(r#"Expected loop variable name after "for"."#);
        }

        if self.version().has_typed_for_loops && matches!(self.current.0, Token::Colon) {
            self.advance();

            let datatype_specifier = self.parse_type(false);
            self.node_pool[r#for].datatype_specifier = datatype_specifier;
            if self.node_pool[r#for].datatype_specifier.is_none() {
                self.push_error(r#"Expected type specifier after ":"."#);
            }
        }

        if self.version().has_typed_for_loops && self.node_pool[r#for].datatype_specifier.is_none()
        {
            self.consume(
                TokenType::In,
                r#"Expected "in" or ":" after "for" variable name."#,
            );
        } else {
            self.consume(
                TokenType::In,
                r#"Expected "in" after "for" variable type specifier."#,
            );
        }

        let list = self.parse_expression(false, false);
        self.node_pool[r#for].list = list;

        if self.node_pool[r#for].list.is_none() {
            self.push_error(r#"Expected iterable after "in"."#);
        }

        self.consume(TokenType::Colon, r#"Expected ":" after "for" condition."#);

        // Save break/continue state.
        let could_break = self.can_break;
        let could_continue = self.can_continue;

        // Allow break/continue.
        self.can_break = true;
        self.can_continue = true;

        let (suite, suite_guard) = self.alloc_node::<SuiteNode>();

        /*:
        if (n_for->variable) {
            const SuiteNode::Local &local = current_suite->get_local(n_for->variable->name);
            if (local.type != SuiteNode::Local::UNDEFINED) {
                push_error(vformat(R"(There is already a %s named "%s" declared in this scope.)", local.get_name(), n_for->variable->name), n_for->variable);
            }
            suite->add_local(SuiteNode::Local(n_for->variable, current_function));
        }
        */

        if self.version().has_77744_suite_changes {
            self.node_pool[suite].is_in_loop = true;
        }
        self.parse_suite(r#""for" block"#, suite, suite_guard, false);
        if !self.version().has_77744_suite_changes {
            self.node_pool[suite].is_in_loop = true;
        }
        self.node_pool[r#for].loop_suite = Some(suite);

        // Reset break/continue state.
        self.can_break = could_break;
        self.can_continue = could_continue;

        r#for
    }

    fn parse_while(&mut self) -> NodeRef<WhileNode> {
        let (r#while, _guard) = self.alloc_node::<WhileNode>();

        let condition = self.parse_expression(false, false);
        self.node_pool[r#while].condition = condition;
        if self.node_pool[r#while].condition.is_none() {
            self.push_error(r#"Expected conditional expression after "while"."#);
        }

        self.consume(TokenType::Colon, r#"Expected ":" after "while" condition."#);

        // Save break/continue state.
        let could_break = self.can_break;
        let could_continue = self.can_continue;

        // Allow break/continue.
        self.can_break = true;
        self.can_continue = true;

        let (suite, suite_guard) = self.alloc_node::<SuiteNode>();
        if self.version().has_77744_suite_changes {
            self.node_pool[suite].is_in_loop = true;
        }
        self.parse_suite(r#""while" block"#, suite, suite_guard, false);
        if !self.version().has_77744_suite_changes {
            self.node_pool[suite].is_in_loop = true;
        }
        self.node_pool[r#while].loop_suite = Some(suite);

        // Reset break/continue state.
        self.can_break = could_break;
        self.can_continue = could_continue;

        r#while
    }

    fn parse_match(&mut self) -> NodeRef<MatchNode> {
        let (r#match, match_guard) = self.alloc_node::<MatchNode>();

        let test = self.parse_expression(false, false);
        self.node_pool[r#match].test = test;
        if self.node_pool[r#match].test.is_none() {
            self.push_error(r#"Expected expression to test after "match"."#);
        }

        self.consume(
            TokenType::Colon,
            r#"Expected ":" after "match" expression."#,
        );
        self.consume(
            TokenType::Newline,
            r#"Expected a newline after "match" statement."#,
        );

        if matches!(self.current.0, Token::Indent) {
            self.advance();
        } else {
            self.push_error(r#"Expected an indented block after "match" statement."#);
            return r#match;
        }

        let mut all_have_return = true;
        let mut have_wildcard = false;

        //: List<AnnotationNode *> match_branch_annotation_stack;

        while !matches!(self.current.0, Token::Dedent) && !self.is_at_end() {
            if matches!(self.current.0, Token::Pass) {
                self.advance();
                self.consume(TokenType::Newline, r#"Expected newline after "pass"."#);
                continue;
            }

            if matches!(self.current.0, Token::Annotation(_)) {
                self.advance();
                let annotation = self.parse_annotation();
                if annotation.is_none() {
                    continue;
                }

                /*:
                if (annotation->name != SNAME("@warning_ignore")) {
                    push_error(vformat(R"(Annotation "%s" is not allowed in this level.)", annotation->name), annotation);
                    continue;
                }
                match_branch_annotation_stack.push_back(annotation);
                */

                continue;
            }

            let Some(branch) = self.parse_match_branch() else {
                self.advance();
                continue;
            };

            /*:
            for (AnnotationNode *annotation : match_branch_annotation_stack) {
                branch->annotations.push_back(annotation);
            }
            match_branch_annotation_stack.clear();
            */

            have_wildcard = have_wildcard || self.node_pool[branch].has_wildcard;
            all_have_return =
                all_have_return && self.node_pool[self.node_pool[branch].block.unwrap()].has_return;

            self.node_pool[r#match].branches.push(branch);
        }
        drop(match_guard);

        self.consume(
            TokenType::Dedent,
            r#"Expected an indented block after "match" statement."#,
        );
        if all_have_return
            && have_wildcard
            && let Some(current_suite) = self.current_suite
        {
            self.node_pool[current_suite].has_return = true;
        }
        /*:
        for (const AnnotationNode *annotation : match_branch_annotation_stack) {
            push_error(vformat(R"(Annotation "%s" does not precede a valid target, so it will have no effect.)", annotation->name), annotation);
        }
        match_branch_annotation_stack.clear();
        */

        r#match
    }

    fn parse_match_branch(&mut self) -> Option<NodeRef<MatchBranchNode>> {
        let (branch, branch_guard) = self.alloc_node::<MatchBranchNode>();
        self.reset_span(branch, self.current.1);

        let mut has_bind = false;

        do_while!({
            let Some(pattern) = self.parse_match_pattern(None) else {
                continue;
            };

            if !self.node_pool[pattern].binds.is_empty() {
                has_bind = true;
            }

            if !self.node_pool[branch].patterns.is_empty() && has_bind {
                self.push_error("Cannot use a variable bind with multiple patterns.");
            }

            if self.node_pool[pattern].pattern_type == Some(PatternType::Rest) {
                self.push_error(
                    r#"Rest pattern can only be used inside array and dictionary patterns."#,
                );
            } else if let Some(PatternType::Bind(_)) = self.node_pool[pattern].pattern_type {
                self.node_pool[branch].has_wildcard = true;
            } else if self.node_pool[pattern].pattern_type == Some(PatternType::Wildcard) {
                self.node_pool[branch].has_wildcard = true;
            }

            self.node_pool[branch].patterns.push(pattern);
        } while {
            if matches!(self.current.0, Token::Comma) {
                self.advance();
                true
            } else {
                false
            }
        });

        if self.node_pool[branch].patterns.is_empty() {
            self.push_error(r#"No pattern found for "match" branch."#);
        }

        let mut has_guard = false;

        if self.version().has_when && matches!(self.current.0, Token::When) {
            self.advance();

            // Pattern guard.
            // Create block for guard because it also needs to access the bound variables from patterns, and we don't want to add them to the outer scope.
            let (guard_body, _guard) = self.alloc_node::<SuiteNode>();
            self.node_pool[branch].guard_body = Some(guard_body);

            /*:
            if (branch->patterns.size() > 0) {
                for (const KeyValue<StringName, IdentifierNode *> &E : branch->patterns[0]->binds) {
                    SuiteNode::Local local(E.value, current_function);
                    local.type = SuiteNode::Local::PATTERN_BIND;
                    branch->guard_body->add_local(local);
                }
            }
            */

            let parent_block = self.current_suite;
            self.node_pool[guard_body].parent_block = parent_block;
            self.current_suite = Some(guard_body);

            let guard = self.parse_expression(false, false);
            if let Some(guard) = guard {
                self.node_pool[guard_body].statements.push(guard.upcast());
            } else {
                self.push_error(r#"Expected expression for pattern guard after "when"."#);
            }

            self.current_suite = parent_block;

            has_guard = true;
            self.node_pool[branch].has_wildcard = false; // If it has a guard, the wildcard might still not match.
        }

        if matches!(self.current.0, Token::Colon) {
            self.advance();
        } else {
            self.push_error(format!(
                r#"Expected ":"{} after "match" {}.)"#,
                if has_guard { "" } else { r#"( or "when")"# },
                if has_guard {
                    "pattern guard"
                } else {
                    "patterns"
                }
            ));

            if self.version().has_match_error_recovery {
                self.node_pool[branch].block = Some(self.alloc_recovery_suite());
            }
            drop(branch_guard);

            if self.version().has_match_error_recovery {
                // Consume the whole line and treat the next one as new match branch.
                while !matches!(self.current.0, Token::Newline { .. }) && !self.is_at_end() {
                    self.advance();
                }
                if !self.is_at_end() {
                    self.advance();
                }
            } else {
                return None;
            }

            return Some(branch);
        }

        let (suite, suite_guard) = self.alloc_node::<SuiteNode>();
        /*:
        if (branch->patterns.size() > 0) {
            for (const KeyValue<StringName, IdentifierNode *> &E : branch->patterns[0]->binds) {
                SuiteNode::Local local(E.value, current_function);
                local.type = SuiteNode::Local::PATTERN_BIND;
                suite->add_local(local);
            }
        }
        */

        self.parse_suite("match pattern block", suite, suite_guard, false);
        self.node_pool[branch].block = Some(suite);

        Some(branch)
    }

    fn parse_match_pattern(
        &mut self,
        root_pattern: Option<NodeRef<PatternNode>>,
    ) -> Option<NodeRef<PatternNode>> {
        let (pattern, _guard) = self.alloc_node::<PatternNode>();

        match self.current.0 {
            Token::Var => {
                // Bind.
                self.advance();

                if matches!(self.current.0, Token::Identifier(_)) {
                    self.advance();
                } else {
                    self.push_error(r#"Expected bind name after "var"."#);
                    self.node_pool.try_free(pattern);
                    return None;
                }

                let identifier = self.parse_identifier();
                self.node_pool[pattern].pattern_type = Some(PatternType::Bind(identifier));

                if let Some(_root_pattern) = root_pattern {
                    // TODO: check for existing bind
                }

                if let Some(_current_suite) = self.current_suite {
                    // TODO: check for existing local
                }

                let root_pattern = root_pattern.unwrap_or(pattern);
                let identifier_name = self.node_pool[identifier].name.clone();
                self.node_pool[root_pattern]
                    .binds
                    .insert(identifier_name, identifier);
            }

            Token::Underscore => {
                // Wildcard.
                self.advance();
                self.node_pool[pattern].pattern_type = Some(PatternType::Wildcard);
            }

            Token::PeriodPeriod => {
                // Rest.
                self.advance();
                self.node_pool[pattern].pattern_type = Some(PatternType::Rest);
            }

            Token::BracketOpen => {
                // Array.
                if self.version().allow_multiline_array_dictionary_patterns {
                    self.push_multiline(true);
                }

                self.advance();

                let mut inner = ArrayPattern::default();

                do_while!({
                    if matches!(self.current.0, Token::Eof | Token::BracketClose) {
                        break;
                    }

                    let Some(sub_pattern) =
                        self.parse_match_pattern(root_pattern.or(Some(pattern)))
                    else {
                        continue;
                    };

                    if inner.rest_used {
                        self.push_error(
                            r#"The ".." pattern must be the last element in the pattern array."#,
                        );
                    } else if matches!(
                        self.node_pool[sub_pattern].pattern_type,
                        Some(PatternType::Rest)
                    ) {
                        inner.rest_used = true;
                    }

                    inner.elements.push(sub_pattern);
                } while {
                    if matches!(self.current.0, Token::Comma) {
                        self.advance();
                        true
                    } else {
                        false
                    }
                });

                self.consume(
                    TokenType::BracketClose,
                    r#"Expected "]" to close the array pattern."#,
                );

                if self.version().allow_multiline_array_dictionary_patterns {
                    self.pop_multiline();
                }

                self.node_pool[pattern].pattern_type = Some(PatternType::Array(inner));
            }

            Token::BraceOpen => {
                // Dictionary.
                if self.version().allow_multiline_array_dictionary_patterns {
                    self.push_multiline(true);
                }

                self.advance();

                let mut inner = DictionaryPattern::default();

                do_while!({
                    if matches!(self.current.0, Token::Eof | Token::BraceClose) {
                        break;
                    }

                    if matches!(self.current.0, Token::PeriodPeriod) {
                        self.advance();

                        if inner.rest_used {
                            self.push_error(r#"The ".." pattern must be the last element in the pattern dictionary."#);
                        } else {
                            inner.rest_used = true;

                            let (sub_pattern, guard) = self.alloc_node::<PatternNode>();
                            drop(guard);
                            self.node_pool[sub_pattern].pattern_type = Some(PatternType::Rest);
                            inner.elements.push(DictionaryPatternEntry {
                                key: None,
                                value: Some(sub_pattern),
                            });
                        }
                    } else {
                        let key = self.parse_expression(false, false);

                        if key.is_none() {
                            self.push_error(
                                r#"Expected expression as key for dictionary pattern."#,
                            );
                        }

                        if matches!(self.current.0, Token::Colon) {
                            self.advance();

                            let Some(sub_pattern) =
                                self.parse_match_pattern(root_pattern.or(Some(pattern)))
                            else {
                                continue;
                            };

                            if inner.rest_used {
                                self.push_error(r#"The ".." pattern must be the last element in the pattern dictionary."#);
                            } else if matches!(
                                self.node_pool[sub_pattern].pattern_type,
                                Some(PatternType::Rest)
                            ) {
                                self.push_error(r#"The ".." pattern cannot be used as a value."#);
                            } else {
                                inner.elements.push(DictionaryPatternEntry {
                                    key,
                                    value: Some(sub_pattern),
                                })
                            }
                        } else {
                            // Key match only.
                            inner
                                .elements
                                .push(DictionaryPatternEntry { key, value: None });
                        }
                    }
                } while {
                    if matches!(self.current.0, Token::Comma) {
                        self.advance();
                        true
                    } else {
                        false
                    }
                });

                self.consume(
                    TokenType::BraceClose,
                    r#"Expected "}" to close the dictionary pattern."#,
                );

                if self.version().allow_multiline_array_dictionary_patterns {
                    self.pop_multiline();
                }
            }

            _ => {
                // Expression.
                let Some(expression) = self.parse_expression(false, false) else {
                    self.push_error(r#"Expected expression for match pattern."#);
                    self.node_pool.try_free(pattern);
                    return None;
                };

                self.node_pool[pattern].pattern_type = Some(
                    if matches!(self.node_pool[expression], ExpressionNode::Literal(_)) {
                        PatternType::Literal(expression.downcast())
                    } else {
                        PatternType::Expression(expression)
                    },
                );
            }
        }

        Some(pattern)
    }

    fn parse_break(&mut self) -> NodeRef<BreakNode> {
        if !self.can_break {
            self.push_error(r#"Cannot use "break" outside of a loop."#);
        }

        let (node, guard) = self.alloc_node::<BreakNode>();
        drop(guard);
        self.end_statement(r#""break""#);
        node
    }

    fn parse_continue(&mut self) -> NodeRef<ContinueNode> {
        if !self.can_continue {
            self.push_error(r#"Cannot use "continue" outside of a loop."#);
        }

        let (node, guard) = self.alloc_node::<ContinueNode>();
        drop(guard);
        self.end_statement(r#""continue""#);
        node
    }

    fn parse_assert(&mut self) -> Option<NodeRef<AssertNode>> {
        let (assert, assert_guard) = self.alloc_node::<AssertNode>();

        self.push_multiline(true);
        self.consume(
            TokenType::ParenthesisOpen,
            r#"Expected "(" after "assert"."#,
        );

        let condition = self.parse_expression(false, false);
        self.node_pool[assert].condition = condition;

        if self.node_pool[assert].condition.is_none() {
            self.push_error("Expected expression to assert.");
            self.pop_multiline();
            return None;
        }

        if matches!(self.current.0, Token::Comma) {
            self.advance();

            if !matches!(self.current.0, Token::ParenthesisClose) {
                let message = self.parse_expression(false, false);
                self.node_pool[assert].message = message;

                if self.node_pool[assert].message.is_none() {
                    self.push_error(r#"Expected error message for assert after ","."#);
                    self.pop_multiline();
                    return None;
                }

                if matches!(self.current.0, Token::Comma) {
                    self.advance();
                }
            }
        }

        self.pop_multiline();
        self.consume(
            TokenType::ParenthesisClose,
            r#"(Expected ")" after assert expression."#,
        );

        drop(assert_guard);
        self.end_statement(r#""assert""#);

        Some(assert)
    }

    fn parse_class(&mut self) -> Option<NodeRef<ClassNode>> {
        let (class, guard) = self.alloc_node::<ClassNode>();

        let previous_class = self.current_class;
        self.current_class = Some(class);
        self.node_pool[class].outer = previous_class;

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();

            let identifier = self.parse_identifier();
            self.node_pool[class].identifier = Some(identifier);
        } else {
            self.push_error(r#"Expected identifier for the class name after "class"."#);
        }

        if matches!(self.current.0, Token::Extends) {
            self.advance();
            self.parse_extends();
        }

        self.consume(TokenType::Colon, r#"Expected ":" after class declaration."#);

        let multiline = if matches!(self.current.0, Token::Newline { .. }) {
            self.advance();
            true
        } else {
            false
        };

        if multiline {
            if matches!(self.current.0, Token::Indent) {
                self.advance();
            } else {
                self.push_error("Expected indented block after class declaration.");

                self.current_class = previous_class;
                return Some(class);
            }
        }

        if matches!(self.current.0, Token::Extends) {
            self.advance();

            if self.node_pool[class].extends_used {
                self.push_error(r#"Cannot use "extends" more than once in the same class."#);
            }

            self.parse_extends();
            self.end_statement("superclass");
        }

        self.parse_class_body(multiline);
        drop(guard);

        if multiline {
            self.consume(
                TokenType::Dedent,
                "Missing unindent at the end of the class body.",
            );
        }

        self.current_class = previous_class;
        Some(class)
    }

    fn parse_enum(&mut self) -> Option<NodeRef<EnumNode>> {
        let (r#enum, guard) = self.alloc_node::<EnumNode>();
        let mut named = false;

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();

            let identifier = self.parse_identifier();
            self.node_pool[r#enum].identifier = Some(identifier);
            named = true;
        }

        self.push_multiline(true);

        if matches!(self.current.0, Token::BraceOpen) {
            self.advance();
        } else {
            self.push_error(format!(
                r#"Expected "{{" after {}."#,
                if named { "enum name" } else { r#""enum""# }
            ));
        };

        let mut elements: HashMap<StringName, usize> = HashMap::new();

        do_while!({
            if matches!(self.current.0, Token::BraceClose) {
                break; // Allow trailing comma.
            }

            if matches!(self.current.0, Token::Identifier(_)) {
                self.advance();

                let identifier = self.parse_identifier();

                let mut item = EnumValue {
                    identifier: Some(identifier),
                    parent_enum: Some(r#enum),
                    ..Default::default()
                };

                /*:
                item.line = previous.start_line;
                item.start_column = previous.start_column;
                item.end_column = previous.end_column;
                */

                let identifier_name = self.node_pool[identifier].name.clone();
                let identifier_span = self.node_pool.span(identifier);
                if let Some(line) = elements.get(&identifier_name) {
                    self.push_spanned_error(
                        format!(
                            r#"Name "{}" was already in this enum (at line {}).)"#,
                            identifier_name.0, line
                        ),
                        Some(identifier_span),
                    );
                } else if !named {
                    /*:
                    if (current_class->members_indices.has(item.identifier->name)) {
                        push_error(vformat(R"(Name "%s" is already used as a class %s.)", item.identifier->name, current_class->get_member(item.identifier->name).get_type_name()));
                    }
                    */
                }

                elements.insert(identifier_name, self.previous().1.start.line);

                if matches!(self.current.0, Token::Equal) {
                    self.advance();

                    let value = self.parse_expression(false, false);
                    if value.is_none() {
                        self.push_error(r#"Expected expression value after "="."#);
                    }

                    item.custom_value = value.map(|u| u.upcast());
                }

                item.index = self.node_pool[r#enum].values.len();
                self.node_pool[r#enum].values.push(item);

                /*:
                if (!named) {
                    // Add as member of current class.
                    current_class->add_member(item);
                }
                */
            } else {
                self.push_error("Expected identifier for enum key.");
            }
        } while {
             if matches!(self.current.0, Token::Comma) {
                self.advance();
                true
            } else {
                false
            }
        });

        self.pop_multiline();
        self.consume(TokenType::BraceClose, r#"Expected closing "}" for enum."#);
        drop(guard);
        self.end_statement("enum");

        Some(r#enum)
    }

    fn parse_property(
        &mut self,
        variable: NodeRef<VariableNode>,
        guard: InProgressGuard,
        need_indent: bool,
    ) -> Option<NodeRef<VariableNode>> {
        if need_indent {
            if matches!(self.current.0, Token::Indent) {
                self.advance();
            } else {
                self.push_error(r#"Expected indented block for property after ":"."#);
                return None;
            }
        }

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
        } else {
            self.push_error(r#"Expected "get" or "set" for property declaration."#);
            return None;
        }

        let mut function = self.parse_identifier();

        if matches!(self.current.0, Token::Equal) {
            self.node_pool[variable].property = Property::SetGet(Default::default());
        } else {
            self.node_pool[variable].property = Property::Inline(Default::default());
            if !need_indent {
                self.push_error("Property with inline code must go to an indented block.");
            }
        }

        let mut getter_used = false;
        let mut setter_used = false;

        // Run with a loop because order doesn't matter.
        for i in 0..2 {
            if self.node_pool[function].name.0 == "set" {
                if setter_used {
                    self.push_error("Properties can only have one setter.");
                } else {
                    self.parse_property_setter(variable);
                    setter_used = true;
                }
            } else if self.node_pool[function].name.0 == "get" {
                if getter_used {
                    self.push_error("Properties can only have one getter.");
                } else {
                    self.parse_property_getter(variable);
                    getter_used = true;
                }
            } else {
                self.push_error(r#"Expected "get" or "set" for property declaration."#);
            }

            if i == 0
                && let Property::SetGet { .. } = self.node_pool[variable].property
            {
                if matches!(self.current.0, Token::Comma) {
                    self.advance();

                    // Consume potential newline.
                    if matches!(self.current.0, Token::Newline { .. }) {
                        self.advance();

                        if !need_indent {
                            self.push_error(r#"Inline setter/getter setting cannot span across multiple lines (use "\\"" if needed)."#);
                        }
                    }
                } else {
                    break;
                }
            }

            if matches!(self.current.0, Token::Identifier(_)) {
                self.advance();
            } else {
                break;
            }

            function = self.parse_identifier();
        }

        drop(guard);

        if let Property::SetGet { .. } = self.node_pool[variable].property {
            self.end_statement("property declaration");
        }

        if need_indent {
            self.consume(
                TokenType::Dedent,
                r#"Expected end of indented block for property."#,
            );
        }

        Some(variable)
    }

    fn parse_property_setter(&mut self, variable: NodeRef<VariableNode>) {
        match self.node_pool[variable].property {
            Property::Inline(mut inline_property) => {
                let (function, _guard) = self.alloc_node::<FunctionNode>();
                let (identifier, identifier_guard) = self.alloc_node::<IdentifierNode>();
                drop(identifier_guard);

                let name = format!(
                    "@{}_setter",
                    self.node_pool[self.node_pool[variable].base.identifier.unwrap()]
                        .name
                        .0
                );
                self.node_pool[identifier].name = StringName(name.into());
                self.node_pool[function].identifier = Some(identifier);
                self.node_pool[function].is_static = self.node_pool[variable].is_static;

                self.consume(TokenType::ParenthesisOpen, r#"Expected "(" after "set"."#);

                let (parameter, parameter_guard) = self.alloc_node::<ParameterNode>();
                if matches!(self.current.0, Token::Identifier(_)) {
                    self.advance();

                    self.reset_span(parameter, self.previous().1);
                    let identifier = self.parse_identifier();

                    inline_property.setter.parameter = Some(identifier);
                    self.node_pool[variable].property = Property::Inline(inline_property);

                    self.node_pool[parameter].0.identifier = Some(identifier);

                    let identifier_name = self.node_pool[identifier].name.clone();
                    self.node_pool[function]
                        .parameters
                        .insert(identifier_name, parameter);
                } else {
                    self.push_error(r#"Expected parameter name after "("."#);
                }

                drop(parameter_guard);

                self.consume(
                    TokenType::ParenthesisClose,
                    r#"Expected ")" after parameter name."#,
                );
                self.consume(TokenType::Colon, r#"Expected ":" after ")"."#);

                let previous_function = self.current_function;
                self.current_function = Some(function);

                if inline_property.setter.parameter.is_some() {
                    let (body, body_guard) = self.alloc_node::<SuiteNode>();
                    //: body->add_local(parameter, function);
                    self.parse_suite("setter declaration", body, body_guard, false);
                    self.node_pool[function].body_suite = Some(body);

                    inline_property.setter.function = Some(function);
                    self.node_pool[variable].property = Property::Inline(inline_property);
                }

                self.current_function = previous_function;
            }

            Property::SetGet(mut setget_property) => {
                self.consume(TokenType::Equal, r#"Expected "=" after "set"."#);

                if matches!(self.current.0, Token::Identifier(_)) {
                    self.advance();
                    let identifier = self.parse_identifier();

                    setget_property.setter = Some(identifier);
                    self.node_pool[variable].property = Property::SetGet(setget_property);
                } else {
                    self.push_error(r#"Expected setter function name after "="."#);
                }
            }

            Property::None => {} // Unreachable.
        }
    }

    fn parse_property_getter(&mut self, variable: NodeRef<VariableNode>) {
        match self.node_pool[variable].property {
            Property::Inline(mut inline_property) => {
                let (function, _guard) = self.alloc_node::<FunctionNode>();

                if self.version().allow_empty_parentheses_in_getter_declaration {
                    if matches!(self.current.0, Token::ParenthesisOpen) {
                        self.advance();
                        self.consume(TokenType::ParenthesisClose, r#"Expected ")" after "get("."#);
                        self.consume(TokenType::Colon, r#"Expected ":" after "get()"."#);
                    } else {
                        self.consume(TokenType::Colon, r#"Expected ":" or "(" after "get"."#);
                    }
                } else {
                    self.consume(TokenType::Colon, r#"Expected ":" after "get()"."#);
                }

                let (identifier, identifier_guard) = self.alloc_node::<IdentifierNode>();
                drop(identifier_guard);
                let name = format!(
                    "@{}_getter",
                    self.node_pool[self.node_pool[variable].base.identifier.unwrap()]
                        .name
                        .0
                );
                self.node_pool[identifier].name = StringName(name.into());
                self.node_pool[function].identifier = Some(identifier);
                self.node_pool[function].is_static = self.node_pool[variable].is_static;

                let previous_function = self.current_function;
                self.current_function = Some(function);

                let (body, body_guard) = self.alloc_node::<SuiteNode>();
                self.parse_suite("getter declaration", body, body_guard, false);
                self.node_pool[function].body_suite = Some(body);

                inline_property.getter = Some(function);
                self.node_pool[variable].property = Property::Inline(inline_property);

                self.current_function = previous_function;
            }

            Property::SetGet(mut setget_property) => {
                self.consume(TokenType::Equal, r#"Expected "=" after "get"."#);

                if matches!(self.current.0, Token::Identifier(_)) {
                    self.advance();
                    let identifier = self.parse_identifier();

                    setget_property.getter = Some(identifier);
                    self.node_pool[variable].property = Property::SetGet(setget_property);
                } else {
                    self.push_error(r#"Expected getter function name after "="."#);
                }
            }

            Property::None => {} // Unreachable.
        }
    }

    fn parse_annotation(&mut self) -> Option<NodeRef<AnnotationNode>> {
        let (node, guard) = self.alloc_node::<AnnotationNode>();

        self.node_pool[node].name = self.previous_as_string().unwrap_or_default();

        let mut valid = true;

        if matches!(self.current.0, Token::ParenthesisOpen) {
            if self.version().has_72979_annotation_parsing {
                self.push_multiline(true);
            }

            self.advance();

            if !self.version().has_72979_annotation_parsing {
                if !matches!(self.current.0, Token::ParenthesisClose) && !self.is_at_end() {
                    self.push_multiline(true);

                    do_while!({
                        let Some(argument) = self.parse_expression(false, false) else {
                            valid = false;
                            continue;
                        };

                        self.node_pool[node].arguments.push(argument);
                    } while {
                        if matches!(self.current.0, Token::Comma) {
                            self.advance();
                            true
                        } else {
                            false
                        }
                    });
                }
            } else {
                do_while!({
                    if matches!(self.current.0, Token::ParenthesisClose) {
                        // Allow for trailing comma.
                        break;
                    }

                    let argument = self.parse_expression(false, false);

                    match argument {
                        Some(argument) => self.node_pool[node].arguments.push(argument),
                        None => {
                            self.push_error("Expected expression as the annotation argument.");

                            if valid {
                                self.node_pool.try_free(node);
                            }

                            valid = false;
                        }
                    }
                } while {
                    if matches!(self.current.0, Token::Comma) {
                        self.advance();
                        true
                    } else {
                        false
                    }
                });
            }

            self.pop_multiline();
            self.consume(
                TokenType::ParenthesisClose,
                r#"Expected ")" after annotation arguments."#,
            );
        }

        drop(guard);
        if matches!(self.current.0, Token::Newline { .. }) {
            self.advance(); // Newline after annotation is optional.
        }

        if valid { Some(node) } else { None }
    }

    fn parse_expression(
        &mut self,
        can_assign: bool,
        stop_on_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        self.parse_precedence(Precedence::Assignment, can_assign, stop_on_assign)
    }

    fn parse_precedence(
        &mut self,
        precedence: Precedence,
        can_assign: bool,
        stop_on_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        if matches!(
            self.current.0,
            Token::ParenthesisOpen | Token::BraceOpen | Token::BracketOpen
        ) {
            self.push_multiline(true);
        }

        let token = self.current.clone();
        let mut token_type = token.0.typ();
        if token.0.is_identifier() {
            // Allow keywords that can be treated as identifiers.
            token_type = TokenType::Identifier;
        }

        let Some(prefix_rule) = get_rule(token_type).prefix else {
            // Expected expression. Let the caller give the proper error message.
            return None;
        };

        self.advance(); // Only consume the token if there's a valid rule.
        let mut previous_operand = prefix_rule(self, can_assign)?;

        while precedence <= get_rule(self.current.0.typ()).precedence() {
            if (stop_on_assign && matches!(self.current.0, Token::Equal)) || self.lambda_ended {
                return Some(previous_operand);
            }

            if matches!(self.current.0, Token::ParenthesisOpen | Token::BracketOpen) {
                self.push_multiline(true);
            }

            self.advance();

            let previous_type = self.previous().0.typ();
            let infix = get_rule(previous_type).infix.expect("no infix for rule");

            {
                let result = (infix.parser)(self, previous_operand, can_assign)?;
                previous_operand = result;
            }
        }

        Some(previous_operand)
    }

    fn parse_parameter(&mut self) -> Option<NodeRef<ParameterNode>> {
        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
        } else {
            self.push_error("Expected parameter name.");
            return None;
        }

        let (node, _guard) = self.alloc_node::<ParameterNode>();

        self.node_pool[node].0.identifier = Some(self.parse_identifier());

        if matches!(self.current.0, Token::Colon) {
            self.advance();

            if matches!(self.current.0, Token::Equal) {
                // Infer type.
                self.node_pool[node].0.infer_datatype = true;
            } else {
                // Parse type.
                self.node_pool[node].0.datatype_specifier = self.parse_type(false);
            }
        }

        self.node_pool[node].0.initializer = if matches!(self.current.0, Token::Equal) {
            self.advance();

            // Default value.
            self.parse_expression(false, false)
        } else {
            None
        };

        Some(node)
    }

    fn parse_type(&mut self, allow_void: bool) -> Option<NodeRef<TypeNode>> {
        let (node, _guard) = self.alloc_node::<TypeNode>();

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
        } else {
            if matches!(self.current.0, Token::Void) {
                self.advance();

                if allow_void {
                    return Some(node.upcast());
                } else {
                    self.push_error(r#""void" is only allowed for a function return type."#);
                }
            }

            // Leave error message to the caller who knows the context.
            self.node_pool.try_free(node);
            return None;
        }

        let mut type_element = self.parse_identifier();
        self.node_pool[node].type_chain.push(type_element);

        if matches!(self.current.0, Token::BracketOpen) {
            self.advance();

            // Typed collection (like Array[int], Dictionary[String, int]).
            let mut first_pass = true;
            let mut return_none = false;

            do_while!({
                let container_type = self.parse_type(false); // Don't allow void for element type.

                match container_type {
                    None => {
                        self.push_error(format!(
                            r#"Expected type for collection after "{}"."#,
                            if first_pass { "[" } else { "," }
                        ));
                        return_none = true;
                        break;
                    }

                    Some(container) => {
                        if self.node_pool[container].container_types.is_empty() {
                            self.node_pool[node].container_types.push(container);
                        } else {
                            self.push_error("Nested typed collections are not supported.");
                        }
                    }
                }

                first_pass = false;
            } while {
                if matches!(self.current.0, Token::Comma) {
                    self.advance();
                    true
                } else {
                    false
                }
            });

            self.consume(
                TokenType::BracketClose,
                r#"Expected closing "]" after collection type."#,
            );

            if return_none {
                return None;
            }

            return Some(node.upcast());
        }

        loop {
            if matches!(self.current.0, Token::Period) {
                self.advance();

                if matches!(self.current.0, Token::Identifier(_)) {
                    self.advance();
                    type_element = self.parse_identifier();
                    self.node_pool[node].type_chain.push(type_element);
                } else {
                    self.push_error(r#"Expected inner type name after "."."#);
                }
            } else {
                break;
            }
        }

        Some(node.upcast())
    }

    fn parse_literal(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        let variant = match &self.previous {
            Some((Token::Literal(variant), _)) => variant.clone(),
            _ => panic!("parsing literal node without literal token"),
        };

        let (node, _guard) = self.alloc_node::<LiteralNode>();
        self.update_span(node);
        self.node_pool[node].value = variant;
        Some(node.upcast())
    }

    fn parse_binary_operator(
        &mut self,
        previous_operand: NodeRef<ExpressionNode>,
        _can_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        let (node, guard) = self.alloc_node::<BinaryOpNode>();
        let op_token = self.previous().clone();
        self.reset_span(node, self.node_pool.span(previous_operand));
        self.update_span(node);
        self.node_pool[node].left = Some(previous_operand);

        let precedence = get_rule(op_token.0.typ())
            .precedence()
            .next()
            .expect("no further precedence");
        self.node_pool[node].right = self.parse_precedence(precedence, false, false);
        drop(guard);

        if self.node_pool[node].right.is_none() {
            self.push_error(format!(
                r#"Expected expression after "{}" operator"#,
                op_token.0.token_name()
            ));
        };

        self.node_pool[node].operation = match op_token.0 {
            Token::Plus => BinaryOperation::Addition,
            Token::Minus => BinaryOperation::Subtraction,
            Token::Star => BinaryOperation::Multiplication,
            Token::Slash => BinaryOperation::Division,
            Token::Percent => BinaryOperation::Modulo,
            Token::StarStar => BinaryOperation::Power,
            Token::LessLess => BinaryOperation::BitLeftShift,
            Token::GreaterGreater => BinaryOperation::BitRightShift,
            Token::Ampersand => BinaryOperation::BitAnd,
            Token::Pipe => BinaryOperation::BitOr,
            Token::Caret => BinaryOperation::BitXor,
            Token::And | Token::AmpersandAmpersand => BinaryOperation::LogicAnd,
            Token::Or | Token::PipePipe => BinaryOperation::LogicOr,
            Token::In => BinaryOperation::ContentTest,
            Token::EqualEqual => BinaryOperation::CompEqual,
            Token::BangEqual => BinaryOperation::CompNotEqual,
            Token::Less => BinaryOperation::CompLess,
            Token::LessEqual => BinaryOperation::CompLessEqual,
            Token::Greater => BinaryOperation::CompGreater,
            Token::GreaterEqual => BinaryOperation::CompGreaterEqual,
            _ => unreachable!(),
        };

        Some(node.upcast())
    }

    fn parse_unary_operator(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        let (node, _guard) = self.alloc_node::<UnaryOpNode>();
        let op_token = self.previous();

        let (operation, operand) = match op_token.0 {
            Token::Minus => {
                let operand = self.parse_precedence(Precedence::Sign, false, false);

                if operand.is_none() {
                    self.push_error(r#"Expected expression after "-" operator."#);
                }

                (UnaryOperation::Negate, operand)
            }

            Token::Plus => {
                let operand = self.parse_precedence(Precedence::Sign, false, false);

                if operand.is_none() {
                    self.push_error(r#"Expected expression after "+" operator."#);
                }

                (UnaryOperation::Positive, operand)
            }

            Token::Tilde => {
                let operand = self.parse_precedence(Precedence::BitNot, false, false);

                if operand.is_none() {
                    self.push_error(r#"Expected expression after "~" operator."#);
                }

                (UnaryOperation::Complement, operand)
            }

            Token::Not => {
                let operand = self.parse_precedence(Precedence::LogicNot, false, false);

                if operand.is_none() {
                    self.push_error(r#"Expected expression after "not" operator."#);
                }

                (UnaryOperation::LogicalNot, operand)
            }

            Token::Bang => {
                let operand = self.parse_precedence(Precedence::LogicNot, false, false);

                if operand.is_none() {
                    self.push_error(r#"Expected expression after "!" operator."#);
                }

                (UnaryOperation::LogicalNot, operand)
            }

            _ => unreachable!(),
        };

        self.node_pool[node].operation = operation;
        self.node_pool[node].operand = operand;
        Some(node.upcast())
    }

    fn parse_binary_not_in_operator(
        &mut self,
        previous_operand: NodeRef<ExpressionNode>,
        can_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        let (node, _guard) = self.alloc_node::<UnaryOpNode>();
        self.reset_span(node, self.node_pool.span(previous_operand));
        self.update_span(node);
        self.node_pool[node].operation = UnaryOperation::LogicalNot;

        self.consume(
            TokenType::In,
            r#"Expected "in" after "not" in content-test operator."#,
        );

        self.node_pool[node].operand = self.parse_binary_operator(previous_operand, can_assign);

        Some(node.upcast())
    }

    fn parse_get_node(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        if !self.current.0.is_node_name()
            && !matches!(
                self.current.0,
                Token::Literal(Variant::String(_)) | Token::Slash | Token::Percent
            )
        {
            self.push_error(format!(
                r#"Expected node path as string or identifier after "{}"."#,
                self.previous().0.token_name()
            ));
            return None;
        }

        let (node, _guard) = self.alloc_node::<GetNodeNode>();

        self.node_pool[node].use_dollar = if matches!(self.previous().0, Token::Dollar) {
            // Detect initial slash, which will be handled in the loop if it matches.
            if matches!(self.current.0, Token::Slash) {
                self.advance();
            }

            true
        } else {
            false
        };

        enum PathState {
            Start,
            Slash,
            Percent,
            NodeName,
        }

        let mut path_state = PathState::Start;
        let mut full_path = String::new();

        do_while!({
            if matches!(self.previous().0, Token::Percent) {
                if !matches!(path_state, PathState::Start | PathState::Slash) {
                    self.push_error(r#""%" is only valid in the beginning of a node name (either after "$" or after "/")"#);
                    return None;
                }

                full_path += "%";
                path_state = PathState::Percent;
            } else if matches!(self.previous().0, Token::Slash) {
                if !matches!(path_state, PathState::Start | PathState::NodeName) {
                    self.push_error(
                        r#""/" is only valid at the beginning of the path or after a node name."#,
                    );

                    return None;
                }

                full_path += "%";
                path_state = PathState::Slash;
            }

            if let Token::Literal(literal) = &self.current.0 {
                let Variant::String(inner) = literal else {
                    self.advance();

                    let previous_token = match path_state {
                        PathState::Start => "$",
                        PathState::Percent => "%",
                        PathState::Slash => "/",
                        PathState::NodeName => unreachable!(),
                    };

                    self.push_error(format!(
                        r#"Expected node path as string or identifier after "{}""#,
                        previous_token
                    ));

                    return None;
                };

                full_path += inner;
                self.advance();
                path_state = PathState::NodeName;
            } else if self.current.0.is_node_name() {
                self.advance();

                let identifier = self.previous().0.get_identifier();
                full_path += identifier;
                path_state = PathState::NodeName;
            } else if !matches!(self.current.0, Token::Slash | Token::Percent) {
                self.push_error(format!(
                    r#"Unexpected "{}" in node path."#,
                    self.current.0.token_name()
                ));

                return None;
            }
        } while {
            if matches!(self.current.0, Token::Slash | Token::Percent) {
                self.advance();
                true
            } else {
                false
            }
        });

        self.node_pool[node].full_path = full_path;
        Some(node.upcast())
    }

    fn parse_assignment(
        &mut self,
        previous_operand: NodeRef<ExpressionNode>,
        can_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        if !can_assign {
            self.push_error("Assignment is not allowed inside an expression.");

            return self.parse_expression(false, false);
        }

        if !matches!(
            self.node_pool[previous_operand],
            ExpressionNode::Identifier(_) | ExpressionNode::SubScript(_)
        ) {
            self.push_error("Only identifier, attribute access, and subscription access can be used as assignment target.");
            return self.parse_expression(false, false);
        }

        let (node, _guard) = self.alloc_node::<AssignmentNode>();
        self.reset_span(node, self.node_pool.span(previous_operand));
        self.update_span(node);

        self.node_pool[node].assignee = Some(previous_operand);

        self.node_pool[node].operation = match self.previous().0 {
            Token::Equal => AssignmentOperation::None,
            Token::PlusEqual => AssignmentOperation::Addition,
            Token::MinusEqual => AssignmentOperation::Subtraction,
            Token::StarEqual => AssignmentOperation::Multiplication,
            Token::StarStarEqual => AssignmentOperation::Power,
            Token::SlashEqual => AssignmentOperation::Division,
            Token::PercentEqual => AssignmentOperation::Modulo,
            Token::LessLessEqual => AssignmentOperation::BitShiftLeft,
            Token::GreaterGreaterEqual => AssignmentOperation::BitShiftRight,
            Token::AmpersandEqual => AssignmentOperation::BitAnd,
            Token::PipeEqual => AssignmentOperation::BitOr,
            Token::CaretEqual => AssignmentOperation::BitXor,
            _ => unreachable!(),
        };

        self.node_pool[node].assigned_value = self.parse_expression(false, false);

        if self.node_pool[node].assigned_value.is_none() {
            self.push_error(r#"Expected an expression after "="."#);
        }

        Some(node.upcast())
    }

    fn parse_ternary_operator(
        &mut self,
        previous_operand: NodeRef<ExpressionNode>,
        _can_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        let (node, _guard) = self.alloc_node::<TernaryOpNode>();
        self.reset_span(node, self.node_pool.span(previous_operand));
        self.update_span(node);

        self.node_pool[node].if_true = Some(previous_operand);

        self.node_pool[node].condition = self.parse_precedence(Precedence::Ternary, false, false);

        if self.node_pool[node].condition.is_none() {
            self.push_error(r#"Expected expression as ternary condition after "if"."#);
        }

        self.consume(
            TokenType::Else,
            r#"Expected "else" after ternary operator condition."#,
        );

        self.node_pool[node].if_false = self.parse_precedence(Precedence::Ternary, false, false);

        if self.node_pool[node].if_false.is_none() {
            self.push_error(r#"Expected expression after "else"."#);
        }

        Some(node.upcast())
    }

    fn parse_cast(
        &mut self,
        previous_operand: NodeRef<ExpressionNode>,
        _can_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        let (node, _guard) = self.alloc_node::<CastNode>();
        self.node_pool[node].operand = Some(previous_operand);

        self.reset_span(node, self.node_pool.span(previous_operand));
        self.update_span(node);

        let cast_type = self.parse_type(false);

        if cast_type.is_none() {
            self.push_error(r#"Expected type specifier after "as"."#);
            self.node_pool.try_free(node);
            return Some(previous_operand);
        };

        self.node_pool[node].cast_type = cast_type;
        Some(node.upcast())
    }

    fn parse_await(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        let (node, _guard) = self.alloc_node::<AwaitNode>();
        self.node_pool[node].to_await = self.parse_precedence(Precedence::Await, false, false);

        if self.node_pool[node].to_await.is_none() {
            self.push_error(r#"Expected signal or coroutine after "await"."#);
        }

        if let Some(current_function) = self.current_function {
            self.node_pool[current_function].is_coroutine = true;
        }

        Some(node.upcast())
    }

    fn parse_lambda(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        let (lambda, lambda_guard) = self.alloc_node::<LambdaNode>();
        self.node_pool[lambda].parent_function = self.current_function;
        self.node_pool[lambda].parent_lambda = self.current_lambda;

        let (function, function_guard) = self.alloc_node::<FunctionNode>();
        self.node_pool[function].source_lambda = Some(lambda);
        self.node_pool[function].is_static = self
            .current_function
            .map(|r| self.node_pool[r].is_static)
            .unwrap_or(false);

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
            self.node_pool[function].identifier = Some(self.parse_identifier());
        }

        let multiline_context = *self.multiline_stack.back().unwrap();

        // Reset the multiline stack since we don't want the multiline mode one in the lambda body.
        self.push_multiline(false);
        if multiline_context {
            self.tokenizer.push_expression_indented_block();
        }

        self.push_multiline(true); // For the parameters.
        if self.node_pool[function].identifier.is_some() {
            self.consume(
                TokenType::ParenthesisOpen,
                r#"Expected opening "(" after lambda name."#,
            );
        } else {
            self.consume(
                TokenType::ParenthesisOpen,
                r#"Expected opening "(" after "func"."#,
            );
        }

        let previous_function = self.current_function.replace(function);
        let previous_lambda = self.current_lambda.replace(lambda);

        let (body, body_guard) = self.alloc_node::<SuiteNode>();
        self.node_pool[body].parent_function = Some(function);
        self.node_pool[body].parent_block = self.current_suite;

        let previous_suite = self.current_suite.replace(body);

        self.parse_function_signature(function, body, "lambda");
        self.current_suite = previous_suite;

        let previous_in_lambda = mem::replace(&mut self.in_lambda, true);

        // Save break/continue state.
        let could_break = mem::replace(&mut self.can_break, false);
        let could_continue = mem::replace(&mut self.can_continue, false);

        self.parse_suite("lambda declaration", body, body_guard, true);
        drop(function_guard);
        drop(lambda_guard);

        self.pop_multiline();

        if multiline_context {
            while matches!(
                self.current.0,
                Token::Dedent | Token::Indent | Token::Newline { .. }
            ) {
                self.current = self.advance_inner();
            }

            let difference = self.tokenizer.pop_expression_indented_block();
            assert!(difference <= 0, "lambda became less indented");
            self.pending_indents_at_newline += difference;
        }

        self.current_function = previous_function;
        self.current_lambda = previous_lambda;
        self.in_lambda = previous_in_lambda;
        self.node_pool[lambda].function = Some(function);
        self.can_break = could_break;
        self.can_continue = could_continue;

        Some(lambda.upcast())
    }

    fn parse_type_test(
        &mut self,
        previous_operand: NodeRef<ExpressionNode>,
        _can_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        let mut not_node = None;
        let mut not_guard = None;

        if self.version().has_is_not && matches!(self.current.0, Token::Not) {
            self.advance();

            let (node, guard) = self.alloc_node::<UnaryOpNode>();
            self.node_pool[node].operation = UnaryOperation::LogicalNot;
            self.reset_span(node, self.node_pool.span(previous_operand));
            self.update_span(node);

            not_node = Some(node);
            not_guard = Some(guard);
        };

        let (type_test, type_test_guard) = self.alloc_node::<TypeTestNode>();
        self.reset_span(type_test, self.node_pool.span(previous_operand));
        self.update_span(type_test);

        self.node_pool[type_test].operand = Some(previous_operand);
        self.node_pool[type_test].test_type = self.parse_type(false);
        drop(type_test_guard);

        if let Some(not_node) = not_node {
            self.node_pool[not_node].operand = Some(type_test.upcast());
        }

        drop(not_guard);

        if self.node_pool[type_test].test_type.is_none() {
            if not_node.is_some() {
                self.push_error(r#"Expected type specifier after "is"."#);
            } else {
                self.push_error(r#"Expected type specifier after "is not"."#);
            }
        }

        if let Some(not_node) = not_node {
            Some(not_node.upcast())
        } else {
            Some(type_test.upcast())
        }
    }

    fn parse_preload(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        let (node, _guard) = self.alloc_node::<PreloadNode>();
        self.push_multiline(true);
        self.consume(
            TokenType::ParenthesisOpen,
            r#"Expected "(" after "preload"."#,
        );

        self.node_pool[node].path = self.parse_expression(false, false);

        if self.node_pool[node].path.is_none() {
            self.push_error(r#"Expected resource path after "("."#);
        }

        if self.version().allow_preload_trailing_comma && matches!(self.current.0, Token::Comma) {
            self.advance();
        }

        self.pop_multiline();
        self.consume(
            TokenType::ParenthesisClose,
            r#"Expected ")" after preload path."#,
        );

        Some(node.upcast())
    }

    fn parse_self(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        if let Some(current_function) = self.current_function
            && self.node_pool[current_function].is_static
        {
            self.push_error(r#"Cannot use "self" inside a static function."#);
        }

        let (node, _guard) = self.alloc_node::<SelfNode>();
        Some(node.upcast())
    }

    fn parse_call_prefix(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        self.parse_call(None)
    }

    fn parse_call_infix(
        &mut self,
        previous_operand: NodeRef<ExpressionNode>,
        _can_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        self.parse_call(Some(previous_operand))
    }

    fn parse_call(
        &mut self,
        previous_operand: Option<NodeRef<ExpressionNode>>,
    ) -> Option<NodeRef<ExpressionNode>> {
        let (node, guard) = self.alloc_node::<CallNode>();
        drop(guard);

        if matches!(self.previous().0, Token::Super) {
            self.node_pool[node].is_super = true;
            if !self.version().has_fixed_multiline_handling_in_super_calls {
                self.push_multiline(true);
            }

            if matches!(self.current.0, Token::ParenthesisOpen) {
                if self.version().has_fixed_multiline_handling_in_super_calls {
                    self.push_multiline(true);
                }
                self.advance();

                if self.current_function.is_none() {
                    self.push_error(r#"Cannot use implicit "super" call outside of a function."#);
                    self.pop_multiline();
                    self.node_pool.try_free(node);
                    return None;
                }
            } else {
                self.consume(TokenType::Period, r#"Expected "." or "(" after "super"."#);

                if matches!(self.current.0, Token::Identifier(_)) {
                    self.advance();
                } else {
                    self.push_error(r#"Expected function name after "."."#);
                    if !self.version().has_fixed_multiline_handling_in_super_calls {
                        self.pop_multiline();
                    }
                    self.node_pool.try_free(node);
                    return None;
                }

                self.node_pool[node].callee = Some(self.parse_identifier().upcast());

                if matches!(self.current.0, Token::ParenthesisOpen) {
                    if self.version().has_fixed_multiline_handling_in_super_calls {
                        self.push_multiline(true);
                    }
                    self.advance();
                } else {
                    self.push_error(r#"Expected "(" after function name."#);
                    if self.version().has_early_bail_in_super_calls {
                        if !self.version().has_fixed_multiline_handling_in_super_calls {
                            self.pop_multiline();
                        }
                        self.node_pool.try_free(node);
                        return None;
                    }
                }
            }
        } else {
            self.node_pool[node].callee = previous_operand;

            let name = if let Some(previous_operand) = previous_operand {
                match &self.node_pool[previous_operand] {
                    ExpressionNode::Identifier(identifier) => Some(identifier.name.clone()),
                    ExpressionNode::SubScript(subscript) => {
                        if let Some(SubscriptNodeInner::Attribute(attr)) = subscript.inner {
                            Some(self.node_pool[attr].name.clone())
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            } else {
                None
            };

            if let Some(name) = name {
                self.node_pool[node].function_name = name;
            } else {
                self.push_error(
                    r#"Cannot call on an expression. Use ".call()" if it's a Callable."#,
                );
            }
        }

        // Arguments.
        do_while!({
            if matches!(self.current.0, Token::ParenthesisClose) {
                // Allow for trailing comma.
                break;
            }

            if let Some(argument) = self.parse_expression(false, false) {
                self.node_pool[node].arguments.push(argument);
            } else {
                self.push_error(r#"Expected expression as the function argument."#);
            }
        } while {
            if matches!(self.current.0, Token::Comma) {
                self.advance();
                true
            } else {
                false
            }
        });

        self.pop_multiline();
        self.consume(
            TokenType::ParenthesisClose,
            r#"Expected closing ")" after call arguments."#,
        );
        Some(node.upcast())
    }

    fn parse_yield(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        self.push_error(r#""yield" was removed in Godot 4. Use "await" instead."#);
        None
    }

    fn parse_array(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        let (node, _guard) = self.alloc_node::<ArrayNode>();

        if !matches!(self.current.0, Token::BracketClose) {
            do_while!({
                if matches!(self.current.0, Token::BracketClose) {
                    // Allow for trailing comma.
                    break;
                }

                if let Some(element) = self.parse_expression(false, false) {
                    self.node_pool[node].elements.push(element);
                } else {
                    self.push_error("Expected expression as array element.");
                }
            } while {
                if matches!(self.current.0, Token::Comma) {
                    self.advance();
                    !self.is_at_end()
                } else {
                    false
                }
            });
        }

        self.pop_multiline();
        self.consume(
            TokenType::BracketClose,
            r#"Expected closing "]" after array elements."#,
        );

        Some(node.upcast())
    }

    fn parse_subscript(
        &mut self,
        previous_operand: NodeRef<ExpressionNode>,
        _can_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        let (node, _guard) = self.alloc_node::<SubscriptNode>();
        self.reset_span(node, self.node_pool.span(previous_operand));
        self.update_span(node);

        self.node_pool[node].base = Some(previous_operand);

        if let Some(expr) = self.parse_expression(false, false) {
            self.node_pool[node].inner = Some(SubscriptNodeInner::Index(expr));
        } else {
            self.push_error(r#"Expected expression after "["."#);
        }

        self.pop_multiline();
        self.consume(
            TokenType::BracketClose,
            r#"Expected "]" after subscription index."#,
        );

        Some(node.upcast())
    }

    fn parse_dictionary(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        let (node, _guard) = self.alloc_node::<DictionaryNode>();
        let mut decided_style = false;

        if !matches!(self.current.0, Token::BraceClose) {
            do_while!({
                if matches!(self.current.0, Token::BraceClose) {
                    break;
                }

                let key = self.parse_expression(false, true);

                if key.is_none() {
                    self.push_error(r#"Expected expression as dictionary key."#);
                }

                if !decided_style {
                    self.node_pool[node].style = match self.current.0 {
                        Token::Colon => DictionaryStyle::PythonDict,
                        Token::Equal => DictionaryStyle::LuaTable,
                        _ => {
                            self.push_error(r#"Expected ":" or "=" after dictionary key."#);
                            DictionaryStyle::default()
                        }
                    };
                    decided_style = true;
                }

                match self.node_pool[node].style {
                    DictionaryStyle::LuaTable => {
                        let key = key.map(|r| &self.node_pool[r]);

                        if key.is_some() {
                            if !matches!(
                                key,
                                Some(ExpressionNode::Identifier(_) | ExpressionNode::Literal(_))
                            ) {
                                self.push_error(r#"Expected identifier or string as Lua-style dictionary key (e.g "{ key = value }")."#);
                                if !self.version().has_dictionary_error_recovery {
                                    self.advance();
                                    break;
                                }
                            } else if let Some(ExpressionNode::Literal(literal)) = key
                                && !matches!(literal.value, Variant::String(_))
                            {
                                self.push_error(r#"Expected identifier or string as Lua-style dictionary key (e.g "{ key = value }")."#);
                                if !self.version().has_dictionary_error_recovery {
                                    self.advance();
                                    break;
                                }
                            }
                        }

                        if matches!(self.current.0, Token::Equal) {
                            self.advance();
                        } else {
                            if matches!(self.current.0, Token::Colon) {
                                self.push_error(r#"Expected "=" after dictionary key. Mixing dictionary styles is not allowed."#);
                                self.advance();
                            } else {
                                self.advance();
                                self.push_error(r#"Expected "=" after dictionary key."#);
                            }
                        }
                    }
                    DictionaryStyle::PythonDict => {
                        if matches!(self.current.0, Token::Colon) {
                            self.advance();
                        } else {
                            if matches!(self.current.0, Token::Equal) {
                                self.push_error(r#"Expected ":" after dictionary key. Mixing dictionary styles is not allowed."#);
                                self.advance();
                            } else {
                                self.advance();
                                self.push_error(r#"Expected ":" after dictionary key."#);
                            }
                        }
                    }
                }

                let value = self.parse_expression(false, false);

                if value.is_none() {
                    self.push_error(r#"Expected expression as dictionary value."#);
                }

                fn alloc_dummy_literal(pool: &mut NodePool) -> NodeRef<ExpressionNode> {
                    let dummy = LiteralNode {
                        value: Variant::Nil(Nil),
                    };
                    let r = pool.push((dummy, Span::zero()));
                    r.upcast()
                }

                match (key, value) {
                    (Some(key), Some(value)) => {
                        self.node_pool[node].elements.push((key, value));
                    }

                    (Some(key), None) => {
                        if self.version().has_dictionary_error_recovery {
                            let dummy = alloc_dummy_literal(&mut self.node_pool);
                            self.node_pool[node].elements.push((key, dummy));
                        }
                    }

                    (None, Some(value)) => {
                        if self.version().has_dictionary_error_recovery {
                            let dummy = alloc_dummy_literal(&mut self.node_pool);
                            self.node_pool[node].elements.push((dummy, value));
                        }
                    }

                    (None, None) => {}
                }
            } while {
                if matches!(self.current.0, Token::Comma) {
                    self.advance();
                    !self.is_at_end()
                } else {
                    false
                }
            });
        }

        self.pop_multiline();
        self.consume(
            TokenType::BraceClose,
            r#"Expected closing "}" after dictionary elements."#,
        );
        Some(node.upcast())
    }

    fn parse_grouping(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        let grouped = self.parse_expression(false, false);
        self.pop_multiline();

        if grouped.is_none() {
            self.push_error(r#"Expected grouping expression."#);
        } else {
            self.consume(
                TokenType::ParenthesisClose,
                r#"Expected closing ")" after grouping expression."#,
            );
        }

        grouped
    }

    fn parse_attribute(
        &mut self,
        previous_operand: NodeRef<ExpressionNode>,
        _can_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        let (node, _guard) = self.alloc_node::<SubscriptNode>();
        self.reset_span(node, self.node_pool.span(previous_operand));
        self.update_span(node);

        self.node_pool[node].base = Some(previous_operand);

        if self.version().allow_keywords_as_attributes && self.current.0.is_node_name() {
            self.current.0 = Token::Identifier(self.current.0.get_identifier().to_owned());
        }

        if matches!(self.current.0, Token::Identifier(_)) {
            self.advance();
        } else {
            self.push_error(r#"Expected identifier after "." for attribute access."#);
            self.node_pool.try_free(node);
            return None;
        }

        let identifier = self.parse_identifier();
        self.node_pool[node].inner = Some(SubscriptNodeInner::Attribute(identifier));
        Some(node.upcast())
    }

    fn parse_builtin_constant(&mut self, _can_assign: bool) -> Option<NodeRef<ExpressionNode>> {
        let (node, guard) = self.alloc_node::<LiteralNode>();
        drop(guard);

        self.node_pool[node].value = match self.previous().0 {
            Token::ConstPi => f64::consts::PI.into(),
            Token::ConstTau => f64::consts::TAU.into(),
            Token::ConstInf => f64::INFINITY.into(),
            Token::ConstNan => f64::NAN.into(),
            _ => unreachable!(),
        };

        Some(node.upcast())
    }

    fn parse_invalid_token(
        &mut self,
        previous_operand: NodeRef<ExpressionNode>,
        _can_assign: bool,
    ) -> Option<NodeRef<ExpressionNode>> {
        // Just for better error messages.
        match self.previous().0 {
            Token::QuestionMark => {
                self.push_error(r#"Unexpected "?" in source. If you want a ternary operator, use "truthy_value if true_condition else falsy_value"."#);
            }
            _ => unreachable!(),
        }

        Some(previous_operand)
    }
}

pub fn parse_to_tokens(tokenizer: &mut dyn Tokenizer) -> color_eyre::Result<Vec<Spanned<Token>>> {
    let mut parser = Parser::new(tokenizer);
    parser.parse();

    if parser.errors.is_empty() {
        Ok(parser.consumed_tokens)
    } else {
        for error in &parser.errors {
            error!("parse error: {} @ {}", &error.message, error.position.start);
        }

        bail!(
            "failed to parse script due to {} errors",
            parser.errors.len()
        );
    }
}
