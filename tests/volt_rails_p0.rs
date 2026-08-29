//! VoltRails P0-bounds derivation over multi-rail layouts.
//!
//! GB10 (DGX Spark, Blackwell) exposes two rails with *different* status
//! entry types — rail 0 (core) type 1, rail 1 (Xbar) type 3 — both carrying
//! the same six-value status layout. The lookups must match by `rail_bit`
//! only; filtering on `entry_type == 1` made rail 1's target wall read as 0
//! and blocked absolute-target overvolt derivation with a spurious
//! "status target wall is 0" error.

use nvapi::{VoltRailEntry, VoltRails};

fn entry(rail_bit: u32, entry_type: u32, values: [i32; 6]) -> VoltRailEntry {
    VoltRailEntry {
        rail_bit,
        entry_type,
        values,
    }
}

/// Live GB10 snapshot (driver 610-era, idle dGPU pre-wake hook active):
/// rail 0 type 1 `[925000, 1200000, 0, 1200000, 1200000, 640000]`,
/// rail 1 type 3 `[940000, 1025000, 0, 1200000, 1025000, 660000]`.
fn gb10_rails() -> VoltRails {
    VoltRails {
        rail_mask: 0x3,
        control: vec![
            entry(0, 3, [175000, 0, 0, 0, 0, 0]),
            entry(1, 3, [0, 0, 0, 0, 0, 0]),
        ],
        status: vec![
            entry(0, 1, [925000, 1200000, 0, 1200000, 1200000, 640000]),
            entry(1, 3, [940000, 1025000, 0, 1200000, 1025000, 660000]),
        ],
        rail_descriptors: Vec::new(),
    }
}

#[test]
fn p0_bounds_for_matches_type3_xbar_status_by_rail_bit() {
    let rails = gb10_rails();
    // The regression: the type-3 status entry must parse like a type-1 one.
    let xbar = rails.p0_bounds_for(1).expect("type-3 Xbar status must parse");
    assert_eq!(xbar.current_uV, 940000);
    assert_eq!(xbar.target_wall_uV, 1025000);
    assert_eq!(xbar.vbios_wall_uV, 0);
    assert_eq!(xbar.vrm_max_wall_uV, 1200000);
    assert_eq!(xbar.effective_wall_uV, 1025000);
    assert_eq!(xbar.min_hold_uV, 660000);

    let core = rails.p0_bounds_for(0).expect("type-1 core status must parse");
    assert_eq!(core.current_uV, 925000);
    assert_eq!(core.target_wall_uV, 1200000);
}

#[test]
fn p0_bounds_prefers_lowest_rail_regardless_of_type() {
    let rails = gb10_rails();
    let core = rails.p0_bounds().expect("core rail must parse");
    assert_eq!(core.current_uV, 925000);
}

#[test]
fn p0_bounds_for_rejects_implausible_layout() {
    // A driver that populates status with garbage (all-zero walls while
    // idle) must degrade to None instead of deriving a bogus base wall.
    let rails = VoltRails {
        rail_mask: 0x2,
        control: vec![entry(1, 3, [0; 6])],
        status: vec![entry(1, 3, [0; 6])],
        rail_descriptors: Vec::new(),
    };
    assert!(rails.p0_bounds_for(1).is_none());
    assert!(rails.p0_bounds().is_none());
}

#[test]
fn offset_ceiling_uses_the_requested_rails_status() {
    let rails = gb10_rails();
    // Rail 1: ceiling = vrm_max 1200000 − base (effective 1025000 − offset 0).
    assert_eq!(rails.offset_ceiling_uV(1), Some(175000));
    // Rail 0 with a +175000 offset already applied: base = effective −
    // offset = 1025000, ceiling = 1200000 − 1025000 = 175000. The result is
    // the max *absolute* offset the driver honours (offsets are absolute,
    // not deltas), so it stays 175000 even with 175000 already written.
    assert_eq!(rails.offset_ceiling_uV(0), Some(175000));
    // Absent rail must not silently borrow another rail's status (the old
    // cross-rail type-1 fallback did exactly that).
    assert_eq!(rails.offset_ceiling_uV(5), None);
}
