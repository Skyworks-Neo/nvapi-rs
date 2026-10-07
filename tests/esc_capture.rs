//! ESC 传输抓包 harness(2026-10-07,TDR 根因研究)。
//!
//! 原理:nvapi64_impl.dll 在本进程内经 `\\.\NvAdminDevice` 的 DeviceIoControl
//! 发 ESC 命令(0x08DE0008/0x08DE0010,缓冲 = 12B 前缀 + w,w+0x34 = cmd)。
//! 本测试 IAT-hook 该 dll 的 DeviceIoControl:
//! - 每次调用先落盘(seq/ioctl/size/缓冲 hex + 头字段解析);
//! - 命中 NVOC_ESC_BLOCK_CMD(默认 0x2080E61B,即毒 SET)时**拦截不下发**,
//!   返回失败让上层流程优雅中止 —— 零 TDR 风险;
//! - 其余调用放行。
//!
//! 步骤:① percent 写(0xad95f5ed,安全路径,对照);② watt 写(毒路径,SET 被拦)。
//! 环境变量:NVOC_ESC_CAPTURE(输出目录)、NVOC_ESC_BLOCK_CMD(十六进制 cmd)。
#![cfg(windows)]

use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

type DeviceIoControlFn = unsafe extern "system" fn(
    isize,
    u32,
    *mut c_void,
    u32,
    *mut c_void,
    u32,
    *mut u32,
    *mut c_void,
) -> i32;

const BLOCK_CMD_DEFAULT: u32 = 0x2080_E61B;

static HOOK_COUNT: AtomicUsize = AtomicUsize::new(0);
static mut ORIG_DEVICE_IO_CONTROL: Option<DeviceIoControlFn> = None;

// ---- 裸 FFI(nvapi 包无 windows-sys 依赖,自带声明) ----
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> isize;
    fn VirtualProtect(addr: *mut c_void, size: usize, new: u32, old: *mut u32) -> i32;
    fn GetCurrentProcess() -> isize;
    fn LoadLibraryA(name: *const u8) -> isize;
    fn GetProcAddress(h: isize, name: *const u8) -> *const c_void;
    fn VirtualAlloc(addr: *mut c_void, size: usize, alloc: u32, protect: u32) -> *mut c_void;
    fn GetFinalPathNameByHandleW(h: isize, buf: *mut u16, sz: u32, flags: u32) -> u32;
    fn VirtualQuery(addr: *mut c_void, buf: *mut u8, sz: usize) -> usize;
}

/// 扫进程内已提交可写区,定位毒 payload(共享节里的 stamp/watt)。毒 TgpWatt
/// 家族的 GET/SET ioctl 缓冲逐字节相同,数据只能走驱动映射进本进程的节。
fn scan_process_memory(needles: &[(u32, &str)]) {
    const MEM_COMMIT: u32 = 0x1000;
    const MEM_MAPPED: u32 = 0x40000;
    let mut addr: usize = 0;
    let mut mbi = [0u8; 48];
    let mut report = String::new();
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let mut regions = 0usize;
    let mut idx = 0usize;
    while addr < 0x7fff_ffff_0000 {
        let n = unsafe { VirtualQuery(addr as *mut c_void, mbi.as_mut_ptr(), 48) };
        if n == 0 {
            break;
        }
        let region_base = usize::from_le_bytes(mbi[0..8].try_into().unwrap());
        let region_size = usize::from_le_bytes(mbi[24..32].try_into().unwrap());
        let state = u32::from_le_bytes(mbi[32..36].try_into().unwrap());
        let protect = u32::from_le_bytes(mbi[36..40].try_into().unwrap());
        let ty = u32::from_le_bytes(mbi[40..44].try_into().unwrap());
        // 只看 MEM_MAPPED(驱动发布的共享节必是 mapped),避开私有堆自扰。
        let examine = state == MEM_COMMIT
            && ty == MEM_MAPPED
            && (protect & 0x100) == 0
            && region_size > 0
            && region_size < 0x4000_0000;
        if examine {
            regions += 1;
            let data =
                unsafe { core::slice::from_raw_parts(region_base as *const u8, region_size) };
            let mut found = Vec::new();
            for (val, name) in needles {
                let pat = val.to_le_bytes();
                let mut off = 0usize;
                while off + 4 <= data.len() {
                    if data[off..off + 4] == pat {
                        found.push((region_base + off, *val, (*name).to_string()));
                    }
                    off += 4;
                }
            }
            report.push_str(&format!(
                "\n=== mapped[{idx}] base={region_base:#x} size={region_size:#x} prot={protect:#x} hits={} ===\n",
                found.len()
            ));
            for (a, v, nm) in &found {
                report.push_str(&format!("  {nm} ({v:#x}) @ {a:#x}\n"));
            }
            // 倾掉头部(前 0x200)便于识别表结构。
            let head = &data[..data.len().min(0x200)];
            for (i, chunk) in head.chunks(16).enumerate() {
                let hex: String = chunk
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<Vec<_>>()
                    .join(" ");
                let asc: String = chunk
                    .iter()
                    .map(|&b| {
                        if (0x20..0x7f).contains(&b) {
                            b as char
                        } else {
                            '.'
                        }
                    })
                    .collect();
                report.push_str(&format!(
                    "{:#012x}  {hex:<48} {asc}\n",
                    region_base + i * 16
                ));
            }
            let _ = std::fs::write(
                dir.join(format!("mapped-{idx:03}-{region_base:x}.txt")),
                report.split("\n=== mapped[").last().unwrap_or(""),
            );
            idx += 1;
        }
        addr = region_base + region_size.max(0x1000);
    }
    println!("--- mapped scan: {regions} MEM_MAPPED regions ---");
    println!("{report}");
    let _ = std::fs::write(dir.join("scan.txt"), report);
}

/// 把设备句柄解析成 `\\.\...` 设备路径,用于区分毒传输(0x470807)与 ESC
/// (0x8de000x)是否同一设备。
fn device_path(h: isize) -> String {
    let mut buf = [0u16; 512];
    let n = unsafe { GetFinalPathNameByHandleW(h, buf.as_mut_ptr(), 512, 0) };
    if n == 0 || n as usize > 512 {
        return format!("h={h:#x}");
    }
    format!("{}(h={h:#x})", String::from_utf16_lossy(&buf[..n as usize]))
}

