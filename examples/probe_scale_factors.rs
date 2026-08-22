//! Probe the two undecoded V/F-points sibling IDs — IDA-verified real RM
//! handlers (NOT stubs) that form a GetInfo/GetStatus/SetStatus sub-trio
//! within the private ClockClient V/F-points family:
//!
//!   0xCF08E934  GetInfo-type   (handler sub_180211FB0, magic 0x10A8C=68236,
//!                               struct 2700B = 32×76B + 268B header,
//!                               sub-cmd 0x20809079, RM escape 0x07000049 arg5=1 READ)
//!   0x4F11EAA4  GetStatus-type (handler sub_180211910, magic 0x12148=74056,
//!                               struct 8520B = 32×264B + 72B header,
//!                               sub-cmd 0x2080907B, same escape, READ)
//!   0x6BD3BB9E  SetStatus-type (sub-cmd 0x2080D07C, WRITE — NOT CALLED here)
//!
//! Both return ONE scalar per V/F-point (bitmask-selected, 32 points) — the
//! hypothesized per-voltage scaling factor C(V) from the mode-1 delta model
//!   effect(V, delta) = C × (delta - D0(V))
//! empirically derived on an A100 sweep as C = 0.3 MHz/delta (global, clean
//! linear R²=1) and D0(V) = 0 @ 800mV, 50 @ ~730mV. This probe reads the raw
//! per-point values to confirm whether they encode C (expect ~0.3 const) or
//! D0 (expect 0/50 pattern) or something else entirely.
//!
//! LAYOUT (from decompilation of sub_180211FB0 / sub_180211910):
//!   GetInfo output:  [0]=magic, [1]=status(0/1/15), [2]=point bitmask,
//!                     per-point @ byte 268 + 76*idx: status, enum(19-way), value
//!   GetStatus:       [0]=magic, [1]=point bitmask (IN, seeded from GetInfo[2]),
//!                     per-point @ byte 72 + 264*idx: status, value
//!
//! Read-only and safe. IDA source: reverse/nvapi64_impl.dll.
use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::api::{NvAPI_EnumPhysicalGPUs, NvAPI_GPU_GetFullName};
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi_QueryInterface;
use nvapi::sys::types::NvAPI_ShortString;

const SF_GET_INFO: u32 = 0xCF08E934;
const SF_GET_STATUS: u32 = 0x4F11EAA4;

const INFO_MAGIC: u32 = 0x10A8C; // = 68236 (version 1 | size 0xA8C)
const STATUS_MAGIC: u32 = 0x12148; // = 74056 (version 1 | size 0x2148)
const INFO_SIZE: usize = 2700; // 32 pts × 76B + 268B header
const STATUS_SIZE: usize = 8520; // 32 pts × 264B + 72B header

// per-point field byte offsets (from decompile)
const INFO_PT_BASE: usize = 268; // point 0 starts at byte 268
const INFO_PT_STRIDE: usize = 76; // status@+0, enum@+4, value@+8
const STATUS_PT_BASE: usize = 72; // point 0 starts at byte 72
const STATUS_PT_STRIDE: usize = 264; // status@+0, value@+4

type QIFn = unsafe extern "C" fn(usize, *mut u8) -> i32;

fn resolve(id: u32, label: &str) -> Option<QIFn> {
    match nvapi_QueryInterface(id) {
        Ok(ptr) => {
            println!("0x{id:08X} {label:30} -> 0x{ptr:016X}  RESOLVED");
            Some(unsafe { std::mem::transmute::<usize, QIFn>(ptr) })
        }
        Err(e) => {
            println!("0x{id:08X} {label:30} -> NULL  ({e:?})");
            None
        }
    }
}

fn status_name(st: i32) -> &'static str {
    match st {
        0 => "OK",
        -4 => "NOT_INITIALIZED",
        -9 => "INVALID_ARGUMENT (version/size mismatch?)",
        -13 => "API_NOT_INTIALIZED",
        -101 => "INVALID_HANDLE?",
        -130 => "NO_IMPLEMENTATION / alloc failed",
        -163 => "NOT_SUPPORTED",
        _ => "other",
    }
}

