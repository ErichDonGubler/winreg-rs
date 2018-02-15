// Copyright 2015, Igor Shaula
// Licensed under the MIT License <LICENSE or
// http://opensource.org/licenses/MIT>. This file
// may not be copied, modified, or distributed
// except according to those terms.

//! Traits for loading/saving Registry values
use std::slice;
use std::io;
use std::ffi::{OsStr,OsString};
use std::os::windows::ffi::{OsStrExt,OsStringExt};
use super::winapi::shared::winerror;
use super::{RegValue};
use super::enums::*;
use super::{to_utf16,v16_to_v8};

/// A trait for types that can be loaded from registry values.
///
/// **NOTE:** Uses `from_utf16_lossy` when converting to `String`.
///
/// **NOTE:** When converting to `String`, trailing `NULL` characters are trimmed
/// and line separating `NULL` characters in `REG_MULTI_SZ` are replaced by `\n`.
/// When converting to `OsString`, all `NULL` characters are left as is.
pub trait FromRegValue : Sized {
    fn from_reg_value(val: &RegValue) -> io::Result<Self>;
}

impl FromRegValue for String {
    fn from_reg_value(val: &RegValue) -> io::Result<String> {
        match val.vtype {
            REG_SZ | REG_EXPAND_SZ | REG_MULTI_SZ => {
                let words = unsafe {
                    slice::from_raw_parts(val.bytes.as_ptr() as *const u16, val.bytes.len() / 2)
                };
                let mut s = String::from_utf16_lossy(words);
                while s.ends_with('\u{0}') {s.pop();}
                if val.vtype == REG_MULTI_SZ {
                    return Ok(s.replace("\u{0}", "\n"))
                }
                Ok(s)
            },
            _ => werr!(winerror::ERROR_BAD_FILE_TYPE)
        }
    }
}

impl FromRegValue for OsString {
    fn from_reg_value(val: &RegValue) -> io::Result<OsString> {
        match val.vtype {
            REG_SZ | REG_EXPAND_SZ | REG_MULTI_SZ => {
                let words = unsafe {
                    slice::from_raw_parts(val.bytes.as_ptr() as *const u16, val.bytes.len() / 2)
                };
                let s = OsString::from_wide(words);
                Ok(s)
            },
            _ => werr!(winerror::ERROR_BAD_FILE_TYPE)
        }
    }
}

impl FromRegValue for u32 {
    fn from_reg_value(val: &RegValue) -> io::Result<u32> {
        match val.vtype {
            REG_DWORD => {
                Ok(unsafe{ *(val.bytes.as_ptr() as *const u32) })
            },
            _ => werr!(winerror::ERROR_BAD_FILE_TYPE)
        }
    }
}

impl FromRegValue for u64 {
    fn from_reg_value(val: &RegValue) -> io::Result<u64> {
        match val.vtype {
            REG_QWORD => {
                Ok(unsafe{ *(val.bytes.as_ptr() as *const u64) })
            },
            _ => werr!(winerror::ERROR_BAD_FILE_TYPE)
        }
    }
}

/// A trait for types that can be written into registry values.
///
/// **NOTE:** Adds trailing `NULL` character to `str` and `String` values
/// but **not** to `OsStr` values.
pub trait ToRegValue {
    fn to_reg_value(&self) -> RegValue;
}

impl ToRegValue for String {
    fn to_reg_value(&self) -> RegValue {
        RegValue{
            bytes: v16_to_v8(&to_utf16(self)),
            vtype: REG_SZ
        }
    }
}

impl<'a> ToRegValue for &'a str {
    fn to_reg_value(&self) -> RegValue {
        RegValue{
            bytes: v16_to_v8(&to_utf16(self)),
            vtype: REG_SZ
        }
    }
}

impl<'a> ToRegValue for &'a OsStr {
    fn to_reg_value(&self) -> RegValue {
        RegValue{
            bytes: v16_to_v8(&(self.encode_wide().collect::<Vec<_>>())),
            vtype: REG_SZ
        }
    }
}

impl ToRegValue for u32 {
    fn to_reg_value(&self) -> RegValue {
        let bytes: Vec<u8> = unsafe {
            slice::from_raw_parts((self as *const u32) as *const u8, 4).to_vec()
        };
        RegValue{
            bytes: bytes,
            vtype: REG_DWORD
        }
    }
}

