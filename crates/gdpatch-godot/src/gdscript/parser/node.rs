#![allow(dead_code)]

use crate::gdscript::Span;
use crate::private::Sealed;
use crate::{
    gdscript::Spanned,
    variant::{StringName, Variant},
};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::ops::{Index, IndexMut};

/// Trait implemented by nodes that can have annotations applied to them.
pub trait Annotatable {
    fn annotations(&self) -> &[NodeRef<AnnotationNode>];
    fn annotations_mut(&mut self) -> &mut Vec<NodeRef<AnnotationNode>>;
}

macro_rules! impl_annotatable {
    ($ty:ty) => {
        impl Annotatable for $ty {
            fn annotations(&self) -> &[NodeRef<AnnotationNode>] {
                &self.annotations
            }

            fn annotations_mut(&mut self) -> &mut Vec<NodeRef<AnnotationNode>> {
                &mut self.annotations
            }
        }
    };
}

/// Trait implemented by nodes to downcast them from the `Node` enum.
pub trait DowncastFromNode: Sealed + Into<Node> {
    fn downcast(node: &Node) -> Option<&Self>;
    fn downcast_mut(node: &mut Node) -> Option<&mut Self>;
}

/// Arena allocator for node types.
#[derive(Debug, Default)]
pub struct NodePool(Vec<Spanned<Node>>);

impl NodePool {
    /// Allocates a [`Node`] into this node pool and returns a reference to it.
    pub fn push<T>(&mut self, t: Spanned<T>) -> NodeRef<T>
    where
        T: DowncastFromNode,
    {
        let upcast: Node = T::into(t.0);
        self.0.push((upcast, t.1));
        let idx = self.0.len();

        NodeRef {
            idx_plus_one: NonZeroU32::new(idx as u32).unwrap(),
            marker: PhantomData,
        }
    }

    pub fn get<T>(&self, index: NodeRef<T>) -> Spanned<&T>
    where
        T: DowncastFromNode,
    {
        let (node, span) = &self.0[index.idx_plus_one.get() as usize - 1];
        let downcast = T::downcast(node).unwrap();
        (downcast, *span)
    }

    pub fn get_mut<T>(&mut self, index: NodeRef<T>) -> Spanned<&mut T>
    where
        T: DowncastFromNode,
    {
        let (node, span) = &mut self.0[index.idx_plus_one.get() as usize - 1];
        let downcast = T::downcast_mut(node).unwrap();
        (downcast, *span)
    }

    pub fn span<T>(&self, index: NodeRef<T>) -> Span
    where
        T: DowncastFromNode,
    {
        self.get(index).1
    }

    pub fn span_mut<T>(&mut self, index: NodeRef<T>) -> &mut Span
    where
        T: DowncastFromNode,
    {
        let (_, span) = &mut self.0[index.idx_plus_one.get() as usize - 1];
        span
    }

    pub fn try_free<T>(&mut self, index: NodeRef<T>)
    where
        T: DowncastFromNode,
    {
        if index.idx_plus_one.get() as usize == self.0.len() {
            self.0.pop();
        }
    }
}

impl<T> Index<NodeRef<T>> for NodePool
where
    T: DowncastFromNode,
{
    type Output = T;

    fn index(&self, index: NodeRef<T>) -> &Self::Output {
        self.get(index).0
    }
}

impl<T> IndexMut<NodeRef<T>> for NodePool
where
    T: DowncastFromNode,
{
    fn index_mut(&mut self, index: NodeRef<T>) -> &mut Self::Output {
        self.get_mut(index).0
    }
}

/// Reference to a node in a node pool.
pub struct NodeRef<T> {
    // for Option<NodeRef<T>> null pointer optimization
    idx_plus_one: NonZeroU32,
    marker: PhantomData<T>,
}

impl<T> NodeRef<T> {
    pub fn upcast<U>(self) -> NodeRef<U>
    where
        T: Into<U>,
        U: DowncastFromNode,
    {
        NodeRef {
            idx_plus_one: self.idx_plus_one,
            marker: PhantomData,
        }
    }

    pub fn downcast<U>(self) -> NodeRef<U>
    where
        T: Into<Node>,
        U: DowncastFromNode,
    {
        NodeRef {
            idx_plus_one: self.idx_plus_one,
            marker: PhantomData,
        }
    }
}

impl<T> Debug for NodeRef<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("NodeRef").field(&self.idx_plus_one).finish()
    }
}

impl<T> Copy for NodeRef<T> {}

impl<T> Clone for NodeRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq<Self> for NodeRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx_plus_one == other.idx_plus_one
    }
}

impl<T> Eq for NodeRef<T> {}

