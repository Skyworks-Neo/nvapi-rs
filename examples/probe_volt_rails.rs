//! Live read-only probe for the private VoltRails control family — the
//! mechanism melonVolt (OCN RTX 5090 thread) drives via runtime code-scanning,
//! but which is reachable directly through stable QueryInterface IDs that this
//! driver branch (R610.74 mobile) exposes in the PUBLIC QI table.
//!
//! Run: `cargo run --release -p nvapi --example probe_volt_rails`
//!
//! RE summary (see reverse/melonvolt/ANALYSIS.md for the full chain):
//!   0x2C73AFDC  VoltVoltRailsGetInfo    "rail builder": fn(hGPU, rail_struct)
//!               rail struct V2 version 0x2184C = 6220 bytes (V1 0x10ACC =
//!               2764 bytes also accepted). Out: rail mask u32 @+4, then
//!               per-rail entries at 192-byte stride indexed by rail BIT
//!               (type discriminator u32 @ entry+76).
//!               RM layer: escape 0x07000191, ctrl cmd 0x2080A601, ~500 KB buf.
//!   0xA3070DB0  VoltVoltRailsGetControl "getter": fn(hGPU, ctrl_struct)
//!               ctrl struct V2 version 0x20AC8 = 2760 bytes (V1 0x10AC8
//!               accepted, same size). In: mask u32 @+4 + dense per-rail
//!               entries at 84-byte stride, seed/type u32 @ entry+72 copied
//!               from the rail entry. Out: type validated @+72 and SIX u32
//!               filled @+76.. (+76 = value in µV on type-3 entries).
//!               RM layer: escape 0x07000191, ctrl cmd 0x2080A613.
//!   0x87C55C8A  VoltVoltRailsSetControl "commit" — RESOLVED ONLY, NEVER
//!               CALLED by this probe (voltage write; needs snapshot/restore).
//!   0x5D0634EE  VoltVoltRailsGetStatus  (melonVolt: "Live voltage:
//!               intentionally not displayed") — resolved only.
//!
//! On the RTX 5090 melonVolt targets, MSVDD is rail bit 1 (mask 0x2) with
//! entry type 3 carrying a µV offset. On other GPUs the mask/type layout
//! differs — this probe just prints whatever the driver reports.
//!
//! Everything here is read-only; the only calls made are the builder (GET
//! semantics) and the control GET.

use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::api::{NvAPI_EnumPhysicalGPUs, NvAPI_GPU_GetFullName};
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi_QueryInterface;
use nvapi::sys::types::NvAPI_ShortString;

const VOLT_RAILS_GET_INFO: u32 = 0x2C73AFDC; // rail builder
const VOLT_RAILS_GET_STATUS: u32 = 0x5D0634EE; // live rail status
const VOLT_RAILS_GET_CONTROL: u32 = 0xA3070DB0; // control-object GET
const VOLT_RAILS_SET_CONTROL: u32 = 0x87C55C8A; // control-object SET — never called

const RAIL_V2: u32 = 0x2_184C; // 6220 bytes
const CTRL_V2: u32 = 0x2_0AC8; // 2760 bytes

type VoltFn = unsafe extern "C" fn(usize, *mut u8) -> i32;

