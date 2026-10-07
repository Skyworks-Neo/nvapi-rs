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
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};

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

/// `ntdll!NtDeviceIoControlFile` 原型(kernel32!DeviceIoControl 的下一站)。
type NtDeviceIoControlFileFn = unsafe extern "system" fn(
    isize,
    isize,
    *mut c_void,
    *mut c_void,
    *mut c_void,
    u32,
    *mut c_void,
    u32,
    *mut c_void,
    u32,
) -> i32;

const BLOCK_CMD_DEFAULT: u32 = 0x2080_E61B;

static HOOK_COUNT: AtomicUsize = AtomicUsize::new(0);
static NT_HOOK_COUNT: AtomicUsize = AtomicUsize::new(0);
/// ntdll 旁路拨号记录的内存缓冲:在 hook 里做 std 的 println!/文件 I/O 会
/// 0xc0000005(见 hook_nt_device_io_control 注释),所以先攒在内存,测试末尾
/// 统一落盘。
static NT_LOG: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
static NT_DUMP_SECTIONS: AtomicBool = AtomicBool::new(false);
/// 被补掉的首个 `NtDeviceIoControlFile` 原函数(非空即表示至少补到一个导入槽)。
static NTDLL_ORIG: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
static mut ORIG_DEVICE_IO_CONTROL: Option<DeviceIoControlFn> = None;
static mut ORIG_NT_DEVICE_IO_CONTROL: Option<NtDeviceIoControlFileFn> = None;

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
        emit(&format!("  nvapi {id:#010x} -> NULL(未实现)"));
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
            emit(&format!("  nvapi {id:#010x} -> {addr:#x} in {name}"));
            return;
        }
    }
    emit(&format!("  nvapi {id:#010x} -> {addr:#x} (模块未识别)"));
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

/// `ntdll!NtDeviceIoControlFile` 的旁路记录。r590/r610 新驱动上 kernel32!
/// DeviceIoControl 全程看不到任何写路径调用,只有两种解释:新驱动的 SET 已经
/// 变成共享节直写(无内核传输),或者它绕过 kernel32 直接走 ntdll 系统调用。
/// 这个 hook 就是用来判定后者的 —— 若这里也静默,则"无内核传输"成立。
unsafe extern "system" fn hook_nt_device_io_control(
    h: isize,
    event: isize,
    apc: *mut c_void,
    apcctx: *mut c_void,
    iostat: *mut c_void,
    ioctl: u32,
    inbuf: *mut c_void,
    insz: u32,
    outbuf: *mut c_void,
    outsz: u32,
) -> i32 {
    // 绝不能在这里 panic:本函数是 extern "system",panic 穿出去是 UB(实测为
    // 0xc0000005)。所以取不到原函数就退化成 ntdll 里的裸指针,再不行直接放弃。
    let orig = match unsafe { ORIG_NT_DEVICE_IO_CONTROL } {
        Some(o) => o,
        None => {
            let p = NTDLL_ORIG.load(Ordering::SeqCst);
            if p.is_null() {
                return -1;
            }
            unsafe { std::mem::transmute(p) }
        }
    };
    NT_HOOK_COUNT.fetch_add(1, Ordering::SeqCst);
    let seq = HOOK_COUNT.fetch_add(1, Ordering::SeqCst);
    let line = format!(
        "seq={seq:03} via=ntdll ioctl={ioctl:#x} insize={insz} outsize={outsz} dev={}",
        device_path(h)
    );
    // 只往内存缓冲里塞,**不在 hook 里做 std 的 println!/文件 I/O**。实测:这个
    // hook 会被 kernelbase/advapi32 在任意线程上调用,在 hook 内 println! 会
    // 0xc0000005(与补丁无关 —— 纯透传/纯计数/纯格式化都正常),所以落盘统一
    // 挪到测试末尾 `flush_nt_log()`。
    {
        let mut g = NT_LOG.lock().unwrap_or_else(|e| e.into_inner());
        if g.len() < 4096 {
            g.push(line);
        }
    }
    let r = unsafe {
        orig(
            h, event, apc, apcctx, iostat, ioctl, inbuf, insz, outbuf, outsz,
        )
    };
    // 0x470807 的 payload 在共享节里:也要 dump,但同样只做内存读取。
    if ioctl == 0x0004_70807 && NT_DUMP_SECTIONS.load(Ordering::SeqCst) {
        dump_sections(&log_dir(), seq);
    }
    r
}

