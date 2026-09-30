// Read-only E0 instrument for the P100 private VfTable audit
// (docs/reverse-engineering/nvapi/p100-58241-privatevftable-audit.md).
// ZERO writes. Dumps:
//   1. GetControl (0xDA025C3E): per present record — user type byte, mode
//      dword, value (u32 + i32 view), flag byte @+96, plus every non-zero
//      dword in the record (any hit outside the 4 known fields is a
//      structure discovery — the 582.41 pack/unpack only consumes those 4)
//   2. GetStatus (0x7FEE9032): type / freq_def +0x24 / freq_cur +0x64 /
//      uV +0x58 / uV +0x68
//   3. VoltRails family (P100 rail count is an in-repo blank) +
//      ClientVoltRailsGetStatus core voltage
//   4. a 3 s double-sample diff of both VFP buffers (idle telemetry fields)
//   5. JSON snapshot → ../reverse/p100-58241-vfp-control-raw.json
//      (rename between experiments to keep before/after baselines)
//
// Run: cargo test -p nvapi --test p100_vfp_control_raw_dump -- --nocapture --ignored

#![allow(unused_must_use)]

use core::ptr;
use nvapi::PhysicalGpu;
use nvapi::sys::api::{
    NvAPI_GPU_ClockClkVfPointsGetControl, NvAPI_GPU_ClockClkVfPointsGetInfo,
    NvAPI_GPU_ClockClkVfPointsGetStatus,
};
use nvapi::sys::gpu::clock::undocumented::{
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE,
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
    NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE, clk_vfp_control,
};
use nvapi::sys::nvapi::NvVersion;

/// mask dword for `idx` in `bank`; rest offset = abs - 4 (same as p100_vfp_diag).
fn info_mask_bit(
    info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
    bank: usize,
    idx: usize,
) -> bool {
    let base = if bank == 0 { 4 } else { 0x34304 };
    let off = base + 4 * (idx >> 5) - 4;
    let dword = u32::from_le_bytes(info.rest[off..off + 4].try_into().unwrap());
    dword & (1 << (idx & 31)) != 0
}

fn control_mask_bit(
    ctrl: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE,
    bank: usize,
    idx: usize,
) -> bool {
    let base = if bank == 0 {
        clk_vfp_control::MASK1
    } else {
        clk_vfp_control::MASK2
    };
    let off = base + 4 * (idx >> 5) - 4;
    let dword = u32::from_le_bytes(ctrl.rest[off..off + 4].try_into().unwrap());
    dword & (1 << (idx & 31)) != 0
}

fn rec_base(bank: usize) -> usize {
    if bank == 0 {
        clk_vfp_control::REC1
    } else {
        clk_vfp_control::REC2
    }
}

/// Every non-zero dword in one 1060B record, as (offset, value).
fn record_nonzero_dwords(
    ctrl: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE,
    abs: usize,
) -> Vec<(usize, u32)> {
    let off0 = abs - 4;
    let mut out = Vec::new();
    for o in (0..clk_vfp_control::STRIDE).step_by(4) {
        let p = off0 + o;
        if p + 4 > ctrl.rest.len() {
            break;
        }
        let d = u32::from_le_bytes(ctrl.rest[p..p + 4].try_into().unwrap());
        if d != 0 {
            out.push((o, d));
        }
    }
    out
}

fn capture_control(
    gpu: &PhysicalGpu,
    info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
) -> Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> {
    let mut ctrl = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(clk_vfp_control::MAGIC);
        b.seed_masks_from_info(info);
        b
    };
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetControl(*gpu.handle(), ptr::from_mut(&mut *ctrl).cast())
    };
    eprintln!("GetControl: status={:#x}", st as i32);
    assert_eq!(st, 0, "GetControl failed");
    ctrl
}

fn capture_status(
    gpu: &PhysicalGpu,
    info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
) -> Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE> {
    let mut status = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(2000388u32);
        b
    };
    status.rest[..128].copy_from_slice(&info.rest[..128]); // seed masks
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetStatus(*gpu.handle(), ptr::from_mut(&mut *status).cast())
    };
    eprintln!("GetStatus: status={:#x}", st as i32);
    assert_eq!(st, 0, "GetStatus failed");
    status
}

fn dump_changed_dwords(tag: &str, a: &[u8], b: &[u8]) {
    let n = a.len().min(b.len()) / 4 * 4;
    let mut changed = 0usize;
    for off in (0..n).step_by(4) {
        let da = u32::from_le_bytes(a[off..off + 4].try_into().unwrap());
        let db = u32::from_le_bytes(b[off..off + 4].try_into().unwrap());
        if da != db {
            changed += 1;
            // abs offset in the caller's layout = rest offset + 4
            eprintln!("  [{tag}] +{:#06x}: {da:#010x} -> {db:#010x}", off + 4);
        }
    }
    eprintln!("[{tag}] changed dwords: {changed}");
}

