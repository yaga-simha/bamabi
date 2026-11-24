//! Shared internal JSON/binary field codecs and handle-aware `bevy_reflect` serde helpers
//! deduplicated across [`crate::reflect`] and [`crate::savegame`] (`#![forbid(unsafe_code)]`).

use bamabi_host_sys::loader::ExportedFieldRecord;
use bamabi_host_sys::{
    BamFieldBytePatch, BamFieldDescriptor, BamFieldPodKind, WitnessFieldDescriptor,
};
use bamabi_schema::{
    TargetLayout, WireTypeSpec, canonical_leaf_ident, format_uuid_hyphenated, lookup_wire_type,
    try_parse_uuid_hex,
};
use bamabi_sys::{BamComponent, ComponentUuid, FfiAssetHandle, FfiResult};
use bevy_asset::{
    AssetId, AssetPath, AssetServer, EphemeralHandleBehavior, HandleDeserializeProcessor,
    HandleSerializeProcessor, LoadFromPath, UntypedHandle,
};
use bevy_reflect::serde::{TypedReflectDeserializer, TypedReflectSerializer};
use bevy_reflect::{PartialReflect, TypeRegistration, TypeRegistry};
use core::any::TypeId;
use serde::de::DeserializeSeed;
use serde_json::{Map, Number, Value};

/// Looks up the canonical [`WireTypeSpec`] from [`bamabi_schema`] for `type_tag` on [`TargetLayout::HOST`].
pub(crate) fn canonical_wire_type_spec(type_tag: &str) -> Option<WireTypeSpec> {
    lookup_wire_type(canonical_leaf_ident(type_tag), TargetLayout::HOST)
}

/// Returns `true` when `desc` describes a wire [`FfiAssetHandle`] field.
pub(crate) fn is_ffi_asset_handle_desc(desc: &BamFieldDescriptor) -> bool {
    canonical_wire_type_spec(&desc.type_tag)
        .is_some_and(|spec| spec.canonical_tag == "FfiAssetHandle")
}

/// Formats a [`ComponentUuid`] as canonical hyphenated lowercase UUID string (`8-4-4-4-12`).
pub(crate) fn format_component_uuid_hyphenated(uuid: ComponentUuid) -> String {
    let buf = format_uuid_hyphenated(uuid.as_bytes());
    String::from_utf8_lossy(&buf).into_owned()
}

/// Decodes a native-endian 32-byte [`FfiAssetHandle`] wire payload and validates invariants.
pub(crate) fn decode_ffi_asset_handle_ne_bytes(bytes: &[u8]) -> Result<FfiAssetHandle, String> {
    decode_ffi_asset_handle_bytes_inner(bytes, u32::from_ne_bytes)
}

/// Decodes a little-endian 32-byte [`FfiAssetHandle`] witness payload and validates invariants.
pub(crate) fn decode_ffi_asset_handle_le_bytes(bytes: &[u8]) -> Result<FfiAssetHandle, String> {
    decode_ffi_asset_handle_bytes_inner(bytes, u32::from_le_bytes)
}