macro_rules! impl_node_enum {
    ($($variant:ident($struct:ident)),*$(,)?) => {
        #[derive(Debug, Clone)]
        pub enum Node {
            $($variant($struct),)*
        }

        $(
        impl Sealed for $struct {}

        impl From<$struct> for Node {
            fn from(value: $struct) -> Self {
                Self::$variant(value)
            }
        }

        impl DowncastFromNode for $struct {
            fn downcast(node: &Node) -> Option<&Self> {
                let Node::$variant(s) = node else { return None };
                Some(s)
            }

            fn downcast_mut(node: &mut Node) -> Option<&mut Self> {
                let Node::$variant(s) = node else { return None };
                Some(s)
            }
        }
        )*
    };
}

impl_node_enum! {
    Expr(ExpressionNode),

    Annotation(AnnotationNode),
    Assert(AssertNode),
    Break(BreakNode),
    Breakpoint(BreakpointNode),
    Enum(EnumNode),
    Class(ClassNode),
    Constant(ConstantNode),
    Continue(ContinueNode),
    For(ForNode),
    Function(FunctionNode),
    If(IfNode),
    Match(MatchNode),
    MatchBranch(MatchBranchNode),
    Parameter(ParameterNode),
    Pass(PassNode),
    Pattern(PatternNode),
    Return(ReturnNode),
    Signal(SignalNode),
    Suite(SuiteNode),
    Type(TypeNode),
    Variable(VariableNode),
    While(WhileNode),
}

impl Sealed for Node {}

impl DowncastFromNode for Node {
    fn downcast(node: &Node) -> Option<&Self> {
        Some(node)
    }

    fn downcast_mut(node: &mut Node) -> Option<&mut Self> {
        Some(node)
    }
}

macro_rules! impl_expression_node_enum {
    ($($variant:ident($struct:ident)),*$(,)?) => {
        #[derive(Debug, Clone)]
        pub enum ExpressionNode {
            $($variant($struct),)*
        }

        $(
        impl Sealed for $struct {}

        impl From<$struct> for Node {
            fn from(value: $struct) -> Self {
                Self::Expr(ExpressionNode::$variant(value))
            }
        }

        impl From<$struct> for ExpressionNode {
            fn from(value: $struct) -> Self {
                Self::$variant(value)
            }
        }

        impl DowncastFromNode for $struct {
            fn downcast(node: &Node) -> Option<&Self> {
                let Node::Expr(ExpressionNode::$variant(s)) = node else { return None };
                Some(s)
            }

            fn downcast_mut(node: &mut Node) -> Option<&mut Self> {
                let Node::Expr(ExpressionNode::$variant(s)) = node else { return None };
                Some(s)
            }
        }
        )*
    };
}

impl_expression_node_enum! {
    Array(ArrayNode),
    Assignment(AssignmentNode),
    Await(AwaitNode),
    BinaryOp(BinaryOpNode),
    Call(CallNode),
    Cast(CastNode),
    Dictionary(DictionaryNode),
    GetNode(GetNodeNode),
    Identifier(IdentifierNode),
    Lambda(LambdaNode),
    Literal(LiteralNode),
    Preload(PreloadNode),
    _Self(SelfNode),
    SubScript(SubscriptNode),
    TernaryOp(TernaryOpNode),
    TypeTest(TypeTestNode),
    UnaryOp(UnaryOpNode),
}

