//! VoltRails control 槽位 A/B 探针(2026-10-06,mVolt+ 交叉确认后落)。
//!
//! 背景:mVolt+ 以 `--nvvdd/--msvdd-offsets VMIN,REL,ALT[,OV]` 编辑电压策略
//! 偏移(VMIN=最低电压策略,REL=可靠性墙,ALT/OP=Vop,OV=过压顶),而我们只
//! 接线了 control 条目 payload 槽 0(µV offset)。本探针对槽 1..6 逐槽做
//! 「+step 扰动 → 读回 → 观测 status p0_min_hold/effective_wall 是否跟随 →
//! 恢复」,把 槽位→语义 映射表钉死。
//!
//! 安全口径:电压策略偏移面(与 set-volt-rail-limit 同一控制面),步进取
//! VoltDevices 报告的 step(典型 6.25 mV),每槽即改即还原;恢复失败响亮报错。
//!
//! Run: cargo test -p nvapi --test volt_rails_slot_probe_live -- --ignored --nocapture

#![allow(unused_must_use)]

use nvapi::PhysicalGpu;

#[test]
#[ignore]
fn volt_rails_slot_mapping_probe() {
    nvapi::initialize().expect("init");
    let gpus = PhysicalGpu::enumerate().expect("enumerate");
    let gpu = gpus.first().expect("no gpu");
    println!("GPU: {:?}", gpu.full_name());

    let before = gpu.volt_rails().expect("volt_rails");
    println!(
        "rail_mask=0x{:08X}, control entries={}, status entries={}",
        before.rail_mask,
        before.control.len(),
        before.status.len()
    );
    for entry in &before.control {
        println!(
            "  ctrl rail{} type{}: {:?}",
            entry.rail_bit, entry.entry_type, entry.values
        );
    }
    for entry in &before.status {
        println!(
            "  stat rail{} type{}: {:?}",
            entry.rail_bit, entry.entry_type, entry.values
        );
    }

    // 电压域步长(volt_devices;失败用 6250 µV 兜底 = 常见 6.25 mV step)
    let step = gpu
        .volt_devices()
        .ok()
        .and_then(|devices| devices.first().map(|d| d.step_uV))
        .filter(|step| *step > 0)
        .unwrap_or(6250);
    println!("step = {step} uV");

    for entry in &before.control {
        let rail = entry.rail_bit;
        let original = entry.values;
        // slots 4/5 crashed the driver on 4060 Laptop / 610 (2026-10-06
        // field report, -300 write ×2) — skipped unless explicitly forced.
        let unsafe_slots = std::env::var("NVOC_VOLT_SLOT_PROBE_UNSAFE").is_ok();
        let last_slot = if unsafe_slots { original.len() } else { 4 };
        for (slot, &base) in original
            .iter()
            .enumerate()
            .skip(1)
            .take(last_slot.saturating_sub(1))
        {
            let perturbed = base.wrapping_add(step as i32);
            println!("--- rail{rail} slot{slot}: {base} -> {perturbed} ---");
            match gpu.set_volt_rail_slot(rail, slot, perturbed) {
                Ok(retained) => {
                    println!("    SET retained={retained}");
                    let after = gpu.volt_rails().expect("volt_rails after");
                    for se in &after.status {
                        if se.rail_bit == rail {
                            println!("    status after: {:?}", se.values);
                        }
                    }
                    // restore
                    match gpu.set_volt_rail_slot(rail, slot, original[slot]) {
                        Ok(back) => println!("    restored={back}"),
                        Err(err) => panic!(
                            "rail{rail} slot{slot} 恢复失败({err:?})——手工写回 {}!",
                            original[slot]
                        ),
                    }
                }
                Err(err) => println!("    SET refused: {err:?}"),
            }
        }
    }
    // V2 STATUS (0x21620): the nine-slot full-fidelity form. values[6..8]
    // are RM dwords +120/+124/+128 (live 4060L: 0/625000/0, semantics open).
    println!("--- V2 status (values[0..8] + enum + type0 tail) ---");
    match gpu.volt_rails_status_v2() {
        Ok(entries) => {
            for (bit, typ, values, enum_byte, tail) in &entries {
                println!(
                    "  v2 rail{bit} type{typ}: {:?} enum={enum_byte} tail={tail:?}",
                    values
                );
            }
        }
        Err(err) => println!("  V2 rejected: {err:?}"),
    }

    println!("DONE — 槽位→语义以「status 哪个位跟随扰动」判定");
    println!(
        "已钉:slot1=VBIOS max wall 偏移(values[2],用户 A/B)、slot2=VRM max wall 偏移(status[3] 跟随)、slot3=VMIN 偏移(status[5] 跟随);⚠️slot4/5 在 4060L/610 上写入即爆驱动(-300 实测),默认跳过,强制需 NVOC_VOLT_SLOT_PROBE_UNSAFE=1"
    );
}