/// 同时给所有可能发起内核传输的模块打 `NtDeviceIoControlFile` 导入槽(以及
/// nvapi64.dll 自己的 DeviceIoControl)。IAT 补丁不做代码改写,没有指令对齐
/// 风险;代价是漏掉用 GetProcAddress 动态解析的调用方,因此返回值交给调用方
/// 做**存活性自检**(见 `nt_ioctl_hook_live`),静默必须是"确实没调用"而不是
/// "hook 没装上"。
unsafe fn install_nt_ioctl_hook() -> Option<NtDeviceIoControlFileFn> {
    let hook = hook_nt_device_io_control as *mut c_void;
    // 先把 ntdll 里的真函数解析并**在打补丁之前**存好:补丁一落地就可能有
    // 其它线程立刻走到 hook,而 hook 里 `ORIG_NT_DEVICE_IO_CONTROL` 为空会
    // 在 extern "system" 里 panic —— 那是 UB/AV,不是干净的报错。
    let ntdll = unsafe { LoadLibraryA(b"ntdll.dll\0".as_ptr()) };
    let real = unsafe { GetProcAddress(ntdll, b"NtDeviceIoControlFile\0".as_ptr()) };
    if real.is_null() {
        eprintln!("!!! 拿不到 ntdll!NtDeviceIoControlFile");
        return None;
    }
    unsafe { ORIG_NT_DEVICE_IO_CONTROL = Some(std::mem::transmute(real)) };
    eprintln!(
        "ntdll!NtDeviceIoControlFile @{:x} prologue={:02x?}",
        real as usize,
        unsafe { core::slice::from_raw_parts(real as *const u8, 24) }
    );
    // 不靠固定清单:每台机器上真正发起系统调用的模块不同(kernel32 只是转发
    // 体,实现落在 kernelbase 等),固定清单会整条漏掉。直接扫**所有**确实导入
    // NtDeviceIoControlFile 的已驻留模块。
    let hits = unsafe { patch_all_importers(b"NtDeviceIoControlFile", hook) };
    eprintln!("NtDeviceIoControlFile: 命中 {hits} 个模块的导入槽");
    unsafe {
        if let Some(o) = patch_iat_fn(
            "nvapi64.dll",
            b"DeviceIoControl",
            hook_device_io_control as *mut c_void,
        ) {
            eprintln!("nvapi64.dll!DeviceIoControl IAT 已补 -> {o:p}");
            if ORIG_DEVICE_IO_CONTROL.is_none() {
                ORIG_DEVICE_IO_CONTROL = Some(std::mem::transmute(o));
            }
        }
    }
    if hits == 0 {
        eprintln!("!!! ntdll 传输 hook 一个都没装上:整个进程没有模块导入它");
    }
    NTDLL_ORIG.store(real as *mut c_void, Ordering::SeqCst);
    Some(unsafe { std::mem::transmute(real) })
}