/// 内联 hook:`kernel32!DeviceIoControl` 入口前 15 字节是三条 5 字节
/// `mov [rsp+X],reg`(本机 0x7ffe71581ac0 实测 `48 89 5c 24 08 / 48 89 6c 24 18
/// / 48 89 74 24 20`),第 15 字节正好是指令边界。入口写 12 字节绝对跳转
/// (`mov rax,imm64; jmp rax`)直取 hook;把被偷的 15 字节搬进 trampoline 并追加
/// 跳回 `entry+15`,于是缓存过函数指针的调用方也一并被捕获(纯 IAT 补丁会漏)。
unsafe fn install_inline_hook() -> Option<DeviceIoControlFn> {
    let h = unsafe { LoadLibraryA(b"kernel32.dll\0".as_ptr()) };
    let target = unsafe { GetProcAddress(h, b"DeviceIoControl\0".as_ptr()) } as *mut u8;
    if target.is_null() {
        eprintln!("找不到 kernel32!DeviceIoControl");
        return None;
    }
    let pro = unsafe { core::slice::from_raw_parts(target, 16) };
    eprintln!("DeviceIoControl @{target:p} prologue={:02x?}", &pro[..16]);
    let three_stores = pro[0] == 0x48
        && pro[1] == 0x89
        && pro[5] == 0x48
        && pro[6] == 0x89
        && pro[10] == 0x48
        && pro[11] == 0x89;
    if !three_stores {
        eprintln!("prologue 非预期的三条 5B store,回退纯 IAT");
        return None;
    }
    const STEAL: usize = 15;
    // RWX trampoline:被偷的 15B + `mov rax,entry+15; jmp rax`
    let tramp = unsafe { VirtualAlloc(std::ptr::null_mut(), 64, 0x3000, 0x40) } as *mut u8;
    if tramp.is_null() {
        eprintln!("VirtualAlloc trampoline 失败");
        return None;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(target, tramp, STEAL);
        let back = target.add(STEAL) as u64;
        *tramp.add(STEAL) = 0x48;
        *tramp.add(STEAL + 1) = 0xB8;
        std::ptr::copy_nonoverlapping(back.to_le_bytes().as_ptr(), tramp.add(STEAL + 2), 8);
        *tramp.add(STEAL + 10) = 0xFF;
        *tramp.add(STEAL + 11) = 0xE0;
    }
    let orig: DeviceIoControlFn = unsafe { std::mem::transmute(tramp) };
    let mut old = 0u32;
    unsafe { VirtualProtect(target as *mut c_void, 16, 0x40, &mut old) };
    unsafe {
        *target = 0x48;
        *target.add(1) = 0xB8;
        let hookaddr = hook_device_io_control as *const () as usize as u64;
        std::ptr::copy_nonoverlapping(hookaddr.to_le_bytes().as_ptr(), target.add(2), 8);
        *target.add(10) = 0xFF;
        *target.add(11) = 0xE0;
        VirtualProtect(target as *mut c_void, 16, old, &mut old);
    }
    eprintln!("内联 hook 已装: tramp={tramp:p} entry->hook");
    Some(orig)
}

/// IAT 补丁(仅补 nvapi64_impl.dll;被上面的内联 hook 取代,保留作参考)。
#[allow(dead_code)]

/// 找出某 NVAPI ID 的实现落在哪个模块(用于判断毒 SET 是否真在 impl 内)。
fn where_is(id: u32) {
    let h = unsafe { LoadLibraryA(b"nvapi64.dll\0".as_ptr()) };
    if h == 0 {
        eprintln!("LoadLibrary nvapi64.dll 失败");
        return;
    }
    let qi = unsafe { GetProcAddress(h, b"nvapi_QueryInterface\0".as_ptr()) };
    if qi.is_null() {
        eprintln!("无 nvapi_QueryInterface");
        return;
    }
    let qf: unsafe extern "C" fn(u32) -> *const c_void = unsafe { std::mem::transmute(qi) };
    let ptr = unsafe { qf(id) };
    let addr = ptr as usize;
    if addr == 0 {
        println!("  nvapi {id:#010x} -> NULL(未实现)");
        return;
    }
    let mut mods = [0isize; 512];
    let mut needed = 0u32;
    unsafe {
        EnumProcessModules(
            GetCurrentProcess(),
            mods.as_mut_ptr(),
            (mods.len() * 8) as u32,
            &mut needed,
        );
    }
    let count = (needed as usize / 8).min(mods.len());
    for m in &mods[..count] {
        let mut buf = [0u16; 260];
        let n = unsafe { GetModuleFileNameExW(GetCurrentProcess(), *m, buf.as_mut_ptr(), 260) };
        if n == 0 {
            continue;
        }
        // 模块归属:读该模块自己的 PE SizeOfImage 做精确 contain 判定。
        let base = *m as usize;
        let img_size = unsafe {
            let b = base as *const u8;
            let lief = *(b.add(0x3C) as *const u32) as usize;
            *(b.add(lief + 24 + 56) as *const u32) as usize
        };
        if addr >= base && addr < base + img_size {
            let name = String::from_utf16_lossy(&buf[..n as usize]);
            println!("  nvapi {id:#010x} -> {addr:#x} in {name}");
            return;
        }
    }
    println!("  nvapi {id:#010x} -> {addr:#x} (模块未识别)");
}

#[link(name = "psapi")]
extern "system" {
    fn EnumProcessModules(h: isize, out: *mut isize, sz: u32, needed: *mut u32) -> i32;
    fn GetModuleFileNameExW(h: isize, m: isize, name: *mut u16, sz: u32) -> u32;
}

