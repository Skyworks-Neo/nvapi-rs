//! Live read-only probe for the private ClockClient clock-domain family —
//! the Windows NVAPI wrappers around the NV2080 RM ClockClient commands that
//! the Loong0x00 Blackwell-XBAR article (LACT #1147, xbar_clock_demo.c) drives
//! via the Linux RM-ioctl path. On Windows these are reached through stable
//! QueryInterface IDs already registered in nvid.rs (nvClocks.spec block):
//!
//!   0x57B5A5DF  ClockClkDomainsGetInfo     = CLK_CLK_DOMAINS_GET_INFO    (0x20809019)
//!   0xF58938F5  ClockClkDomainsGetControl  = CLK_CLK_DOMAINS_GET_CONTROL (0x2080901b)
//!   0xD14B69CF  ClockClkDomainsSetControl  = CLK_CLK_DOMAINS_SET_CONTROL (0x2080d01c)  [RESOLVED ONLY]
//!   0xFB8F61EC  ClockCounterMeasureAvgFreq = CLK_MEASURE_FREQ            (0x20809006)
//!   0x7FEE9032  ClockClkVfPointsGetStatus  = VF_POINTS_GET_STATUS        (private; XBAR bank)
//!
//! Run: `cargo run --release -p nvapi --example probe_clk_domains`
//!
//! Article layout reference (R610.57.04 Linux, GB202):
//!   GET_INFO      size 0x3030  -> controllable-domain mask + offset ranges
//!   GET_CONTROL   size 0x083c  -> full domain-control block
//!   SET_CONTROL   size 0x083c  -> write full block (NOT CALLED by this probe)
//!   MEASURE_FREQ  size 0x0008  -> {domain_mask in, measured kHz out}
//!   Control block: +0x04 controllable mask; domain header 0x3c, stride 0x40;
//!     XBAR domain idx 1, base 0x7c; +0x84 freq-offset mode (byte, 0=apply);
//!     +0x88 signed freq offset kHz; +0x8c rail-offset base; +0x90 signed
//!     MSVDD offset µV (rail 1).
//!
//! Windows R572 branch uses DIFFERENT sizes: GET_INFO=0x2730, GET/SET_CONTROL=0x7bc
//! (article, Xbar.txt:138). This dev machine is R575.74 (Ada 4060 Laptop) —
//! expect R57x-family sizes, AND expect XBAR may NOT be a separately
//! controllable domain on Ada (the article is GB202/Blackwell-specific). The
//! probe discovers both by trying the known size table and reading whatever
//! mask the driver returns. Per the article's safety rule: unknown/mismatched
//! size = refuse to interpret; report only.

use nvapi::sys::NVAPI_MAX_PHYSICAL_GPUS;
use nvapi::sys::api::{NvAPI_EnumPhysicalGPUs, NvAPI_GPU_GetFullName};
use nvapi::sys::handles::NvPhysicalGpuHandle;
use nvapi::sys::nvapi_QueryInterface;
use nvapi::sys::types::NvAPI_ShortString;

// The 4 article ClockClient IDs + the private V/F status. All are already
// registered as enum variants in nvapi-rs/sys/src/nvid.rs (nvClocks.spec).
const CLK_DOMAINS_GET_INFO: u32 = 0x57B5A5DF;
const CLK_DOMAINS_GET_CONTROL: u32 = 0xF58938F5;
const CLK_DOMAINS_SET_CONTROL: u32 = 0xD14B69CF; // resolved only — never called
const CLK_MEASURE_FREQ: u32 = 0xFB8F61EC;
const CLK_VF_POINTS_GET_STATUS: u32 = 0x7FEE9032;