fn decode_ffi_asset_handle_bytes_inner(
    bytes: &[u8],
    decode_u32: fn([u8; 4]) -> u32,
) -> Result<FfiAssetHandle, String> {
    if bytes.len() != core::mem::size_of::<FfiAssetHandle>() {
        return Err(format!(
            "FfiAssetHandle expected {} bytes, got {}",
            core::mem::size_of::<FfiAssetHandle>(),
            bytes.len()
        ));
    }
    let kind = bytes[0];
    let reserved0 = [bytes[1], bytes[2], bytes[3]];
    let asset_type = BamComponent::from_raw(decode_u32([bytes[4], bytes[5], bytes[6], bytes[7]]));
    let index = decode_u32([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let generation = decode_u32([bytes[12], bytes[13], bytes[14], bytes[15]]);
    let mut uuid_bytes = [0u8; 16];
    uuid_bytes.copy_from_slice(&bytes[16..32]);
    let handle = FfiAssetHandle {
        kind,
        reserved0,
        asset_type,
        index,
        generation,
        uuid: ComponentUuid::from_bytes(uuid_bytes),
    };
    if handle.validate() != FfiResult::OK {
        return Err(format!(
            "invalid FfiAssetHandle payload (kind={}, reserved0={:?})",
            handle.kind, handle.reserved0
        ));
    }
    Ok(handle)
}

/// Encodes an [`FfiAssetHandle`] into native-endian 32-byte wire representation.
pub(crate) fn encode_ffi_asset_handle_ne_bytes(handle: &FfiAssetHandle) -> Result<Vec<u8>, String> {
    encode_ffi_asset_handle_bytes_inner(handle, u32::to_ne_bytes)
}

/// Encodes an [`FfiAssetHandle`] into little-endian 32-byte witness representation.
pub(crate) fn encode_ffi_asset_handle_le_bytes(handle: &FfiAssetHandle) -> Result<Vec<u8>, String> {
    encode_ffi_asset_handle_bytes_inner(handle, u32::to_le_bytes)
}

fn encode_ffi_asset_handle_bytes_inner(
    handle: &FfiAssetHandle,
    encode_u32: fn(u32) -> [u8; 4],
) -> Result<Vec<u8>, String> {
    if handle.validate() != FfiResult::OK {
        return Err(format!(
            "invalid FfiAssetHandle value (kind={}, reserved0={:?})",
            handle.kind, handle.reserved0
        ));
    }
    let mut out = Vec::with_capacity(core::mem::size_of::<FfiAssetHandle>());
    out.push(handle.kind);
    out.extend_from_slice(&handle.reserved0);
    out.extend_from_slice(&encode_u32(handle.asset_type.to_raw()));
    out.extend_from_slice(&encode_u32(handle.index));
    out.extend_from_slice(&encode_u32(handle.generation));
    out.extend_from_slice(handle.uuid.as_bytes());
    Ok(out)
}

/// Converts a validated [`FfiAssetHandle`] into canonical JSON.
pub(crate) fn ffi_asset_handle_to_json(handle: &FfiAssetHandle) -> Result<Value, String> {
    if handle.validate() != FfiResult::OK {
        return Err(format!(
            "invalid FfiAssetHandle value (kind={}, reserved0={:?})",
            handle.kind, handle.reserved0
        ));
    }
    let kind = match handle.kind {
        FfiAssetHandle::KIND_NONE => "none",
        FfiAssetHandle::KIND_INDEX => "index",
        FfiAssetHandle::KIND_UUID => "uuid",
        other => {
            return Err(format!("invalid FfiAssetHandle kind {other}"));
        }
    };
    let mut obj = Map::with_capacity(5);
    obj.insert("kind".to_owned(), Value::String(kind.to_owned()));
    obj.insert(
        "asset_type".to_owned(),
        Value::from(handle.asset_type.to_raw()),
    );
    obj.insert("index".to_owned(), Value::from(handle.index));
    obj.insert("generation".to_owned(), Value::from(handle.generation));
    obj.insert(
        "uuid".to_owned(),
        Value::String(format_component_uuid_hyphenated(handle.uuid)),
    );
    Ok(Value::Object(obj))
}

/// Parses a JSON [`Value`] into a validated [`FfiAssetHandle`].
pub(crate) fn json_to_ffi_asset_handle(
    field_name: &str,
    val: &Value,
) -> Result<FfiAssetHandle, String> {
    if val.is_null() {
        return Ok(FfiAssetHandle::NONE);
    }
    let obj = val
        .as_object()
        .ok_or_else(|| format!("field '{field_name}' expected FfiAssetHandle object or null"))?;
    let kind_val = obj
        .get("kind")
        .ok_or_else(|| format!("field '{field_name}' FfiAssetHandle missing 'kind'"))?;
    let kind = match kind_val {
        Value::String(s) => match s.as_str() {
            "none" | "NONE" => FfiAssetHandle::KIND_NONE,
            "index" | "INDEX" => FfiAssetHandle::KIND_INDEX,
            "uuid" | "UUID" => FfiAssetHandle::KIND_UUID,
            other => {
                return Err(format!(
                    "field '{field_name}' unknown FfiAssetHandle kind '{other}'"
                ));
            }
        },
        other => u8::try_from(json_as_i128(field_name, other)?)
            .map_err(|_| format!("field '{field_name}' FfiAssetHandle kind out of range"))?,
    };
    let asset_type = match obj.get("asset_type") {
        Some(v) => BamComponent::from_raw(
            u32::try_from(json_as_i128(field_name, v)?)
                .map_err(|_| format!("field '{field_name}' asset_type out of range for u32"))?,
        ),
        None => BamComponent::INVALID,
    };
    let handle = match kind {
        FfiAssetHandle::KIND_NONE => FfiAssetHandle::NONE,
        FfiAssetHandle::KIND_INDEX => {
            let index = u32::try_from(json_as_i128(
                field_name,
                obj.get("index")
                    .ok_or_else(|| format!("field '{field_name}' missing 'index'"))?,
            )?)
            .map_err(|_| format!("field '{field_name}' index out of range for u32"))?;
            let generation = u32::try_from(json_as_i128(
                field_name,
                obj.get("generation")
                    .ok_or_else(|| format!("field '{field_name}' missing 'generation'"))?,
            )?)
            .map_err(|_| format!("field '{field_name}' generation out of range for u32"))?;
            FfiAssetHandle::from_index(asset_type, index, generation)
        }
        FfiAssetHandle::KIND_UUID => {
            let uuid_str = obj
                .get("uuid")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("field '{field_name}' missing string 'uuid'"))?;
            let uuid_bytes = try_parse_uuid_hex(uuid_str)
                .ok_or_else(|| format!("field '{field_name}' invalid UUID '{uuid_str}'"))?;
            FfiAssetHandle::from_uuid(asset_type, ComponentUuid::from_bytes(uuid_bytes))
        }
        other => {
            return Err(format!(
                "field '{field_name}' invalid FfiAssetHandle kind {other}"
            ));
        }
    };
    if handle.validate() != FfiResult::OK {
        return Err(format!(
            "field '{field_name}' produced invalid FfiAssetHandle (kind={})",
            handle.kind
        ));
    }
    Ok(handle)
}

/// Encodes `bytes` as lowercase hexadecimal.
pub(crate) fn encode_hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[inline]
pub(crate) fn f32_to_json(v: f32) -> Value {
    Number::from_f64(f64::from(v))
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

#[inline]
pub(crate) fn f64_to_json(v: f64) -> Value {
    Number::from_f64(v)
        .map(Value::Number)
        .unwrap_or(Value::Null)
}

#[inline]
fn json_as_i128(field_name: &str, v: &Value) -> Result<i128, String> {
    if let Some(n) = v.as_i64() {
        Ok(i128::from(n))
    } else if let Some(n) = v.as_u64() {
        Ok(i128::from(n))
    } else {
        Err(format!("field '{field_name}' expected integer value"))
    }
}

#[inline]
fn json_as_f64(field_name: &str, v: &Value) -> Result<f64, String> {
    v.as_f64()
        .ok_or_else(|| format!("field '{field_name}' expected numeric value"))
}

/// Decodes a native-endian `BamFieldPodKind` byte slice into a JSON [`Value`].
pub(crate) fn decode_pod_field_bytes_to_json(kind: BamFieldPodKind, bytes: &[u8]) -> Option<Value> {
    match kind {
        BamFieldPodKind::Bool => Some(Value::Bool(*bytes.first()? != 0)),
        BamFieldPodKind::U8 => Some(Value::from(*bytes.first()?)),
        BamFieldPodKind::I8 => Some(Value::from(i8::from_ne_bytes(bytes.try_into().ok()?))),
        BamFieldPodKind::U16 => Some(Value::from(u16::from_ne_bytes(bytes.try_into().ok()?))),
        BamFieldPodKind::I16 => Some(Value::from(i16::from_ne_bytes(bytes.try_into().ok()?))),
        BamFieldPodKind::U32 | BamFieldPodKind::Component => {
            Some(Value::from(u32::from_ne_bytes(bytes.try_into().ok()?)))
        }
        BamFieldPodKind::I32 => Some(Value::from(i32::from_ne_bytes(bytes.try_into().ok()?))),
        BamFieldPodKind::U64 | BamFieldPodKind::Entity => {
            Some(Value::from(u64::from_ne_bytes(bytes.try_into().ok()?)))
        }
        BamFieldPodKind::I64 => Some(Value::from(i64::from_ne_bytes(bytes.try_into().ok()?))),
        BamFieldPodKind::Usize => Some(Value::from(
            usize::from_ne_bytes(bytes.try_into().ok()?) as u64
        )),
        BamFieldPodKind::Isize => Some(Value::from(
            isize::from_ne_bytes(bytes.try_into().ok()?) as i64
        )),
        BamFieldPodKind::F32 => Some(f32_to_json(f32::from_ne_bytes(bytes.try_into().ok()?))),
        BamFieldPodKind::F64 => Some(f64_to_json(f64::from_ne_bytes(bytes.try_into().ok()?))),
        BamFieldPodKind::Vec2 | BamFieldPodKind::Vec3 | BamFieldPodKind::Vec4 => {
            let mut arr = Vec::with_capacity(bytes.len() / 4);
            for &c in bytes.as_chunks::<4>().0 {
                arr.push(f32_to_json(f32::from_ne_bytes(c)));
            }
            Some(Value::Array(arr))
        }
        BamFieldPodKind::DVec2 | BamFieldPodKind::DVec3 | BamFieldPodKind::DVec4 => {
            let mut arr = Vec::with_capacity(bytes.len() / 8);
            for &c in bytes.as_chunks::<8>().0 {
                arr.push(f64_to_json(f64::from_ne_bytes(c)));
            }
            Some(Value::Array(arr))
        }
        BamFieldPodKind::UVec2 | BamFieldPodKind::UVec3 | BamFieldPodKind::UVec4 => {
            let mut arr = Vec::with_capacity(bytes.len() / 4);
            for &c in bytes.as_chunks::<4>().0 {
                arr.push(Value::from(u32::from_ne_bytes(c)));
            }
            Some(Value::Array(arr))
        }
        BamFieldPodKind::IVec2 | BamFieldPodKind::IVec3 | BamFieldPodKind::IVec4 => {
            let mut arr = Vec::with_capacity(bytes.len() / 4);
            for &c in bytes.as_chunks::<4>().0 {
                arr.push(Value::from(i32::from_ne_bytes(c)));
            }
            Some(Value::Array(arr))
        }
        BamFieldPodKind::Bytes2
        | BamFieldPodKind::Bytes3
        | BamFieldPodKind::Bytes4
        | BamFieldPodKind::Bytes8
        | BamFieldPodKind::Bytes16 => Some(Value::Array(
            bytes.iter().map(|&b| Value::from(b)).collect(),
        )),
    }
}

/// Encodes a JSON [`Value`] into native-endian POD field bytes matching `kind`.
pub(crate) fn encode_json_to_pod_field_bytes(
    field_name: &str,
    kind: BamFieldPodKind,
    val: &Value,
) -> Result<Vec<u8>, String> {
    match kind {
        BamFieldPodKind::Bool => val
            .as_bool()
            .map(|b| vec![u8::from(b)])
            .ok_or_else(|| format!("field '{field_name}' expected bool")),
        BamFieldPodKind::U8 => u8::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_ne_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for u8")),
        BamFieldPodKind::I8 => i8::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_ne_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for i8")),
        BamFieldPodKind::U16 => u16::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_ne_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for u16")),
        BamFieldPodKind::I16 => i16::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_ne_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for i16")),
        BamFieldPodKind::U32 | BamFieldPodKind::Component => {
            u32::try_from(json_as_i128(field_name, val)?)
                .map(|x| x.to_ne_bytes().to_vec())
                .map_err(|_| format!("field '{field_name}' out of range for u32"))
        }
        BamFieldPodKind::I32 => i32::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_ne_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for i32")),
        BamFieldPodKind::U64 | BamFieldPodKind::Entity => {
            u64::try_from(json_as_i128(field_name, val)?)
                .map(|x| x.to_ne_bytes().to_vec())
                .map_err(|_| format!("field '{field_name}' out of range for u64"))
        }
        BamFieldPodKind::I64 => i64::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_ne_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for i64")),
        BamFieldPodKind::Usize => usize::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_ne_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for usize")),
        BamFieldPodKind::Isize => isize::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_ne_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for isize")),
        BamFieldPodKind::F32 => Ok((json_as_f64(field_name, val)? as f32)
            .to_ne_bytes()
            .to_vec()),
        BamFieldPodKind::F64 => Ok(json_as_f64(field_name, val)?.to_ne_bytes().to_vec()),
        BamFieldPodKind::Vec2 | BamFieldPodKind::Vec3 | BamFieldPodKind::Vec4 => {
            let expected = kind.byte_size() / 4;
            let arr = val
                .as_array()
                .ok_or_else(|| format!("field '{field_name}' expected array"))?;
            if arr.len() != expected {
                return Err(format!(
                    "field '{field_name}' expected {expected} elements, got {}",
                    arr.len()
                ));
            }
            let mut out = Vec::with_capacity(kind.byte_size());
            for el in arr {
                out.extend_from_slice(&(json_as_f64(field_name, el)? as f32).to_ne_bytes());
            }
            Ok(out)
        }
        BamFieldPodKind::DVec2 | BamFieldPodKind::DVec3 | BamFieldPodKind::DVec4 => {
            let expected = kind.byte_size() / 8;
            let arr = val
                .as_array()
                .ok_or_else(|| format!("field '{field_name}' expected array"))?;
            if arr.len() != expected {
                return Err(format!(
                    "field '{field_name}' expected {expected} elements, got {}",
                    arr.len()
                ));
            }
            let mut out = Vec::with_capacity(kind.byte_size());
            for el in arr {
                out.extend_from_slice(&json_as_f64(field_name, el)?.to_ne_bytes());
            }
            Ok(out)
        }
        BamFieldPodKind::UVec2 | BamFieldPodKind::UVec3 | BamFieldPodKind::UVec4 => {
            let expected = kind.byte_size() / 4;
            let arr = val
                .as_array()
                .ok_or_else(|| format!("field '{field_name}' expected array"))?;
            if arr.len() != expected {
                return Err(format!(
                    "field '{field_name}' expected {expected} elements, got {}",
                    arr.len()
                ));
            }
            let mut out = Vec::with_capacity(kind.byte_size());
            for el in arr {
                let n = u32::try_from(json_as_i128(field_name, el)?)
                    .map_err(|_| format!("field '{field_name}' element out of range for u32"))?;
                out.extend_from_slice(&n.to_ne_bytes());
            }
            Ok(out)
        }
        BamFieldPodKind::IVec2 | BamFieldPodKind::IVec3 | BamFieldPodKind::IVec4 => {
            let expected = kind.byte_size() / 4;
            let arr = val
                .as_array()
                .ok_or_else(|| format!("field '{field_name}' expected array"))?;
            if arr.len() != expected {
                return Err(format!(
                    "field '{field_name}' expected {expected} elements, got {}",
                    arr.len()
                ));
            }
            let mut out = Vec::with_capacity(kind.byte_size());
            for el in arr {
                let n = i32::try_from(json_as_i128(field_name, el)?)
                    .map_err(|_| format!("field '{field_name}' element out of range for i32"))?;
                out.extend_from_slice(&n.to_ne_bytes());
            }
            Ok(out)
        }
        BamFieldPodKind::Bytes2
        | BamFieldPodKind::Bytes3
        | BamFieldPodKind::Bytes4
        | BamFieldPodKind::Bytes8
        | BamFieldPodKind::Bytes16 => {
            let expected = kind.byte_size();
            let arr = val
                .as_array()
                .ok_or_else(|| format!("field '{field_name}' expected byte array"))?;
            if arr.len() != expected {
                return Err(format!(
                    "field '{field_name}' expected {expected} bytes, got {}",
                    arr.len()
                ));
            }
            let mut out = Vec::with_capacity(expected);
            for el in arr {
                let b = u8::try_from(json_as_i128(field_name, el)?)
                    .map_err(|_| format!("field '{field_name}' element out of range for u8"))?;
                out.push(b);
            }
            Ok(out)
        }
    }
}

/// Converts a JSON field map into [`BamFieldBytePatch`]es matching `fields`.
pub(crate) fn json_map_to_bam_field_patches(
    obj: &Map<String, Value>,
    fields: &[BamFieldDescriptor],
) -> Result<Vec<BamFieldBytePatch>, String> {
    let mut patches = Vec::with_capacity(obj.len());
    for (k, v) in obj {
        let desc = fields
            .iter()
            .find(|f| f.name == *k)
            .ok_or_else(|| format!("unknown field '{k}'"))?;
        let bytes = if is_ffi_asset_handle_desc(desc) {
            if desc.size != core::mem::size_of::<FfiAssetHandle>() {
                return Err(format!(
                    "field '{k}' FfiAssetHandle size mismatch: descriptor has {} bytes, expected {}",
                    desc.size,
                    core::mem::size_of::<FfiAssetHandle>()
                ));
            }
            let handle = json_to_ffi_asset_handle(k, v)?;
            encode_ffi_asset_handle_ne_bytes(&handle)?
        } else {
            let pod_kind = desc
                .pod_kind
                .ok_or_else(|| format!("field '{k}' is opaque/non-POD and cannot be mutated"))?;
            encode_json_to_pod_field_bytes(k, pod_kind, v)?
        };
        patches.push(BamFieldBytePatch {
            offset: desc.offset,
            bytes,
        });
    }
    Ok(patches)
}

/// Converts exported witness records into a JSON field map.
pub(crate) fn exported_records_to_json_map(records: &[ExportedFieldRecord]) -> Map<String, Value> {
    let mut map = Map::with_capacity(records.len());
    for rec in records {
        if let Some(v) = decode_witness_record_to_json(&rec.type_tag, &rec.bytes) {
            map.insert(rec.field_name.clone(), v);
        }
    }
    map
}

/// Decodes an exported witness record payload into a JSON [`Value`].
pub(crate) fn decode_witness_record_to_json(type_tag: &str, bytes: &[u8]) -> Option<Value> {
    match canonical_leaf_ident(type_tag) {
        "bool" => Some(Value::Bool(*bytes.first()? != 0)),
        "u8" => Some(Value::from(*bytes.first()?)),
        "i8" => Some(Value::from(i8::from_le_bytes(bytes.try_into().ok()?))),
        "u16" => Some(Value::from(u16::from_le_bytes(bytes.try_into().ok()?))),
        "i16" => Some(Value::from(i16::from_le_bytes(bytes.try_into().ok()?))),
        "u32" | "BamComponent" => Some(Value::from(u32::from_le_bytes(bytes.try_into().ok()?))),
        "i32" => Some(Value::from(i32::from_le_bytes(bytes.try_into().ok()?))),
        "u64" | "Entity" | "FfiEntity" | "TargetEntity" => {
            Some(Value::from(u64::from_le_bytes(bytes.try_into().ok()?)))
        }
        "i64" => Some(Value::from(i64::from_le_bytes(bytes.try_into().ok()?))),
        "usize" => Some(Value::from(
            usize::from_le_bytes(bytes.try_into().ok()?) as u64
        )),
        "isize" => Some(Value::from(
            isize::from_le_bytes(bytes.try_into().ok()?) as i64
        )),
        "f32" => Some(f32_to_json(f32::from_le_bytes(bytes.try_into().ok()?))),
        "f64" => Some(f64_to_json(f64::from_le_bytes(bytes.try_into().ok()?))),
        "ComponentUuid" => {
            let uuid_bytes: [u8; 16] = bytes.try_into().ok()?;
            Some(Value::String(format_component_uuid_hyphenated(
                ComponentUuid::from_bytes(uuid_bytes),
            )))
        }
        "FfiAssetHandle" => decode_ffi_asset_handle_le_bytes(bytes)
            .ok()
            .and_then(|h| ffi_asset_handle_to_json(&h).ok()),
        "String" | "__variant" => core::str::from_utf8(bytes)
            .ok()
            .map(|s| Value::String(s.to_owned())),
        "Vec<u8>" => Some(Value::Array(
            bytes.iter().map(|&b| Value::from(b)).collect(),
        )),
        "Vec<u32>" => {
            if !bytes.len().is_multiple_of(4) {
                return None;
            }
            Some(Value::Array(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|&c| Value::from(u32::from_le_bytes(c)))
                    .collect(),
            ))
        }
        "Vec<i32>" => {
            if !bytes.len().is_multiple_of(4) {
                return None;
            }
            Some(Value::Array(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|&c| Value::from(i32::from_le_bytes(c)))
                    .collect(),
            ))
        }
        "Vec<f32>" => {
            if !bytes.len().is_multiple_of(4) {
                return None;
            }
            Some(Value::Array(
                bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|&c| f32_to_json(f32::from_le_bytes(c)))
                    .collect(),
            ))
        }
        "FfiOption<u32>" => match *bytes.first()? {
            0 => Some(Value::Null),
            _ if bytes.len() == 5 => Some(Value::from(u32::from_le_bytes([
                bytes[1], bytes[2], bytes[3], bytes[4],
            ]))),
            _ => None,
        },
        "FfiOption<i32>" => match *bytes.first()? {
            0 => Some(Value::Null),
            _ if bytes.len() == 5 => Some(Value::from(i32::from_le_bytes([
                bytes[1], bytes[2], bytes[3], bytes[4],
            ]))),
            _ => None,
        },
        "FfiOption<f32>" => match *bytes.first()? {
            0 => Some(Value::Null),
            _ if bytes.len() == 5 => Some(f32_to_json(f32::from_le_bytes([
                bytes[1], bytes[2], bytes[3], bytes[4],
            ]))),
            _ => None,
        },
        _ => Some(Value::String(encode_hex_lower(bytes))),
    }
}