fn list_nv_modules() {
    let mut mods = [0isize; 512];
    let mut needed = 0u32;
    let ok = unsafe {
        EnumProcessModules(
            GetCurrentProcess(),
            mods.as_mut_ptr(),
            (mods.len() * 8) as u32,
            &mut needed,
        )
    };
    if ok == 0 {
        eprintln!("EnumProcessModules 失败");
        return;
    }
    let count = (needed as usize / 8).min(mods.len());
    for m in &mods[..count] {
        let mut buf = [0u16; 260];
        let n = unsafe { GetModuleFileNameExW(GetCurrentProcess(), *m, buf.as_mut_ptr(), 260) };
        if n == 0 {
            continue;
        }
        let s = String::from_utf16_lossy(&buf[..n as usize]);
        let low = s.to_ascii_lowercase();
        if low.contains("nv") && low.ends_with(".dll") {
            println!("  module: {s}");
        }
    }
}

fn log_dir() -> std::path::PathBuf {
    // 环境变量优先;否则落到 %TEMP%\nvoc-esc-capture,任何机器/路径都能跑。
    std::env::var("NVOC_ESC_CAPTURE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("nvoc-esc-capture"))
}

fn block_cmd() -> u32 {
    std::env::var("NVOC_ESC_BLOCK_CMD")
        .ok()
        .and_then(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok())
        .unwrap_or(BLOCK_CMD_DEFAULT)
}

/// 需要整体拦下的 ioctl(十六进制);毒 payload 在下发前沿已写入共享节,拦下
/// 0x470807 即可零 TDR 复现。
fn block_ioctl() -> u32 {
    std::env::var("NVOC_ESC_BLOCK_IOCTL")
        .ok()
        .and_then(|s| u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok())
        .unwrap_or(0)
}

fn parse_w(buf: &[u8]) -> String {
    if buf.len() < 0x38 {
        return format!("short({})", buf.len());
    }
    let rd32 = |o: usize| u32::from_le_bytes(buf[o..o + 4].try_into().unwrap());
    format!(
        "magic={:#x} ver={:#x} size={:#x} magic2={:#x} hGpu={:#x} cmd={:#x}",
        rd32(0),
        rd32(4),
        rd32(8),
        rd32(12),
        rd32(0x30),
        rd32(0x34)
    )
}

fn hexdump(buf: &[u8]) -> String {
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn append(dir: &std::path::Path, name: &str, text: &str) {
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join(name))
        .and_then(|mut f| std::io::Write::write_all(&mut f, text.as_bytes()));
}

unsafe extern "system" fn hook_device_io_control(
    h: isize,
    ioctl: u32,
    inbuf: *mut c_void,
    insz: u32,
    outbuf: *mut c_void,
    outsz: u32,
    ret: *mut u32,
    overlapped: *mut c_void,
) -> i32 {
    let orig = unsafe { ORIG_DEVICE_IO_CONTROL.expect("hook installed") };
    let _ = orig;
    // 不过滤:记录 nvapi 模块发出的所有 ioctl。
    let seq = HOOK_COUNT.fetch_add(1, Ordering::SeqCst);
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    let slice: &[u8] = if !inbuf.is_null() && insz > 0 {
        unsafe { std::slice::from_raw_parts(inbuf as *const u8, insz as usize) }
    } else {
        &[]
    };
    let w = if slice.len() > 12 {
        &slice[12..]
    } else {
        slice
    };
    let cmd_w = if w.len() >= 0x38 {
        u32::from_le_bytes(w[0x34..0x38].try_into().unwrap())
    } else {
        0
    };
    let blocked = !NO_BLOCK.load(Ordering::SeqCst)
        && ((cmd_w == block_cmd() && cmd_w != 0 && block_cmd() != 0xFFFF_FFFF)
            || (block_ioctl() != 0 && ioctl == block_ioctl()));
    let note = if blocked { " BLOCKED" } else { "" };
    let dev = device_path(h);
    let line = if ioctl == 0x0004_70807 {
        // 毒 TgpWatt 家族的传输:整个输入缓冲就是 40B GUID 描述符
        // {size=0x28, f4, GUID 29D92575-..., code};实际 watt/表数据不在本
        // 缓冲(经共享节传递)。按 slice 原样解码并附全量 hex,避免偏移猜测。
        let rd = |o: usize| {
            if slice.len() >= o + 4 {
                u32::from_le_bytes(slice[o..o + 4].try_into().unwrap())
            } else {
                0
            }
        };
        let guid: String = (8..24)
            .map(|i| format!("{:02x}", slice.get(i).copied().unwrap_or(0)))
            .collect();
        format!(
            "seq={seq:03} ioctl={ioctl:#x} insize={insz} outsize={outsz} dev={dev} | guid_desc size={:#x} f4={:#x} guid={guid} code={:#x} raw={}{note}",
            rd(0),
            rd(4),
            rd(0x24),
            hexdump(&slice[..slice.len().min(insz as usize)])
        )
    } else {
        format!(
            "seq={seq:03} ioctl={ioctl:#x} insize={insz} outsize={outsz} dev={dev} cmd={cmd_w:#x}{note} | {}",
            parse_w(w)
        )
    };
    println!("{line}");
    if slice.len() <= 0x2000 {
        let _ = std::fs::write(dir.join(format!("esc-{seq:03}-in.hex")), hexdump(slice));
    } else {
        let _ = std::fs::write(
            dir.join(format!("esc-{seq:03}-in.hex")),
            format!(
                "len={}\n{}",
                slice.len(),
                hexdump(&slice[..0x40.min(slice.len())])
            ),
        );
    }
    append(
        &dir,
        "index.txt",
        &format!("tid={} {line}\n", unsafe { GetCurrentThreadId() }),
    );
    if ioctl == 0x0004_70807 {
        // 毒家族:实际 payload 在共享节里,ioctl 只是"提交"指令。
        dump_sections(&dir, seq);
    }
    if blocked {
        if !ret.is_null() {
            unsafe { *ret = 1 };
        }
        return 0;
    }
    let rc = unsafe { orig(h, ioctl, inbuf, insz, outbuf, outsz, ret, overlapped) };
    if !outbuf.is_null() && outsz > 0 {
        let _ = std::fs::write(
            dir.join(format!("esc-{seq:03}-out.hex")),
            hexdump(unsafe { std::slice::from_raw_parts(outbuf as *const u8, outsz as usize) }),
        );
    }
    rc
}

/// 在指定模块内定位名为 `fname` 的导入槽,全部换上 `hook`,返回首个原函数指针。
unsafe fn patch_iat_fn(module: &str, fname: &[u8], hook: *mut c_void) -> Option<*mut c_void> {
    let name: Vec<u16> = module.encode_utf16().chain(std::iter::once(0)).collect();
    let hmod = unsafe { GetModuleHandleW(name.as_ptr()) };
    if hmod == 0 {
        eprintln!("{module} 未驻留");
        return None;
    }
    let base = hmod as *const u8;
    let e_lfanew = unsafe { *(base.add(0x3C) as *const u32) } as usize;
    let nt = unsafe { base.add(e_lfanew) };
    let import_rva = unsafe { *(nt.add(24 + 120) as *const u32) } as usize; // DataDirectory[1] for PE32+
    if import_rva == 0 {
        eprintln!("无导入表");
        return None;
    }
    let mut first_orig: Option<*mut c_void> = None;
    // 内存映射镜像:RVA 即相对 base 的偏移(不做文件节区转换)。
    let mut desc = base.add(import_rva) as *const u8;
    loop {
        let name_rva = unsafe { *(desc.add(12) as *const u32) } as usize;
        if name_rva == 0 {
            eprintln!("导入表里未找到 {}", String::from_utf8_lossy(fname));
            return first_orig;
        }
        let orig_first = unsafe { *(desc as *const u32) } as usize;
        let first = unsafe { *(desc.add(16) as *const u32) } as usize;
        if orig_first != 0 && first != 0 {
            let mut i = 0usize;
            loop {
                let int_rva = unsafe { *(base.add(orig_first + i * 8) as *const u64) } as usize;
                if int_rva == 0 {
                    break;
                }
                if int_rva & 0x8000_0000_0000_0000 == 0 {
                    let hint = unsafe { base.add((int_rva & 0xFFFF_FFFF) + 2) };
                    let nm = unsafe { core::slice::from_raw_parts(hint, fname.len()) };
                    if nm == fname {
                        let slot = unsafe { base.add(first + i * 8) } as *mut *mut c_void;
                        let orig = unsafe { *slot };
                        eprintln!(
                            "patch {} in {module} slot {slot:p} orig {orig:p}",
                            String::from_utf8_lossy(fname)
                        );
                        let mut old = 0u32;
                        unsafe { VirtualProtect(slot as *mut c_void, 8, 0x40, &mut old) };
                        unsafe { *slot = hook };
                        unsafe { VirtualProtect(slot as *mut c_void, 8, old, &mut old) };
                        if first_orig.is_none() {
                            first_orig = Some(orig);
                        }
                    }
                }
                i += 1;
            }
        }
        desc = unsafe { desc.add(20) };
    }
}

/// 记录被 impl 映射的共享节(毒 payload 的实际载体,不在 ioctl 缓冲里)。
static SECTION_MAPS: std::sync::Mutex<Vec<(usize, usize)>> = std::sync::Mutex::new(Vec::new());
static mut ORIG_MV: Option<unsafe extern "system" fn(isize, u32, u32, u32, usize) -> *mut c_void> =
    None;
static mut ORIG_MVEX: Option<
    unsafe extern "system" fn(isize, u32, u32, u32, usize, *mut c_void) -> *mut c_void,
> = None;

unsafe extern "system" fn hook_mv(
    h: isize,
    access: u32,
    off_hi: u32,
    off_lo: u32,
    size: usize,
) -> *mut c_void {
    let orig = unsafe { ORIG_MV.expect("mv hook") };
    let p = unsafe { orig(h, access, off_hi, off_lo, size) };
    record_map(p, size);
    p
}

unsafe extern "system" fn hook_mvex(
    h: isize,
    access: u32,
    off_hi: u32,
    off_lo: u32,
    size: usize,
    base: *mut c_void,
) -> *mut c_void {
    let orig = unsafe { ORIG_MVEX.expect("mvex hook") };
    let p = unsafe { orig(h, access, off_hi, off_lo, size, base) };
    record_map(p, size);
    p
}

fn record_map(p: *mut c_void, size: usize) {
    if p.is_null() {
        return;
    }
    let mut m = SECTION_MAPS.lock().unwrap();
    m.push((p as usize, size));
    eprintln!("MapViewOfFile -> {p:p} size={size:#x}");
}

/// 在毒 ioctl 时刻把已记录的共享节内容落盘 —— 这里才藏着 watt/表数据。
fn dump_sections(dir: &std::path::Path, seq: usize) {
    let maps = SECTION_MAPS.lock().unwrap().clone();
    for (i, (base, size)) in maps.iter().enumerate() {
        let take = (*size).min(0x20000);
        let data = unsafe { core::slice::from_raw_parts(*base as *const u8, take) };
        let _ = std::fs::write(
            dir.join(format!("sec-{seq:03}-{i}-{base:x}.hex")),
            hexdump(data),
        );
    }
}

// ===================================================================
// 跨机对比工具(2026-10-08):身份/版本 + 映射节指纹差分。
//
// 目的:在"30 系可用"的对照机上重跑同一序列,回收:
//   ① GPU/驱动/NVAPI/模块版本(定位代际差异);
//   ② 每步 NVAPI 调用对应的 ioctl 序列(GET 与 SET 是否同 ioctl);
//   ③ 毒写入真正改动的映射节字节(payload 载体,用的是"前后指纹变化"
//      定位,不依赖驱动私有的 stamp 常量,换机/换驱动也成立)。
// ===================================================================

static NO_BLOCK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[link(name = "version")]
extern "system" {
    fn GetFileVersionInfoSizeW(name: *const u16, handle: *mut u32) -> u32;
    fn GetFileVersionInfoW(name: *const u16, handle: u32, len: u32, data: *mut c_void) -> i32;
    fn VerQueryValueW(
        block: *const c_void,
        sub: *const u16,
        out: *mut *mut c_void,
        outlen: *mut u32,
    ) -> i32;
}

#[repr(C)]
struct OsVersionInfo {
    size: u32,
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    csd: [u16; 128],
    sp_major: u16,
    sp_minor: u16,
}

#[link(name = "ntdll")]
extern "system" {
    fn RtlGetVersion(info: *mut OsVersionInfo) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleFileNameW(h: isize, buf: *mut u16, sz: u32) -> u32;
    fn CloseHandle(h: isize) -> i32;
    fn GetCurrentThreadId() -> u32;
}

#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(process: isize, access: u32, token: *mut isize) -> i32;
    fn GetTokenInformation(
        token: isize,
        class: u32,
        info: *mut c_void,
        len: u32,
        ret: *mut u32,
    ) -> i32;
}

