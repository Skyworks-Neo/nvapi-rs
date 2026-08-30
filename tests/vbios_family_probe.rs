// VBIOS-family ID presence probe: resolve QueryInterface pointers WITHOUT
// calling — distinguishes "implemented in this driver" from "absent".
// Zero risk (no function invocation). Windows 582.41 vs Linux
// libnvidia-api.sh.1 comparison for the get-vbios platform gap.
// Run: cargo test --test vbios_family_probe -- --nocapture --ignored

#![allow(unused_must_use)]

use nvapi::sys::nvapi::nvapi_QueryInterface;

#[test]
#[ignore]
fn vbios_family_presence() {
    nvapi::initialize().expect("init");
    let ids: &[(u32, &str)] = &[
        (0xacc3da0a, "GetVbiosRevision"),
        (0x2d43fb31, "GetVbiosOEMRevision"),
        (0xa561fd7d, "GetVbiosVersionString"),
        (0xfc13ee11, "GetVbiosImage"),
        (0xe1d5daba, "GetVbiosMxmVersion"),
        (0x8011c22c, "GetVbiosStatusString"),
        (0x8c3a58c3, "GetVbiosExtractionInfo"),
        (0x8d3ac6b9, "GetVbiosSecurityInfo"),
        (0xbca92ad5, "GetVbiosOemInfo"),
        (0xdb66cada, "GetVbiosProjectInfo"),
    ];
    for (id, name) in ids {
        let status = match nvapi_QueryInterface(*id) {
            Ok(p) if !p.is_null() => "IMPLEMENTED",
            _ => "absent",
        };
        eprintln!("{name:28} {id:#010x}  {status}");
    }
}

#[test]
#[ignore]
fn vbios_meta_call_probe() {
    use nvapi::PhysicalGpu;
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    let h = *gpu.handle();
    // (handle, void* out) — the overwhelming nvapi Get* convention.
    // 4KB zeroed buffer: handlers that version-gate reject before writing;
    // handlers that fill, fill into an oversized buffer.
    let calls: &[(u32, &str)] = &[
        (0x8011c22c, "GetVbiosStatusString"),
        (0x8c3a58c3, "GetVbiosExtractionInfo"),
        (0x8d3ac6b9, "GetVbiosSecurityInfo"),
    ];
    for (id, name) in calls {
        let ptr = match nvapi_QueryInterface(*id) {
            Ok(p) if !p.is_null() => p,
            _ => {
                eprintln!("{name}: absent");
                continue;
            }
        };
        let mut buf = vec![0u8; 4096];
        let st = unsafe {
            let f: extern "system" fn(usize, *mut u8) -> i32 = std::mem::transmute(ptr);
            f(h.as_ptr() as usize, buf.as_mut_ptr())
        };
        let nz: Vec<u8> = buf.iter().copied().filter(|&b| b != 0).collect();
        eprintln!(
            "{name}: status={st}  nonzero_bytes={}  head={:02x?}",
            nz.len(),
            &buf[..64.min(buf.len())]
        );
    }
}

#[test]
#[ignore]
fn vbios_meta_magic_sweep() {
    use nvapi::PhysicalGpu;
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    let h = *gpu.handle();
    for (id, name) in [
        (0x8c3a58c3u32, "GetVbiosExtractionInfo"),
        (0x8d3ac6b9u32, "GetVbiosSecurityInfo"),
    ] {
        let ptr = match nvapi_QueryInterface(id) {
            Ok(p) if !p.is_null() => p,
            _ => continue,
        };
        eprintln!("--- {name} magic sweep ---");
        for ver in 1u32..=3 {
            for size in [4u32, 8, 12, 16, 20, 24, 28, 32, 36, 40, 48, 56, 64] {
                let magic = (ver << 16) | size;
                let mut buf = vec![0u8; 4096];
                buf[..4].copy_from_slice(&magic.to_le_bytes());
                let st = unsafe {
                    let f: extern "system" fn(usize, *mut u8) -> i32 = std::mem::transmute(ptr);
                    f(h.as_ptr() as usize, buf.as_mut_ptr())
                };
                if st != -9 {
                    let nz: Vec<u8> = buf.iter().copied().filter(|&b| b != 0).collect();
                    eprintln!(
                        "  ver={ver} size={size} magic={magic:#x} status={st} nonzero={} head={:02x?}",
                        nz.len(),
                        &buf[..48.min(buf.len())]
                    );
                }
            }
        }
    }
}

#[test]
#[ignore]
fn vbios_status_string_enum_sweep() {
    use nvapi::PhysicalGpu;
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    let h = *gpu.handle();
    let ptr = match nvapi_QueryInterface(0x8011c22c) {
        Ok(p) if !p.is_null() => p,
        _ => return,
    };
    for code in 0u32..80 {
        let mut buf = vec![0u8; 512];
        buf[..4].copy_from_slice(&code.to_le_bytes());
        let st = unsafe {
            let f: extern "system" fn(usize, *mut u8) -> i32 = std::mem::transmute(ptr);
            f(h.as_ptr() as usize, buf.as_mut_ptr())
        };
        let s = String::from_utf8_lossy(&buf[4..])
            .trim_end_matches('\0')
            .to_string();
        if st == 0 && !s.is_empty() && s != "Unexpected value" {
            eprintln!("code={code:3}  {s:?}");
        }
    }
}

#[test]
#[ignore]
fn vbios_wrapped_methods() {
    use nvapi::PhysicalGpu;
    nvapi::initialize().expect("init");
    let gpu = PhysicalGpu::enumerate().expect("enumerate").remove(0);
    match gpu.vbios_security_flags() {
        Ok(flags) => eprintln!("wrapped security flags: {flags:#010x}"),
        Err(e) => eprintln!("wrapped security flags: Err {e:?}"),
    }
    match gpu.vbios_status_string() {
        Ok(s) => eprintln!("wrapped status string: {s:?}"),
        Err(e) => eprintln!("wrapped status string: Err {e:?}"),
    }
}

#[test]
#[ignore]
fn plane_a_presence() {
    // Plane-A (pstate-delta) family presence — QueryInterface only, no call.
    // 0x4C0B519A SetPstates20Private (32524B block, escape 0x0700016A);
    // 0xB23B70EE EnableOverclockedPstates (56B, escape 0x070000BA).
    nvapi::initialize().expect("init");
    for (id, name) in [
        (0x4c0b519au32, "SetPstates20Private"),
        (0xb23b70eeu32, "EnableOverclockedPstates"),
    ] {
        let st = match nvapi_QueryInterface(id) {
            Ok(p) if !p.is_null() => "IMPLEMENTED",
            _ => "absent",
        };
        eprintln!("{name:26} {id:#010x}  {st}");
    }
}
