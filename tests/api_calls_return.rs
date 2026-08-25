// Tests in this file call apis and simply expect them to return without causing
// segfaults, etc.

// Not actually testing results. Just calling the api.
#![allow(unused_must_use)]

use nvapi::{ConnectedIdsFlags, PhysicalGpu};

#[test]
fn physicalgpu_display_ids_connected() {
    if nvapi::initialize().is_ok() {
        if let Ok(gpus) = PhysicalGpu::enumerate() {
            for gpu in gpus {
                // Bug: if there are zero connected displays this may crash.
                gpu.display_ids_connected(ConnectedIdsFlags::empty());
            }
        }
    }
}

#[test]
fn physicalgpu_display_ids_all() {
    if nvapi::initialize().is_ok() {
        if let Ok(gpus) = PhysicalGpu::enumerate() {
            for gpu in gpus {
                // Bug: if there are zero connected displays this may crash.
                gpu.display_ids_all();
            }
        }
    }
}
