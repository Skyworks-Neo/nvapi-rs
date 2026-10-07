//! TDR 隔离实验 M1(2026-10-07):只调 `private_lifecycle_init`,不碰任何
//! GET/SET。背景:compact NVAPI 写在本机(TU104/610.47)"干净 -1 + TDR"并存,
//! 写路径独有组件 = lifecycle init 与 SetStatus 内部命令序列;本探针隔离前者。
//! 需要提权;有 TDR 风险(用户已授权),实验间隔 ≥30s。
#![cfg(windows)]

#[test]
#[ignore = "live TDR-risk isolation probe (lifecycle init only); elevated, explicit run"]
fn lifecycle_init_only() {
    let gpu = nvapi::PhysicalGpu::enumerate()
        .expect("enumerate")
        .into_iter()
        .next()
        .expect("no gpu");
    let r = gpu.private_lifecycle_init();
    println!("private_lifecycle_init → {:?}", r);
}