/// 毒 SET(以及 percent SET)的写路径需要管理员令牌,否则 NVAPI 在**下发 ioctl
/// 之前**就以 InvalidUserPrivilege 返回 —— 抓到的将只有 GET,毫无对比价值。
fn is_elevated() -> bool {
    unsafe {
        let mut tok = 0isize;
        if OpenProcessToken(GetCurrentProcess(), 0x0008 /* TOKEN_QUERY */, &mut tok) == 0 {
            return false;
        }
        let mut elev = 0u32;
        let mut ret = 0u32;
        let ok = GetTokenInformation(
            tok,
            20, /* TokenElevation */
            (&mut elev as *mut u32).cast(),
            4,
            &mut ret,
        );
        CloseHandle(tok);
        ok != 0 && elev != 0
    }
}

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 从磁盘 PE 的资源块里取 FileVersion(四段数字)。驱动包里的 nvapi64.dll /
/// nvapi64_impl.dll 版本直接决定毒路径的客户端实现,是代际对比的关键字段。
fn file_version_of(path: &str) -> String {
    let w = wide(path);
    unsafe {
        let mut dummy = 0u32;
        let sz = GetFileVersionInfoSizeW(w.as_ptr(), &mut dummy);
        if sz == 0 {
            return "?".into();
        }
        let mut buf = vec![0u8; sz as usize];
        if GetFileVersionInfoW(w.as_ptr(), 0, sz, buf.as_mut_ptr().cast()) == 0 {
            return "?".into();
        }
        let mut p: *mut c_void = std::ptr::null_mut();
        let mut len = 0u32;
        let root = wide("\\");
        if VerQueryValueW(buf.as_ptr().cast(), root.as_ptr(), &mut p, &mut len) == 0 || p.is_null()
        {
            return "?".into();
        }
        let b = p as *const u8;
        let rd = |o: usize| {
            u32::from_le_bytes(core::slice::from_raw_parts(b.add(o), 4).try_into().unwrap())
        };
        let (ms, ls) = (rd(8), rd(12));
        format!("{}.{}.{}.{}", ms >> 16, ms & 0xffff, ls >> 16, ls & 0xffff)
    }
}