/// Encodes a JSON [`Value`] into a witness [`ExportedFieldRecord`].
pub(crate) fn encode_json_to_witness_record(
    field_name: &str,
    type_tag: &str,
    val: &Value,
) -> Result<ExportedFieldRecord, String> {
    let bytes = match canonical_leaf_ident(type_tag) {
        "bool" => {
            vec![u8::from(val.as_bool().ok_or_else(|| {
                format!("field '{field_name}' expected bool")
            })?)]
        }
        "u8" => u8::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_le_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for u8"))?,
        "i8" => i8::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_le_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for i8"))?,
        "u16" => u16::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_le_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for u16"))?,
        "i16" => i16::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_le_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for i16"))?,
        "u32" | "BamComponent" => u32::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_le_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for u32"))?,
        "i32" => i32::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_le_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for i32"))?,
        "u64" | "Entity" | "FfiEntity" | "TargetEntity" => {
            u64::try_from(json_as_i128(field_name, val)?)
                .map(|x| x.to_le_bytes().to_vec())
                .map_err(|_| format!("field '{field_name}' out of range for u64"))?
        }
        "i64" => i64::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_le_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for i64"))?,
        "usize" => usize::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_le_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for usize"))?,
        "isize" => isize::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_le_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for isize"))?,
        "u128" => u128::try_from(json_as_i128(field_name, val)?)
            .map(|x| x.to_le_bytes().to_vec())
            .map_err(|_| format!("field '{field_name}' out of range for u128"))?,
        "i128" => json_as_i128(field_name, val)?.to_le_bytes().to_vec(),
        "f32" => (json_as_f64(field_name, val)? as f32)
            .to_le_bytes()
            .to_vec(),
        "f64" => json_as_f64(field_name, val)?.to_le_bytes().to_vec(),
        "ComponentUuid" => {
            let uuid_str = val
                .as_str()
                .ok_or_else(|| format!("field '{field_name}' expected UUID string"))?;
            try_parse_uuid_hex(uuid_str)
                .ok_or_else(|| format!("field '{field_name}' invalid UUID '{uuid_str}'"))?
                .to_vec()
        }
        "FfiAssetHandle" => {
            let handle = json_to_ffi_asset_handle(field_name, val)?;
            encode_ffi_asset_handle_le_bytes(&handle)?
        }
        "String" | "__variant" => val
            .as_str()
            .ok_or_else(|| format!("field '{field_name}' expected string"))?
            .as_bytes()
            .to_vec(),
        "Vec<u8>" => {
            let arr = val
                .as_array()
                .ok_or_else(|| format!("field '{field_name}' expected array"))?;
            let mut out = Vec::with_capacity(arr.len());
            for el in arr {
                out.push(
                    u8::try_from(json_as_i128(field_name, el)?)
                        .map_err(|_| format!("field '{field_name}' element out of range for u8"))?,
                );
            }
            out
        }
        "Vec<u32>" => {
            let arr = val
                .as_array()
                .ok_or_else(|| format!("field '{field_name}' expected array"))?;
            let mut out = Vec::with_capacity(arr.len() * 4);
            for el in arr {
                let n = u32::try_from(json_as_i128(field_name, el)?)
                    .map_err(|_| format!("field '{field_name}' element out of range for u32"))?;
                out.extend_from_slice(&n.to_le_bytes());
            }
            out
        }
        "Vec<i32>" => {
            let arr = val
                .as_array()
                .ok_or_else(|| format!("field '{field_name}' expected array"))?;
            let mut out = Vec::with_capacity(arr.len() * 4);
            for el in arr {
                let n = i32::try_from(json_as_i128(field_name, el)?)
                    .map_err(|_| format!("field '{field_name}' element out of range for i32"))?;
                out.extend_from_slice(&n.to_le_bytes());
            }
            out
        }
        "Vec<f32>" => {
            let arr = val
                .as_array()
                .ok_or_else(|| format!("field '{field_name}' expected array"))?;
            let mut out = Vec::with_capacity(arr.len() * 4);
            for el in arr {
                out.extend_from_slice(&(json_as_f64(field_name, el)? as f32).to_le_bytes());
            }
            out
        }
        "FfiOption<u32>" => {
            if val.is_null() {
                vec![0u8]
            } else {
                let n = u32::try_from(json_as_i128(field_name, val)?)
                    .map_err(|_| format!("field '{field_name}' out of range for u32"))?;
                let mut out = vec![1u8];
                out.extend_from_slice(&n.to_le_bytes());
                out
            }
        }
        "FfiOption<i32>" => {
            if val.is_null() {
                vec![0u8]
            } else {
                let n = i32::try_from(json_as_i128(field_name, val)?)
                    .map_err(|_| format!("field '{field_name}' out of range for i32"))?;
                let mut out = vec![1u8];
                out.extend_from_slice(&n.to_le_bytes());
                out
            }
        }
        "FfiOption<f32>" => {
            if val.is_null() {
                vec![0u8]
            } else {
                let f = json_as_f64(field_name, val)? as f32;
                let mut out = vec![1u8];
                out.extend_from_slice(&f.to_le_bytes());
                out
            }
        }
        other => {
            return Err(format!(
                "unsupported witness field type_tag '{other}' for field '{field_name}'"
            ));
        }
    };
    Ok(ExportedFieldRecord {
        field_name: field_name.to_owned(),
        type_tag: type_tag.to_owned(),
        bytes,
    })
}

