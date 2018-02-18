// Copyright 2015, Igor Shaula
// Licensed under the MIT License <LICENSE or
// http://opensource.org/licenses/MIT>. This file
// may not be copied, modified, or distributed
// except according to those terms.

//! Traits for loading/saving Registry values
use std::slice;
use std::io;
use super::winapi::shared::winerror;
use super::RegValue;
use super::enums::*;

use windows_registry_models::RegistryData;

#[cfg(windows)]
use std::ffi::{OsStr,OsString};
#[cfg(windows)]
use std::os::windows::ffi::{OsStrExt,OsStringExt};

#[cfg(windows)]
impl FromRegValue for OsString {
    fn from_reg_value(val: RegValue) -> io::Result<OsString> {
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

#[cfg(windows)]
impl<'a> ToRegValue for &'a OsStr {
    fn to_reg_value(&self) -> RegValue {
        RegValue {
            bytes: v16_to_v8(to_utf16(self)),
            vtype: REG_SZ
        }
    }
}

/// A trait for types that can be loaded from registry values.
///
/// **NOTE:** Uses `from_utf16_lossy` when converting to `String`.
///
/// **NOTE:** When converting to `String`, trailing `NULL` characters are trimmed
/// and line separating `NULL` characters in `REG_MULTI_SZ` are replaced by `\n`.
/// When converting to `OsString`, all `NULL` characters are left as is.
pub trait FromRegValue : Sized {
    fn from_reg_value(val: RegValue) -> io::Result<Self>;
}

/// A trait for types that can be written into registry values.
///
/// **NOTE:** Adds trailing `NULL` character to `str` and `String` values
/// but **not** to `OsStr` values.
pub trait ToRegValue {
    fn to_reg_value(&self) -> RegValue;
}

impl FromRegValue for String {
    fn from_reg_value(val: RegValue) -> io::Result<String> {
        match RegistryData::from_reg_value(val) {
            RegistryData::RegSz(s) | RegistryData::RegExpandSz(s) => s,
            RegistryData::RegMultiSz(s) => {
                return Ok(s.replace("\u{0}", "\n"))
            },
            _ => werr!(winerror::ERROR_BAD_FILE_TYPE)
        }
    }
}

impl ToRegValue for String {
    fn to_reg_value(&self) -> RegValue {
        RegValue{
            bytes: v16_to_v8(to_utf16(self)),
            vtype: REG_SZ
        }
    }
}

impl FromRegValue for windows_registry_models::RawRegistryData {
    fn from_reg_value(val: RegValue) -> io::Result<Self> {
        Ok(windows_registry_models::RawRegistryData {
            data_type: vtype,
            data: bytes,
        })
    }
}

impl ToRegValue for windows_registry_models::RawRegistryData {
    fn to_reg_value(self) -> RegValue {
        RegValue{
            vtype: self.data_type,
            bytes: self.data,
        }
    }
}

impl FromRegValue for RegistryData {
    fn from_reg_value(val: RegValue) -> io::Result<Self> {
        let raw = RawRegistryData::from_raw_registry_data(val);
        RegistryData::from_raw_registry_data(raw)
    }
}

impl ToRegValue for RegistryData {
    fn to_reg_value(self) -> RegValue {
        self.to_raw_registry_data().to_reg_value()
    }
}

impl FromRegValue for u32 {
    fn from_reg_value(val: &RegValue) -> io::Result<u32> {
        Ok(match RegistryData::from_reg_value(val) {
            RegistryData::RegDword(value) => value,
            _ => werr!(winerror::ERROR_BAD_FILE_TYPE),
        })
    }
}

impl ToRegValue for u32 {
    fn to_reg_value(&self) -> RegValue {
        RegistryData::RegDword(self).to_reg_value()
    }
}

impl FromRegValue for u64 {
    fn from_reg_value(val: &RegValue) -> io::Result<u64> {
        Ok(match RegistryData::from_reg_value(val) {
            RegistryData::RegQword(value) => value,
            _ => werr!(winerror::ERROR_BAD_FILE_TYPE),
        })
    }
}

impl ToRegValue for u64 {
    fn to_reg_value(&self) -> RegValue {
        RegistryData::RegQword(self).to_reg_value()
    }
}

impl<'a> ToRegValue for &'a str {
    fn to_reg_value(&self) -> RegValue {
        RegistryData::RegSz(self.to_owned()).to_reg_value()
    }
}