// IDA-discovered version magics (sub_18020A8A0/sub_1802091B0/sub_18021DC90
// in nvapi64_impl_live.dll, R575.74). The Windows NVAPI layer does NOT use
// the article's raw Linux RM param sizes (0x3030/0x83c/0x08); it repacks
// into its own versioned structs. Each handler validates (*a2) against a
// small set of accepted version magics and writes the corresponding NV2080
// RM cmd ID into v6[13] (confirmed: 0x20809019 / 0x2080901b / 0x20809006 —
// identical to the article). Escape for all three: sub_180389320/4A0 with
// 117440585 = 0x07000109 (same 0x0700_01xx private family as VoltRails).
//
//   GetInfo magics:    0x109B8(67992) 0x21614(136724) 0x34098(213272) 0x4868C(296588) 0x5058C(329356)
//   GetControl magics: 0x10964(67940) 0x26154(156068)
//   MeasureFreq magics:0x10020(65568=V1) 0x20020(131104=V2) 0x30138(196984=V3)
//
// We probe the LARGEST accepted magic of each (newest layout) first, then
// fall back. MEASURE V1 (0x10020) is the simplest single-domain read.
const GET_INFO_MAGICS: &[(u32, usize)] = &[
    (0x5058C, 0x5058C as usize & 0xFFFFF), // 329356 — but struct size is the magic's low bits+?
];
// NOTE: the NVAPI version-magic convention (nvversion!) encodes (version<<16)|size
// in some structs. We pass the magic at +0 and size the buffer to the magic value
// itself interpreted as a byte count ceiling. The handler memsets a 0x98240
// (623168) internal scratch buf regardless; the USER buffer just needs the magic
// at +0 and enough room for the per-domain payload. We allocate generously.
const GET_INFO_BUF_SIZE: usize = 0x5058C + 0x1000; // headroom past largest magic
const GET_CONTROL_MAGICS: &[(u32, usize)] = &[(0x26154, 0x26154), (0x10964, 0x10964)];
const GET_CONTROL_BUF_SIZE: usize = 0x26154 + 0x1000;
const MEASURE_V1: u32 = 0x10020;
const MEASURE_V1_BUF_SIZE: usize = 0x10020 + 0x100;

// Article domain masks (GB202 PMU clock-domain table, Xbar.txt:22-27):
//   GPCCLK 0x1, XBARCLK 0x2, SYSCLK 0x4, MCLK 0x10, XBAR2CLK 0x40000
const DOMAIN_NAMES: &[(u32, &str)] = &[
    (0x1, "GPCCLK"),
    (0x2, "XBARCLK"),
    (0x4, "SYSCLK"),
    (0x10, "MCLK"),
    (0x40000, "XBAR2CLK"),
];

type RawFn = unsafe extern "C" fn(usize, *mut u8) -> i32;