/// 报告 + 落盘(report.txt 是跨机对比的主交付物,必须自包含)。
fn emit(line: &str) {
    println!("{line}");
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    append(&dir, "report.txt", &format!("{line}\n"));
}

fn report_identity(gpu: &nvapi::PhysicalGpu) {
    emit("=== identity ===");
    emit(&format!("gpu.full_name           = {:?}", gpu.full_name()));
    emit(&format!(
        "gpu.architecture        = {:?}",
        gpu.architecture().map(|a| a.to_string())
    ));
    emit(&format!(
        "gpu.pci_identifiers     = {:?}",
        gpu.pci_identifiers().map(|p| p.to_string())
    ));
    emit(&format!("gpu.bus_id              = {:?}", gpu.bus_id()));
    emit(&format!(
        "nvapi.driver_version    = {:?}",
        nvapi::driver_version()
    ));
    emit(&format!(
        "nvapi.interface_version = {:?}",
        nvapi::interface_version()
    ));
    let mut os = OsVersionInfo {
        size: core::mem::size_of::<OsVersionInfo>() as u32,
        major: 0,
        minor: 0,
        build: 0,
        platform: 0,
        csd: [0; 128],
        sp_major: 0,
        sp_minor: 0,
    };
    unsafe { RtlGetVersion(&mut os) };
    emit(&format!(
        "windows                 = {}.{} build {}",
        os.major, os.minor, os.build
    ));
}

/// 枚举进程内 nv* 模块并打印其磁盘文件版本(毒路径实现体在哪、哪个版本)。
fn report_nv_module_versions() {
    emit("--- loaded nv* modules (path / file-version / base) ---");
    let mut mods = [0isize; 512];
    let mut needed = 0u32;
    unsafe {
        EnumProcessModules(
            GetCurrentProcess(),
            mods.as_mut_ptr(),
            (mods.len() * 8) as u32,
            &mut needed,
        );
    }
    let count = (needed as usize / 8).min(mods.len());
    for m in &mods[..count] {
        let mut buf = [0u16; 320];
        let n = unsafe { GetModuleFileNameW(*m, buf.as_mut_ptr(), 320) };
        if n == 0 {
            continue;
        }
        let s = String::from_utf16_lossy(&buf[..n as usize]);
        let low = s.to_ascii_lowercase();
        if low.contains("nv") && low.ends_with(".dll") {
            emit(&format!(
                "module {s} ver={} base={:#x}",
                file_version_of(&s),
                *m as usize
            ));
        }
    }
}

const MEM_MAPPED_KIND: u32 = 0x40000;

/// 一个映射节的指纹(仅取前 2MB 做 FNV-1a;够覆盖毒表,避免全量读大节)。
#[derive(Clone)]
struct RegionFp {
    base: usize,
    size: usize,
    protect: u32,
    hash: u64,
    head: Vec<u8>,
}