impl ToRegValue for u64 {
    fn to_reg_value(&self) -> RegValue {
        let bytes: Vec<u8> = unsafe {
            slice::from_raw_parts((self as *const u64) as *const u8, 8).to_vec()
        };
        RegValue{
            bytes: bytes,
            vtype: REG_QWORD
        }
    }
}

/// A strongly-typed enumeration of [registry data types](https://msdn.microsoft.com/en-us/library/windows/desktop/bb773476(v=vs.85).aspx).
///
/// **NOTE:** There are several caveats to remember when dealing with string types in the Windows
/// API:
///
/// 1. As with conversions to and from `RegValue`, conversions between UTF-8 (Rust
///     representation) and WTF-16 happen when actually making API calls to the Windows operating
///     system.
/// 2. Null string pointers are indistinguishable from empty strings when stored as null-terminated
///    strings. Both cases are deserialized as an empty string.
/// 3. The `RegMultiSz` variant also is affected by the above caveat in the case of storing a
///    single string. However, instead.
///
///    Some examples of what serialized data will deserialize as when using `RegMultiSz`:
///
///    TODO: Turn this into a tested code block!
///    // These shouldn't be surprising.
///    * `["data"]` => `["data"]`
///    * `[]` => `[]`
///    * `["", ""]` => `["", ""]`
///    * `[""]` => `[]` // But here, empty string is indistinguishable from having no string at
///    all.
///
/// XXX: `Option` doesn't feel appropriate to use here, but forcing users to check for empty
/// strings sounds pretty annoying too. Thoughts to improve this?
#[derive(Clone, Debug, PartialEq)]
pub enum StronglyTypedRegistryData { // TODO: Come up with a better name
    RegNone,
    /// A normal string.
    ///
    /// Note that null-terminated storage caveats apply to this type -- see above.
    RegSz(String),
    /// A string that contains unexpanded references to environmental variables, i.e., `"%HOME%"`.
    ///
    /// Note that null-terminated storage caveats apply to this type -- see above.
    RegExpandSz(String),
    /// Simple binary data.
    RegBinary(Vec<u8>),
    /// A 32-bit integral that is stored as a little-endian integer.
    RegDwordLittleEndian(u32),
    /// A 32-bit integral that is stored as a big-endian integer.
    RegDwordBigEndian(u32),
    /// A symbolic link in the registry.
    ///
    /// Note that null-terminated storage caveats apply to this type -- see above.
    RegLink(String),
    /// An array of strings.
    ///
    /// Note that null-terminated storage caveats apply to this type -- see above.
    RegMultiSz(Vec<String>),
    /// Binary data representing a "resource list".
    ///
    /// The format of this type is unknown, and exposed as raw binary.
    RegResourceList(Vec<u8>),
    /// Binary data representing a "full resource descriptor".
    ///
    /// The format of this type is unknown, and exposed as raw binary.
    RegFullResourceDescriptor(Vec<u8>),
    /// Binary data representing a "resource requirements list".
    ///
    /// The format of this type is unknown, and exposed as raw binary.
    RegResourceRequirementsList(Vec<u8>),
    /// A 64-bit integral that is stored as a little-endian integer.
    ///
    /// There is no big-endian variant of this type.
    RegQword(u64),
}

impl StronglyTypedRegistryData {
    /// Retrieves the type discriminant defined by Windows for this type (see [here](https://msdn.microsoft.com/en-us/library/windows/desktop/ms724884(v=vs.85).aspx) for reference).
    pub fn discriminant(&self) -> u32 {
        use self::RegistryData::*;
        match self {
            &RegNone => REG_NONE,
            &RegSz(_) => REG_SZ,
            &RegExpandSz(_) => REG_EXPAND_SZ,
            &RegBinary(_) => REG_BINARY,
            &RegDwordLittleEndian(_) => REG_DWORD,
            &RegDwordBigEndian(_) => REG_DWORD_BIG_ENDIAN,
            &RegLink(_) => REG_LINK,
            &RegMultiSz(_) => REG_MULTI_SZ,
            &RegResourceList(_) => REG_RESOURCE_LIST,
            &RegFullResourceDescriptor(_) => REG_FULL_RESOURCE_DESCRIPTOR,
            &RegResourceRequirementsList(_) => REG_RESOURCE_REQUIREMENTS_LIST,
            &RegQword(_) => REG_QWORD,
        }
    }
}

impl ToRegValue for StronglyTypedRegistryData {
    fn from_reg_value(val: &RegValue) -> io::Result<Self> {
        // TODO:
    }
}