fn resolve(id: u32, label: &str) -> Option<VoltFn> {
    match nvapi_QueryInterface(id) {
        Ok(ptr) => {
            println!("0x{id:08X} {label:34} -> 0x{ptr:016X}  RESOLVED");
            Some(unsafe { std::mem::transmute::<usize, VoltFn>(ptr) })
        }
        Err(e) => {
            println!("0x{id:08X} {label:34} -> NULL  ({e:?})");
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
    // GCOFF dance (mirrors core::run's pre-wake hook): a 610 mobile dGPU in
    // GC6 makes Initialize fail, and until Initialize succeeds the System32
    // nvapi64 loader never loads nvapi64_impl.dll — so EVERY forwarded ID
    // (public anchors included) resolves NULL. Wake, then initialize, retry.
    let mut initialized = false;
    for attempt in 0..5 {
        match nvapi::initialize() {
            Ok(()) => {
                println!("(NvAPI_Initialize ok on attempt {attempt})\n");
                initialized = true;
                break;
            }
            Err(e) => {
                println!("(NvAPI_Initialize attempt {attempt}: {e:?})");
                let gpus = nvapi::PhysicalGpu::enumerate().unwrap_or_default();
                if let Some(gpu) = gpus.first() {
                    match gpu.force_gc6_exit() {
                        Ok(()) => println!("  force_gc6_exit ok"),
                        Err(w) => println!("  force_gc6_exit: {w:?}"),
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(700));
            }
        }
    }
    if !initialized {
        println!(
            "(could not initialize — QI table may still resolve, but the call chain will not)\n"
        );
    }

    println!("=== VoltRails private family (melonVolt path) ===");
    let builder = resolve(VOLT_RAILS_GET_INFO, "VoltVoltRailsGetInfo (builder)");
    resolve(VOLT_RAILS_GET_STATUS, "VoltVoltRailsGetStatus (live)");
    let getter = resolve(VOLT_RAILS_GET_CONTROL, "VoltVoltRailsGetControl (getter)");
    resolve(VOLT_RAILS_SET_CONTROL, "VoltVoltRailsSetControl (commit)");

    // reference: the public anchors melonVolt starts from
    println!("\n=== public anchors (already wrapped in nvapi-rs) ===");
    resolve(0x9DF23CA1, "ClientVoltRailsGetControl (V1 72B)");
    resolve(0x465F9BCF, "ClientVoltRailsGetStatus");
    resolve(0xB9306D9B, "ClientVoltRailsSetControl");

    let (Some(builder), Some(getter)) = (builder, getter) else {
        println!("\nbuilder/getter unresolved — nothing more to do");
        return;
    };

    let mut handles = [NvPhysicalGpuHandle::default(); NVAPI_MAX_PHYSICAL_GPUS];
    let mut count = 0u32;
    let st = unsafe { NvAPI_EnumPhysicalGPUs(&mut handles, &mut count) };
    println!("\nEnumPhysicalGPUs: status={st} count={count}");
    if st != 0 || count == 0 {
        return;
    }
    let gpu = handles[0];
    let h = gpu.as_ptr() as usize;
    let mut name = NvAPI_ShortString::default();
    let _ = unsafe { NvAPI_GPU_GetFullName(gpu, &mut name) };
    println!("GPU[0]: {}", name.to_string_lossy());

    // ---- 1. rail builder: rail struct V2 (6220B) --------------------------
    let mut rail = vec![0u8; RAIL_V2 as usize & 0xFFFF];
    rail[0..4].copy_from_slice(&RAIL_V2.to_le_bytes());
    let st = unsafe { builder(h, rail.as_mut_ptr()) };
    println!(
        "\nrail builder (V2 0x{RAIL_V2:X}): status={st} ({})",
        status_name(st)
    );
    if st != 0 {
        return;
    }
    let mask = u32::from_le_bytes(rail[4..8].try_into().unwrap());
    println!("rail mask = 0x{mask:08X}  rails: {mask:032b}");
    let mut bits = Vec::new();
    for bit in 0..32 {
        if mask & (1 << bit) != 0 {
            bits.push(bit);
        }
    }
    println!("set bits: {bits:?}");
    for &bit in &bits {
        let base = 192 * bit;
        let typ = u32::from_le_bytes(rail[base + 76..base + 80].try_into().unwrap());
        let d0 = u32::from_le_bytes(rail[base + 80..base + 84].try_into().unwrap());
        println!(
            "  rail[{bit:2}] @{base:#06x}: type={typ} seed_dword[+80]=0x{d0:08X} (+0..+76 zero-checked: {})",
            rail[base..base + 76].iter().all(|&b| b == 0),
        );
    }

    // ---- 2. control GET: ctrl struct V2 (2760B) + dense seeded entries ----
    let mut ctrl = vec![0u8; CTRL_V2 as usize & 0xFFFF];
    ctrl[0..4].copy_from_slice(&CTRL_V2.to_le_bytes());
    ctrl[4..8].copy_from_slice(&mask.to_le_bytes());
    let mut dense = 0usize;
    for bit in 0..32 {
        if mask & (1 << bit) != 0 {
            let seed = &rail[192 * bit + 76..192 * bit + 80];
            ctrl[84 * dense + 72..84 * dense + 76].copy_from_slice(seed);
            dense += 1;
        }
    }
    println!("\ncontrol GET (V2 0x{CTRL_V2:X}, {dense} dense entries):");
    let st = unsafe { getter(h, ctrl.as_mut_ptr()) };
    println!("status={st} ({})", status_name(st));
    if st != 0 {
        return;
    }
    println!(
        "ctrl+8 byte (public API surfaces this as boost) = {}",
        ctrl[8]
    );
    let mut dense = 0usize;
    for bit in 0..32 {
        if mask & (1 << bit) == 0 {
            continue;
        }
        // NOTE: payload is type@+72 + SIX u32 @+76..+100 — this spans past the
        // 84-byte slot stride into the next slot's unused leading bytes (the
        // driver's own getter copies exactly these 6 dwords).
        let base = 84 * dense + 72;
        let typ = u32::from_le_bytes(ctrl[base..base + 4].try_into().unwrap());
        let mut vals = [0i32; 6];
        for (i, v) in vals.iter_mut().enumerate() {
            let off = base + 4 + 4 * i;
            *v = i32::from_le_bytes(ctrl[off..off + 4].try_into().unwrap());
        }
        println!(
            "  rail[{bit:2}] type={typ}  +76..+100: value0={} (0x{:08X}={}.{:03} mV)  rest={:?}",
            vals[0],
            vals[0] as u32,
            vals[0] / 1000,
            vals[0].unsigned_abs() % 1000,
            &vals[1..],
        );
        dense += 1;
    }

    // ---- 3. GetStatus (live rail values) ---------------------------------
    // GPUMon probes this ID with magics 68296 (0x10AC8, 2760B) / 68300
    // (0x10ACC, 2764B) — V1-era stamps, NOT the 6220B rail V2.
    println!("\n=== VoltRailsGetStatus (live values) ===");
    if let Ok(ptr) = nvapi_QueryInterface(VOLT_RAILS_GET_STATUS) {
        let getstatus: VoltFn = unsafe { std::mem::transmute(ptr) };
        for (ver, size) in [(0x1_0ACCu32, 2764usize), (0x1_0AC8, 2760), (CTRL_V2, 2760)] {
            let mut buf = vec![0u8; size];
            buf[0..4].copy_from_slice(&ver.to_le_bytes());
            if ver != 0x1_0ACC {
                // seed like the control GET: mask + dense entries from the rail builder
                buf[4..8].copy_from_slice(&mask.to_le_bytes());
                let mut d = 0usize;
                for bit in 0..32 {
                    if mask & (1 << bit) != 0 {
                        buf[84 * d + 72..84 * d + 76]
                            .copy_from_slice(&rail[192 * bit + 76..192 * bit + 80]);
                        d += 1;
                    }
                }
            }
            let st = unsafe { getstatus(h, buf.as_mut_ptr()) };
            println!(
                "GetStatus version 0x{ver:X} ({size}B): status={st} ({})",
                status_name(st)
            );
            if st == 0 {
                let mask2 = u32::from_le_bytes(buf[4..8].try_into().unwrap());
                println!("mask @+4 = 0x{mask2:08X}");
                let mut d = 0usize;
                for bit in 0..32 {
                    if mask & (1 << bit) == 0 {
                        continue;
                    }
                    let base = 84 * d + 72;
                    if base + 28 > size {
                        break;
                    }
                    let typ = u32::from_le_bytes(buf[base..base + 4].try_into().unwrap());
                    let vals: Vec<i32> = (0..6)
                        .map(|i| {
                            i32::from_le_bytes(
                                buf[base + 4 + 4 * i..base + 8 + 4 * i].try_into().unwrap(),
                            )
                        })
                        .collect();
                    println!("  rail[{bit:2}] type={typ} values(µV?)={vals:?}");
                    d += 1;
                }
                break;
            }
        }
    }
}