fn mapped_scan() -> Vec<RegionFp> {
    let mut out = Vec::new();
    let mut addr: usize = 0;
    let mut mbi = [0u8; 48];
    while addr < 0x7fff_ffff_0000 {
        let r = unsafe { VirtualQuery(addr as *mut c_void, mbi.as_mut_ptr(), 48) };
        if r == 0 {
            break;
        }
        let base = usize::from_le_bytes(mbi[0..8].try_into().unwrap());
        let size = usize::from_le_bytes(mbi[24..32].try_into().unwrap());
        let state = u32::from_le_bytes(mbi[32..36].try_into().unwrap());
        let protect = u32::from_le_bytes(mbi[36..40].try_into().unwrap());
        let ty = u32::from_le_bytes(mbi[40..44].try_into().unwrap());
        if state == 0x1000 && ty == MEM_MAPPED_KIND && (protect & 0x100) == 0 && size >= 0x1000 {
            let take = size.min(0x20_0000);
            let data = unsafe { core::slice::from_raw_parts(base as *const u8, take) };
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            for &b in data {
                h ^= b as u64;
                h = h.wrapping_mul(0x0000_0100_0000_01b3);
            }
            out.push(RegionFp {
                base,
                size,
                protect,
                hash: h,
                head: data[..data.len().min(0x40)].to_vec(),
            });
        }
        addr = base + size.max(0x1000);
    }
    out
}

fn fp_report(tag: &str, r: &[RegionFp]) {
    emit(&format!(
        "--- mapped fingerprint [{tag}]: {} MEM_MAPPED regions ---",
        r.len()
    ));
    for (i, f) in r.iter().enumerate() {
        emit(&format!(
            "  [{tag}] {i:02} base={:#x} size={:#x} prot={:#x} hash={:#018x} head={}",
            f.base,
            f.size,
            f.protect,
            f.hash,
            hexdump(&f.head)
        ));
    }
}