#[derive(Debug, Clone, Default)]
pub struct AssignableNode {
    pub identifier: Option<NodeRef<IdentifierNode>>,
    pub initializer: Option<NodeRef<ExpressionNode>>,
    pub expression: Option<NodeRef<ExpressionNode>>,
    pub datatype_specifier: Option<NodeRef<TypeNode>>,
    pub infer_datatype: bool,
    pub use_conversion_assign: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AnnotationNode {
    pub name: StringName,
    pub arguments: Vec<NodeRef<ExpressionNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct ArrayNode {
    pub elements: Vec<NodeRef<ExpressionNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct AssertNode {
    pub condition: Option<NodeRef<ExpressionNode>>,
    pub message: Option<NodeRef<ExpressionNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct AssignmentNode {
    pub operation: AssignmentOperation,
    pub assignee: Option<NodeRef<ExpressionNode>>,
    pub assigned_value: Option<NodeRef<ExpressionNode>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum AssignmentOperation {
    #[default]
    None,
    Addition,
    Subtraction,
    Multiplication,
    Division,
    Modulo,
    Power,
    BitShiftLeft,
    BitShiftRight,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Debug, Clone, Default)]
pub struct AwaitNode {
    pub to_await: Option<NodeRef<ExpressionNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct BinaryOpNode {
    pub operation: BinaryOperation,
    pub left: Option<NodeRef<ExpressionNode>>,
    pub right: Option<NodeRef<ExpressionNode>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum BinaryOperation {
    #[default]
    Addition,
    Subtraction,
    Multiplication,
    Division,
    Modulo,
    Power,
    BitLeftShift,
    BitRightShift,
    BitAnd,
    BitOr,
    BitXor,
    LogicAnd,
    LogicOr,
    ContentTest,
    CompEqual,
    CompNotEqual,
    CompLess,
    CompLessEqual,
    CompGreater,
    CompGreaterEqual,
}

#[derive(Debug, Clone, Default)]
pub struct BreakNode;

#[derive(Debug, Clone, Default)]
pub struct BreakpointNode;

#[derive(Debug, Clone, Default)]
pub struct CallNode {
    pub is_super: bool,
    pub function_name: StringName,
    pub callee: Option<NodeRef<ExpressionNode>>,
    pub arguments: Vec<NodeRef<ExpressionNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct CastNode {
    pub operand: Option<NodeRef<ExpressionNode>>,
    pub cast_type: Option<NodeRef<TypeNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct EnumNode {
    pub identifier: Option<NodeRef<IdentifierNode>>,
    pub values: Vec<EnumValue>,
}

#[derive(Debug, Clone, Default)]
pub struct EnumValue {
    pub identifier: Option<NodeRef<IdentifierNode>>,
    pub parent_enum: Option<NodeRef<EnumNode>>,
    pub custom_value: Option<NodeRef<ExpressionNode>>,
    pub index: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ClassNode {
    pub annotations: Vec<NodeRef<AnnotationNode>>,
    pub identifier: Option<NodeRef<IdentifierNode>>,
    pub members: Vec<NodeRef<ClassMember>>,
    pub extends: Vec<NodeRef<IdentifierNode>>,
    pub outer: Option<NodeRef<ClassNode>>,
    pub has_static_data: bool,
    pub extends_used: bool,
    pub extends_path: Option<StringName>,
}

impl_annotatable!(ClassNode);

#[derive(Debug, Clone, Default)]
pub enum ClassMember {
    #[default]
    Undefined,
    Class(Box<ClassNode>),
    Constant(Box<ConstantNode>),
    Function(Box<FunctionNode>),
    Signal(Box<SignalNode>),
    Variable(Box<VariableNode>),
    Enum(Box<EnumNode>),
    EnumValue(EnumValue),
    Group(Box<AnnotationNode>),
}

#[derive(Debug, Clone, Default)]
pub struct ConstantNode(pub AssignableNode);

#[derive(Debug, Clone, Default)]
pub struct ContinueNode;

#[derive(Debug, Clone, Default)]
pub struct DictionaryNode {
    pub style: DictionaryStyle,
    pub elements: Vec<(NodeRef<ExpressionNode>, NodeRef<ExpressionNode>)>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum DictionaryStyle {
    LuaTable,
    #[default]
    PythonDict,
}

#[derive(Debug, Clone, Default)]
pub struct ForNode {
    pub variable: Option<NodeRef<IdentifierNode>>,
    pub datatype_specifier: Option<NodeRef<TypeNode>>,
    pub use_conversion_assign: bool,
    pub list: Option<NodeRef<ExpressionNode>>,
    pub loop_suite: Option<NodeRef<SuiteNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct FunctionNode {
    pub identifier: Option<NodeRef<IdentifierNode>>,
    pub parameters: IndexMap<StringName, NodeRef<ParameterNode>>,
    pub rest_parameter: Option<NodeRef<ParameterNode>>,
    pub return_type: Option<NodeRef<TypeNode>>,
    pub body_suite: Option<NodeRef<SuiteNode>>,
    pub is_abstract: bool,
    pub is_static: bool,
    pub is_coroutine: bool,
    pub source_lambda: Option<NodeRef<LambdaNode>>,
}

#[derive(Debug, Clone)]
pub struct GetNodeNode {
    pub full_path: String,
    pub use_dollar: bool,
}

impl Default for GetNodeNode {
    fn default() -> Self {
        Self {
            full_path: Default::default(),
            use_dollar: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IdentifierNode {
    pub name: StringName,
}

#[derive(Debug, Clone, Default)]
pub struct IfNode {
    pub condition: Option<NodeRef<ExpressionNode>>,
    pub true_block: Option<NodeRef<SuiteNode>>,
    pub false_block: Option<NodeRef<SuiteNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct LambdaNode {
    pub function: Option<NodeRef<FunctionNode>>,
    pub captures: IndexMap<StringName, NodeRef<IdentifierNode>>,
    pub use_self: bool,
    pub parent_function: Option<NodeRef<FunctionNode>>,
    pub parent_lambda: Option<NodeRef<LambdaNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct LiteralNode {
    pub value: Variant,
}

#[derive(Debug, Clone, Default)]
pub struct MatchNode {
    pub test: Option<NodeRef<ExpressionNode>>,
    pub branches: Vec<NodeRef<MatchBranchNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct MatchBranchNode {
    pub patterns: Vec<NodeRef<PatternNode>>,
    pub block: Option<NodeRef<SuiteNode>>,
    pub has_wildcard: bool,
    pub guard_body: Option<NodeRef<SuiteNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct ParameterNode(pub AssignableNode);

#[derive(Debug, Clone, Default)]
pub struct PassNode;

#[derive(Debug, Clone, Default)]
pub struct PatternNode {
    pub pattern_type: Option<PatternType>,
    pub rest_used: bool,
    pub binds: HashMap<StringName, NodeRef<IdentifierNode>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternType {
    Literal(NodeRef<LiteralNode>),
    Expression(NodeRef<ExpressionNode>),
    Bind(NodeRef<IdentifierNode>),
    Array(ArrayPattern),
    Dictionary(DictionaryPattern),
    Rest,
    Wildcard,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArrayPattern {
    pub elements: Vec<NodeRef<PatternNode>>,
    pub rest_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DictionaryPattern {
    pub elements: Vec<DictionaryPatternEntry>,
    pub rest_used: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DictionaryPatternEntry {
    pub key: Option<NodeRef<ExpressionNode>>,
    pub value: Option<NodeRef<PatternNode>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PreloadNode {
    pub path: Option<NodeRef<ExpressionNode>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ReturnNode {
    pub value: Option<NodeRef<ExpressionNode>>,
    pub void_return: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SelfNode;

#[derive(Debug, Clone, Default)]
pub struct SignalNode {
    pub identifier: Option<NodeRef<IdentifierNode>>,
    pub parameters: IndexMap<StringName, NodeRef<ParameterNode>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SubscriptNode {
    pub base: Option<NodeRef<ExpressionNode>>,
    pub inner: Option<SubscriptNodeInner>,
}

#[derive(Debug, Copy, Clone)]
pub enum SubscriptNodeInner {
    Index(NodeRef<ExpressionNode>),
    Attribute(NodeRef<IdentifierNode>),
}

#[derive(Debug, Clone, Default)]
pub struct SuiteNode {
    pub parent_block: Option<NodeRef<SuiteNode>>,
    pub statements: Vec<NodeRef<Node>>,
    pub empty: Option<SuiteLocal>,
    pub locals: IndexMap<StringName, SuiteLocal>,

    pub parent_function: Option<NodeRef<FunctionNode>>,
    pub parent_if: Option<NodeRef<IfNode>>,

    pub has_return: bool,
    pub has_continue: bool,
    pub has_unreachable_code: bool, // Just so warnings aren't given more than once per block.
    pub is_in_loop: bool,           // The block is nested in a loop (directly or indirectly).
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SuiteLocal {
    pub typ: SuiteLocalType,
    pub source_function: Option<NodeRef<FunctionNode>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum SuiteLocalType {
    #[default]
    Undefined,
    Constant(NodeRef<ConstantNode>),
    Variable(NodeRef<VariableNode>),
    Parameter(NodeRef<ParameterNode>),
    ForVariable,
    Bind(NodeRef<IdentifierNode>),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TernaryOpNode {
    pub condition: Option<NodeRef<ExpressionNode>>,
    pub if_true: Option<NodeRef<ExpressionNode>>,
    pub if_false: Option<NodeRef<ExpressionNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct TypeNode {
    pub type_chain: Vec<NodeRef<IdentifierNode>>,
    pub container_types: Vec<NodeRef<TypeNode>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TypeTestNode {
    pub operand: Option<NodeRef<ExpressionNode>>,
    pub test_type: Option<NodeRef<TypeNode>>,
    // pub test_datatype: DataType,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct UnaryOpNode {
    pub operation: UnaryOperation,
    pub operand: Option<NodeRef<ExpressionNode>>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default)]
pub enum UnaryOperation {
    #[default]
    Positive,
    Negate,
    Complement,
    LogicalNot,
}

#[derive(Debug, Clone, Default)]
pub struct VariableNode {
    pub base: AssignableNode,
    pub property: Property,
    pub is_static: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Property {
    #[default]
    None,
    Inline(InlineProperty),
    SetGet(SetGetProperty),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct InlineProperty {
    pub getter: Option<NodeRef<FunctionNode>>,
    pub setter: SetterProperty,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SetterProperty {
    pub function: Option<NodeRef<FunctionNode>>,
    pub parameter: Option<NodeRef<IdentifierNode>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SetGetProperty {
    pub getter: Option<NodeRef<IdentifierNode>>,
    pub setter: Option<NodeRef<IdentifierNode>>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WhileNode {
    pub condition: Option<NodeRef<ExpressionNode>>,
    pub loop_suite: Option<NodeRef<SuiteNode>>,
}