/// Converts a JSON field map into witness [`ExportedFieldRecord`]s matching `fields`.
pub(crate) fn json_map_to_witness_records(
    obj: &Map<String, Value>,
    fields: &[WitnessFieldDescriptor],
) -> Result<Vec<ExportedFieldRecord>, String> {
    let mut out = Vec::with_capacity(obj.len());
    for (k, v) in obj {
        let desc = fields
            .iter()
            .find(|f| f.name == *k)
            .ok_or_else(|| format!("unknown field '{k}'"))?;
        out.push(encode_json_to_witness_record(k, &desc.type_tag, v)?);
    }
    Ok(out)
}

/// Fallback [`LoadFromPath`] used in headless worlds that lack an [`AssetServer`].
///
/// Constant UUID handles (`Handle::Uuid`) deserialize without touching `load_from_path`, while
/// path-backed handles record the first unresolvable path so the caller returns a clean typed
/// serialization error instead of panicking.
#[derive(Default)]
struct MissingAssetServerLoader {
    first_missing_path: Option<String>,
}

impl LoadFromPath for MissingAssetServerLoader {
    fn load_from_path_erased(
        &mut self,
        type_id: TypeId,
        path: AssetPath<'static>,
    ) -> UntypedHandle {
        if self.first_missing_path.is_none() {
            self.first_missing_path = Some(path.to_string());
        }
        UntypedHandle::Uuid {
            type_id,
            uuid: AssetId::<()>::DEFAULT_UUID,
        }
    }
}