fn resolve(id: u32, label: &str) -> Option<RawFn> {
    match nvapi_QueryInterface(id) {
        Ok(ptr) => {
            println!("0x{id:08X} {label:34} -> 0x{ptr:016X}  RESOLVED");
            if ptr == 0 {
                None
            } else {
                Some(unsafe { std::mem::transmute::<usize, RawFn>(ptr) })
            }
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

fn domain_name(mask: u32) -> String {
    let mut names = Vec::new();
    for (bit, name) in DOMAIN_NAMES {
        if mask & bit != 0 {
            names.push(*name);
        }
    }
    if names.is_empty() {
        format!("0x{mask:08X} (unknown domains)")
    } else {
        format!("0x{mask:08X} = {}", names.join("|"))
    }
}

fn main() {
    // GCOFF dance (mirrors core::run pre-wake + probe_volt_rails): a mobile
    // dGPU in GC6 makes Initialize fail and the System32 nvapi64 loader never
    // loads nvapi64_impl.dll, so EVERY forwarded ID resolves NULL. Wake first.
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
        println!("(could not initialize — QI may still resolve, call chain will not)\n");
    }

    println!("=== ClockClient private family (XBAR article path) ===");
    let get_info = resolve(CLK_DOMAINS_GET_INFO, "ClockClkDomainsGetInfo");
    let get_control = resolve(CLK_DOMAINS_GET_CONTROL, "ClockClkDomainsGetControl");
    resolve(
        CLK_DOMAINS_SET_CONTROL,
        "ClockClkDomainsSetControl (SET — never called)",
    );
    let measure = resolve(CLK_MEASURE_FREQ, "ClockCounterMeasureAvgFreq");
    resolve(
        CLK_VF_POINTS_GET_STATUS,
        "ClockClkVfPointsGetStatus (private V/F)",
    );

    if get_info.is_none() && get_control.is_none() && measure.is_none() {
        println!("\nall key IDs unresolved — nothing more to do");
        return;
    }

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

    // ---- 1. GET_INFO: discover controllable-domain mask -------------------
    // IDA (sub_18020A8A0): handler reads user struct: if *a2==magic accept,
    // else alloc 0x106AC scratch and stamp magic. After escape it writes the
    // mask at v10[2]=v6[15] (user +8) and per-domain records. The USER struct
    // layout: +0 magic, +4 ?, +8 domain-mask-out, +12.. per-domain @524*idx.
    // We try the largest accepted magic (0x5058C) then smaller.
    let mut controllable_mask: Option<u32> = None;
    let mut info_magic_used: Option<u32> = None;
    if let Some(info_fn) = get_info {
        println!("\n--- GET_INFO (controllable-domain discovery) ---");
        for &magic in &[0x5058Cu32, 0x4868C, 0x34098, 0x21614, 0x109B8] {
            let mut buf = vec![0u8; GET_INFO_BUF_SIZE];
            buf[0..4].copy_from_slice(&magic.to_le_bytes());
            let st = unsafe { info_fn(h, buf.as_mut_ptr()) };
            println!(
                "GET_INFO magic 0x{magic:X}: status={st} ({})",
                status_name(st)
            );
            if st == 0 {
                info_magic_used = Some(magic);
                // mask at user +8 per IDA (v10[2] = v6[15]; v6[15]=*(a2+8))
                let mask = u32::from_le_bytes(buf[8..12].try_into().unwrap());
                controllable_mask = Some(mask);
                println!("  controllable-domain mask @+8 = {}", domain_name(mask));
                println!("  +0..+0x40: {}", hexdump(&buf[..0x40]));
                break;
            }
        }
        if info_magic_used.is_none() {
            println!("  -> GET_INFO rejected all IDA-discovered magics.");
        }
    }

    // ---- 2. GET_CONTROL: read domain-control block (READ-ONLY) -----------
    // IDA (sub_1802091B0): handler reads *(a2) magic {0x10964,0x26154}, reads
    // domain mask from user +8 (v6[15]=*(a2+8)), writes v6[13]=0x2080901B.
    // V1 (0x10964): per-domain records at a2+72*idx, record field at +0x64
    //   (v21[25] type via sub_18015BB30), values at v21[36..40].
    // V2 (0x26154): per-domain at a2+292+772*idx, values at v27[67..76].
    // We pass mask at +8, let the handler fill records, then dump them.
    let mut ctrl_magic_used: Option<u32> = None;
    if let Some(ctrl_fn) = get_control {
        println!("\n--- GET_CONTROL (domain-control block, READ-ONLY) ---");
        for &magic in &[0x26154u32, 0x10964] {
            let mut buf = vec![0u8; GET_CONTROL_BUF_SIZE];
            buf[0..4].copy_from_slice(&magic.to_le_bytes());
            if let Some(m) = controllable_mask {
                buf[8..12].copy_from_slice(&m.to_le_bytes());
            } else {
                // article default controllable mask on GB202 is 0xFF
                buf[8..12].copy_from_slice(&0xFFu32.to_le_bytes());
            }
            let st = unsafe { ctrl_fn(h, buf.as_mut_ptr()) };
            println!(
                "GET_CONTROL magic 0x{magic:X}, mask-seeded: status={st} ({})",
                status_name(st)
            );
            if st == 0 {
                ctrl_magic_used = Some(magic);
                let mask = u32::from_le_bytes(buf[8..12].try_into().unwrap());
                println!("  controllable mask @+8 = {}", domain_name(mask));
                // dump per-domain records for the known clock domains
                let (rec_stride, rec_base, val_off) = if magic == 0x26154 {
                    (772usize, 292usize, 67) // V2: a2+292+772*idx, vals at v27[67..]
                } else {
                    (72, 0, 36) // V1: a2+72*idx, vals at v21[36..]
                };
                for &dbit in &[0x1u32, 0x2, 0x4, 0x10, 0x40000] {
                    if mask & dbit == 0 {
                        continue;
                    }
                    let idx = dbit.trailing_zeros() as usize;
                    let base = rec_base + rec_stride * idx;
                    if base + 0x40 <= buf.len() {
                        let rec = &buf[base..base + 0x40];
                        let dname = DOMAIN_NAMES
                            .iter()
                            .find(|(b, _)| *b == dbit)
                            .map(|(_, n)| *n)
                            .unwrap_or("?");
                        println!(
                            "  {dname:8} (bit 0x{dbit:X}, idx {idx}) @0x{base:04X}: {}",
                            hexdump(rec)
                        );
                    }
                }
                break;
            }
        }
        if ctrl_magic_used.is_none() {
            println!("  -> GET_CONTROL rejected both IDA-discovered magics.");
        }
    }

    // ---- 3. MEASURE_FREQ: hardware-counter physical clock read -------------
    // IDA (sub_18021DC90): handler validates *(a2) magic {0x10020(V1),0x20020(V2),
    // 0x30138(V3)}, then `sub_18017A680(v6+14, a2[1])` validates a2[1] (offset 4)
    // as a SEQUENTIAL domain index and writes the mask into the internal scratch.
    // Domain index→mask table (sub_18017A680): 0→0x1(GPC),1→0x2(XBAR),2→0x4(SYS),
    // 4→0x10(MCLK),10→0x20000,22→0x40000(XBAR2). So for XBARCLK pass index 1.
    //
    // V1 (0x10020) I/O path:
    //   input : v6[80] = a2[2]          (offset 8, u32 — request/sample config)
    //           v6[72] = *(u64*)(a2+8)  (offset 8..15)
    //   escape 0x07000109
    //   output: a2[2]   = v6[20]  (offset 8  ← v6+80, read-modify-write)
    //           *(u64*)(a2+2) = v6[9]  (offset 16 ← v6+72)
    //           a2[6]   = v6[22]  (offset 24 ← v6+88)
    // The freq kHz is one of these three; dump all 32 bytes to disambiguate.
    if let Some(meas_fn) = measure {
        println!("\n--- CLK_MEASURE_FREQ (hardware-counter physical clocks) ---");
        // Decisive unit test: sample each domain TWICE with a sleep between.
        //  - if +8 is a direct frequency: reading2 ≈ reading1 (stable clock)
        //  - if +8 is a cycle counter: reading2 = reading1 + freq*Δt (grows)
        // +16 is a ns timestamp (QPC). freq = Δcounter / Δtime_ns (GHz) if counter.
        for (didx, dbit, dname) in [
            (0u32, 0x1u32, "GPCCLK"),
            (1u32, 0x2u32, "XBARCLK"),
            (2u32, 0x4u32, "SYSCLK"),
            (4u32, 0x10u32, "MCLK"),
            (10u32, 0x20000u32, "XBAR2CLK"),
        ] {
            let mut buf1 = vec![0u8; MEASURE_V1_BUF_SIZE];
            buf1[0..4].copy_from_slice(&MEASURE_V1.to_le_bytes());
            buf1[4..8].copy_from_slice(&didx.to_le_bytes());
            let st1 = unsafe { meas_fn(h, buf1.as_mut_ptr()) };
            let c1 = u32::from_le_bytes(buf1[8..12].try_into().unwrap()) as u64;
            let t1 = u64::from_le_bytes(buf1[16..24].try_into().unwrap());

            std::thread::sleep(std::time::Duration::from_millis(100));

            let mut buf2 = vec![0u8; MEASURE_V1_BUF_SIZE];
            buf2[0..4].copy_from_slice(&MEASURE_V1.to_le_bytes());
            buf2[4..8].copy_from_slice(&didx.to_le_bytes());
            let st2 = unsafe { meas_fn(h, buf2.as_mut_ptr()) };
            let c2 = u32::from_le_bytes(buf2[8..12].try_into().unwrap()) as u64;
            let t2 = u64::from_le_bytes(buf2[16..24].try_into().unwrap());

            let dc = c2 as i64 - c1 as i64;
            let dt_ns = t2 as i64 - t1 as i64;
            println!(
                "  {dname:8} (idx {didx}) s1={st1}({}) s2={st2}({})",
                status_name(st1),
                status_name(st2)
            );
            println!(
                "      +8: c1={c1}  c2={c2}  Δc={dc}      +16: t1={t1}  t2={t2}  Δt_ns={dt_ns}"
            );
            if dt_ns > 0 {
                // counter hypothesis: freq_Hz = Δc / Δt_ns * 1e9
                let freq_from_delta = dc as f64 / dt_ns as f64 * 1e9;
                println!(
                    "      if-counter: freq = Δc/Δt = {freq_from_delta:.0} Hz ({:.3} MHz)",
                    freq_from_delta / 1e6
                );
            }
            // direct-freq hypothesis: +8 IS the freq (report as Hz and kHz)
            println!(
                "      if-direct-Hz: c1={:.3} MHz   if-direct-kHz: c1={:.3} MHz",
                c1 as f64 / 1e6,
                c1 as f64 / 1e3
            );
        }
    }

    println!("\n=== summary ===");
    println!(
        "GET_INFO magic:    {:?}",
        info_magic_used.map(|m| format!("0x{m:X}"))
    );
    println!(
        "GET_CONTROL magic: {:?}",
        ctrl_magic_used.map(|m| format!("0x{m:X}"))
    );
    println!(
        "controllable mask: {:?}",
        controllable_mask.map(|m| domain_name(m))
    );
    println!("\nNOTE: SET_CONTROL (0xD14B69CF) was resolved-only, NEVER called.");
    println!("NOTE: live-verified mask 0xFF on Ada 4060 Laptop INCLUDES XBARCLK bit");
    println!("      0x2 — XBAR-as-controllable is NOT Blackwell-only (the article's");
    println!("      GB202-specificity was about the independent V/F curve behavior).");
}

fn hexdump(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 3);
    for (i, &byte) in b.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        s.push_str(&format!("{byte:02X}"));
    }
    s
}
