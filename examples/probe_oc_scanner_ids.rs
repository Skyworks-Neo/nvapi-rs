//! Driver-side OC Scanner family probe — resolve-only for the four IDs
//! wrapped in sys (Start/Stop/Revert/ RegisterForOcScannerStatusUpdates)
//! plus the unbound GetLastOcScannerResults. We do NOT start a scan here:
//! ClientStartOcScanner would hand the GPU to the driver's scanner and
//! overwrite the V/F curve. Resolve-only is safe.
//!
//! Run: cargo run --release -p nvapi --example probe_oc_scanner_ids

use nvapi::initialize;
use nvapi::sys::api::private::{
    NvAPI_GPU_ClientRegisterForOcScannerStatusUpdates, NvAPI_GPU_ClientRevertOc,
    NvAPI_GPU_ClientStartOcScanner, NvAPI_GPU_ClientStopOcScanner,
};
use nvapi::sys::nvapi_QueryInterface;

fn resolve(id: u32, name: &str) {
    let ok = match nvapi_QueryInterface(id) {
        Ok(_) => "RESOLVED",
        Err(_) => "NULL/Err",
    };
    println!("{name:44} 0x{id:08X} -> {ok}");
}

fn main() {
    let _ = initialize();

    // FFI-bound four
    resolve(0xBC4AEE25, "ClientStartOcScanner");
    resolve(0xC28B73DE, "ClientStopOcScanner");
    resolve(0xCC727B22, "ClientRevertOc");
    resolve(0x1CB41116, "ClientRegisterForOcScannerStatusUpdates");
    // Registered but not bound (layout unknown)
    resolve(0x593E8E72, "ClientGetLastOcScannerResults (unbound)");

    // Symbols exist (compile check); never called.
    let _ = (
        NvAPI_GPU_ClientStartOcScanner,
        NvAPI_GPU_ClientStopOcScanner,
        NvAPI_GPU_ClientRevertOc,
        NvAPI_GPU_ClientRegisterForOcScannerStatusUpdates,
    );
}
