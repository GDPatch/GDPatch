use crate::Error::{BadData, TooShort};
use crate::marshalling::{
    ENCODED_OBJECT_AS_ID_CLASS_NAME, ReadableMarshalBuffer, WritableMarshalBuffer,
};
use crate::variant::{
    Aabb, Array, Basis, Callable, Color, Dictionary, Object, ObjectKind, Plane, Projection,
    Quaternion, Rect2, Rect2i, Signal, StringName, Transform2d, Transform3d, Vector2, Vector2i,
    Vector3, Vector3i, Vector4, Vector4i,
};
use crate::variant::{ContainerType, ContainerTypeKind, Rid};
use crate::variant::{Nil, NodePath, Variant, VariantType};
use std::borrow::Cow;

fn decode_vector2(buf: &mut ReadableMarshalBuffer<'_>, as_64: bool) -> crate::Result<Vector2> {
    let x = buf.decode_real(as_64)?;
    let y = buf.decode_real(as_64)?;
    Ok(Vector2::new(x, y))
}

fn encode_vector2(buf: &mut WritableMarshalBuffer, v: Vector2) {
    buf.encode_real(v.x);
    buf.encode_real(v.y);
}

fn decode_vector3(buf: &mut ReadableMarshalBuffer<'_>, as_64: bool) -> crate::Result<Vector3> {
    let x = buf.decode_real(as_64)?;
    let y = buf.decode_real(as_64)?;
    let z = buf.decode_real(as_64)?;
    Ok(Vector3::new(x, y, z))
}

fn encode_vector3(buf: &mut WritableMarshalBuffer, v: Vector3) {
    buf.encode_real(v.x);
    buf.encode_real(v.y);
    buf.encode_real(v.z);
}

fn decode_vector4(buf: &mut ReadableMarshalBuffer<'_>, as_64: bool) -> crate::Result<Vector4> {
    let x = buf.decode_real(as_64)?;
    let y = buf.decode_real(as_64)?;
    let z = buf.decode_real(as_64)?;
    let w = buf.decode_real(as_64)?;
    Ok(Vector4::new(x, y, z, w))
}

fn encode_vector4(buf: &mut WritableMarshalBuffer, v: Vector4) {
    buf.encode_real(v.x);
    buf.encode_real(v.y);
    buf.encode_real(v.z);
    buf.encode_real(v.w);
}

fn decode_basis(buf: &mut ReadableMarshalBuffer<'_>, as_64: bool) -> crate::Result<Basis> {
    let r0 = decode_vector3(buf, as_64)?;
    let r1 = decode_vector3(buf, as_64)?;
    let r2 = decode_vector3(buf, as_64)?;
    Ok(Basis::new(r0, r1, r2))
}

fn decode_color(buf: &mut ReadableMarshalBuffer<'_>) -> crate::Result<Color> {
    let r = buf.decode_float()?;
    let g = buf.decode_float()?;
    let b = buf.decode_float()?;
    let a = buf.decode_float()?;
    Ok(Color::new(r, g, b, a))
}

fn encode_color(buf: &mut WritableMarshalBuffer, v: Color) {
    buf.encode_float(v.r.0);
    buf.encode_float(v.g.0);
    buf.encode_float(v.b.0);
    buf.encode_float(v.a.0);
}

fn decode_string(buf: &mut ReadableMarshalBuffer<'_>) -> crate::Result<String> {
    let len = buf.decode_uint32()?;
    let mut pad = 0;

    // Handle padding.
    if (len % 4) != 0 {
        pad = 4 - len % 4;
    }

    buf.mark();

    // Ensure buffer is big enough.
    let total_len = match len.checked_add(pad) {
        Some(n) => {
            if n as usize > buf.remaining() {
                buf.reset_to_mark();
                return Err(TooShort);
            } else {
                n
            }
        }
        None => {
            buf.reset_to_mark();
            return Err(BadData);
        }
    };

    let str = match str::from_utf8(&buf.buffer()[..len as usize]) {
        Ok(s) => s.to_owned(),
        Err(_) => {
            buf.reset_to_mark();
            return Err(BadData);
        }
    };

    buf.advance(total_len as usize);
    buf.mark();

    Ok(str)
}

