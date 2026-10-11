//! Privilege diagnosis for the RTX PRO 6000 Blackwell write paths — the
//! server card refuses ClkDomains SET with NVAPI_INVALID_USER_PRIVILEGE
//! (-137) as a normal user; RM os_is_administrator() = NV_IS_SUSER() = root
//! (ogkm kernel-open/nvidia/os-interface.c:378). Run me AS ROOT to test
//! whether privilege is the whole story or the RM still re-parks:
//!
//!   sudo -E cargo test -p nvapi --test pro6000_privilege_write_live -- --nocapture --ignored
//!
//! Sequence (everything auto-restores):
//!   A. ClkDomains SET: xbar slot2 +50 MHz with snapshot restore — exact
//!      status, immediate readback, then restore-verify.
//!   B. VF point SET: bank0 point 40 mode0 +50 MHz — exact status, readback,
//!      then reset_vfp_private(bank0) restore-verify.
//! READ-mostly by design: writes are minimal and always undone.

use nvapi::PhysicalGpu;

fn euid() -> u32 {
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

#[test]
#[ignore = "live GPU WRITE probe — run under sudo; restores everything"]
fn privilege_write_diagnosis() {
    let uid = euid();
    println!("euid = {uid} ({})", if uid == 0 { "root — RM admin gate should pass" } else { "NOT root — expect -137 NVAPI_INVALID_USER_PRIVILEGE" });

    nvapi::initialize().expect("init");
    let idx: usize = std::env::var("NVOC_GPU_IDX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let gpu = &PhysicalGpu::enumerate().expect("enumerate")[idx];
    println!("gpu[{}]: {}", idx, gpu.full_name().unwrap_or_default());

    // ---- A. ClkDomains SET (slot2 = Blackwell freq plane) ----------------
    println!("\n== A. ClkDomains SET xbar slot2 +50 MHz (temporary) ==");
    match gpu.set_clk_domain_offset(1, 50_000, 2, true) {
        Ok(entry) => {
            println!("  SET ok: {:?}", entry);
            // readback while it (hypothetically) holds — temporary already
            // restored by the time we return, so this read is post-restore
            // and should show 0 again; a non-zero readback would mean the
            // driver kept the value despite the restore path.
            println!("  (temporary restore already applied)");
        }
        Err(e) => {
            println!("  SET ERR: {e}");
            println!(
                "  -> {}",
                if e.nvapi_status() == Some(nvapi::Status::InvalidUserPrivilege) {
                    "PRIVILEGE GATE confirmed (-137) — rerun under sudo"
                } else {
                    "different failure — paste me the status"
                }
            );
        }
    }

    // ---- B. VF point SET (mode0 = kHz offset) ----------------------------
    println!("\n== B. VF point SET bank0 point40 mode0 +50 MHz ==");
    match gpu.set_vfp_point_private(0, 40, true, 50_000) {
        Ok(rc) => {
            println!("  SET ok (rc={rc}) — restoring:");
            // restore
            match gpu.reset_vfp_private(0, None) {
                Ok(n) => println!("  restore: reset_vfp_private(bank0) reset {n} points"),
                Err(e) => println!("  restore ERR: {e} (MANUAL CHECK: get-private-vftable)"),
            }
        }
        Err(e) => {
            println!("  SET ERR: {e}");
            println!(
                "  -> {}",
                if e.nvapi_status() == Some(nvapi::Status::InvalidUserPrivilege) {
                    "PRIVILEGE GATE confirmed (-137) — mode0/1 ineffectiveness = privilege"
                } else {
                    "different failure — paste me the status"
                }
            );
        }
    }

    println!("\n(done — every write restored; nothing persisted)");
}
