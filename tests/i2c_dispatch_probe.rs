// Which handler does QueryInterface(0x4D7B0709) actually return, and in
// which module? nvapi64.dll turned out to be a forwarder; the resolved
// pointer lives in the impl DLL. This probe names the host module and
// prints the RVA inside it for matching against the RE'd dispatch tables.
//
// Run: cargo test -p nvapi --test i2c_dispatch_probe -- --nocapture --ignored

#![cfg(windows)]
#![allow(unused_must_use, unused_unsafe)]

#[test]
#[ignore]
fn i2c_readex_dispatch_rva() {
    use std::os::raw::{c_char, c_void};

    extern "system" {
        fn LoadLibraryA(name: *const c_char) -> isize;
        fn GetModuleHandleExA(flags: u32, name: *const c_void, module: *mut isize) -> i32;
        fn GetModuleFileNameA(module: isize, buffer: *mut c_char, size: u32) -> u32;
    }

    const FROM_ADDRESS: u32 = 0x4;
    const UNCHANGED_REFCOUNT: u32 = 0x2;

    unsafe {
        LoadLibraryA(b"nvapi64.dll\0".as_ptr() as _);
        let ours = nvapi::sys::nvapi::nvapi_QueryInterface(0x4D7B_0709).expect("our loader QI");
        let ptr = ours as usize;
        println!("QI(ReadEx) resolved = {ptr:#x}");

        let mut host = 0isize;
        let ok = GetModuleHandleExA(
            FROM_ADDRESS | UNCHANGED_REFCOUNT,
            ptr as *const c_void,
            &mut host,
        );
        assert!(ok != 0, "no module contains the handler pointer");
        let mut name = [0u8; 512];
        GetModuleFileNameA(host, name.as_mut_ptr() as *mut c_char, 512);
        let end = name.iter().position(|&b| b == 0).unwrap_or(512);
        let path = String::from_utf8_lossy(&name[..end]).to_string();
        let rva = ptr.wrapping_sub(host as usize);
        println!("host module         = {path}");
        println!("host base           = {host:#x}");
        println!("handler RVA         = {rva:#x}");
    }
}