/// 遍历所有已驻留模块,把导入 `fname` 的槽全部补上,返回命中的模块数。
/// `patch_iat_fn` 只认模块基名,所以这里从完整路径里取文件名再回填;顺带把
/// 首个原函数指针记进 `NTDLL_ORIG`,供存活性自检/复位使用。
unsafe fn patch_all_importers(fname: &[u8], hook: *mut c_void) -> usize {
    let mut mods = [0isize; 512];
    let mut needed = 0u32;
    unsafe {
        EnumProcessModules(
            GetCurrentProcess(),
            mods.as_mut_ptr(),
            (mods.len() * 8) as u32,
            &mut needed,
        )
    };
    let count = (needed as usize / 8).min(mods.len());
    let mut hits = 0usize;
    for (k, m) in mods[..count].iter().enumerate() {
        let mut buf = [0u16; 260];
        let n = unsafe { GetModuleFileNameExW(GetCurrentProcess(), *m, buf.as_mut_ptr(), 260) };
        if n == 0 {
            continue;
        }
        let path = String::from_utf16_lossy(&buf[..n as usize]);
        let base = path.rsplit(['\\', '/']).next().unwrap_or(&path).to_string();
        if std::env::var_os("NVOC_TRACE_IAT").is_some() {
            eprintln!("  [{k}/{count}] 扫 {base}");
        }
        if let Some(o) = unsafe { patch_iat_fn(&base, fname, hook) } {
            hits += 1;
            eprintln!("  导入方: {base} (orig={o:p})");
            if NTDLL_ORIG.load(Ordering::SeqCst).is_null() {
                NTDLL_ORIG.store(o, Ordering::SeqCst);
            }
        }
    }
    hits
}