fn hexdump_full(tag: &str, base: usize, size: usize) -> String {
    let take = size.min(0x10_0000);
    let d = unsafe { core::slice::from_raw_parts(base as *const u8, take) };
    let mut s = format!("=== {tag} base={base:#x} size={size:#x} dumped={take:#x}\n");
    for (i, chunk) in d.chunks(16).enumerate() {
        let hex: String = chunk
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let asc: String = chunk
            .iter()
            .map(|&b| {
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        s.push_str(&format!("{:#012x}  {hex:<48} {asc}\n", base + i * 16));
    }
    s
}

/// 前后指纹差分:内容发生变化的映射节 = 该 NVAPI 调用真正写入的载体。把改动节
/// 的 前/后 字节都落盘,换机换驱动都成立(不依赖私有 stamp)。
fn dump_changed(prefix: &str, before: &[RegionFp], after: &[RegionFp]) {
    let find = |v: &[RegionFp], base: usize| v.iter().find(|f| f.base == base).cloned();
    let mut n = 0usize;
    for a in after {
        match find(before, a.base) {
            Some(b) if b.hash != a.hash => {
                let mut s = String::new();
                s.push_str(&hexdump_full(&format!("{prefix} AFTER"), a.base, a.size));
                s.push_str(&hexdump_full(&format!("{prefix} BEFORE"), b.base, b.size));
                let _ = std::fs::write(
                    log_dir().join(format!("changed-{prefix}-{n:02}-{:x}.txt", a.base)),
                    &s,
                );
                emit(&format!(
                    "CHANGED[{prefix}] #{} base={:#x} size={:#x} hash {:018x} -> {:018x}",
                    n, a.base, a.size, b.hash, a.hash
                ));
                n += 1;
            }
            None => emit(&format!(
                "NEW[{prefix}] base={:#x} size={:#x} hash={:#018x}",
                a.base, a.size, a.hash
            )),
            _ => {}
        }
    }
    for b in before {
        if find(after, b.base).is_none() {
            emit(&format!(
                "GONE[{prefix}] base={:#x} size={:#x}",
                b.base, b.size
            ));
        }
    }
    if n == 0 {
        emit(&format!("CHANGED[{prefix}] (none)"));
    }
}

#[test]
#[ignore = "live ESC capture: IAT-hook + percent 对照 + watt SET 拦截;非提权可跑"]
fn esc_capture_percent_vs_watt() {
    // 先强制加载 nvapi64.dll(impl 随之驻留)并装好 hook,使后续
    // enumerate/生命周期里发生的共享节映射也能被记录。
    unsafe { LoadLibraryA(b"nvapi64.dll\0".as_ptr()) };
    let orig = unsafe { install_inline_hook() };
    match orig {
        Some(f) => {
            unsafe { ORIG_DEVICE_IO_CONTROL = Some(f) };
            eprintln!("inline hook installed; block_cmd={:#x}", block_cmd());
        }
        None => {
            let o = unsafe {
                patch_iat_fn(
                    "nvapi64_impl.dll",
                    b"DeviceIoControl",
                    hook_device_io_control as *mut c_void,
                )
            };
            assert!(o.is_some(), "IAT patch 失败");
            unsafe { ORIG_DEVICE_IO_CONTROL = Some(std::mem::transmute(o.unwrap())) };
            eprintln!("IAT fallback hook; block_cmd={:#x}", block_cmd());
        }
    }
    // 共享节捕获:impl 用 CreateFileMappingW+MapViewOfFileEx 拿驱动发布的节,
    // 毒 TgpWatt 家族的 watt/表数据经此传递(ioctl 缓冲里看不到)。
    unsafe {
        if let Some(o) = patch_iat_fn("nvapi64_impl.dll", b"MapViewOfFile", hook_mv as *mut c_void)
        {
            ORIG_MV = Some(std::mem::transmute(o));
        }
        if let Some(o) = patch_iat_fn(
            "nvapi64_impl.dll",
            b"MapViewOfFileEx",
            hook_mvex as *mut c_void,
        ) {
            ORIG_MVEX = Some(std::mem::transmute(o));
        }
    }

    let gpu = nvapi::PhysicalGpu::enumerate()
        .expect("enumerate")
        .into_iter()
        .next()
        .expect("no gpu");
    let _ = gpu.private_lifecycle_init();
    println!("--- loaded nv* modules ---");
    list_nv_modules();
    println!("--- NVAPI ID 归属 ---");
    for id in [
        0xAFFC2279u32,
        0x8B3E7343,
        0xad95f5ed,
        0x70916171,
        0x67F31384,
    ] {
        where_is(id);
    }

    println!("--- step1: set_power_limit(percent 100.0) [安全对照] ---");
    let r1 = gpu.set_power_limit([nvapi::Percentage1000(100_000)]);
    println!("percent → {r1:?}");

    println!("--- step2: set_tgp_watt(180, idx 2) [SET BLOCKED] ---");
    let r2 = gpu.set_tgp_watt(180, 2);
    println!("watt → {r2:?}");
    scan_process_memory(&[
        (180_000, "watt180"),
        (0x0001_0A4C, "stamp10A4C"),
        (0x0001_2720, "stamp12720"),
        (0x0005_B0B0, "stamp5B0B0"),
    ]);

    println!("--- step3: tgp_watt_range [GET-only 对照] ---");
    let r3 = gpu.tgp_watt_range();
    println!("range → {r3:?}");

    println!("--- step4: power_channel_policies ---");
    match gpu.power_channel_policies() {
        Ok(p) => {
            println!("policies → {} rows", p.len());
            for row in &p {
                println!(
                    "  idx={} min={} def={} max={}",
                    row.index, row.min_raw, row.default_raw, row.max_raw
                );
            }
            println!("--- step5: power_channel_control [GetStatus probe] ---");
            println!("control → {:?}", gpu.power_channel_control(&p));
        }
        Err(e) => println!("policies → {e:?}"),
    }

    println!("--- step6: tgp_watt_status [GetStatus 公开包装] ---");
    println!("status → {:?}", gpu.tgp_watt_status());

    println!("done; hooks fired = {}", HOOK_COUNT.load(Ordering::SeqCst));
    println!("capture dir = {}", log_dir().display());
    // NVAPI 后台线程可能不让进程退出,强制收尾。
    std::process::exit(0);
}

/// 序列分隔线:同时进 report 和 ioctl 索引,使每条 ioctl 可归因到具体 NVAPI 调用。
fn step(tag: &str) {
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    append(
        &dir,
        "index.txt",
        &format!("\n### STEP {tag} seq={} tid={}\n", cur_seq(), cur_tid()),
    );
    emit(&format!("### STEP {tag}"));
}

/// 只进 ioctl 索引的细粒度打点(`TAG>` 开始 / `TAG<` 结束),用于把每条 ioctl
/// 精确归因到某一次 NVAPI 调用(写 vs 回读 vs control 探测)。带全局序号与
/// 线程号:DeviceIoControl 是同步调用,同线程内序号区间即该次调用的 ioctl。
fn mark(tag: &str) {
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    append(
        &dir,
        "index.txt",
        &format!("--- {tag} seq={} tid={}\n", cur_seq(), cur_tid()),
    );
}

fn cur_seq() -> usize {
    HOOK_COUNT.load(Ordering::SeqCst)
}

fn cur_tid() -> u32 {
    unsafe { GetCurrentThreadId() }
}

/// 把本次抓包里所有 nv* 模块版本、身份、以及毒序列与 ioctl 索引汇成一份
/// 可单独回传的 summary。
fn write_summary() {
    let dir = log_dir();
    let mut s = String::from("=== nvoc TGPWatt wire capture summary ===\n");
    for name in ["report.txt", "index.txt", "scan.txt"] {
        if let Ok(t) = std::fs::read_to_string(dir.join(name)) {
            s.push_str(&format!("\n########## {name} ##########\n{t}\n"));
        }
    }
    let _ = std::fs::write(dir.join("SUMMARY.txt"), &s);
    println!("SUMMARY -> {}", dir.join("SUMMARY.txt").display());
}

/// 跨机对比测试(在"30 系可用"的对照机上跑这个)。同一序列在毒机(2070/610)
/// 上会 TDR,在本机上应全部成功;两份 report/SUMMARY 的 ioctl 序列与改动节
/// 差分是定位代际差异的直接证据。**提权运行**;本测试全程放行、不拦截。
#[test]
#[ignore = "对照机(30系)跑:身份/版本 + GET/SET 序列 + 毒写入载体差分;需提权"]
fn tgpwatt_wire_diff() {
    NO_BLOCK.store(true, Ordering::SeqCst);
    let _ = std::fs::create_dir_all(log_dir());
    let _ = std::fs::write(log_dir().join("report.txt"), "");
    let _ = std::fs::write(log_dir().join("index.txt"), "");
    emit(&format!("elevated = {}", is_elevated()));
    if !is_elevated() {
        emit("!!! 警告:未提权 —— 写路径会在下发 ioctl 前以 InvalidUserPrivilege 返回,");
        emit("!!! 本份抓包只有 GET,没有对比价值。请以管理员身份重跑。");
    }
    // 先驻留 nvapi64.dll(impl 随之加载)再装 hook,避免漏掉初始化期的映射。
    unsafe { LoadLibraryA(b"nvapi64.dll\0".as_ptr()) };
    match unsafe { install_inline_hook() } {
        Some(f) => unsafe { ORIG_DEVICE_IO_CONTROL = Some(f) },
        None => {
            let o = unsafe {
                patch_iat_fn(
                    "nvapi64_impl.dll",
                    b"DeviceIoControl",
                    hook_device_io_control as *mut c_void,
                )
            };
            assert!(o.is_some(), "DeviceIoControl hook 失败");
            unsafe { ORIG_DEVICE_IO_CONTROL = Some(std::mem::transmute(o.unwrap())) };
        }
    }
    unsafe {
        if let Some(o) = patch_iat_fn("nvapi64_impl.dll", b"MapViewOfFile", hook_mv as *mut c_void)
        {
            ORIG_MV = Some(std::mem::transmute(o));
        }
        if let Some(o) = patch_iat_fn(
            "nvapi64_impl.dll",
            b"MapViewOfFileEx",
            hook_mvex as *mut c_void,
        ) {
            ORIG_MVEX = Some(std::mem::transmute(o));
        }
    }

    let gpus = nvapi::PhysicalGpu::enumerate().expect("enumerate");
    emit(&format!("gpu count = {}", gpus.len()));
    for (i, g) in gpus.iter().enumerate() {
        emit(&format!("gpu[{i}] = {:?}", g.full_name()));
    }
    let gpu = gpus.into_iter().next().expect("no gpu");
    let _ = gpu.private_lifecycle_init();

    report_identity(&gpu);
    report_nv_module_versions();
    emit("--- NVAPI ID 归属(毒族 vs 安全族)---");
    for id in [
        0xAFFC2279u32,
        0x8B3E7343,
        0xad95f5ed,
        0x70916171,
        0x67F31384,
    ] {
        where_is(id);
    }

    // ---- GET-only 基线(不含任何写) ----
    step("GET baseline");
    mark("G1>tgp_watt_range");
    let range = gpu.tgp_watt_range();
    emit(&format!("tgp_watt_range  -> {range:?}"));
    mark("G2>tgp_watt_status");
    let status0 = gpu.tgp_watt_status();
    emit(&format!("tgp_watt_status(before) -> {status0:?}"));
    mark("G3>power_channel_policies");
    let policies = gpu.power_channel_policies();
    mark("G3<");
    match &policies {
        Ok(p) => {
            emit(&format!("power_channel_policies -> {} rows", p.len()));
            for row in p {
                emit(&format!(
                    "  idx={} policy_id={} subtype={} min={} def={} max={} ocp={} board={}",
                    row.index,
                    row.policy_id,
                    row.subtype,
                    row.min_raw,
                    row.default_raw,
                    row.max_raw,
                    row.is_ocp_current(),
                    row.is_board_power()
                ));
            }
        }
        Err(e) => emit(&format!("power_channel_policies -> {e:?}")),
    }
    if let Ok(p) = &policies {
        mark("G4>power_channel_control");
        emit(&format!(
            "power_channel_control -> {:?}",
            gpu.power_channel_control(p)
        ));
        mark("G4<");
    }

    // ---- 安全对照:percent 写(ClientPowerPoliciesSetStatus,历史安全接口) ----
    step("OP1 percent set_power_limit(100%)");
    mark("OP1>percent 100");
    emit(&format!(
        "percent 100%% -> {:?}",
        gpu.set_power_limit([nvapi::Percentage1000(100_000)])
    ));
    mark("OP1<");

    let pre = mapped_scan();
    fp_report("pre-toxic", &pre);

    // ---- 毒 OP2:绝对 TGP 写(0xAFFC2279)——对照机成功,毒机 TDR ----
    step("OP2 set_tgp_watt(180, idx2) TOXIC");
    mark("OP2>set_tgp_watt(180,2) TOXIC");
    let r2 = gpu.set_tgp_watt(180, 2);
    mark("OP2<");
    emit(&format!("set_tgp_watt(180,2) -> {r2:?}"));
    mark("OP2.1>status after");
    emit(&format!(
        "tgp_watt_status(after) -> {:?}",
        gpu.tgp_watt_status()
    ));
    mark("OP2.2>range after");
    emit(&format!(
        "tgp_watt_range(after)  -> {:?}",
        gpu.tgp_watt_range()
    ));
    let post = mapped_scan();
    fp_report("post-watt", &post);
    dump_changed("watt", &pre, &post);

    // ---- 毒 OP3:OCP 电流行(写一个与当前不同的值) ----
    let mut ocp_restore: Option<(u32, u32, u32)> = None;
    if let Ok(p) = &policies {
        if let Some(ocp) = p.iter().find(|r| r.is_ocp_current()) {
            let target = ocp.default_raw.saturating_sub(10_000).max(ocp.min_raw);
            ocp_restore = Some((ocp.policy_id, ocp.subtype, ocp.default_raw));
            let bp = mapped_scan();
            step(&format!("OP3 OCP-current write idx{}", ocp.index));
            mark(&format!(
                "OP3>ocp({},{}) write {target} (def {})",
                ocp.policy_id, ocp.subtype, ocp.default_raw
            ));
            let r3 = gpu.set_power_channel_value(ocp.policy_id, ocp.subtype, target);
            mark("OP3<");
            emit(&format!(
                "ocp({},{}) {target} -> {r3:?}",
                ocp.policy_id, ocp.subtype
            ));
            mark("OP3.1>control after");
            emit(&format!(
                "power_channel_control(after) -> {:?}",
                gpu.power_channel_control(p)
            ));
            let ap = mapped_scan();
            dump_changed("ocp", &bp, &ap);
        }
    }

    // ---- 毒 OP4:板功率行(写一个与当前不同的值) ----
    if let Ok(p) = &policies {
        if let Some(board) = p.iter().find(|r| r.is_board_power()) {
            let target = (board.default_raw + 20_000).min(board.max_raw);
            let bp = mapped_scan();
            step(&format!("OP4 board-power write idx{}", board.index));
            mark(&format!(
                "OP4>board({},{}) write {target} (def {})",
                board.policy_id, board.subtype, board.default_raw
            ));
            let r4 = gpu.set_power_channel_value(board.policy_id, board.subtype, target);
            mark("OP4<");
            emit(&format!(
                "board({},{}) {target} -> {r4:?}",
                board.policy_id, board.subtype
            ));
            mark("OP4.1>control after");
            emit(&format!(
                "power_channel_control(after) -> {:?}",
                gpu.power_channel_control(p)
            ));
            let ap = mapped_scan();
            dump_changed("board", &bp, &ap);
        }
    }

    // ---- 复位 ----
    step("restore");
    mark("R1>restore percent 100");
    emit(&format!(
        "restore percent 100%% -> {:?}",
        gpu.set_power_limit([nvapi::Percentage1000(100_000)])
    ));
    mark("R1<");
    if let Some((pid, st, def)) = ocp_restore {
        mark("R2>restore ocp");
        emit(&format!(
            "restore ocp({pid},{st}) -> {:?}",
            gpu.set_power_channel_value(pid, st, def)
        ));
        mark("R2<");
    }
    mark("S>needle scan");
    scan_process_memory(&[
        (180_000, "watt180"),
        (0x0001_0A4C, "stamp10A4C"),
        (0x0001_2720, "stamp12720"),
        (0x0005_B0B0, "stamp5B0B0"),
    ]);

    write_summary();
    emit(&format!(
        "done; ioctls captured = {}",
        HOOK_COUNT.load(Ordering::SeqCst)
    ));
    emit(&format!("capture dir = {}", log_dir().display()));
    std::process::exit(0);
}