#[test]
#[ignore]
fn p100_vfp_control_raw_dump() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    eprintln!("GPU: {:?}", gpu.full_name());

    // --- GetInfo: masks ---
    let mut info = unsafe {
        let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
        let mut b = b.assume_init();
        b.version = NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC);
        b
    };
    let st = unsafe {
        NvAPI_GPU_ClockClkVfPointsGetInfo(*gpu.handle(), ptr::from_mut(&mut *info).cast())
    };
    eprintln!("GetInfo: status={:#x}", st as i32);
    assert_eq!(st, 0, "GetInfo failed");

    // --- capture #1 ---
    let ctrl1 = capture_control(gpu, &info);
    let status1 = capture_status(gpu, &info);

    // header byte @+260 (RM +316 round-trip field)
    eprintln!("control header byte @+260 = {:#04x}", ctrl1.rest[260 - 4]);

    // --- GetControl records: known fields + full non-zero census ---
    let mut json_records = String::new();
    for bank in 0..2usize {
        for idx in 0..2048usize {
            if !info_mask_bit(&info, bank, idx) || !control_mask_bit(&ctrl1, bank, idx) {
                continue;
            }
            let abs = rec_base(bank) + clk_vfp_control::STRIDE * idx;
            let typ = ctrl1.rest[abs - 4];
            let mode = u32::from_le_bytes(
                ctrl1.rest[abs + clk_vfp_control::MODE - 4..abs + clk_vfp_control::MODE]
                    .try_into()
                    .unwrap(),
            );
            let val_u32 = u32::from_le_bytes(
                ctrl1.rest[abs + clk_vfp_control::VALUE - 4..abs + clk_vfp_control::VALUE]
                    .try_into()
                    .unwrap(),
            );
            let flag = ctrl1.rest[abs + clk_vfp_control::FLAG - 4];
            let nz = record_nonzero_dwords(&ctrl1, abs);
            // outside the 4 known fields (type dword @0, mode @36, value @56, flag @96)
            let extras: Vec<_> = nz
                .iter()
                .copied()
                .filter(|(o, _)| *o != 0 && *o != 36 && *o != 56 && *o != 96)
                .collect();
            eprintln!(
                "b{bank} #{idx:4}: type={typ} mode={mode} value_u32={val_u32} value_i32={} flag={flag:#04x} nonzero_dwords={}{}",
                val_u32 as i32,
                nz.len(),
                if extras.is_empty() {
                    String::new()
                } else {
                    let list: Vec<String> = extras
                        .iter()
                        .map(|(o, d)| format!("{o:#x}:{d:#010x}", d = d))
                        .collect();
                    format!("  EXTRAS{{{}}}", list.join(", "))
                }
            );
            if !json_records.is_empty() {
                json_records.push(',');
            }
            json_records.push_str(&format!(
                "{{\"bank\":{bank},\"index\":{idx},\"type\":{typ},\"mode\":{mode},\"value_u32\":{val_u32},\"flag\":{flag}}}"
            ));
        }
    }

    // --- GetStatus records (the decoded view, incl. all-zero voltage proof) ---
    for bank in 0..2usize {
        let srec = if bank == 0 { 772 } else { 1000964 };
        for idx in 0..2048usize {
            if !info_mask_bit(&info, bank, idx) {
                continue;
            }
            let rec = srec + 488 * idx - 4;
            let rd = |off: usize| -> u32 {
                u32::from_le_bytes(status1.rest[rec + off..rec + off + 4].try_into().unwrap())
            };
            eprintln!(
                "status b{bank} #{idx:4}: type={} freq_def={:6} freq_cur={:6} uV@58={:9} uV@68={:9}",
                status1.rest[rec],
                rd(0x24),
                rd(0x64),
                rd(0x58),
                rd(0x68),
            );
        }
    }

    // --- VoltRails: rail count + live µV (read-only hi-level) ---
    match gpu.volt_rails() {
        Ok(vr) => {
            eprintln!(
                "volt_rails: rail_mask={:#010x} ({} rails)",
                vr.rail_mask,
                vr.rail_mask.count_ones()
            );
            for d in &vr.rail_descriptors {
                eprintln!(
                    "  rail {} desc: type={} class={} uV_a={} uV_b={}",
                    d.rail_bit,
                    d.entry_type(),
                    d.class(),
                    d.uv_a(),
                    d.uv_b()
                );
            }
            for c in &vr.control {
                eprintln!(
                    "  control rail {}: type={} values={:?}",
                    c.rail_bit, c.entry_type, c.values
                );
            }
            for s in &vr.status {
                eprintln!(
                    "  status  rail {}: type={} values={:?}",
                    s.rail_bit, s.entry_type, s.values
                );
            }
        }
        Err(e) => eprintln!("volt_rails: Err {e:?}"),
    }
    match gpu.core_voltage() {
        Ok(v) => eprintln!("client_core_voltage: {v} uV"),
        Err(e) => eprintln!("client_core_voltage: Err {e:?}"),
    }

    // --- double sample (3 s apart): idle-telemetry domains in both buffers ---
    eprintln!("-- sleeping 3 s for the diff sample --");
    std::thread::sleep(std::time::Duration::from_secs(3));
    let ctrl2 = capture_control(gpu, &info);
    let status2 = capture_status(gpu, &info);
    dump_changed_dwords("control", &ctrl1.rest, &ctrl2.rest);
    dump_changed_dwords("status", &status1.rest, &status2.rest);

    // --- JSON snapshot for machine diff (rename between experiments) ---
    let json = format!(
        "{{\"captured_at\":\"{}\",\"gpu\":{:?},\"header_byte_260\":{},\"records\":[{}],\"note\":\"value_u32 is the raw mode-0 dword; interpret as i32 kHz (Pascal 2x axis: effect = i32/2000)\"}}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        gpu.full_name(),
        ctrl2.rest[260 - 4],
        json_records
    );
    let path = "../reverse/p100-58241-vfp-control-raw.json";
    match std::fs::write(path, &json) {
        Ok(()) => eprintln!("snapshot written: {path}"),
        Err(e) => eprintln!("snapshot write failed ({e:?}) — JSON follows:\n{json}"),
    }
}
