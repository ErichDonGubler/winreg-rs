winreg
======

Rust bindings to MS Windows Registry API. Work in progress.

## Usage

```rust
extern crate winreg;
use std::path::Path;
use winreg::types::*;

fn main() {
    let hklm = winreg::RegKey::predef(HKEY_LOCAL_MACHINE);
    let cur_ver = hklm.open(Path::new("SOFTWARE\\Microsoft\\Windows\\CurrentVersion"), KEY_READ).unwrap();
    let program_files: String = cur_ver.get_value(Path::new("ProgramFilesDir")).unwrap();
    let common_files: String = cur_ver.get_value(Path::new("CommonFilesDir")).unwrap();
    println!("ProgramFiles = {}\nCommonFiles = {}", program_files, common_files);
}
```