/// 存活性自检:用一个**已经打开的假设备句柄**,经 kernel32 的**真实函数体**
/// (内联 hook 的 trampoline,而不是我们自己的 hook)发一次 DeviceIoControl。
/// kernel32 无论如何都会把它转给 ntdll 系统调用(拿回 STATUS_INVALID_HANDLE),
/// 所以 `NT_HOOK_COUNT` 自增即证明 ntdll 侧 hook 确实在被调;不自增就说明本次
/// "写路径静默"是 hook 失效,而不是真的没有内核传输。
unsafe fn nt_ioctl_hook_live() -> bool {
    let Some(orig) = (unsafe { ORIG_DEVICE_IO_CONTROL }) else {
        eprintln!("自检跳过:kernel32 hook 未装(无 trampoline 可调)");
        return false;
    };
    let nul = wide("\\\\.\\NUL");
    let h = unsafe { CreateFileW(nul.as_ptr(), 0, 3, std::ptr::null_mut(), 3, 0, 0) };
    if h == -1 {
        eprintln!("自检跳过:CreateFileW(\\\\.\\NUL) 失败");
        return false;
    }
    let before = NT_HOOK_COUNT.load(Ordering::SeqCst);
    let mut out = 0u32;
    let mut ret = 0u32;
    // ioctl 码随便是多少:参数错/句柄错也照样走到 NtDeviceIoControlFile。
    unsafe {
        orig(
            h,
            0,
            std::ptr::null_mut(),
            0,
            (&mut out as *mut u32).cast(),
            4,
            &mut ret,
            std::ptr::null_mut(),
        )
    };
    unsafe { CloseHandle(h) };
    NT_HOOK_COUNT.load(Ordering::SeqCst) > before
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
    let img_size = unsafe { *(nt.add(24 + 56) as *const u32) } as usize; // SizeOfImage
    let import_rva = unsafe { *(nt.add(24 + 120) as *const u32) } as usize; // DataDirectory[1] for PE32+
    if import_rva == 0 || import_rva >= img_size {
        eprintln!("{module}: 无导入表/越界(rva={import_rva:#x} size={img_size:#x})");
        return None;
    }
    let mut first_orig: Option<*mut c_void> = None;
    // 内存映射镜像:RVA 即相对 base 的偏移(不做文件节区转换)。整个导入表在
    // 模块镜像内,所有 RVA 都按 SizeOfImage 做上界校验,避免在畸形/特殊模块
    // 上走出镜像触发 AV —— 逐模块扫描必须做到这点。
    let mut desc = base.add(import_rva) as *const u8;
    loop {
        if (desc as usize) - (base as usize) + 20 > img_size {
            eprintln!("导入表里未找到 {}", String::from_utf8_lossy(fname));
            return first_orig;
        }
        let name_rva = unsafe { *(desc.add(12) as *const u32) } as usize;
        if name_rva == 0 {
            eprintln!("导入表里未找到 {}", String::from_utf8_lossy(fname));
            return first_orig;
        }
        let orig_first = unsafe { *(desc as *const u32) } as usize;
        let first = unsafe { *(desc.add(16) as *const u32) } as usize;
        if orig_first != 0 && first != 0 && orig_first < img_size && first + 8 <= img_size {
            let mut i = 0usize;
            loop {
                if orig_first + i * 8 + 8 > img_size || first + i * 8 + 8 > img_size {
                    break;
                }
                let int_rva = unsafe { *(base.add(orig_first + i * 8) as *const u64) } as usize;
                if int_rva == 0 {
                    break;
                }
                if int_rva & 0x8000_0000_0000_0000 == 0 {
                    let name_off = int_rva & 0xFFFF_FFFF;
                    if name_off + 2 + fname.len() > img_size {
                        i += 1;
                        continue;
                    }
                    let hint = unsafe { base.add(name_off + 2) };
                    let nm = unsafe { core::slice::from_raw_parts(hint, fname.len()) };
                    if nm == fname {
                        let slot = unsafe { base.add(first + i * 8) } as *mut *mut c_void;
                        let orig = unsafe { *slot };
                        eprintln!(
                            "patch {} in {module} slot {slot:p} orig {orig:p}",
                            String::from_utf8_lossy(fname)
                        );
                        let mut old = 0u32;
                        let tr = std::env::var_os("NVOC_TRACE_IAT").is_some();
                        if tr {
                            eprintln!("  VP1 前 slot={slot:p}");
                        }
                        let ok1 = unsafe { VirtualProtect(slot as *mut c_void, 8, 0x40, &mut old) };
                        if tr {
                            eprintln!("  VP1 后 ok={ok1} old={old:#x}");
                        }
                        unsafe { *slot = hook };
                        if tr {
                            eprintln!("  写入完成 {i}");
                        }
                        let ok2 = unsafe { VirtualProtect(slot as *mut c_void, 8, old, &mut old) };
                        if tr {
                            eprintln!("  VP2 后 ok={ok2}");
                        }
                        if first_orig.is_none() {
                            first_orig = Some(orig);
                        }
                    }
                }
                i += 1;
            }
        }
        desc = unsafe { desc.add(20) };
        if std::env::var_os("NVOC_TRACE_IAT").is_some() {
            eprintln!("  描述符前进一档");
        }
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
    fn CreateFileW(
        path: *const u16,
        access: u32,
        share: u32,
        sa: *mut c_void,
        disp: u32,
        flags: u32,
        tmpl: isize,
    ) -> isize;
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
/// 单节逐字节快照上限:够覆盖观察到的全部载体节(最大 0x181000);更大的节
/// 只留指纹,不做差分。
const SNAPSHOT_CAP: usize = 0x40_0000;

/// 一个映射节的指纹(仅取前 2MB 做 FNV-1a;够覆盖毒表,避免全量读大节)与
/// 全量快照(≤ `SNAPSHOT_CAP`),后者用于逐字节差分定位真正的载体字段。
#[derive(Clone)]
struct RegionFp {
    base: usize,
    size: usize,
    protect: u32,
    hash: u64,
    head: Vec<u8>,
    data: Vec<u8>,
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
            let snapshot = if size <= SNAPSHOT_CAP {
                unsafe { core::slice::from_raw_parts(base as *const u8, size) }.to_vec()
            } else {
                Vec::new()
            };
            out.push(RegionFp {
                base,
                size,
                protect,
                hash: h,
                head: data[..data.len().min(0x40)].to_vec(),
                data: snapshot,
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

#[allow(dead_code)]
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

/// 找出两段字节里所有不同的区间(允许 ≤8B 的间隙合并成一段连续改写)。
fn diff_runs(before: &[u8], after: &[u8]) -> Vec<(usize, usize)> {
    let m = before.len().min(after.len());
    let mut runs = Vec::new();
    let mut i = 0usize;
    while i < m {
        if before[i] == after[i] {
            i += 1;
            continue;
        }
        let start = i;
        let mut last = i;
        let mut j = i + 1;
        while j < m {
            if before[j] != after[j] {
                last = j;
                j += 1;
            } else if j - last <= 8 {
                j += 1;
            } else {
                break;
            }
        }
        runs.push((start, last));
        i = j;
    }
    runs
}

/// 一段区间的 前/后 对照行(带改变位标记)。显示窗口带 ±16B 上下文并按 16 对齐。
fn diff_window(base: usize, before: &[u8], after: &[u8], start: usize, end: usize) -> String {
    const CTX: usize = 16;
    const MAX_ROWS: usize = 16;
    let lo = (start.saturating_sub(CTX)) & !0xF;
    let hi = (((end + CTX) / 16 + 1) * 16)
        .min(before.len())
        .min(after.len());
    let mut s = format!(
        "  base+{start:#06x}..={end:#06x} (len={})\n",
        end - start + 1
    );
    let mut i = lo;
    let mut rows = 0usize;
    while i < hi {
        let e = (i + 16).min(hi);
        if rows >= MAX_ROWS {
            s.push_str(&format!("  ... (窗口截断,共 {}B)\n", hi - lo));
            break;
        }
        let hexb: String = before[i..e]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let hexa: String = after[i..e]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<Vec<_>>()
            .join(" ");
        let mark: String = (i..e)
            .map(|k| if before[k] == after[k] { ' ' } else { '^' })
            .collect();
        s.push_str(&format!("  {:#012x} B {hexb:<48}\n", base + i));
        s.push_str(&format!("  {:#012x} A {hexa:<48}\n", base + i));
        s.push_str(&format!("  {:12}   {mark:<48}\n", ""));
        i = e;
        rows += 1;
    }
    s
}

/// 前后指纹差分:内容发生变化的映射节 = 该 NVAPI 调用真正写入的载体。落盘的是
/// **逐字节差异区间**(不是整节 dump),并标出 `needle`(本次写入的原始值)真正
/// 落在哪个偏移 —— 这样毒机与对照机的改动偏移可以直接逐行对齐比较。
fn dump_changed(
    prefix: &str,
    before: &[RegionFp],
    after: &[RegionFp],
    needle: Option<(u32, &str)>,
) {
    let find = |v: &[RegionFp], base: usize| v.iter().find(|f| f.base == base).cloned();
    let mut n = 0usize;
    for a in after {
        match find(before, a.base) {
            Some(b) if b.hash != a.hash => {
                if a.data.is_empty() || b.data.is_empty() {
                    emit(&format!(
                        "CHANGED[{prefix}] base={:#x} size={:#x} hash {:018x} -> {:018x}(> 4MB,仅指纹)",
                        a.base, a.size, b.hash, a.hash
                    ));
                    n += 1;
                    continue;
                }
                let runs = diff_runs(&b.data, &a.data);
                let mut s = format!(
                    "=== {prefix} base={:#x} size={:#x} changed_bytes={} runs={} ===\n",
                    a.base,
                    a.size,
                    runs.iter().map(|(x, y)| y - x + 1).sum::<usize>(),
                    runs.len()
                );
                for (st, en) in &runs {
                    s.push_str(&diff_window(a.base, &b.data, &a.data, *st, *en));
                }
                let _ = std::fs::write(
                    log_dir().join(format!("changed-{prefix}-{n:02}-{:x}.txt", a.base)),
                    &s,
                );
                let head: Vec<String> = runs
                    .iter()
                    .take(6)
                    .map(|(x, y)| format!("+{x:#x}({})", y - x + 1))
                    .collect();
                let tail = if runs.len() > 6 {
                    format!(" …共{}段", runs.len())
                } else {
                    String::new()
                };
                emit(&format!(
                    "CHANGED[{prefix}] #{} base={:#x} size={:#x} 改动 {}B/{} 段: {}{}",
                    n,
                    a.base,
                    a.size,
                    runs.iter().map(|(x, y)| y - x + 1).sum::<usize>(),
                    runs.len(),
                    head.join(" "),
                    tail
                ));
                if let Some((val, name)) = needle {
                    // 明文不一定等于调用侧的值:同一物理量在驱动里可能按 µW/mW、
                    // 或加固定偏移/编码存放(实测 180W 在改动的 4 字节里找不到
                    // 180000 的明文)。所以把几种常见标度一起试,命中哪一个直接
                    // 说明载体用的是哪种编码。
                    let cands: [(u64, &str); 5] = [
                        (val as u64, "原值"),
                        (val as u64 * 1000, "原值×1000(µW)"),
                        (val as u64 / 1000, "原值/1000"),
                        (val as u64 * 65536, "原值×65536(Q16)"),
                        (val as u64 & 0xFFFF, "原值低16位"),
                    ];
                    let mut hit = false;
                    for (st, en) in &runs {
                        let win = &a.data[*st..(en + 1).min(a.data.len())];
                        for (c, label) in cands {
                            for w in [4usize, 8] {
                                if w == 8 && c > u32::MAX as u64 {
                                    continue;
                                }
                                let pat = if w == 4 {
                                    (c as u32).to_le_bytes().to_vec()
                                } else {
                                    c.to_le_bytes().to_vec()
                                };
                                let mut k = 0usize;
                                while k + w <= win.len() {
                                    if win[k..k + w] == pat[..] {
                                        emit(&format!(
                                            "CARRIES[{prefix}] {name} 值 {val} 以 {label}({w}B) 落在 base+{:#x}",
                                            st + k
                                        ));
                                        hit = true;
                                        k += w;
                                    } else {
                                        k += 1;
                                    }
                                }
                            }
                        }
                    }
                    if !hit {
                        emit(&format!(
                            "CARRIES[{prefix}] {name} 值 {val} 在改动字节里没有明文命中(原值/µW/Q16/低16位都试过)—— 载体是索引或其它编码"
                        ));
                    }
                }
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
        emit(&format!("CHANGED[{prefix}] (没有节发生变化)"));
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

/// 把 ntdll 旁路 hook 攒下的记录落进 index.txt(**必须在 hook 之外**做,见
/// `hook_nt_device_io_control` 的注释)。带 `via=ntdll` 前缀,和 kernel32 那条
/// 通道的记录区分开 —— 这正是"新驱动是绕过 kernel32 直呼系统调用"的直接证据。
fn flush_nt_log() {
    let lines: Vec<String> = {
        let mut g = NT_LOG.lock().unwrap_or_else(|e| e.into_inner());
        std::mem::take(&mut *g)
    };
    let dir = log_dir();
    let _ = std::fs::create_dir_all(&dir);
    for l in &lines {
        append(&dir, "index.txt", &format!("{l}\n"));
    }
    emit(&format!(
        "ntdll 旁路记录 {} 条(全部为 via=ntdll,已写入 index.txt)",
        lines.len()
    ));
}

/// 把序号区间 [from,to] 内捕获到的 ioctl 归到一次 NVAPI 调用名下 —— 这是
/// "这次写到底走没走内核传输"的直接答案。空(NV 设备上)= 该调用期间没有
/// DeviceIoControl / NtDeviceIoControlFile 落到显卡设备上,即改动只落在用户态
/// 共享节里。判定前必须先看 LIVENESS;存活却仍为空才算"无内核传输"。
///
/// 进程里的 DeviceIoControl 流量并不都是显卡的(kernelbase/advapi32 导入方一
/// 补上,连控制台之类的小设备都会冒出来,实测在非提权跑里 0x500016/h=0x64 能
/// 刷几百条)。所以先建立"NV 设备句柄集合"——凡出现过 NV 特征 ioctl 的句柄都
/// 算 —— 归因只看这个集合,集合外的单独计数,免得噪声淹没结论。
fn transport_of(tag: &str, from: usize, to: usize) {
    // ntdll 旁路的记录末尾才落盘,所以这里必须同时看内存缓冲,否则会把
    // "经 ntdll 的传输"误判成"无内核传输"。
    let mut txt = std::fs::read_to_string(log_dir().join("index.txt")).unwrap_or_default();
    {
        let g = NT_LOG.lock().unwrap_or_else(|e| e.into_inner());
        for l in g.iter() {
            txt.push_str(l);
            txt.push('\n');
        }
    }
    const NV_SIG: [u32; 5] = [0x8de0004, 0x8de0008, 0x470807, 0x470813, 0x320004];
    let mut handles: Vec<u32> = Vec::new();
    for line in txt.lines() {
        if let (Some(d), Some(i)) = (parse_dev(line), parse_ioctl(line)) {
            if NV_SIG.contains(&i) && !handles.contains(&d) {
                handles.push(d);
            }
        }
    }
    let mut nv: Vec<&str> = Vec::new();
    let mut other = 0usize;
    for line in txt.lines() {
        if parse_ioctl(line).is_none() {
            continue;
        }
        let Some(s) = parse_seq(line) else { continue };
        if s < from || s > to {
            continue;
        }
        match parse_dev(line) {
            Some(d) if handles.contains(&d) => nv.push(line),
            _ => other += 1,
        }
    }
    let hx = || {
        handles
            .iter()
            .map(|h| format!("{h:#x}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    if nv.is_empty() {
        emit(&format!(
            "TRANSPORT[{tag}] NV 设备上无内核传输(序号 {from}..{to};NV 句柄=[{}];已过滤其它设备 {other} 条)",
            hx()
        ));
    } else {
        emit(&format!(
            "TRANSPORT[{tag}] NV 设备上 {} 条内核传输(NV 句柄=[{}];已过滤其它设备 {other} 条):",
            nv.len(),
            hx()
        ));
        for h in nv.iter().take(14) {
            emit(&format!("    {h}"));
        }
    }
}

/// 从一行 ioctl 记录里取 `seq=`(全局序号)。
fn parse_seq(line: &str) -> Option<usize> {
    let p = line.find("seq=")?;
    line.get(p + 4..p + 7)?.parse::<usize>().ok()
}

/// 从一行 ioctl 记录里取 `ioctl=`(十六进制 ioctl 码)。
fn parse_ioctl(line: &str) -> Option<u32> {
    let p = line.find("ioctl=")?;
    let rest = &line[p + 6..];
    let end = rest
        .find(|c: char| !c.is_ascii_hexdigit() && c != 'x' && c != 'X')
        .unwrap_or(rest.len());
    u32::from_str_radix(rest.get(..end)?.trim_start_matches("0x"), 16).ok()
}

/// 从一行 ioctl 记录里取 `dev=h=0x..`(设备句柄),用于把流量按设备分开。
fn parse_dev(line: &str) -> Option<u32> {
    let p = line.find("dev=h=")?;
    let rest = &line[p + 6..];
    let end = rest
        .find(|c: char| !c.is_ascii_hexdigit() && c != 'x' && c != 'X')
        .unwrap_or(rest.len());
    u32::from_str_radix(rest.get(..end)?.trim_start_matches("0x"), 16).ok()
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
    // 0x470807 的 payload 在共享节里:走 ntdll 旁路的那些调用也要 dump 节内容。
    // (hook 内只做内存读取 + 文件写;唯一在 hook 内会炸的是 println!,已移出。)
    NT_DUMP_SECTIONS.store(true, Ordering::SeqCst);
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

    // ---- 第二层探针:直接抓 ntdll 系统调用导入,覆盖"绕过 kernel32 直呼
    // NtDeviceIoControlFile"的可能;装完立刻做存活性自检,把"hook 死了"和
    // "真没有内核传输"彻底分开。----
    let nt_installed = unsafe { install_nt_ioctl_hook() }.is_some();
    emit(&format!("ntdll 传输 hook 安装 = {nt_installed}"));
    let live = unsafe { nt_ioctl_hook_live() };
    emit(&format!("LIVENESS ntdll 传输 hook 存活自检 = {live}"));
    if !live {
        emit(
            "!!! 自检失败 —— 下面的 TRANSPORT[...]=无 只能说明'没抓到',不能作为'无内核传输'的证据",
        );
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
    unsafe { LoadLibraryA(b"nvapi64_impl.dll\0".as_ptr()) };
    emit(&format!(
        "nvapi64_impl.dll 驻留 = {}",
        unsafe { GetModuleHandleW(wide("nvapi64_impl.dll").as_ptr()) } != 0
    ));

    // ---- GET-only 基线(不含任何写) ----
    let s_base = cur_seq();
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
    transport_of("GET baseline(纯读)", s_base, cur_seq());

    // ---- 安全对照:percent 写(ClientPowerPoliciesSetStatus,历史安全接口) ----
    let s_op1 = cur_seq();
    step("OP1 percent set_power_limit(100%)");
    mark("OP1>percent 100");
    emit(&format!(
        "percent 100%% -> {:?}",
        gpu.set_power_limit([nvapi::Percentage1000(100_000)])
    ));
    mark("OP1<");
    transport_of("OP1 percent100(安全接口)", s_op1, cur_seq());

    let pre = mapped_scan();
    fp_report("pre-toxic", &pre);

    // ---- 毒 OP2:绝对 TGP 写(0xAFFC2279)——对照机成功,毒机 TDR ----
    let s_op2 = cur_seq();
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
    transport_of("OP2 set_tgp_watt(毒接口)", s_op2, cur_seq());
    let post = mapped_scan();
    fp_report("post-watt", &post);
    dump_changed("watt", &pre, &post, Some((180_000, "tgp180W")));

    // ---- 毒 OP3:OCP 电流行(写一个与当前不同的值) ----
    let mut ocp_restore: Option<(u32, u32, u32)> = None;
    if let Ok(p) = &policies {
        if let Some(ocp) = p.iter().find(|r| r.is_ocp_current()) {
            let target = ocp.default_raw.saturating_sub(10_000).max(ocp.min_raw);
            ocp_restore = Some((ocp.policy_id, ocp.subtype, ocp.default_raw));
            let bp = mapped_scan();
            step(&format!("OP3 OCP-current write idx{}", ocp.index));
            let s_op3 = cur_seq();
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
            transport_of("OP3 OCP 电流写(毒族)", s_op3, cur_seq());
            let ap = mapped_scan();
            dump_changed("ocp", &bp, &ap, Some((target, "ocpTarget")));
        }
    }

    // ---- 毒 OP4:板功率行(写一个与当前不同的值) ----
    if let Ok(p) = &policies {
        if let Some(board) = p.iter().find(|r| r.is_board_power()) {
            let target = (board.default_raw + 20_000).min(board.max_raw);
            let bp = mapped_scan();
            step(&format!("OP4 board-power write idx{}", board.index));
            let s_op4 = cur_seq();
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
            transport_of("OP4 板功率写(毒族)", s_op4, cur_seq());
            let ap = mapped_scan();
            dump_changed("board", &bp, &ap, Some((target, "boardTarget")));
        }
    }

    // ---- 复位 ----
    let s_restore = cur_seq();
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
    transport_of("restore", s_restore, cur_seq());
    flush_nt_log();
    mark("S>needle scan");
    scan_process_memory(&[
        (180_000, "watt180"),
        (0x0001_0A4C, "stamp10A4C"),
        (0x0001_2720, "stamp12720"),
        (0x0005_B0B0, "stamp5B0B0"),
    ]);

    write_summary();
    emit(&format!(
        "done; kernel32 DeviceIoControl = {}, ntdll NtDeviceIoControlFile = {}, hook 存活={}",
        HOOK_COUNT.load(Ordering::SeqCst),
        NT_HOOK_COUNT.load(Ordering::SeqCst),
        live
    ));
    emit(&format!("capture dir = {}", log_dir().display()));
    std::process::exit(0);
}
