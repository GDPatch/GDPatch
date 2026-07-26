//! Lua bindings for Godot variants.

// TODO:
// - expose arrays properly
// - implement extra methods (e.g. array stuff, maybe things like vector helper methods?)

use gdpatch_godot::variant::{
    Aabb, Array, Basis, Callable, Color, ContainerType, Dictionary, Nil, NodePath, Object,
    ObjectKind, Plane, Projection, Quaternion, Rect2, Rect2i, Rid, Signal, StringName, Transform2d,
    Transform3d, Variant, Vector2, Vector2i, Vector3, Vector3i, Vector4, Vector4i,
};
use mlua::{AnyUserData, FromLua, IntoLua, Lua, LuaSerdeExt, UserData, Value};
use ordered_float::OrderedFloat;
use std::{collections::BTreeMap, str::FromStr};

pub fn register_module(lua: &Lua) -> mlua::Result<()> {
    let table = lua.create_table()?;

    table.set(LuaVector2::NAME, lua.create_proxy::<LuaVector2>()?)?;
    table.set(LuaVector2i::NAME, lua.create_proxy::<LuaVector2i>()?)?;
    table.set(LuaRect2::NAME, lua.create_proxy::<LuaRect2>()?)?;
    table.set(LuaRect2i::NAME, lua.create_proxy::<LuaRect2i>()?)?;
    table.set(LuaVector3::NAME, lua.create_proxy::<LuaVector3>()?)?;
    table.set(LuaVector3i::NAME, lua.create_proxy::<LuaVector3i>()?)?;
    table.set(LuaTransform2d::NAME, lua.create_proxy::<LuaTransform2d>()?)?;
    table.set(LuaVector4::NAME, lua.create_proxy::<LuaVector4>()?)?;
    table.set(LuaVector4i::NAME, lua.create_proxy::<LuaVector4i>()?)?;
    table.set(LuaPlane::NAME, lua.create_proxy::<LuaPlane>()?)?;
    table.set(LuaQuaternion::NAME, lua.create_proxy::<LuaQuaternion>()?)?;
    table.set(LuaAabb::NAME, lua.create_proxy::<LuaAabb>()?)?;
    table.set(LuaBasis::NAME, lua.create_proxy::<LuaBasis>()?)?;
    table.set(LuaTransform3d::NAME, lua.create_proxy::<LuaTransform3d>()?)?;
    table.set(LuaProjection::NAME, lua.create_proxy::<LuaProjection>()?)?;
    table.set(LuaColor::NAME, lua.create_proxy::<LuaColor>()?)?;
    table.set(LuaStringName::NAME, lua.create_proxy::<LuaStringName>()?)?;
    table.set(LuaNodePath::NAME, lua.create_proxy::<LuaNodePath>()?)?;
    table.set(LuaRid::NAME, lua.create_proxy::<LuaRid>()?)?;
    table.set(LuaObject::NAME, lua.create_proxy::<LuaObject>()?)?;
    table.set(LuaCallable::NAME, lua.create_proxy::<LuaCallable>()?)?;
    table.set(LuaSignal::NAME, lua.create_proxy::<LuaSignal>()?)?;
    table.set(LuaDictionary::NAME, lua.create_proxy::<LuaDictionary>()?)?;
    table.set(LuaArray::NAME, lua.create_proxy::<LuaArray>()?)?;
    table.set(
        LuaPackedByteArray::NAME,
        lua.create_proxy::<LuaPackedByteArray>()?,
    )?;
    table.set(
        LuaPackedInt32Array::NAME,
        lua.create_proxy::<LuaPackedInt32Array>()?,
    )?;
    table.set(
        LuaPackedInt64Array::NAME,
        lua.create_proxy::<LuaPackedInt64Array>()?,
    )?;
    table.set(
        LuaPackedFloat32Array::NAME,
        lua.create_proxy::<LuaPackedFloat32Array>()?,
    )?;
    table.set(
        LuaPackedFloat64Array::NAME,
        lua.create_proxy::<LuaPackedFloat64Array>()?,
    )?;
    table.set(
        LuaPackedStringArray::NAME,
        lua.create_proxy::<LuaPackedStringArray>()?,
    )?;
    table.set(
        LuaPackedVector2Array::NAME,
        lua.create_proxy::<LuaPackedVector2Array>()?,
    )?;
    table.set(
        LuaPackedVector3Array::NAME,
        lua.create_proxy::<LuaPackedVector3Array>()?,
    )?;
    table.set(
        LuaPackedColorArray::NAME,
        lua.create_proxy::<LuaPackedColorArray>()?,
    )?;
    table.set(
        LuaPackedVector4Array::NAME,
        lua.create_proxy::<LuaPackedVector4Array>()?,
    )?;

    lua.register_module("gdpatch.variant", table)?;
    Ok(())
}

#[derive(Debug, Clone, UserData)]
pub struct LuaVector2(pub Vector2);

#[mlua::userdata_impl]
impl LuaVector2 {
    const NAME: &str = "Vector2";

    #[lua(meta, infallible)]
    fn __call(_: Value, x: f64, y: f64) -> Self {
        Self(Vector2 {
            x: x.into(),
            y: y.into(),
        })
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "x", infallible)]
    fn get_x(&self) -> f64 {
        self.0.x.into()
    }

    #[lua(setter, name = "x", infallible)]
    fn set_x(&mut self, value: f64) {
        self.0.x = value.into();
    }

    #[lua(getter, name = "y", infallible)]
    fn get_y(&self) -> f64 {
        self.0.y.into()
    }

    #[lua(setter, name = "y", infallible)]
    fn set_y(&mut self, value: f64) {
        self.0.y = value.into();
    }
}

#[derive(Debug, UserData)]
pub struct LuaVector2i(pub Vector2i);

#[mlua::userdata_impl]
impl LuaVector2i {
    const NAME: &str = "Vector2i";