fn decode_packed<T, F>(
    buf: &mut ReadableMarshalBuffer<'_>,
    bytes_per_element: usize,
    f: F,
) -> crate::Result<Vec<T>>
where
    F: Fn(&mut ReadableMarshalBuffer<'_>) -> crate::Result<T>,
{
    let count = buf.decode_uint32()? as usize;

    let Some(count_bytes) = count.checked_mul(bytes_per_element) else {
        buf.reset_to_mark();
        return Err(TooShort);
    };

    buf.ensure_remaining(count_bytes)?;
    let mut elements = Vec::with_capacity(count_bytes);

    for _ in 0..count {
        let v = f(buf)?;
        elements.push(v);
    }

    buf.mark();
    Ok(elements)
}

impl Variant {
    const MAX_RECURSION_DEPTH: usize = 1024;

    // Byte 0: `Variant::Type`, byte 1: unused, bytes 2 and 3: additional data.
    const HEADER_TYPE_MASK: u32 = 0xFF;

    // For `Variant::INT`, `Variant::FLOAT` and other math types.
    const HEADER_DATA_FLAG_64: u32 = 1 << 16;

    // For `Variant::OBJECT`.
    const HEADER_DATA_FLAG_OBJECT_AS_ID: u32 = 1 << 16;

    // For `Variant::ARRAY`.
    // Occupies bits 16 and 17.
    const HEADER_DATA_FIELD_TYPED_ARRAY_MASK: u32 = 0b11 << 16;
    const HEADER_DATA_FIELD_TYPED_ARRAY_SHIFT: usize = 16;

    // For `Variant::DICTIONARY`.
    // Occupies bits 16 and 17.
    const HEADER_DATA_FIELD_TYPED_DICTIONARY_KEY_MASK: u32 = 0b11 << 16;
    const HEADER_DATA_FIELD_TYPED_DICTIONARY_KEY_SHIFT: usize = 16;

    // Occupies bits 18 and 19.
    const HEADER_DATA_FIELD_TYPED_DICTIONARY_VALUE_MASK: u32 = 0b11 << 18;
    const HEADER_DATA_FIELD_TYPED_DICTIONARY_VALUE_SHIFT: usize = 18;

    /// Decodes a variant from a buffer. The buffer parameter will be modified to the start of
    /// remaining data.
    pub fn decode(buf: &mut ReadableMarshalBuffer<'_>, allow_objects: bool) -> crate::Result<Self> {
        Self::decode_inner(buf, allow_objects, 0)
    }

    fn get_container_type_kind(header: u32, mask: u32, shift: usize) -> ContainerTypeKind {
        let kind = (header & mask) >> shift;
        ContainerTypeKind::try_from(kind)
            .expect("mask passed to get_container_type_kind is incorrect")
    }

    fn decode_container_type(
        buf: &mut ReadableMarshalBuffer<'_>,
        kind: ContainerTypeKind,
        allow_objects: bool,
    ) -> crate::Result<ContainerType> {
        Ok(match kind {
            ContainerTypeKind::None => ContainerType::None,
            ContainerTypeKind::Builtin => {
                let builtin_type = buf.decode_uint32()?;
                buf.mark();

                if builtin_type > u8::MAX as u32 {
                    return Err(BadData);
                }

                let builtin_type =
                    VariantType::try_from(builtin_type as u8).map_err(|_| BadData)?;

                if builtin_type == VariantType::Object {
                    return Err(BadData);
                }

                ContainerType::Builtin(builtin_type)
            }
            ContainerTypeKind::ClassName => {
                let name = decode_string(buf)?;

                ContainerType::ClassName(if allow_objects {
                    name.to_owned()
                } else {
                    ENCODED_OBJECT_AS_ID_CLASS_NAME.to_owned()
                })
            }
            ContainerTypeKind::Script => {
                let name = decode_string(buf)?;

                ContainerType::Script(if allow_objects {
                    name.to_owned()
                } else {
                    ENCODED_OBJECT_AS_ID_CLASS_NAME.to_owned()
                })
            }
        })
    }

    fn decode_inner(
        buf: &mut ReadableMarshalBuffer<'_>,
        allow_objects: bool,
        depth: usize,
    ) -> crate::Result<Self> {
        if depth >= Self::MAX_RECURSION_DEPTH {
            return Err(BadData);
        }

        let header = buf.decode_uint32()?;
        let as_64 = (header & Self::HEADER_DATA_FLAG_64) != 0;
        let bytes_per_real = if as_64 { 8 } else { 4 };

        let Ok(typ) = VariantType::try_from((header & Self::HEADER_TYPE_MASK) as u8) else {
            buf.reset_to_mark();
            return Err(BadData);
        };

        buf.mark();

        Ok(match typ {
            VariantType::Nil => Variant::Nil(Nil),
            VariantType::Bool => {
                let val = buf.decode_uint32()?;
                buf.mark();

                match val {
                    0 => Variant::Bool(false),
                    1 => Variant::Bool(true),
                    _ => return Err(BadData),
                }
            }
            VariantType::Int => {
                if (header & Self::HEADER_DATA_FLAG_64) != 0 {
                    let value = buf.decode_uint64()?;
                    buf.mark();

                    Variant::Int(value as i64)
                } else {
                    let value = buf.decode_uint32()?;
                    buf.mark();

                    Variant::Int(value as i32 as i64)
                }
            }
            VariantType::Float => {
                let value = buf.decode_real(as_64)?;
                buf.mark();
                Variant::Float(value.into())
            }
            VariantType::String => {
                let str = decode_string(buf)?;
                Variant::String(Cow::Owned(str.to_owned()))
            }
            VariantType::Vector2 => {
                let vec = decode_vector2(buf, as_64)?;
                buf.mark();
                vec.into()
            }
            VariantType::Vector2i => {
                let x = buf.decode_uint32()? as i32;
                let y = buf.decode_uint32()? as i32;
                buf.mark();

                Vector2i::new(x, y).into()
            }
            VariantType::Rect2 => {
                let x = buf.decode_real(as_64)?;
                let y = buf.decode_real(as_64)?;
                let w = buf.decode_real(as_64)?;
                let h = buf.decode_real(as_64)?;
                buf.mark();
                Rect2::new(x, y, w, h).into()
            }
            VariantType::Rect2i => {
                let x = buf.decode_uint32()? as i32;
                let y = buf.decode_uint32()? as i32;
                let w = buf.decode_uint32()? as i32;
                let h = buf.decode_uint32()? as i32;
                buf.mark();
                Rect2i::new(x, y, w, h).into()
            }
            VariantType::Vector3 => {
                let vec = decode_vector3(buf, as_64)?;
                buf.mark();
                vec.into()
            }
            VariantType::Vector3i => {
                let x = buf.decode_uint32()? as i32;
                let y = buf.decode_uint32()? as i32;
                let z = buf.decode_uint32()? as i32;
                buf.mark();
                Vector3i::new(x, y, z).into()
            }
            VariantType::Vector4 => {
                let vec = decode_vector4(buf, as_64)?;
                buf.mark();
                vec.into()
            }
            VariantType::Vector4i => {
                let x = buf.decode_uint32()? as i32;
                let y = buf.decode_uint32()? as i32;
                let z = buf.decode_uint32()? as i32;
                let w = buf.decode_uint32()? as i32;
                buf.mark();
                Vector4i::new(x, y, z, w).into()
            }
            VariantType::Transform2d => {
                let x = decode_vector2(buf, as_64)?;
                let y = decode_vector2(buf, as_64)?;
                let origin = decode_vector2(buf, as_64)?;
                buf.mark();
                Transform2d::new(x, y, origin).into()
            }
            VariantType::Plane => {
                let nx = buf.decode_real(as_64)?;
                let ny = buf.decode_real(as_64)?;
                let nz = buf.decode_real(as_64)?;
                let d = buf.decode_real(as_64)?;
                buf.mark();
                Plane::new(nx, ny, nz, d).into()
            }
            VariantType::Quaternion => {
                let x = buf.decode_real(as_64)?;
                let y = buf.decode_real(as_64)?;
                let z = buf.decode_real(as_64)?;
                let w = buf.decode_real(as_64)?;
                buf.mark();
                Quaternion::new(x, y, z, w).into()
            }
            VariantType::Aabb => {
                let position = decode_vector3(buf, as_64)?;
                let size = decode_vector3(buf, as_64)?;

                buf.mark();
                Aabb::new(position, size).into()
            }
            VariantType::Basis => {
                let basis = decode_basis(buf, as_64)?;
                buf.mark();
                basis.into()
            }
            VariantType::Transform3d => {
                let basis = decode_basis(buf, as_64)?;
                let origin = decode_vector3(buf, as_64)?;
                buf.mark();
                Transform3d::new(basis, origin).into()
            }
            VariantType::Projection => {
                let x = decode_vector4(buf, as_64)?;
                let y = decode_vector4(buf, as_64)?;
                let z = decode_vector4(buf, as_64)?;
                let w = decode_vector4(buf, as_64)?;
                buf.mark();
                Projection::new(x, y, z, w).into()
            }
            VariantType::Color => {
                let color = decode_color(buf)?;
                buf.mark();
                color.into()
            }
            VariantType::StringName => {
                let str = decode_string(buf)?;
                Variant::StringName(str.into())
            }
            VariantType::NodePath => {
                let len = buf.decode_uint32()?;

                if (len & 0x80000000) != 0 {
                    let name_count = len & 0x7FFFFFFF;
                    let mut sub_name_count = buf.decode_uint32()?;
                    let np_flags = buf.decode_uint32()?;
                    buf.mark();

                    if (np_flags & 2) != 0 {
                        // Obsolete format with property separate from subpath.
                        sub_name_count += 1;
                    }

                    let total = name_count + sub_name_count;

                    let mut names = Vec::with_capacity(name_count as usize);
                    let mut sub_names = Vec::with_capacity(sub_name_count as usize);

                    for i in 0..total {
                        let str = decode_string(buf)?;

                        if i < name_count {
                            names.push(str.to_owned());
                        } else {
                            sub_names.push(str.to_owned());
                        }
                    }

                    let is_absolute = (np_flags & 1) != 0;
                    Variant::NodePath(NodePath::new(names, sub_names, is_absolute))
                } else {
                    // TODO: support old format node paths
                    return Err(BadData);
                }
            }
            VariantType::Rid => {
                let id = buf.decode_uint64()?;
                buf.mark();
                Rid(id).into()
            }
            VariantType::Object => {
                if (header & Self::HEADER_DATA_FLAG_OBJECT_AS_ID) != 0 {
                    let id = buf.decode_uint64()?;
                    buf.mark();
                    ObjectKind::ObjectId(id).into()
                } else {
                    if !allow_objects {
                        return Err(BadData);
                    }

                    let class = decode_string(buf)?;
                    let mut object = Object {
                        class: class.to_string(),
                        ..Default::default()
                    };

                    let count = buf.decode_uint32()?;
                    buf.mark();

                    for _ in 0..count {
                        let key = decode_string(buf)?;
                        let value = Self::decode_inner(buf, allow_objects, depth + 1)?;
                        object.properties.insert(key.to_string(), value);
                    }

                    object.into()
                }
            }
            VariantType::Callable => Self::Callable(Callable),
            VariantType::Signal => {
                let name = decode_string(buf)?;

                let object_id = buf.decode_uint64()?;
                buf.mark();

                Signal::new(name.to_string(), object_id).into()
            }
            VariantType::Dictionary => {
                let key_type_kind = Self::get_container_type_kind(
                    header,
                    Self::HEADER_DATA_FIELD_TYPED_DICTIONARY_KEY_MASK,
                    Self::HEADER_DATA_FIELD_TYPED_DICTIONARY_KEY_SHIFT,
                );

                let value_type_kind = Self::get_container_type_kind(
                    header,
                    Self::HEADER_DATA_FIELD_TYPED_DICTIONARY_VALUE_MASK,
                    Self::HEADER_DATA_FIELD_TYPED_DICTIONARY_VALUE_SHIFT,
                );

                let key_type = Self::decode_container_type(buf, key_type_kind, allow_objects)?;
                let value_type = Self::decode_container_type(buf, value_type_kind, allow_objects)?;
                let mut dictionary = Dictionary {
                    key_type,
                    value_type,
                    ..Default::default()
                };

                let count = buf.decode_uint32()?;
                buf.mark();

                for _ in 0..count {
                    let key = Self::decode_inner(buf, allow_objects, depth + 1)?;
                    let value = Self::decode_inner(buf, allow_objects, depth + 1)?;
                    dictionary.inner.insert(key, value);
                }

                dictionary.into()
            }
            VariantType::Array => {
                let element_type_kind = Self::get_container_type_kind(
                    header,
                    Self::HEADER_DATA_FIELD_TYPED_ARRAY_MASK,
                    Self::HEADER_DATA_FIELD_TYPED_ARRAY_SHIFT,
                );

                let element_type =
                    Self::decode_container_type(buf, element_type_kind, allow_objects)?;
                let mut array = Array {
                    element_type,
                    ..Default::default()
                };

                let count = buf.decode_uint32()?;
                buf.mark();

                for _ in 0..count {
                    let elem = Self::decode_inner(buf, allow_objects, depth + 1)?;
                    array.inner.push(elem);
                }

                array.into()
            }
            VariantType::PackedByteArray => {
                let count = buf.decode_uint32()? as usize;
                buf.ensure_remaining(count)?;

                let packed_contents = buf.decode_slice(count)?.to_vec();

                if !count.is_multiple_of(4) {
                    buf.advance(4 - (count % 4));
                }

                buf.mark();

                packed_contents.into()
            }
            VariantType::PackedInt32Array => {
                decode_packed(buf, 4, |buf| Ok(buf.decode_uint32()? as i32))?.into()
            }
            VariantType::PackedInt64Array => {
                decode_packed(buf, 8, |buf| Ok(buf.decode_uint64()? as i64))?.into()
            }
            VariantType::PackedFloat32Array => {
                decode_packed(buf, 4, |buf| buf.decode_float())?.into()
            }
            VariantType::PackedFloat64Array => {
                decode_packed(buf, 8, |buf| buf.decode_double())?.into()
            }
            VariantType::PackedStringArray => {
                let count = buf.decode_uint32()? as usize;
                buf.mark();

                buf.ensure_remaining(count.checked_mul(8).ok_or(BadData)?)?;

                let mut elements = Vec::with_capacity(count);

                for _ in 0..count {
                    let mut v = decode_string(buf)?;
                    if v.ends_with('\0') {
                        v.truncate(v.len() - 1);
                    }
                    elements.push(v.to_owned());
                }

                elements.into()
            }
            VariantType::PackedVector2Array => {
                decode_packed(buf, bytes_per_real * 2, |buf| decode_vector2(buf, as_64))?.into()
            }
            VariantType::PackedVector3Array => {
                decode_packed(buf, bytes_per_real * 3, |buf| decode_vector3(buf, as_64))?.into()
            }
            VariantType::PackedColorArray => decode_packed(buf, 4 * 4, decode_color)?.into(),
            VariantType::PackedVector4Array => {
                decode_packed(buf, bytes_per_real * 4, |buf| decode_vector4(buf, as_64))?.into()
            }
        })
    }

    pub fn encode(&self, buf: &mut WritableMarshalBuffer, full_objects: bool) -> crate::Result<()> {
        self.encode_inner(buf, full_objects, 0)
    }

    fn encode_string(buf: &mut WritableMarshalBuffer, s: &str) {
        buf.encode_uint32(s.len() as u32);
        buf.buffer().extend_from_slice(s.as_bytes());

        while !buf.len().is_multiple_of(4) {
            buf.push(0);
        }
    }

    fn encode_container_type_header(
        header: &mut u32,
        shift: usize,
        typ: &ContainerType,
        full_objects: bool,
    ) {
        let kind = match typ {
            ContainerType::None => ContainerTypeKind::None,
            ContainerType::Builtin(_) => ContainerTypeKind::Builtin,
            ContainerType::Script(_) => {
                if full_objects {
                    ContainerTypeKind::Script
                } else {
                    ContainerTypeKind::ClassName
                }
            }
            ContainerType::ClassName(_) => ContainerTypeKind::ClassName,
        };

        *header |= (kind as u32) << shift;
    }

    fn encode_container_type(
        buf: &mut WritableMarshalBuffer,
        typ: &ContainerType,
        full_objects: bool,
    ) -> crate::Result<()> {
        match typ {
            ContainerType::None => {}
            ContainerType::Builtin(typ) => {
                buf.encode_uint32(*typ as u32);
            }
            ContainerType::ClassName(name) => {
                if full_objects {
                    Self::encode_string(buf, name)
                } else {
                    Self::encode_string(buf, ENCODED_OBJECT_AS_ID_CLASS_NAME)
                }
            }
            ContainerType::Script(path) => {
                if full_objects {
                    Self::encode_string(buf, path)
                } else {
                    Self::encode_string(buf, ENCODED_OBJECT_AS_ID_CLASS_NAME)
                }
            }
        }

        Ok(())
    }

    fn encode_inner(
        &self,
        buf: &mut WritableMarshalBuffer,
        full_objects: bool,
        depth: usize,
    ) -> crate::Result<()> {
        if depth >= Self::MAX_RECURSION_DEPTH {
            return Err(BadData);
        }

        let mut header = self.typ() as u32;
        let mut as_64 = false;

        match self {
            Variant::Int(v) if (*v > (i32::MAX as i64) || *v < (i32::MIN as i64)) => {
                as_64 = true;
            }
            Variant::Float(d) => {
                let f = d.0 as f32;
                if (f as f64) != d.0 {
                    as_64 = true;
                }
            }
            Variant::Object(_) if !full_objects => {
                header |= Self::HEADER_DATA_FLAG_OBJECT_AS_ID;
            }
            Variant::Dictionary(dict) => {
                Self::encode_container_type_header(
                    &mut header,
                    Self::HEADER_DATA_FIELD_TYPED_DICTIONARY_KEY_SHIFT,
                    &dict.key_type,
                    full_objects,
                );
                Self::encode_container_type_header(
                    &mut header,
                    Self::HEADER_DATA_FIELD_TYPED_DICTIONARY_VALUE_SHIFT,
                    &dict.value_type,
                    full_objects,
                );
            }
            Variant::Array(array) => {
                Self::encode_container_type_header(
                    &mut header,
                    Self::HEADER_DATA_FIELD_TYPED_ARRAY_SHIFT,
                    &array.element_type,
                    full_objects,
                );
            }

            Variant::Vector2(_)
            | Variant::Vector3(_)
            | Variant::Vector4(_)
            | Variant::PackedVector2Array(_)
            | Variant::PackedVector3Array(_)
            | Variant::PackedVector4Array(_)
            | Variant::Transform2d(_)
            | Variant::Transform3d(_)
            | Variant::Projection(_)
            | Variant::Quaternion(_)
            | Variant::Plane(_)
            | Variant::Basis(_)
            | Variant::Rect2(_)
            | Variant::Aabb(_)
                if buf.write_reals_as_64_bit() =>
            {
                as_64 = true;
            }

            _ => {}
        };

        if as_64 {
            header |= Self::HEADER_DATA_FLAG_64;
        }

        buf.encode_uint32(header);

        match self {
            Variant::Nil(_) => {}
            Variant::Bool(v) => buf.encode_uint32(*v as u32),
            Variant::Int(v) => {
                if as_64 {
                    buf.encode_uint64(*v as u64);
                } else {
                    buf.encode_uint32(*v as u32);
                }
            }
            Variant::Float(v) => {
                if as_64 {
                    buf.encode_double(v.0);
                } else {
                    buf.encode_float(v.0 as f32);
                }
            }
            Variant::NodePath(np) => {
                buf.encode_uint32(np.names().len() as u32 | 0x80000000);
                buf.encode_uint32(np.sub_names().len() as u32);

                let flags = if np.is_absolute() { 1 } else { 0 };

                buf.encode_uint32(flags);

                let total = np.names().len() + np.sub_names().len();

                for i in 0..total {
                    let s = if i < np.names().len() {
                        &np.names()[i]
                    } else {
                        &np.sub_names()[i - np.names().len()]
                    };

                    let mut pad = 0;

                    if (s.len() % 4) != 0 {
                        pad = 4 - s.len() % 4;
                    }

                    buf.encode_uint32(s.len() as u32);
                    buf.buffer().extend_from_slice(s.as_bytes());

                    for _ in 0..pad {
                        buf.push(0);
                    }
                }
            }
            Variant::String(s) | Variant::StringName(StringName(s)) => Self::encode_string(buf, s),
            Variant::Vector2(v) => {
                encode_vector2(buf, *v);
            }
            Variant::Vector2i(v) => {
                buf.encode_uint32(v.x as u32);
                buf.encode_uint32(v.y as u32);
            }
            Variant::Rect2(v) => {
                encode_vector2(buf, v.position);
                encode_vector2(buf, v.size);
            }
            Variant::Rect2i(v) => {
                buf.encode_uint32(v.position.x as u32);
                buf.encode_uint32(v.position.y as u32);
                buf.encode_uint32(v.size.x as u32);
                buf.encode_uint32(v.size.y as u32);
            }
            Variant::Vector3(v) => {
                encode_vector3(buf, *v);
            }
            Variant::Vector3i(v) => {
                buf.encode_uint32(v.x as u32);
                buf.encode_uint32(v.y as u32);
                buf.encode_uint32(v.z as u32);
            }
            Variant::Transform2d(v) => {
                encode_vector2(buf, v.x);
                encode_vector2(buf, v.y);
                encode_vector2(buf, v.origin);
            }
            Variant::Vector4(v) => {
                buf.encode_real(v.x);
                buf.encode_real(v.y);
                buf.encode_real(v.z);
                buf.encode_real(v.w);
            }
            Variant::Vector4i(v) => {
                buf.encode_uint32(v.x as u32);
                buf.encode_uint32(v.y as u32);
                buf.encode_uint32(v.z as u32);
                buf.encode_uint32(v.w as u32);
            }
            Variant::Plane(v) => {
                encode_vector3(buf, v.normal);
                buf.encode_real(v.d);
            }
            Variant::Quaternion(v) => {
                buf.encode_real(v.x);
                buf.encode_real(v.y);
                buf.encode_real(v.z);
                buf.encode_real(v.w);
            }
            Variant::Aabb(v) => {
                encode_vector3(buf, v.position);
                encode_vector3(buf, v.size);
            }
            Variant::Basis(v) => {
                encode_vector3(buf, v.x);
                encode_vector3(buf, v.y);
                encode_vector3(buf, v.z);
            }
            Variant::Transform3d(v) => {
                encode_vector3(buf, v.basis.x);
                encode_vector3(buf, v.basis.y);
                encode_vector3(buf, v.basis.z);
                encode_vector3(buf, v.origin);
            }
            Variant::Projection(v) => {
                encode_vector4(buf, v.x);
                encode_vector4(buf, v.y);
                encode_vector4(buf, v.z);
                encode_vector4(buf, v.w);
            }
            Variant::Color(v) => {
                encode_color(buf, *v);
            }
            Variant::Rid(v) => {
                buf.encode_uint64(v.0);
            }
            Variant::Object(obj) => {
                // this is a little fucky since `full_objects` isn't exactly how we'd want it to behave
                if full_objects {
                    let obj = match obj {
                        ObjectKind::Object(obj) => obj,
                        _ => return Err(BadData),
                    };

                    Self::encode_string(buf, &obj.class);
                    buf.encode_uint32(obj.properties.len() as u32);
                    for (key, value) in &obj.properties {
                        Self::encode_string(buf, key);
                        value.encode_inner(buf, full_objects, depth + 1)?;
                    }
                } else {
                    let id = match obj {
                        ObjectKind::ObjectId(id) => *id,
                        _ => return Err(BadData),
                    };
                    buf.encode_uint64(id);
                }
            }
            Variant::Callable(_) => {}
            Variant::Signal(signal) => {
                Self::encode_string(buf, &signal.name);
                buf.encode_uint64(signal.object_id);
            }
            Variant::Dictionary(dict) => {
                Self::encode_container_type(buf, &dict.key_type, full_objects)?;
                Self::encode_container_type(buf, &dict.value_type, full_objects)?;
                buf.encode_uint32(dict.inner.len() as u32);

                for (k, v) in &dict.inner {
                    k.encode_inner(buf, full_objects, depth + 1)?;
                    v.encode_inner(buf, full_objects, depth + 1)?;
                }
            }
            Variant::Array(array) => {
                Self::encode_container_type(buf, &array.element_type, full_objects)?;
                buf.encode_uint32(array.inner.len() as u32);

                for elem in &array.inner {
                    elem.encode_inner(buf, full_objects, depth + 1)?;
                }
            }
            Variant::PackedByteArray(v) => {
                buf.encode_uint32(v.len() as u32);
                buf.buffer().extend_from_slice(v);

                while !buf.len().is_multiple_of(4) {
                    buf.push(0);
                }
            }
            Variant::PackedInt32Array(v) => {
                buf.encode_uint32(v.len() as u32);

                for item in v {
                    buf.encode_uint32(*item as u32);
                }
            }
            Variant::PackedInt64Array(v) => {
                buf.encode_uint32(v.len() as u32);

                for item in v {
                    buf.encode_uint64(*item as u64);
                }
            }
            Variant::PackedFloat32Array(v) => {
                buf.encode_uint32(v.len() as u32);

                for item in v {
                    buf.encode_float(item.0);
                }
            }
            Variant::PackedFloat64Array(v) => {
                buf.encode_uint32(v.len() as u32);

                for item in v {
                    buf.encode_double(item.0);
                }
            }
            Variant::PackedStringArray(v) => {
                buf.encode_uint32(v.len() as u32);

                for item in v {
                    // encode_string but with a null terminator lol
                    buf.encode_uint32(item.len() as u32 + 1);
                    buf.buffer().extend_from_slice(item.as_bytes());
                    buf.push(0);

                    while !buf.len().is_multiple_of(4) {
                        buf.push(0);
                    }
                }
            }
            Variant::PackedVector2Array(v) => {
                buf.encode_uint32(v.len() as u32);

                for item in v {
                    encode_vector2(buf, *item);
                }
            }
            Variant::PackedVector3Array(v) => {
                buf.encode_uint32(v.len() as u32);

                for item in v {
                    encode_vector3(buf, *item);
                }
            }
            Variant::PackedColorArray(v) => {
                buf.encode_uint32(v.len() as u32);

                for item in v {
                    encode_color(buf, *item);
                }
            }
            Variant::PackedVector4Array(v) => {
                buf.encode_uint32(v.len() as u32);

                for item in v {
                    encode_vector4(buf, *item);
                }
            }
        }

        Ok(())
    }
}