fn main() {
    let _ = nvapi::initialize();
    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    let st = unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    println!("EnumPhysicalGPUs: status={st} count={count}");
    if st != 0 || count == 0 {
        return;
    }
    let gpu = handles[0];
    let h = gpu.as_ptr() as usize;
    let mut name = NvAPI_ShortString::default();
    let _ = unsafe { NvAPI_GPU_GetFullName(gpu, &mut name) };
    println!("GPU[0]: {}\n", name.to_string_lossy());

    println!("=== scaling-factor sibling IDs (V/F-points family, 5th/6th sibling) ===");
    let info_fn = resolve(SF_GET_INFO, "GetInfo (0xCF08E934)");
    let status_fn = resolve(SF_GET_STATUS, "GetStatus (0x4F11EAA4)");
    // also resolve the SetStatus sibling for awareness — but never call it
    resolve(0x6BD3BB9E, "SetStatus (0x6BD3BB9E, WRITE)");

    let (Some(info_fn), Some(status_fn)) = (info_fn, status_fn) else {
        println!("\nGetInfo/GetStatus unresolved — nothing more to do");
        return;
    };

    // ---------- 1. GetInfo (0xCF08E934) — descriptor census + bitmask ----------
    let mut info = vec![0u8; INFO_SIZE];
    info[0..4].copy_from_slice(&INFO_MAGIC.to_le_bytes());
    let st = unsafe { info_fn(h, info.as_mut_ptr()) };
    println!("\nGetInfo  st={st} ({})  magic=0x{INFO_MAGIC:X} size={INFO_SIZE}", status_name(st));
    if st != 0 {
        return;
    }
    let info_status = u32::from_le_bytes(info[4..8].try_into().unwrap());
    let bitmask = u32::from_le_bytes(info[8..12].try_into().unwrap());
    println!("  status_code={info_status}  bitmask=0x{bitmask:08X} ({}/32 points set)", bitmask.count_ones());

    println!("\n  idx  enum  value");
    for idx in 0..32u32 {
        if bitmask & (1 << idx) == 0 {
            continue;
        }
        let base = INFO_PT_BASE + INFO_PT_STRIDE * idx as usize;
        let pstatus = i32::from_le_bytes(info[base..base + 4].try_into().unwrap());
        let penum = u32::from_le_bytes(info[base + 4..base + 8].try_into().unwrap());
        let pvalue = u32::from_le_bytes(info[base + 8..base + 12].try_into().unwrap());
        println!("  [{idx:2}]  enum={penum:2}  value=0x{pvalue:08X} ({pvalue:12})  pst={pstatus}");
    }

    // ---------- 2. GetStatus (0x4F11EAA4) — per-point scalars ----------
    // bitmask seeded at [1] (byte 4) from GetInfo's [2] (byte 8)
    let mut stat = vec![0u8; STATUS_SIZE];
    stat[0..4].copy_from_slice(&STATUS_MAGIC.to_le_bytes());
    stat[4..8].copy_from_slice(&bitmask.to_le_bytes());
    let st = unsafe { status_fn(h, stat.as_mut_ptr()) };
    println!("\nGetStatus st={st} ({})  magic=0x{STATUS_MAGIC:X} size={STATUS_SIZE}", status_name(st));
    if st != 0 {
        return;
    }

    println!("\n  idx  value         /256      /1024     /1000     as_f32    pst");
    for idx in 0..32u32 {
        if bitmask & (1 << idx) == 0 {
            continue;
        }
        let base = STATUS_PT_BASE + STATUS_PT_STRIDE * idx as usize;
        let pstatus = i32::from_le_bytes(stat[base..base + 4].try_into().unwrap());
        let pvalue = u32::from_le_bytes(stat[base + 4..base + 8].try_into().unwrap());
        let f = f32::from_bits(pvalue);
        let q8 = pvalue as f64 / 256.0;
        let q10 = pvalue as f64 / 1024.0;
        let m = pvalue as f64 / 1000.0;
        println!(
            "  [{idx:2}]  0x{pvalue:08X} {pvalue:12}  {q8:8.4}  {q10:8.4}  {m:8.4}  {f:11.5}  pst={pstatus}"
        );
    }

    // ---------- 3. raw dword scan (catch layout surprises / header fields) ----------
    println!("\n=== raw non-zero dwords ===");
    println!("-- info --");
    for i in 0..(INFO_SIZE / 4) {
        let off = i * 4;
        let v = u32::from_le_bytes(info[off..off + 4].try_into().unwrap());
        if v != 0 {
            println!("  info[{i:3}] @+0x{off:04X} = 0x{v:08X} ({v})");
        }
    }
    println!("-- status --");
    for i in 0..(STATUS_SIZE / 4) {
        let off = i * 4;
        let v = u32::from_le_bytes(stat[off..off + 4].try_into().unwrap());
        if v != 0 {
            println!("  stat[{i:3}] @+0x{off:04X} = 0x{v:08X} ({v})");
        }
    }
}