    #[lua(meta, infallible)]
    fn __call(_: Value, x: i32, y: i32) -> Self {
        Self(Vector2i { x, y })
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "x", infallible)]
    fn get_x(&self) -> i32 {
        self.0.x
    }

    #[lua(setter, name = "x", infallible)]
    fn set_x(&mut self, value: i32) {
        self.0.x = value;
    }

    #[lua(getter, name = "y", infallible)]
    fn get_y(&self) -> i32 {
        self.0.y
    }

    #[lua(setter, name = "y", infallible)]
    fn set_y(&mut self, value: i32) {
        self.0.y = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaRect2 {
    position: AnyUserData,
    size: AnyUserData,
}

impl LuaRect2 {
    pub fn new(lua: &Lua, value: Rect2) -> mlua::Result<Self> {
        let position = lua.create_userdata(LuaVector2(value.position))?;
        let size = lua.create_userdata(LuaVector2(value.size))?;
        Ok(Self { position, size })
    }
}

impl TryFrom<LuaRect2> for Rect2 {
    type Error = mlua::Error;

    fn try_from(value: LuaRect2) -> Result<Self, Self::Error> {
        let position = value.position.borrow::<LuaVector2>()?.0;
        let size = value.size.borrow::<LuaVector2>()?.0;
        Ok(Self { position, size })
    }
}

#[mlua::userdata_impl]
impl LuaRect2 {
    const NAME: &str = "Rect2";

    #[lua(meta, infallible)]
    fn __call(_: Value, position: AnyUserData, size: AnyUserData) -> Self {
        Self { position, size }
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "position", infallible)]
    fn get_position(&self) -> AnyUserData {
        self.position.clone()
    }

    #[lua(setter, name = "position", infallible)]
    fn set_position(&mut self, value: AnyUserData) {
        self.position = value;
    }

    #[lua(getter, name = "size", infallible)]
    fn get_size(&self) -> AnyUserData {
        self.size.clone()
    }

    #[lua(setter, name = "size", infallible)]
    fn set_size(&mut self, value: AnyUserData) {
        self.size = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaRect2i {
    position: AnyUserData,
    size: AnyUserData,
}

impl LuaRect2i {
    pub fn new(lua: &Lua, value: Rect2i) -> mlua::Result<Self> {
        let position = lua.create_userdata(LuaVector2i(value.position))?;
        let size = lua.create_userdata(LuaVector2i(value.size))?;
        Ok(Self { position, size })
    }
}

impl TryFrom<LuaRect2i> for Rect2i {
    type Error = mlua::Error;

    fn try_from(value: LuaRect2i) -> Result<Self, Self::Error> {
        let position = value.position.borrow::<LuaVector2i>()?.0;
        let size = value.size.borrow::<LuaVector2i>()?.0;
        Ok(Self { position, size })
    }
}

#[mlua::userdata_impl]
impl LuaRect2i {
    const NAME: &str = "Rect2i";

    #[lua(meta, infallible)]
    fn __call(_: Value, position: AnyUserData, size: AnyUserData) -> Self {
        Self { position, size }
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "position", infallible)]
    fn get_position(&self) -> AnyUserData {
        self.position.clone()
    }

    #[lua(setter, name = "position", infallible)]
    fn set_position(&mut self, value: AnyUserData) {
        self.position = value;
    }

    #[lua(getter, name = "size", infallible)]
    fn get_size(&self) -> AnyUserData {
        self.size.clone()
    }

    #[lua(setter, name = "size", infallible)]
    fn set_size(&mut self, value: AnyUserData) {
        self.size = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaVector3(pub Vector3);

#[mlua::userdata_impl]
impl LuaVector3 {
    const NAME: &str = "Vector3";

    #[lua(meta, infallible)]
    fn __call(_: Value, x: f64, y: f64, z: f64) -> Self {
        Self(Vector3 {
            x: x.into(),
            y: y.into(),
            z: z.into(),
        })
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "x", infallible)]
    fn get_x(&self) -> f64 {
        self.0.x.into()
    }

    #[lua(setter, name = "x", infallible)]
    fn set_x(&mut self, value: f64) {
        self.0.x = value.into();
    }

    #[lua(getter, name = "y", infallible)]
    fn get_y(&self) -> f64 {
        self.0.y.into()
    }

    #[lua(setter, name = "y", infallible)]
    fn set_y(&mut self, value: f64) {
        self.0.y = value.into();
    }

    #[lua(getter, name = "z", infallible)]
    fn get_z(&self) -> f64 {
        self.0.z.into()
    }

    #[lua(setter, name = "z", infallible)]
    fn set_z(&mut self, value: f64) {
        self.0.z = value.into();
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaVector3i(pub Vector3i);

#[mlua::userdata_impl]
impl LuaVector3i {
    const NAME: &str = "Vector3i";

    #[lua(meta, infallible)]
    fn __call(_: Value, x: i32, y: i32, z: i32) -> Self {
        Self(Vector3i { x, y, z })
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "x", infallible)]
    fn get_x(&self) -> i32 {
        self.0.x
    }

    #[lua(setter, name = "x", infallible)]
    fn set_x(&mut self, value: i32) {
        self.0.x = value;
    }

    #[lua(getter, name = "y", infallible)]
    fn get_y(&self) -> i32 {
        self.0.y
    }

    #[lua(setter, name = "y", infallible)]
    fn set_y(&mut self, value: i32) {
        self.0.y = value;
    }

    #[lua(getter, name = "z", infallible)]
    fn get_z(&self) -> i32 {
        self.0.z
    }

    #[lua(setter, name = "z", infallible)]
    fn set_z(&mut self, value: i32) {
        self.0.z = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaTransform2d {
    pub x: AnyUserData,
    pub y: AnyUserData,
    pub origin: AnyUserData,
}

impl LuaTransform2d {
    pub fn new(lua: &Lua, value: Transform2d) -> mlua::Result<Self> {
        let x = lua.create_userdata(LuaVector2(value.x))?;
        let y = lua.create_userdata(LuaVector2(value.y))?;
        let origin = lua.create_userdata(LuaVector2(value.origin))?;
        Ok(Self { x, y, origin })
    }
}

impl TryFrom<LuaTransform2d> for Transform2d {
    type Error = mlua::Error;

    fn try_from(value: LuaTransform2d) -> Result<Self, Self::Error> {
        let x = value.x.borrow::<LuaVector2>()?.0;
        let y = value.y.borrow::<LuaVector2>()?.0;
        let origin = value.origin.borrow::<LuaVector2>()?.0;
        Ok(Self { x, y, origin })
    }
}

#[mlua::userdata_impl]
impl LuaTransform2d {
    const NAME: &str = "Transform2D";

    #[lua(meta, infallible)]
    fn __call(_: Value, x: AnyUserData, y: AnyUserData, origin: AnyUserData) -> Self {
        Self { x, y, origin }
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "x", infallible)]
    fn get_x(&self) -> AnyUserData {
        self.x.clone()
    }

    #[lua(setter, name = "x", infallible)]
    fn set_x(&mut self, value: AnyUserData) {
        self.x = value;
    }

    #[lua(getter, name = "y", infallible)]
    fn get_y(&self) -> AnyUserData {
        self.y.clone()
    }

    #[lua(setter, name = "y", infallible)]
    fn set_y(&mut self, value: AnyUserData) {
        self.y = value;
    }

    #[lua(getter, name = "origin", infallible)]
    fn get_origin(&self) -> AnyUserData {
        self.origin.clone()
    }

    #[lua(setter, name = "origin", infallible)]
    fn set_origin(&mut self, value: AnyUserData) {
        self.origin = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaVector4(pub Vector4);

#[mlua::userdata_impl]
impl LuaVector4 {
    const NAME: &str = "Vector4";

    #[lua(meta, infallible)]
    fn __call(_: Value, x: f64, y: f64, z: f64, w: f64) -> Self {
        Self(Vector4 {
            x: x.into(),
            y: y.into(),
            z: z.into(),
            w: w.into(),
        })
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "x", infallible)]
    fn get_x(&self) -> f64 {
        self.0.x.into()
    }

    #[lua(setter, name = "x", infallible)]
    fn set_x(&mut self, value: f64) {
        self.0.x = value.into();
    }

    #[lua(getter, name = "y", infallible)]
    fn get_y(&self) -> f64 {
        self.0.y.into()
    }

    #[lua(setter, name = "y", infallible)]
    fn set_y(&mut self, value: f64) {
        self.0.y = value.into();
    }

    #[lua(getter, name = "z", infallible)]
    fn get_z(&self) -> f64 {
        self.0.z.into()
    }

    #[lua(setter, name = "z", infallible)]
    fn set_z(&mut self, value: f64) {
        self.0.z = value.into();
    }

    #[lua(getter, name = "w", infallible)]
    fn get_w(&self) -> f64 {
        self.0.w.into()
    }

    #[lua(setter, name = "w", infallible)]
    fn set_w(&mut self, value: f64) {
        self.0.w = value.into();
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaVector4i(pub Vector4i);

#[mlua::userdata_impl]
impl LuaVector4i {
    const NAME: &str = "Vector4i";

    #[lua(meta, infallible)]
    fn __call(_: Value, x: i32, y: i32, z: i32, w: i32) -> Self {
        Self(Vector4i { x, y, z, w })
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "x", infallible)]
    fn get_x(&self) -> i32 {
        self.0.x
    }

    #[lua(setter, name = "x", infallible)]
    fn set_x(&mut self, value: i32) {
        self.0.x = value;
    }

    #[lua(getter, name = "y", infallible)]
    fn get_y(&self) -> i32 {
        self.0.y
    }

    #[lua(setter, name = "y", infallible)]
    fn set_y(&mut self, value: i32) {
        self.0.y = value;
    }

    #[lua(getter, name = "z", infallible)]
    fn get_z(&self) -> i32 {
        self.0.z
    }

    #[lua(setter, name = "z", infallible)]
    fn set_z(&mut self, value: i32) {
        self.0.z = value;
    }

    #[lua(getter, name = "w", infallible)]
    fn get_w(&self) -> i32 {
        self.0.w
    }

    #[lua(setter, name = "w", infallible)]
    fn set_w(&mut self, value: i32) {
        self.0.w = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPlane {
    pub normal: AnyUserData,
    pub d: f64,
}

impl LuaPlane {
    pub fn new(lua: &Lua, value: Plane) -> mlua::Result<Self> {
        let normal = lua.create_userdata(LuaVector3(value.normal))?;
        Ok(Self {
            normal,
            d: value.d.into(),
        })
    }
}

impl TryFrom<LuaPlane> for Plane {
    type Error = mlua::Error;

    fn try_from(value: LuaPlane) -> Result<Self, Self::Error> {
        let normal = value.normal.borrow::<LuaVector3>()?.0;
        Ok(Self {
            normal,
            d: value.d.into(),
        })
    }
}

#[mlua::userdata_impl]
impl LuaPlane {
    const NAME: &str = "Plane";

    #[lua(meta, infallible)]
    fn __call(_: Value, normal: AnyUserData, d: f64) -> Self {
        Self { normal, d }
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "normal", infallible)]
    fn get_normal(&self) -> AnyUserData {
        self.normal.clone()
    }

    #[lua(setter, name = "normal", infallible)]
    fn set_normal(&mut self, value: AnyUserData) {
        self.normal = value;
    }

    #[lua(getter, name = "d", infallible)]
    fn get_d(&self) -> f64 {
        self.d
    }

    #[lua(setter, name = "d", infallible)]
    fn set_d(&mut self, value: f64) {
        self.d = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaQuaternion(pub Quaternion);

#[mlua::userdata_impl]
impl LuaQuaternion {
    const NAME: &str = "Quaternion";

    #[lua(meta, infallible)]
    fn __call(_: Value, x: f64, y: f64, z: f64, w: f64) -> Self {
        Self(Quaternion {
            x: x.into(),
            y: y.into(),
            z: z.into(),
            w: w.into(),
        })
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "x", infallible)]
    fn get_x(&self) -> f64 {
        self.0.x.into()
    }

    #[lua(setter, name = "x", infallible)]
    fn set_x(&mut self, value: f64) {
        self.0.x = value.into();
    }

    #[lua(getter, name = "y", infallible)]
    fn get_y(&self) -> f64 {
        self.0.y.into()
    }

    #[lua(setter, name = "y", infallible)]
    fn set_y(&mut self, value: f64) {
        self.0.y = value.into();
    }

    #[lua(getter, name = "z", infallible)]
    fn get_z(&self) -> f64 {
        self.0.z.into()
    }

    #[lua(setter, name = "z", infallible)]
    fn set_z(&mut self, value: f64) {
        self.0.z = value.into();
    }

    #[lua(getter, name = "w", infallible)]
    fn get_w(&self) -> f64 {
        self.0.w.into()
    }

    #[lua(setter, name = "w", infallible)]
    fn set_w(&mut self, value: f64) {
        self.0.w = value.into();
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaAabb {
    pub position: AnyUserData,
    pub size: AnyUserData,
}

impl LuaAabb {
    pub fn new(lua: &Lua, value: Aabb) -> mlua::Result<Self> {
        let position = lua.create_userdata(LuaVector3(value.position))?;
        let size = lua.create_userdata(LuaVector3(value.size))?;
        Ok(Self { position, size })
    }
}

impl TryFrom<LuaAabb> for Aabb {
    type Error = mlua::Error;

    fn try_from(value: LuaAabb) -> Result<Self, Self::Error> {
        let position = value.position.borrow::<LuaVector3>()?.0;
        let size = value.size.borrow::<LuaVector3>()?.0;
        Ok(Self { position, size })
    }
}

#[mlua::userdata_impl]
impl LuaAabb {
    const NAME: &str = "AABB";

    #[lua(meta, infallible)]
    fn __call(_: Value, position: AnyUserData, size: AnyUserData) -> Self {
        Self { position, size }
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "position", infallible)]
    fn get_position(&self) -> AnyUserData {
        self.position.clone()
    }

    #[lua(setter, name = "position", infallible)]
    fn set_position(&mut self, value: AnyUserData) {
        self.position = value;
    }

    #[lua(getter, name = "size", infallible)]
    fn get_size(&self) -> AnyUserData {
        self.size.clone()
    }

    #[lua(setter, name = "size", infallible)]
    fn set_size(&mut self, value: AnyUserData) {
        self.size = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaBasis {
    pub x: AnyUserData,
    pub y: AnyUserData,
    pub z: AnyUserData,
}

impl LuaBasis {
    pub fn new(lua: &Lua, value: Basis) -> mlua::Result<Self> {
        let x = lua.create_userdata(LuaVector3(value.x))?;
        let y = lua.create_userdata(LuaVector3(value.y))?;
        let z = lua.create_userdata(LuaVector3(value.z))?;
        Ok(Self { x, y, z })
    }
}

impl TryFrom<LuaBasis> for Basis {
    type Error = mlua::Error;

    fn try_from(value: LuaBasis) -> Result<Self, Self::Error> {
        let x = value.x.borrow::<LuaVector3>()?.0;
        let y = value.y.borrow::<LuaVector3>()?.0;
        let z = value.y.borrow::<LuaVector3>()?.0;
        Ok(Self { x, y, z })
    }
}

#[mlua::userdata_impl]
impl LuaBasis {
    const NAME: &str = "Basis";

    #[lua(meta, infallible)]
    fn __call(_: Value, x: AnyUserData, y: AnyUserData, z: AnyUserData) -> Self {
        Self { x, y, z }
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "x", infallible)]
    fn get_x(&self) -> AnyUserData {
        self.x.clone()
    }

    #[lua(setter, name = "x", infallible)]
    fn set_x(&mut self, value: AnyUserData) {
        self.x = value;
    }

    #[lua(getter, name = "y", infallible)]
    fn get_y(&self) -> AnyUserData {
        self.y.clone()
    }

    #[lua(setter, name = "y", infallible)]
    fn set_y(&mut self, value: AnyUserData) {
        self.y = value;
    }

    #[lua(getter, name = "z", infallible)]
    fn get_z(&self) -> AnyUserData {
        self.z.clone()
    }

    #[lua(setter, name = "z", infallible)]
    fn set_z(&mut self, value: AnyUserData) {
        self.z = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaTransform3d {
    pub basis: AnyUserData,
    pub origin: AnyUserData,
}

impl LuaTransform3d {
    pub fn new(lua: &Lua, value: Transform3d) -> mlua::Result<Self> {
        let basis = lua.create_userdata(LuaBasis::new(lua, value.basis)?)?;
        let origin = lua.create_userdata(LuaVector3(value.origin))?;
        Ok(Self { basis, origin })
    }
}

impl TryFrom<LuaTransform3d> for Transform3d {
    type Error = mlua::Error;

    fn try_from(value: LuaTransform3d) -> Result<Self, Self::Error> {
        let basis = value.basis.borrow::<LuaBasis>()?.clone().try_into()?;
        let origin = value.origin.borrow::<LuaVector3>()?.0;
        Ok(Self { basis, origin })
    }
}

#[mlua::userdata_impl]
impl LuaTransform3d {
    const NAME: &str = "Transform3D";

    #[lua(meta, infallible)]
    fn __call(_: Value, basis: AnyUserData, origin: AnyUserData) -> Self {
        Self { basis, origin }
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "basis", infallible)]
    fn get_basis(&self) -> AnyUserData {
        self.basis.clone()
    }

    #[lua(setter, name = "basis", infallible)]
    fn set_basis(&mut self, value: AnyUserData) {
        self.basis = value;
    }

    #[lua(getter, name = "origin", infallible)]
    fn get_origin(&self) -> AnyUserData {
        self.origin.clone()
    }

    #[lua(setter, name = "origin", infallible)]
    fn set_origin(&mut self, value: AnyUserData) {
        self.origin = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaProjection {
    pub x: AnyUserData,
    pub y: AnyUserData,
    pub z: AnyUserData,
    pub w: AnyUserData,
}

impl LuaProjection {
    pub fn new(lua: &Lua, value: Projection) -> mlua::Result<Self> {
        let x = lua.create_userdata(LuaVector4(value.x))?;
        let y = lua.create_userdata(LuaVector4(value.y))?;
        let z = lua.create_userdata(LuaVector4(value.z))?;
        let w = lua.create_userdata(LuaVector4(value.w))?;
        Ok(Self { x, y, z, w })
    }
}

impl TryFrom<LuaProjection> for Projection {
    type Error = mlua::Error;

    fn try_from(value: LuaProjection) -> Result<Self, Self::Error> {
        let x = value.x.borrow::<LuaVector4>()?.0;
        let y = value.y.borrow::<LuaVector4>()?.0;
        let z = value.z.borrow::<LuaVector4>()?.0;
        let w = value.w.borrow::<LuaVector4>()?.0;
        Ok(Self { x, y, z, w })
    }
}

#[mlua::userdata_impl]
impl LuaProjection {
    const NAME: &str = "Projection";

    #[lua(meta, infallible)]
    fn __call(_: Value, x: AnyUserData, y: AnyUserData, z: AnyUserData, w: AnyUserData) -> Self {
        Self { x, y, z, w }
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "x", infallible)]
    fn get_x(&self) -> AnyUserData {
        self.x.clone()
    }

    #[lua(setter, name = "x", infallible)]
    fn set_x(&mut self, value: AnyUserData) {
        self.x = value;
    }

    #[lua(getter, name = "y", infallible)]
    fn get_y(&self) -> AnyUserData {
        self.y.clone()
    }

    #[lua(setter, name = "y", infallible)]
    fn set_y(&mut self, value: AnyUserData) {
        self.y = value;
    }

    #[lua(getter, name = "z", infallible)]
    fn get_z(&self) -> AnyUserData {
        self.z.clone()
    }

    #[lua(setter, name = "z", infallible)]
    fn set_z(&mut self, value: AnyUserData) {
        self.z = value;
    }

    #[lua(getter, name = "w", infallible)]
    fn get_w(&self) -> AnyUserData {
        self.w.clone()
    }

    #[lua(setter, name = "w", infallible)]
    fn set_w(&mut self, value: AnyUserData) {
        self.w = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaColor(pub Color);

#[mlua::userdata_impl]
impl LuaColor {
    const NAME: &str = "Color";

    #[lua(meta, infallible)]
    fn __call(_: Value, r: f32, g: f32, b: f32, a: f32) -> Self {
        Self(Color {
            r: r.into(),
            g: g.into(),
            b: b.into(),
            a: a.into(),
        })
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "r", infallible)]
    fn get_r(&self) -> f32 {
        self.0.r.into()
    }

    #[lua(setter, name = "r", infallible)]
    fn set_r(&mut self, value: f32) {
        self.0.r = value.into();
    }

    #[lua(getter, name = "g", infallible)]
    fn get_g(&self) -> f32 {
        self.0.g.into()
    }

    #[lua(setter, name = "g", infallible)]
    fn set_g(&mut self, value: f32) {
        self.0.g = value.into();
    }

    #[lua(getter, name = "b", infallible)]
    fn get_b(&self) -> f32 {
        self.0.b.into()
    }

    #[lua(setter, name = "b", infallible)]
    fn set_b(&mut self, value: f32) {
        self.0.b = value.into();
    }

    #[lua(getter, name = "a", infallible)]
    fn get_a(&self) -> f32 {
        self.0.a.into()
    }

    #[lua(setter, name = "a", infallible)]
    fn set_a(&mut self, value: f32) {
        self.0.a = value.into();
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaStringName(pub StringName);

#[mlua::userdata_impl]
impl LuaStringName {
    const NAME: &str = "StringName";

    #[lua(meta, infallible)]
    fn __call(_: Value, value: String) -> Self {
        Self(StringName(value.into()))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> String {
        self.0.0.clone().into()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: String) {
        self.0 = StringName(value.into());
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaNodePath(pub NodePath);

#[mlua::userdata_impl]
impl LuaNodePath {
    const NAME: &str = "NodePath";

    #[lua(meta)]
    fn __call(_: Value, value: String) -> mlua::Result<Self> {
        Ok(Self(NodePath::from_str(&value).map_err(|_| {
            mlua::Error::runtime("failed to parse node path")
        })?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> String {
        self.0.to_string()
    }

    #[lua(setter, name = "value")]
    fn set_value(&mut self, value: String) -> mlua::Result<()> {
        self.0 = NodePath::from_str(&value)
            .map_err(|_| mlua::Error::runtime("failed to parse node path"))?;
        Ok(())
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaRid(pub Rid);

#[mlua::userdata_impl]
impl LuaRid {
    const NAME: &str = "RID";

    #[lua(meta)]
    fn __call(_: Value, value: u64) -> mlua::Result<Self> {
        Ok(Self(Rid(value)))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "id", infallible)]
    fn get_id(&self) -> u64 {
        self.0.0
    }

    #[lua(setter, name = "id", infallible)]
    fn set_id(&mut self, value: u64) {
        self.0 = Rid(value);
    }
}

#[derive(Debug, Clone, UserData)]
pub enum LuaObject {
    ObjectId(u64),
    Object {
        class: String,
        properties: mlua::Table,
    },
}

impl LuaObject {
    pub fn new(lua: &Lua, value: ObjectKind) -> mlua::Result<Self> {
        Ok(match value {
            ObjectKind::ObjectId(id) => Self::ObjectId(id),
            ObjectKind::Object(obj) => {
                let converted_properties = lua.create_table()?;
                for (key, value) in obj.properties {
                    converted_properties.set(key, LuaVariant(value))?;
                }

                Self::Object {
                    class: obj.class,
                    properties: converted_properties,
                }
            }
        })
    }
}

impl TryFrom<LuaObject> for ObjectKind {
    type Error = mlua::Error;

    fn try_from(value: LuaObject) -> Result<Self, Self::Error> {
        Ok(match &value {
            LuaObject::ObjectId(id) => Self::ObjectId(*id),
            LuaObject::Object { class, properties } => {
                let mut converted_properties = BTreeMap::new();
                for pair in properties.pairs::<String, LuaVariant>() {
                    let (key, value) = pair?;
                    converted_properties.insert(key, value.0);
                }

                Self::Object(Object {
                    class: class.clone(),
                    properties: converted_properties,
                })
            }
        })
    }
}

#[mlua::userdata_impl]
impl LuaObject {
    const NAME: &str = "Object";

    #[lua(meta)]
    fn __call(
        lua: &Lua,
        _: Value,
        object_id_or_class: mlua::Value,
        properties: Option<mlua::Table>,
    ) -> mlua::Result<Self> {
        if let Some(id) = object_id_or_class.as_u64() {
            Ok(Self::ObjectId(id))
        } else if let Some(class) = object_id_or_class.as_string() {
            Ok(Self::Object {
                class: class.to_str()?.to_string(),
                properties: properties.map_or_else(|| lua.create_table(), Ok)?,
            })
        } else {
            Err(mlua::Error::external("must pass object ID or class name"))
        }
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "id", infallible)]
    fn get_id(&self) -> Option<u64> {
        match &self {
            Self::ObjectId(id) => Some(*id),
            _ => None,
        }
    }

    #[lua(setter, name = "id", infallible)]
    fn set_id(&mut self, value: u64) {
        *self = Self::ObjectId(value);
    }

    #[lua(getter, name = "class", infallible)]
    fn get_class(&self) -> Option<String> {
        match &self {
            Self::Object { class, .. } => Some(class.clone()),
            _ => None,
        }
    }

    #[lua(setter, name = "class")]
    fn set_class(&mut self, lua: &Lua, value: String) -> mlua::Result<()> {
        match self {
            Self::Object { class, .. } => {
                *class = value.clone();
            }
            _ => {
                *self = Self::Object {
                    class: value,
                    properties: lua.create_table()?,
                };
            }
        }

        Ok(())
    }

    #[lua(getter, name = "properties", infallible)]
    fn get_properties(&self) -> Option<mlua::Table> {
        match &self {
            Self::Object { properties, .. } => Some(properties.clone()),
            _ => None,
        }
    }

    #[lua(setter, name = "properties")]
    fn set_properties(&mut self, value: mlua::Table) -> mlua::Result<()> {
        match self {
            Self::Object { properties, .. } => {
                *properties = value;
                Ok(())
            }
            _ => Err(mlua::Error::external(
                "cannot set properties without defined class",
            )),
        }
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaCallable(pub Callable);

#[mlua::userdata_impl]
impl LuaCallable {
    const NAME: &str = "Callable";

    #[lua(meta)]
    fn __call(_: Value) -> mlua::Result<Self> {
        Ok(Self(Callable))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaSignal(pub Signal);

#[mlua::userdata_impl]
impl LuaSignal {
    const NAME: &str = "Signal";

    #[lua(meta)]
    fn __call(_: Value, object_id: u64, name: String) -> mlua::Result<Self> {
        Ok(Self(Signal { name, object_id }))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "object_id", infallible)]
    fn get_object_id(&self) -> u64 {
        self.0.object_id
    }

    #[lua(setter, name = "object_id", infallible)]
    fn set_object_id(&mut self, value: u64) {
        self.0.object_id = value;
    }

    #[lua(getter, name = "name", infallible)]
    fn get_name(&self) -> String {
        self.0.name.clone()
    }

    #[lua(setter, name = "name", infallible)]
    fn set_name(&mut self, value: String) {
        self.0.name = value;
    }
}

// FIXME: clone required for LuaContainerType because we don't have mlua::Lua in TryFrom
#[derive(Debug, Clone)]
pub struct LuaContainerType(pub ContainerType);

impl FromLua for LuaContainerType {
    fn from_lua(value: Value, lua: &Lua) -> mlua::Result<Self> {
        Ok(Self(lua.from_value(value)?))
    }
}

impl IntoLua for LuaContainerType {
    fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
        lua.to_value(&self.0)
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaDictionary {
    pub key_type: LuaContainerType,
    pub value_type: LuaContainerType,
    pub value: mlua::Table,
}

impl LuaDictionary {
    pub fn new(lua: &Lua, value: Dictionary) -> mlua::Result<Self> {
        let converted_value = lua.create_table()?;
        for (key, value) in value.inner {
            converted_value.set(LuaVariant(key), LuaVariant(value))?;
        }

        Ok(Self {
            key_type: LuaContainerType(value.key_type),
            value_type: LuaContainerType(value.value_type),
            value: converted_value,
        })
    }
}

impl TryFrom<LuaDictionary> for Dictionary {
    type Error = mlua::Error;

    fn try_from(value: LuaDictionary) -> Result<Self, Self::Error> {
        let mut converted_value = BTreeMap::new();
        for pair in value.value.pairs::<LuaVariant, LuaVariant>() {
            let (key, value) = pair?;
            converted_value.insert(key.0, value.0);
        }

        Ok(Self {
            key_type: value.key_type.0,
            value_type: value.value_type.0,
            inner: converted_value,
        })
    }
}

#[mlua::userdata_impl]
impl LuaDictionary {
    const NAME: &str = "Dictionary";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value) -> mlua::Result<Self> {
        Ok(Self {
            key_type: LuaContainerType(ContainerType::None),
            value_type: LuaContainerType(ContainerType::None),
            value: lua.create_table()?,
        })
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "key_type", infallible)]
    fn get_key_type(&self) -> LuaContainerType {
        self.key_type.clone()
    }

    #[lua(setter, name = "key_type", infallible)]
    fn set_key_type(&mut self, value: LuaContainerType) {
        self.key_type = value;
    }

    #[lua(getter, name = "value_type", infallible)]
    fn get_value_type(&self) -> LuaContainerType {
        self.value_type.clone()
    }

    #[lua(setter, name = "value_type", infallible)]
    fn set_value_type(&mut self, value: LuaContainerType) {
        self.value_type = value;
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.value.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.value = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaArray {
    pub element_type: LuaContainerType,
    pub value: mlua::Table,
}

impl LuaArray {
    pub fn new(lua: &Lua, value: Array) -> mlua::Result<Self> {
        let converted_value = lua.create_table()?;
        for item in value.inner {
            converted_value.push(LuaVariant(item))?;
        }

        Ok(Self {
            element_type: LuaContainerType(value.element_type),
            value: converted_value,
        })
    }
}

impl TryFrom<LuaArray> for Array {
    type Error = mlua::Error;

    fn try_from(value: LuaArray) -> Result<Self, Self::Error> {
        let mut converted_value = Vec::new();
        for item in value.value.sequence_values::<LuaVariant>() {
            let item = item?;
            converted_value.push(item.0);
        }

        Ok(Self {
            element_type: value.element_type.0,
            inner: converted_value,
        })
    }
}

#[mlua::userdata_impl]
impl LuaArray {
    const NAME: &str = "Array";

    #[lua(meta)]
    fn __call(lua: &Lua, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self {
            element_type: LuaContainerType(ContainerType::None),
            value: value.map_or_else(|| lua.create_table(), Ok)?,
        })
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "element_type", infallible)]
    fn get_element_type(&self) -> LuaContainerType {
        self.element_type.clone()
    }

    #[lua(setter, name = "element_type", infallible)]
    fn set_element_type(&mut self, value: LuaContainerType) {
        self.element_type = value;
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.value.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.value = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPackedByteArray(pub mlua::Table);

impl LuaPackedByteArray {
    pub fn new(lua: &Lua, value: Vec<u8>) -> mlua::Result<Self> {
        Ok(Self(lua.create_sequence_from(value)?))
    }
}

impl TryFrom<LuaPackedByteArray> for Vec<u8> {
    type Error = mlua::Error;

    fn try_from(value: LuaPackedByteArray) -> Result<Self, Self::Error> {
        value
            .0
            .sequence_values::<u8>()
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .collect()
    }
}

#[mlua::userdata_impl]
impl LuaPackedByteArray {
    const NAME: &str = "PackedByteArray";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self(value.map_or_else(|| lua.create_table(), Ok)?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.0.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.0 = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPackedInt32Array(pub mlua::Table);

impl LuaPackedInt32Array {
    pub fn new(lua: &Lua, value: Vec<i32>) -> mlua::Result<Self> {
        Ok(Self(lua.create_sequence_from(value)?))
    }
}

impl TryFrom<LuaPackedInt32Array> for Vec<i32> {
    type Error = mlua::Error;

    fn try_from(value: LuaPackedInt32Array) -> Result<Self, Self::Error> {
        value
            .0
            .sequence_values::<i32>()
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .collect()
    }
}

#[mlua::userdata_impl]
impl LuaPackedInt32Array {
    const NAME: &str = "PackedInt32Array";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self(value.map_or_else(|| lua.create_table(), Ok)?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.0.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.0 = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPackedInt64Array(pub mlua::Table);

impl LuaPackedInt64Array {
    pub fn new(lua: &Lua, value: Vec<i64>) -> mlua::Result<Self> {
        Ok(Self(lua.create_sequence_from(value)?))
    }
}

impl TryFrom<LuaPackedInt64Array> for Vec<i64> {
    type Error = mlua::Error;

    fn try_from(value: LuaPackedInt64Array) -> Result<Self, Self::Error> {
        value
            .0
            .sequence_values::<i64>()
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .collect()
    }
}

#[mlua::userdata_impl]
impl LuaPackedInt64Array {
    const NAME: &str = "PackedInt64Array";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self(value.map_or_else(|| lua.create_table(), Ok)?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.0.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.0 = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPackedFloat32Array(pub mlua::Table);

impl LuaPackedFloat32Array {
    pub fn new(lua: &Lua, value: Vec<OrderedFloat<f32>>) -> mlua::Result<Self> {
        Ok(Self(lua.create_sequence_from(
            value.iter().map(|p| f32::from(*p)),
        )?))
    }
}

impl TryFrom<LuaPackedFloat32Array> for Vec<OrderedFloat<f32>> {
    type Error = mlua::Error;

    fn try_from(value: LuaPackedFloat32Array) -> Result<Self, Self::Error> {
        Ok(value
            .0
            .sequence_values::<f32>()
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .map(|v| (*v).into())
            .collect())
    }
}

#[mlua::userdata_impl]
impl LuaPackedFloat32Array {
    const NAME: &str = "PackedFloat32Array";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self(value.map_or_else(|| lua.create_table(), Ok)?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.0.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.0 = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPackedFloat64Array(pub mlua::Table);

impl LuaPackedFloat64Array {
    pub fn new(lua: &Lua, value: Vec<OrderedFloat<f64>>) -> mlua::Result<Self> {
        Ok(Self(lua.create_sequence_from(
            value.iter().map(|p| f64::from(*p)),
        )?))
    }
}

impl TryFrom<LuaPackedFloat64Array> for Vec<OrderedFloat<f64>> {
    type Error = mlua::Error;

    fn try_from(value: LuaPackedFloat64Array) -> Result<Self, Self::Error> {
        Ok(value
            .0
            .sequence_values::<f64>()
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .map(|v| (*v).into())
            .collect())
    }
}

#[mlua::userdata_impl]
impl LuaPackedFloat64Array {
    const NAME: &str = "PackedFloat64Array";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self(value.map_or_else(|| lua.create_table(), Ok)?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.0.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.0 = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPackedStringArray(pub mlua::Table);

impl LuaPackedStringArray {
    pub fn new(lua: &Lua, value: Vec<String>) -> mlua::Result<Self> {
        Ok(Self(lua.create_sequence_from(value)?))
    }
}

impl TryFrom<LuaPackedStringArray> for Vec<String> {
    type Error = mlua::Error;

    fn try_from(value: LuaPackedStringArray) -> Result<Self, Self::Error> {
        value
            .0
            .sequence_values::<String>()
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .collect()
    }
}

#[mlua::userdata_impl]
impl LuaPackedStringArray {
    const NAME: &str = "PackedStringArray";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self(value.map_or_else(|| lua.create_table(), Ok)?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.0.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.0 = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPackedVector2Array(pub mlua::Table);

impl LuaPackedVector2Array {
    pub fn new(lua: &Lua, value: Vec<Vector2>) -> mlua::Result<Self> {
        Ok(Self(lua.create_sequence_from(
            value.iter().map(|v| LuaVector2(*v)),
        )?))
    }
}

impl TryFrom<LuaPackedVector2Array> for Vec<Vector2> {
    type Error = mlua::Error;

    fn try_from(value: LuaPackedVector2Array) -> Result<Self, Self::Error> {
        value
            .0
            .sequence_values::<AnyUserData>()
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .map(|v| v.and_then(|v| v.borrow::<LuaVector2>()).map(|v| v.0))
            .collect::<Result<Vec<_>, _>>()
    }
}

#[mlua::userdata_impl]
impl LuaPackedVector2Array {
    const NAME: &str = "PackedVector2Array";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self(value.map_or_else(|| lua.create_table(), Ok)?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.0.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.0 = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPackedVector3Array(pub mlua::Table);

impl LuaPackedVector3Array {
    pub fn new(lua: &Lua, value: Vec<Vector3>) -> mlua::Result<Self> {
        Ok(Self(lua.create_sequence_from(
            value.iter().map(|v| LuaVector3(*v)),
        )?))
    }
}

impl TryFrom<LuaPackedVector3Array> for Vec<Vector3> {
    type Error = mlua::Error;

    fn try_from(value: LuaPackedVector3Array) -> Result<Self, Self::Error> {
        value
            .0
            .sequence_values::<AnyUserData>()
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .map(|v| v.and_then(|v| v.borrow::<LuaVector3>()).map(|v| v.0))
            .collect::<Result<Vec<_>, _>>()
    }
}

#[mlua::userdata_impl]
impl LuaPackedVector3Array {
    const NAME: &str = "PackedVector3Array";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self(value.map_or_else(|| lua.create_table(), Ok)?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.0.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.0 = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPackedColorArray(pub mlua::Table);

impl LuaPackedColorArray {
    pub fn new(lua: &Lua, value: Vec<Color>) -> mlua::Result<Self> {
        Ok(Self(lua.create_sequence_from(
            value.iter().map(|v| LuaColor(*v)),
        )?))
    }
}

impl TryFrom<LuaPackedColorArray> for Vec<Color> {
    type Error = mlua::Error;

    fn try_from(value: LuaPackedColorArray) -> Result<Self, Self::Error> {
        value
            .0
            .sequence_values::<AnyUserData>()
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .map(|v| v.and_then(|v| v.borrow::<LuaColor>()).map(|v| v.0))
            .collect::<Result<Vec<_>, _>>()
    }
}

#[mlua::userdata_impl]
impl LuaPackedColorArray {
    const NAME: &str = "PackedColorArray";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self(value.map_or_else(|| lua.create_table(), Ok)?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.0.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.0 = value;
    }
}

#[derive(Debug, Clone, UserData)]
pub struct LuaPackedVector4Array(pub mlua::Table);

impl LuaPackedVector4Array {
    pub fn new(lua: &Lua, value: Vec<Vector4>) -> mlua::Result<Self> {
        Ok(Self(lua.create_sequence_from(
            value.iter().map(|v| LuaVector4(*v)),
        )?))
    }
}

impl TryFrom<LuaPackedVector4Array> for Vec<Vector4> {
    type Error = mlua::Error;

    fn try_from(value: LuaPackedVector4Array) -> Result<Self, Self::Error> {
        value
            .0
            .sequence_values::<AnyUserData>()
            .collect::<Vec<Result<_, _>>>()
            .into_iter()
            .map(|v| v.and_then(|v| v.borrow::<LuaVector4>()).map(|v| v.0))
            .collect::<Result<Vec<_>, _>>()
    }
}

#[mlua::userdata_impl]
impl LuaPackedVector4Array {
    const NAME: &str = "PackedVector4Array";

    #[lua(meta)]
    fn __call(lua: &Lua, _: Value, value: Option<mlua::Table>) -> mlua::Result<Self> {
        Ok(Self(value.map_or_else(|| lua.create_table(), Ok)?))
    }

    #[lua(meta, field, infallible)]
    fn __name() -> String {
        Self::NAME.into()
    }

    #[lua(getter, name = "value", infallible)]
    fn get_value(&self) -> mlua::Table {
        self.0.clone()
    }

    #[lua(setter, name = "value", infallible)]
    fn set_value(&mut self, value: mlua::Table) {
        self.0 = value;
    }
}

#[derive(Debug)]
pub struct LuaVariant(pub Variant);

fn variant_to_lua(lua: &Lua, variant: &Variant) -> mlua::Result<Value> {
    Ok(match variant {
        Variant::Nil(_) => Value::Nil,
        Variant::Bool(v) => Value::Boolean(*v),
        Variant::Int(v) => Value::Integer(*v),
        Variant::Float(v) => Value::Number(v.0),
        Variant::String(v) => Value::String(lua.create_string(v.as_ref())?),

        Variant::Vector2(v) => Value::UserData(lua.create_userdata(LuaVector2(*v))?),
        Variant::Vector2i(v) => Value::UserData(lua.create_userdata(LuaVector2i(*v))?),
        Variant::Rect2(v) => Value::UserData(lua.create_userdata(LuaRect2::new(lua, *v)?)?),
        Variant::Rect2i(v) => Value::UserData(lua.create_userdata(LuaRect2i::new(lua, *v)?)?),
        Variant::Vector3(v) => Value::UserData(lua.create_userdata(LuaVector3(*v))?),
        Variant::Vector3i(v) => Value::UserData(lua.create_userdata(LuaVector3i(*v))?),
        Variant::Transform2d(v) => {
            Value::UserData(lua.create_userdata(LuaTransform2d::new(lua, *v)?)?)
        }
        Variant::Vector4(v) => Value::UserData(lua.create_userdata(LuaVector4(*v))?),
        Variant::Vector4i(v) => Value::UserData(lua.create_userdata(LuaVector4i(*v))?),
        Variant::Plane(v) => Value::UserData(lua.create_userdata(LuaPlane::new(lua, *v)?)?),
        Variant::Quaternion(v) => Value::UserData(lua.create_userdata(LuaQuaternion(*v))?),
        Variant::Aabb(v) => Value::UserData(lua.create_userdata(LuaAabb::new(lua, *v)?)?),
        Variant::Basis(v) => Value::UserData(lua.create_userdata(LuaBasis::new(lua, *v)?)?),
        Variant::Transform3d(v) => {
            Value::UserData(lua.create_userdata(LuaTransform3d::new(lua, *v)?)?)
        }
        Variant::Projection(v) => {
            Value::UserData(lua.create_userdata(LuaProjection::new(lua, *v)?)?)
        }
        Variant::Color(v) => Value::UserData(lua.create_userdata(LuaColor(*v))?),
        Variant::StringName(v) => Value::UserData(lua.create_userdata(LuaStringName(v.clone()))?),
        Variant::NodePath(v) => Value::UserData(lua.create_userdata(LuaNodePath(v.clone()))?),
        Variant::Rid(v) => Value::UserData(lua.create_userdata(LuaRid(v.clone()))?),
        Variant::Object(v) => {
            Value::UserData(lua.create_userdata(LuaObject::new(lua, v.clone())?)?)
        }
        Variant::Callable(v) => Value::UserData(lua.create_userdata(LuaCallable(*v))?),
        Variant::Signal(v) => Value::UserData(lua.create_userdata(LuaSignal(v.clone()))?),
        Variant::Dictionary(v) => {
            Value::UserData(lua.create_userdata(LuaDictionary::new(lua, v.clone())?)?)
        }
        Variant::Array(v) => Value::UserData(lua.create_userdata(LuaArray::new(lua, v.clone())?)?),
        Variant::PackedByteArray(v) => {
            Value::UserData(lua.create_userdata(LuaPackedByteArray::new(lua, v.clone())?)?)
        }
        Variant::PackedInt32Array(v) => {
            Value::UserData(lua.create_userdata(LuaPackedInt32Array::new(lua, v.clone())?)?)
        }
        Variant::PackedInt64Array(v) => {
            Value::UserData(lua.create_userdata(LuaPackedInt64Array::new(lua, v.clone())?)?)
        }
        Variant::PackedFloat32Array(v) => {
            Value::UserData(lua.create_userdata(LuaPackedFloat32Array::new(lua, v.clone())?)?)
        }
        Variant::PackedFloat64Array(v) => {
            Value::UserData(lua.create_userdata(LuaPackedFloat64Array::new(lua, v.clone())?)?)
        }
        Variant::PackedStringArray(v) => {
            Value::UserData(lua.create_userdata(LuaPackedStringArray::new(lua, v.clone())?)?)
        }
        Variant::PackedVector2Array(v) => {
            Value::UserData(lua.create_userdata(LuaPackedVector2Array::new(lua, v.clone())?)?)
        }
        Variant::PackedVector3Array(v) => {
            Value::UserData(lua.create_userdata(LuaPackedVector3Array::new(lua, v.clone())?)?)
        }
        Variant::PackedColorArray(v) => {
            Value::UserData(lua.create_userdata(LuaPackedColorArray::new(lua, v.clone())?)?)
        }
        Variant::PackedVector4Array(v) => {
            Value::UserData(lua.create_userdata(LuaPackedVector4Array::new(lua, v.clone())?)?)
        }
    })
}

impl IntoLua for LuaVariant {
    fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
        variant_to_lua(lua, &self.0)
    }
}

impl FromLua for LuaVariant {
    fn from_lua(value: Value, _lua: &Lua) -> mlua::Result<Self> {
        let inner = match value {
            Value::Nil => Variant::Nil(Nil),
            Value::Boolean(v) => Variant::Bool(v),
            Value::Integer(v) => Variant::Int(v), // maybe should improve this to make integer/float creation clearer?
            Value::Number(v) => Variant::Float(v.into()),
            Value::String(v) => Variant::String(v.to_str()?.to_string().into()),

            Value::UserData(ud) => {
                if let Ok(vector2) = ud.borrow::<LuaVector2>() {
                    Variant::Vector2(vector2.0)
                } else if let Ok(vector2i) = ud.borrow::<LuaVector2i>() {
                    Variant::Vector2i(vector2i.0)
                } else if let Ok(rect2) = ud.borrow::<LuaRect2>() {
                    Variant::Rect2(rect2.clone().try_into()?)
                } else if let Ok(rect2i) = ud.borrow::<LuaRect2i>() {
                    Variant::Rect2i(rect2i.clone().try_into()?)
                } else if let Ok(vector3) = ud.borrow::<LuaVector3>() {
                    Variant::Vector3(vector3.0)
                } else if let Ok(vector3i) = ud.borrow::<LuaVector3i>() {
                    Variant::Vector3i(vector3i.0)
                } else if let Ok(transform2d) = ud.borrow::<LuaTransform2d>() {
                    Variant::Transform2d(transform2d.clone().try_into()?)
                } else if let Ok(vector4) = ud.borrow::<LuaVector4>() {
                    Variant::Vector4(vector4.0)
                } else if let Ok(vector4i) = ud.borrow::<LuaVector4i>() {
                    Variant::Vector4i(vector4i.0)
                } else if let Ok(plane) = ud.borrow::<LuaPlane>() {
                    Variant::Plane(plane.clone().try_into()?)
                } else if let Ok(quaternion) = ud.borrow::<LuaQuaternion>() {
                    Variant::Quaternion(quaternion.0)
                } else if let Ok(aabb) = ud.borrow::<LuaAabb>() {
                    Variant::Aabb(aabb.clone().try_into()?)
                } else if let Ok(basis) = ud.borrow::<LuaBasis>() {
                    Variant::Basis(basis.clone().try_into()?)
                } else if let Ok(transform3d) = ud.borrow::<LuaTransform3d>() {
                    Variant::Transform3d(transform3d.clone().try_into()?)
                } else if let Ok(projection) = ud.borrow::<LuaProjection>() {
                    Variant::Projection(projection.clone().try_into()?)
                } else if let Ok(color) = ud.borrow::<LuaColor>() {
                    Variant::Color(color.0)
                } else if let Ok(string_name) = ud.borrow::<LuaStringName>() {
                    Variant::StringName(string_name.0.clone())
                } else if let Ok(node_path) = ud.borrow::<LuaNodePath>() {
                    Variant::NodePath(node_path.0.clone())
                } else if let Ok(rid) = ud.borrow::<LuaRid>() {
                    Variant::Rid(rid.0.clone())
                } else if let Ok(object_kind) = ud.borrow::<LuaObject>() {
                    Variant::Object(object_kind.clone().try_into()?)
                } else if let Ok(callable) = ud.borrow::<LuaCallable>() {
                    Variant::Callable(callable.0)
                } else if let Ok(signal) = ud.borrow::<LuaSignal>() {
                    Variant::Signal(signal.0.clone())
                } else if let Ok(dictionary) = ud.borrow::<LuaDictionary>() {
                    Variant::Dictionary(dictionary.clone().try_into()?)
                } else if let Ok(array) = ud.borrow::<LuaArray>() {
                    Variant::Array(array.clone().try_into()?)
                } else if let Ok(byte_array) = ud.borrow::<LuaPackedByteArray>() {
                    Variant::PackedByteArray(byte_array.clone().try_into()?)
                } else if let Ok(int32_array) = ud.borrow::<LuaPackedInt32Array>() {
                    Variant::PackedInt32Array(int32_array.clone().try_into()?)
                } else if let Ok(int64_array) = ud.borrow::<LuaPackedInt64Array>() {
                    Variant::PackedInt64Array(int64_array.clone().try_into()?)
                } else if let Ok(float32_array) = ud.borrow::<LuaPackedFloat32Array>() {
                    Variant::PackedFloat32Array(float32_array.clone().try_into()?)
                } else if let Ok(float64_array) = ud.borrow::<LuaPackedFloat64Array>() {
                    Variant::PackedFloat64Array(float64_array.clone().try_into()?)
                } else if let Ok(string_array) = ud.borrow::<LuaPackedStringArray>() {
                    Variant::PackedStringArray(string_array.clone().try_into()?)
                } else if let Ok(vector2_array) = ud.borrow::<LuaPackedVector2Array>() {
                    Variant::PackedVector2Array(vector2_array.clone().try_into()?)
                } else if let Ok(vector3_array) = ud.borrow::<LuaPackedVector3Array>() {
                    Variant::PackedVector3Array(vector3_array.clone().try_into()?)
                } else if let Ok(color_array) = ud.borrow::<LuaPackedColorArray>() {
                    Variant::PackedColorArray(color_array.clone().try_into()?)
                } else if let Ok(vector4_array) = ud.borrow::<LuaPackedVector4Array>() {
                    Variant::PackedVector4Array(vector4_array.clone().try_into()?)
                } else {
                    return Err(mlua::Error::runtime("unknown variant type"));
                }
            }
            _ => {
                return Err(mlua::Error::runtime(
                    "unsupported type passed to Variant conversion",
                ));
            }
        };

        Ok(Self(inner))
    }
}
