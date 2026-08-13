//! Godot Variant implementation and binary parser.

mod array;
mod dictionary;
mod marshalling;
mod math;
mod node_path;
mod object;
mod rid;
mod signal;
mod string_name;
mod text_parser;

pub use crate::variant::array::Array;
pub use crate::variant::dictionary::Dictionary;
pub use crate::variant::math::{
    Aabb, Basis, Color, Plane, Projection, Quaternion, Real, Rect2, Rect2i, Transform2d,
    Transform3d, Vector2, Vector2i, Vector3, Vector3i, Vector4, Vector4i,
};
pub use crate::variant::node_path::NodePath;
pub use crate::variant::object::{Object, ObjectKind};
pub use crate::variant::rid::Rid;
pub use crate::variant::signal::Signal;
pub use crate::variant::string_name::StringName;
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ContainerType {
    #[default]
    None,
    Builtin(VariantType),
    ClassName(String),
    Script(String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[repr(u32)]
pub enum ContainerTypeKind {
    None = 0b00,
    Builtin = 0b01,
    ClassName = 0b10,
    Script = 0b11,
}

impl TryFrom<u32> for ContainerTypeKind {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(match value {
            0b00 => ContainerTypeKind::None,
            0b01 => ContainerTypeKind::Builtin,
            0b10 => ContainerTypeKind::ClassName,
            0b11 => ContainerTypeKind::Script,
            _ => return Err(()),
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Nil;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Callable;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Deserialize, Serialize)]
#[repr(u8)]
pub enum VariantType {
    Nil,

    // atomic types
    Bool,
    Int,
    Float,
    String,

    // math types
    Vector2,
    Vector2i,
    Rect2,
    Rect2i,
    Vector3,
    Vector3i,
    Transform2d,
    Vector4,
    Vector4i,
    Plane,
    Quaternion,
    Aabb,
    Basis,
    Transform3d,
    Projection,

    // misc types
    Color,
    StringName,
    NodePath,
    Rid,
    Object,
    Callable,
    Signal,
    Dictionary,
    Array,

    // typed arrays
    PackedByteArray,
    PackedInt32Array,
    PackedInt64Array,
    PackedFloat32Array,
    PackedFloat64Array,
    PackedStringArray,
    PackedVector2Array,
    PackedVector3Array,
    PackedColorArray,
    PackedVector4Array,
}

impl TryFrom<u8> for VariantType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(match value {
            0 => Self::Nil,
            1 => Self::Bool,
            2 => Self::Int,
            3 => Self::Float,
            4 => Self::String,
            5 => Self::Vector2,
            6 => Self::Vector2i,
            7 => Self::Rect2,
            8 => Self::Rect2i,
            9 => Self::Vector3,
            10 => Self::Vector3i,
            11 => Self::Transform2d,
            12 => Self::Vector4,
            13 => Self::Vector4i,
            14 => Self::Plane,
            15 => Self::Quaternion,
            16 => Self::Aabb,
            17 => Self::Basis,
            18 => Self::Transform3d,
            19 => Self::Projection,
            20 => Self::Color,
            21 => Self::StringName,
            22 => Self::NodePath,
            23 => Self::Rid,
            24 => Self::Object,
            25 => Self::Callable,
            26 => Self::Signal,
            27 => Self::Dictionary,
            28 => Self::Array,
            29 => Self::PackedByteArray,
            30 => Self::PackedInt32Array,
            31 => Self::PackedInt64Array,
            32 => Self::PackedFloat32Array,
            33 => Self::PackedFloat64Array,
            34 => Self::PackedStringArray,
            35 => Self::PackedVector2Array,
            36 => Self::PackedVector3Array,
            37 => Self::PackedColorArray,
            38 => Self::PackedVector4Array,
            _ => return Err(()),
        })
    }
}

impl Variant {
    pub fn typ(&self) -> VariantType {
        match self {
            Variant::Nil(_) => VariantType::Nil,
            Variant::Bool(_) => VariantType::Bool,
            Variant::Int(_) => VariantType::Int,
            Variant::Float(_) => VariantType::Float,
            Variant::String(_) => VariantType::String,
            Variant::Vector2(_) => VariantType::Vector2,
            Variant::Vector2i(_) => VariantType::Vector2i,
            Variant::Rect2(_) => VariantType::Rect2,
            Variant::Rect2i(_) => VariantType::Rect2i,
            Variant::Vector3(_) => VariantType::Vector3,
            Variant::Vector3i(_) => VariantType::Vector3i,
            Variant::Transform2d(_) => VariantType::Transform2d,
            Variant::Vector4(_) => VariantType::Vector4,
            Variant::Vector4i(_) => VariantType::Vector4i,
            Variant::Plane(_) => VariantType::Plane,
            Variant::Quaternion(_) => VariantType::Quaternion,
            Variant::Aabb(_) => VariantType::Aabb,
            Variant::Basis(_) => VariantType::Basis,
            Variant::Transform3d(_) => VariantType::Transform3d,
            Variant::Projection(_) => VariantType::Projection,
            Variant::Color(_) => VariantType::Color,
            Variant::StringName(_) => VariantType::StringName,
            Variant::NodePath(_) => VariantType::NodePath,
            Variant::Rid(_) => VariantType::Rid,
            Variant::Object(_) => VariantType::Object,
            Variant::Callable(_) => VariantType::Callable,
            Variant::Signal(_) => VariantType::Signal,
            Variant::Dictionary(_) => VariantType::Dictionary,
            Variant::Array(_) => VariantType::Array,
            Variant::PackedByteArray(_) => VariantType::PackedByteArray,
            Variant::PackedInt32Array(_) => VariantType::PackedInt32Array,
            Variant::PackedInt64Array(_) => VariantType::PackedInt64Array,
            Variant::PackedFloat32Array(_) => VariantType::PackedFloat32Array,
            Variant::PackedFloat64Array(_) => VariantType::PackedFloat64Array,
            Variant::PackedStringArray(_) => VariantType::PackedStringArray,
            Variant::PackedVector2Array(_) => VariantType::PackedVector2Array,
            Variant::PackedVector3Array(_) => VariantType::PackedVector3Array,
            Variant::PackedColorArray(_) => VariantType::PackedColorArray,
            Variant::PackedVector4Array(_) => VariantType::PackedVector4Array,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Variant {
    Nil(Nil),

    // atomic types
    Bool(bool),
    Int(i64),
    Float(OrderedFloat<f64>),
    String(Cow<'static, str>),

    // math types
    Vector2(Vector2),
    Vector2i(Vector2i),
    Rect2(Rect2),
    Rect2i(Rect2i),
    Vector3(Vector3),
    Vector3i(Vector3i),
    Transform2d(Transform2d),
    Vector4(Vector4),
    Vector4i(Vector4i),
    Plane(Plane),
    Quaternion(Quaternion),
    Aabb(Aabb),
    Basis(Basis),
    Transform3d(Transform3d),
    Projection(Projection),

    // misc types
    Color(Color),
    StringName(StringName), // TODO: intern this?
    NodePath(NodePath),
    Rid(Rid),
    Object(ObjectKind),
    Callable(Callable),
    Signal(Signal),
    Dictionary(Dictionary),
    Array(Array),

    // typed arrays
    PackedByteArray(Vec<u8>),
    PackedInt32Array(Vec<i32>),
    PackedInt64Array(Vec<i64>),
    PackedFloat32Array(Vec<OrderedFloat<f32>>),
    PackedFloat64Array(Vec<OrderedFloat<f64>>),
    PackedStringArray(Vec<String>),
    PackedVector2Array(Vec<Vector2>),
    PackedVector3Array(Vec<Vector3>),
    PackedColorArray(Vec<Color>),
    PackedVector4Array(Vec<Vector4>),
}

impl Default for Variant {
    fn default() -> Self {
        Self::Nil(Nil)
    }
}

impl Display for Variant {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Variant::Nil(_) => write!(f, "null"),
            Variant::Bool(b) => write!(f, "{}", b),
            Variant::Int(v) => write!(f, "{}", v),
            Variant::Float(float) => write!(f, "{:?}", float),
            Variant::String(s) => write!(f, "\"{}\"", s.escape_debug()),
            // Variant::Vector2 => {}
            // Variant::Vector2i => {}
            // Variant::Rect2 => {}
            // Variant::Rect2i => {}
            // Variant::Vector3 => {}
            // Variant::Vector3i => {}
            // Variant::Transform2d => {}
            // Variant::Vector4 => {}
            // Variant::Vector4i => {}
            // Variant::Plane => {}
            // Variant::Quaternion => {}
            // Variant::Aabb => {}
            // Variant::Basis => {}
            // Variant::Transform3d => {}
            // Variant::Projection => {}
            // Variant::Color => {}
            Variant::StringName(name) => write!(f, "\"{}\"", name.0),
            Variant::NodePath(path) => write!(f, "\"{}\"", path),
            // Variant::Rid => {}
            // Variant::Object => {}
            // Variant::Callable => {}
            // Variant::Signal => {}
            // Variant::Dictionary => {}
            // Variant::Array => {}
            // Variant::PackedByteArray => {}
            // Variant::PackedInt32Array => {}
            // Variant::PackedInt64Array => {}
            // Variant::PackedFloat32Array => {}
            // Variant::PackedFloat64Array => {}
            // Variant::PackedStringArray => {}
            // Variant::PackedVector2Array => {}
            // Variant::PackedVector3Array => {}
            // Variant::PackedColorArray => {}
            // Variant::PackedVector4Array => {}
            _ => todo!(),
        }
    }
}

impl From<Nil> for Variant {
    fn from(value: Nil) -> Self {
        Self::Nil(value)
    }
}

// atomic types
impl From<bool> for Variant {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for Variant {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<f64> for Variant {
    fn from(value: f64) -> Self {
        Self::Float(OrderedFloat(value))
    }
}

impl From<&'static str> for Variant {
    fn from(value: &'static str) -> Self {
        Self::String(Cow::Borrowed(value))
    }
}

impl From<String> for Variant {
    fn from(value: String) -> Self {
        Self::String(Cow::Owned(value))
    }
}

impl From<Cow<'static, str>> for Variant {
    fn from(value: Cow<'static, str>) -> Self {
        Self::String(value)
    }
}

// math types
impl From<Vector2> for Variant {
    fn from(value: Vector2) -> Self {
        Self::Vector2(value)
    }
}

impl From<Vector2i> for Variant {
    fn from(value: Vector2i) -> Self {
        Self::Vector2i(value)
    }
}

impl From<Rect2> for Variant {
    fn from(value: Rect2) -> Self {
        Self::Rect2(value)
    }
}

impl From<Rect2i> for Variant {
    fn from(value: Rect2i) -> Self {
        Self::Rect2i(value)
    }
}

impl From<Vector3> for Variant {
    fn from(value: Vector3) -> Self {
        Self::Vector3(value)
    }
}

impl From<Vector3i> for Variant {
    fn from(value: Vector3i) -> Self {
        Self::Vector3i(value)
    }
}

impl From<Transform2d> for Variant {
    fn from(value: Transform2d) -> Self {
        Self::Transform2d(value)
    }
}

impl From<Vector4> for Variant {
    fn from(value: Vector4) -> Self {
        Self::Vector4(value)
    }
}

impl From<Vector4i> for Variant {
    fn from(value: Vector4i) -> Self {
        Self::Vector4i(value)
    }
}

impl From<Plane> for Variant {
    fn from(value: Plane) -> Self {
        Self::Plane(value)
    }
}

impl From<Quaternion> for Variant {
    fn from(value: Quaternion) -> Self {
        Self::Quaternion(value)
    }
}

impl From<Aabb> for Variant {
    fn from(value: Aabb) -> Self {
        Self::Aabb(value)
    }
}

impl From<Basis> for Variant {
    fn from(value: Basis) -> Self {
        Self::Basis(value)
    }
}

impl From<Transform3d> for Variant {
    fn from(value: Transform3d) -> Self {
        Self::Transform3d(value)
    }
}

impl From<Projection> for Variant {
    fn from(value: Projection) -> Self {
        Self::Projection(value)
    }
}

// misc types
impl From<Color> for Variant {
    fn from(value: Color) -> Self {
        Self::Color(value)
    }
}

impl From<NodePath> for Variant {
    fn from(value: NodePath) -> Self {
        Self::NodePath(value)
    }
}

impl From<Rid> for Variant {
    fn from(value: Rid) -> Self {
        Self::Rid(value)
    }
}

impl From<ObjectKind> for Variant {
    fn from(value: ObjectKind) -> Self {
        Self::Object(value)
    }
}

impl From<Object> for Variant {
    fn from(value: Object) -> Self {
        Self::Object(value.into())
    }
}

impl From<Dictionary> for Variant {
    fn from(value: Dictionary) -> Self {
        Self::Dictionary(value)
    }
}

impl From<Array> for Variant {
    fn from(value: Array) -> Self {
        Self::Array(value)
    }
}

// typed arrays
impl From<Vec<u8>> for Variant {
    fn from(value: Vec<u8>) -> Self {
        Self::PackedByteArray(value)
    }
}

impl From<Vec<i32>> for Variant {
    fn from(value: Vec<i32>) -> Self {
        Self::PackedInt32Array(value)
    }
}

impl From<Vec<i64>> for Variant {
    fn from(value: Vec<i64>) -> Self {
        Self::PackedInt64Array(value)
    }
}

impl From<Vec<OrderedFloat<f32>>> for Variant {
    fn from(value: Vec<OrderedFloat<f32>>) -> Self {
        Self::PackedFloat32Array(value)
    }
}

impl From<Vec<f32>> for Variant {
    fn from(value: Vec<f32>) -> Self {
        let value = value.into_iter().map(OrderedFloat).collect::<Vec<_>>();

        Self::PackedFloat32Array(value)
    }
}

impl From<Vec<OrderedFloat<f64>>> for Variant {
    fn from(value: Vec<OrderedFloat<f64>>) -> Self {
        Self::PackedFloat64Array(value)
    }
}

impl From<Vec<f64>> for Variant {
    fn from(value: Vec<f64>) -> Self {
        let value = value.into_iter().map(OrderedFloat).collect::<Vec<_>>();

        Self::PackedFloat64Array(value)
    }
}

impl From<Vec<String>> for Variant {
    fn from(value: Vec<String>) -> Self {
        Self::PackedStringArray(value)
    }
}

impl From<Vec<Vector2>> for Variant {
    fn from(value: Vec<Vector2>) -> Self {
        Self::PackedVector2Array(value)
    }
}

impl From<Vec<Vector3>> for Variant {
    fn from(value: Vec<Vector3>) -> Self {
        Self::PackedVector3Array(value)
    }
}

impl From<Vec<Color>> for Variant {
    fn from(value: Vec<Color>) -> Self {
        Self::PackedColorArray(value)
    }
}

impl From<Vec<Vector4>> for Variant {
    fn from(value: Vec<Vector4>) -> Self {
        Self::PackedVector4Array(value)
    }
}

impl From<Signal> for Variant {
    fn from(value: Signal) -> Self {
        Self::Signal(value)
    }
}

impl From<StringName> for Variant {
    fn from(value: StringName) -> Self {
        Self::StringName(value)
    }
}

impl From<Callable> for Variant {
    fn from(value: Callable) -> Self {
        Self::Callable(value)
    }
}
