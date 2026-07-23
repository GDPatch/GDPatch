use ordered_float::OrderedFloat;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Real(OrderedFloat<f64>);

impl From<f64> for Real {
    fn from(value: f64) -> Self {
        Self(OrderedFloat::from(value))
    }
}

impl From<Real> for f64 {
    fn from(value: Real) -> Self {
        value.0.0
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Vector2 {
    pub x: Real,
    pub y: Real,
}

impl Vector2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Vector2i {
    pub x: i32,
    pub y: i32,
}

impl Vector2i {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Rect2 {
    pub position: Vector2,
    pub size: Vector2,
}

impl Rect2 {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self {
            position: Vector2::new(x, y),
            size: Vector2::new(w, h),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Rect2i {
    pub position: Vector2i,
    pub size: Vector2i,
}

impl Rect2i {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            position: Vector2i::new(x, y),
            size: Vector2i::new(w, h),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Vector3 {
    pub x: Real,
    pub y: Real,
    pub z: Real,
}

impl Vector3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Vector3i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Vector3i {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Transform2d {
    pub x: Vector2,
    pub y: Vector2,
    pub origin: Vector2,
}

impl Transform2d {
    pub fn new(x: Vector2, y: Vector2, origin: Vector2) -> Self {
        Self { x, y, origin }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Vector4 {
    pub x: Real,
    pub y: Real,
    pub z: Real,
    pub w: Real,
}

impl Vector4 {
    pub fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
            w: w.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Vector4i {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub w: i32,
}

impl Vector4i {
    pub fn new(x: i32, y: i32, z: i32, w: i32) -> Self {
        Self { x, y, z, w }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Plane {
    pub normal: Vector3,
    pub d: Real,
}

impl Plane {
    pub fn new(nx: f64, ny: f64, nz: f64, d: f64) -> Self {
        Self {
            normal: Vector3::new(nx, ny, nz),
            d: d.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Quaternion {
    pub x: Real,
    pub y: Real,
    pub z: Real,
    pub w: Real,
}

impl Quaternion {
    pub fn new(x: f64, y: f64, z: f64, w: f64) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
            z: z.into(),
            w: w.into(),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Aabb {
    pub position: Vector3,
    pub size: Vector3,
}

impl Aabb {
    pub fn new(position: Vector3, size: Vector3) -> Self {
        Self { position, size }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Basis {
    pub x: Vector3,
    pub y: Vector3,
    pub z: Vector3,
}

impl Basis {
    pub fn new(x: Vector3, y: Vector3, z: Vector3) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Transform3d {
    pub basis: Basis,
    pub origin: Vector3,
}

impl Transform3d {
    pub fn new(basis: Basis, origin: Vector3) -> Self {
        Self { basis, origin }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Projection {
    pub x: Vector4,
    pub y: Vector4,
    pub z: Vector4,
    pub w: Vector4,
}

impl Projection {
    pub fn new(x: Vector4, y: Vector4, z: Vector4, w: Vector4) -> Self {
        Self { x, y, z, w }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Color {
    pub r: OrderedFloat<f32>,
    pub g: OrderedFloat<f32>,
    pub b: OrderedFloat<f32>,
    pub a: OrderedFloat<f32>,
}

impl Color {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: r.into(),
            g: g.into(),
            b: b.into(),
            a: a.into(),
        }
    }
}