/// Serializes a concrete reflected value directly (NEVER via `.to_dynamic()`) into unwrapped
/// `TypedReflectSerializer` JSON using Bevy's [`HandleSerializeProcessor`] so `Handle<A>` and
/// `UntypedHandle` fields serialize as stable `Path`/`Uuid` references.
pub(crate) fn serialize_reflect_to_json(
    value: &dyn PartialReflect,
    types: &TypeRegistry,
    ephemeral_handle_behavior: EphemeralHandleBehavior,
) -> Result<Value, String> {
    let processor = HandleSerializeProcessor {
        ephemeral_handle_behavior,
    };
    let ser = TypedReflectSerializer::with_processor(value, types, &processor);
    serde_json::to_value(&ser).map_err(|err| err.to_string())
}

/// Deserializes unwrapped JSON produced by [`serialize_reflect_to_json`] using
/// `TypedReflectDeserializer::with_processor` and Bevy's [`HandleDeserializeProcessor`], loading
/// path-backed asset handles through `asset_server` when present or returning a clear error if a
/// path-backed handle is encountered in a headless world without an [`AssetServer`].
pub(crate) fn deserialize_reflect_from_json(
    registration: &TypeRegistration,
    types: &TypeRegistry,
    value: &Value,
    asset_server: Option<&mut AssetServer>,
    component_name: &str,
) -> Result<Box<dyn PartialReflect>, String> {
    let mut missing_loader = MissingAssetServerLoader::default();
    let load_from_path: &mut dyn LoadFromPath = match asset_server {
        Some(server) => server,
        None => &mut missing_loader,
    };
    let mut processor = HandleDeserializeProcessor { load_from_path };
    let de = TypedReflectDeserializer::with_processor(registration, types, &mut processor);
    let deserialized = de
        .deserialize(value.clone())
        .map_err(|err| format!("reflect deserialize failed for '{component_name}': {err}"))?;
    if let Some(missing_path) = missing_loader.first_missing_path {
        return Err(format!(
            "cannot deserialize path-backed asset handle '{missing_path}' for '{component_name}' without an AssetServer resource"
        ));
    }
    Ok(deserialized)
}
