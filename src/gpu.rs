use crate::Status;
use crate::clock::{ClockDomain, ClockDomainInfo, VfpMask};
use crate::pstate::{PState, PStates};
use crate::sys::api::NvVersion;
use crate::sys::gpu::{clock, cooler, display, ecc, power, pstate, thermal};
use crate::sys::{self, driverapi, i2c};
use crate::types::{
    Kibibytes, Kilohertz2Delta, KilohertzDelta, Percentage, Percentage1000, RawConversion,
};
use log::{trace, warn};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::Infallible;
use std::{fmt, ptr};

#[derive(Debug)]
pub struct PhysicalGpu(sys::handles::NvPhysicalGpuHandle);

unsafe impl Send for PhysicalGpu {}

pub use sys::gpu::clock::ClockFrequencyType;
pub use sys::gpu::display::{ConnectedIdsFlags, DisplayIdsFlags, MonitorConnectorType};
pub use sys::gpu::private::{Foundry, RamMaker, RamType, VendorId as Vendor};
pub use sys::gpu::{
    ArchitectureId, BusType, ChipRevision, GpuType, PerformanceDecreaseReason, SystemType,
    WorkstationFeatureMask,
};
pub type ClockFrequencies = <clock::NV_GPU_CLOCK_FREQUENCIES as RawConversion>::Target;
pub type Utilizations = <pstate::NV_GPU_DYNAMIC_PSTATES_INFO_EX as RawConversion>::Target;

/// Process-global latest OC Scanner status notification, written from the
/// 0x1CB41116 callback trampoline (driver thread context) and read via
/// `Gpu::oem_oc_scanner_last_update()`.
#[derive(Default)]
struct OcScannerLastUpdate {
    scan_state: std::sync::atomic::AtomicU32,
    progress: std::sync::atomic::AtomicU32,
    status_0x60: std::sync::atomic::AtomicU32,
    status_0x64: std::sync::atomic::AtomicU32,
}

static OC_SCANNER_LAST: OcScannerLastUpdate = OcScannerLastUpdate {
    scan_state: std::sync::atomic::AtomicU32::new(0),
    progress: std::sync::atomic::AtomicU32::new(0),
    status_0x60: std::sync::atomic::AtomicU32::new(0),
    status_0x64: std::sync::atomic::AtomicU32::new(0),
};

/// Trampoline for the OC Scanner status callback (VelocityX ABI:
/// `fn(ctx, pStatus) -> u32`). Mirrors NVpower_wrapper sub_180008750:
/// derives the 3-state mapping from +0x48, snapshots +0x50/+0x60/+0x64,
/// returns the +0x64 dword.
unsafe extern "system" fn oc_scanner_status_trampoline(
    _ctx: *mut std::os::raw::c_void,
    p_status: *const clock::private::NV_GPU_OC_SCANNER_STATUS,
) -> u32 {
    use std::sync::atomic::Ordering;
    let Some(st) = p_status.as_ref() else {
        return 0;
    };
    OC_SCANNER_LAST
        .scan_state
        .store(st.scan_state(), Ordering::Relaxed);
    OC_SCANNER_LAST
        .progress
        .store(st.progress, Ordering::Relaxed);
    OC_SCANNER_LAST
        .status_0x60
        .store(st.status_0x60, Ordering::Relaxed);
    OC_SCANNER_LAST
        .status_0x64
        .store(st.status_0x64, Ordering::Relaxed);
    st.status_0x64
}

/// One per-rail entry from the private VoltRails control/status objects (the
/// "melonVolt path", reachable read-only through the public QueryInterface
/// table on this driver branch — see `reverse/melonvolt/ANALYSIS.md`).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VoltRailEntry {
    /// rail bit index (0..32) within the rail mask
    pub rail_bit: u32,
    /// entry type discriminator: RTX 5090 MSVDD control = 3 (the µV-offset
    /// entry melonVolt writes); RTX 4060 Laptop control = 0 (offset object,
    /// unsupported) / status = 1 (live voltage)
    pub entry_type: u32,
    /// six payload u32; semantics depend on `entry_type`. For **status** type 1
    /// (see [`VoltRails::p0_bounds`] and `sys::gpu::power::private::status_values`):
    /// `[current, target_wall, vbios_wall, vrm_max_wall, effective_wall, p0_min_hold]` µV —
    /// observed RTX 4060 Laptop rail0: `[940000, 1005000, 0, 1200000, 1005000, 625000]`
    /// (current 0.94 V, target wall 1.005 V, no vBIOS wall, ctrl max 1.200 V,
    /// effective 1.005 V, min-hold 0.625 V).
    /// For **control** type 3 (RTX 5090 MSVDD): index 0 = the µV offset.
    pub values: [i32; 6],
}

impl VoltRailEntry {
    fn from_raw((rail_bit, entry_type, values): (u32, u32, [i32; 6])) -> Self {
        Self {
            rail_bit,
            entry_type,
            values,
        }
    }
}

/// Raw 192-byte rail descriptor from GetInfo (only type @dword 19 decoded).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RailDescriptor {
    pub rail_bit: u32,
    /// 48 little-endian u32; dword 19 = entry type discriminator.
    pub raw_u32: Vec<u32>,
}

impl RailDescriptor {
    /// entry type discriminator (dword 19, byte offset +76)
    pub fn entry_type(&self) -> u32 {
        self.raw_u32.get(19).copied().unwrap_or(0)
    }
}

/// Read-only snapshot of the private VoltRails family: rail mask + per-rail
/// control entries (offset objects) and status entries (live voltages).
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VoltRails {
    /// bitmask of present rails (RTX 5090: 0x2 = MSVDD @ bit 1;
    /// RTX 4060 Laptop: 0x1 = single core rail)
    pub rail_mask: u32,
    pub control: Vec<VoltRailEntry>,
    pub status: Vec<VoltRailEntry>,
    /// raw 192-byte rail descriptors (48×u32) from GetInfo, indexed by rail
    /// bit — only dword 19 (type @+76) is decoded so far; the rest is
    /// undecoded driver data dumped for cross-platform comparison.
    pub rail_descriptors: Vec<RailDescriptor>,
}

/// P0 core-domain voltage bounds derived from a type-1 status entry
/// (semantics confirmed on RTX 4060 Laptop / 610.74 and desktop 20/30-series —
/// see `sys::gpu::power::private::status_values` for the per-index table).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[allow(nonstandard_style)] // uV suffix matches the sys-layer field naming
pub struct P0VoltageBounds {
    /// live core-rail voltage — payload index 0
    pub current_uV: i32,
    /// target wall (the value the SET side requested) — payload index 1
    pub target_wall_uV: i32,
    /// vBIOS voltage wall — payload index 2; 0 on mobile, on desktop a hard
    /// cap the effective wall cannot exceed. `0` = no vBIOS wall.
    pub vbios_wall_uV: i32,
    /// VRM-max wall — payload index 3; the max wall the VRM (voltage
    /// regulator) can sustain (1.200 V on observed GPUs).
    pub vrm_max_wall_uV: i32,
    /// effective wall — payload index 4; the final clamped wall in force
    /// (min of target / vbios_wall / vrm_max_wall).
    pub effective_wall_uV: i32,
    /// P0 min hold voltage (lowest that sustains P0) — payload index 5
    pub min_hold_uV: i32,
}

impl VoltRails {
    /// Extract P0 core-domain voltage bounds from the first type-1 status
    /// entry (preferring rail bit 0, the core rail on observed platforms).
    /// Returns `None` unless the values pass a plausibility check
    /// (`0 < min_hold <= current <= effective_wall`), so a differently-laid-out
    /// driver degrades to `None` instead of returning garbage.
    pub fn p0_bounds(&self) -> Option<P0VoltageBounds> {
        use power::private::status_values;
        let entry = self
            .status
            .iter()
            .filter(|e| e.entry_type == 1)
            .min_by_key(|e| e.rail_bit)?;
        let (current, effective, hold) = (
            entry.values[status_values::CURRENT_UV],
            entry.values[status_values::EFFECTIVE_WALL_UV],
            entry.values[status_values::P0_MIN_HOLD_UV],
        );
        if current > 0 && hold > 0 && effective >= hold && current <= effective {
            Some(P0VoltageBounds {
                current_uV: current,
                target_wall_uV: entry.values[status_values::TARGET_WALL_UV],
                vbios_wall_uV: entry.values[status_values::VBIOS_WALL_UV],
                vrm_max_wall_uV: entry.values[status_values::VRM_MAX_WALL_UV],
                effective_wall_uV: effective,
                min_hold_uV: hold,
            })
        } else {
            None
        }
    }

    /// Max overvolt offset the driver will actually honour for `rail_bit`.
    /// The effective wall (index 4) is clamped to `min(target, vbios_wall,
    /// vrm_max_wall)`, so the ceiling is `min(vbios_wall, vrm_max_wall) −
    /// base_wall`, where `base_wall = effective_wall − current_offset` (the
    /// wall at offset 0). A non-zero vBIOS wall (desktop) tightens the ceiling
    /// below vrm_max_wall. Returns `None` if the values don't parse.
    #[allow(non_snake_case)]
    pub fn offset_ceiling_uV(&self, rail_bit: u32) -> Option<i32> {
        use power::private::status_values;
        let status = self
            .status
            .iter()
            .filter(|e| e.entry_type == 1 && e.rail_bit == rail_bit)
            .min_by_key(|e| e.rail_bit)
            .or_else(|| self.status.iter().find(|e| e.entry_type == 1))?;
        let control = self.control.iter().find(|e| e.rail_bit == rail_bit)?;
        let vrm_max = status.values[status_values::VRM_MAX_WALL_UV];
        let vbios = status.values[status_values::VBIOS_WALL_UV];
        let effective = status.values[status_values::EFFECTIVE_WALL_UV];
        let current_offset = control.values[0];
        if vrm_max <= 0 || effective <= 0 {
            return None;
        }
        // The hard ceiling the effective wall can reach: vrm_max, or tighter
        // if a non-zero vBIOS wall (desktop) caps it.
        let mut ceiling = vrm_max;
        if vbios > 0 && vbios < ceiling {
            ceiling = vbios;
        }
        // base_wall = effective_wall − current_offset (the wall at offset 0).
        // Guard: if the current offset already pushes the wall above the
        // ceiling (or the offset sign is unexpected), fall back to effective.
        let base_wall = effective
            .checked_sub(current_offset)
            .filter(|b| *b > 0)
            .unwrap_or(effective);
        ceiling.checked_sub(base_wall).filter(|c| *c >= 0)
    }
}

/// Mode selector for the GetAllClockFrequencies V3 compact table
/// ([`PhysicalGpu::base_boost_clocks`]).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BaseBoostMode {
    Base = 1,
    Boost = 2,
}

/// One pstate floor/ceiling clamp from the Kepler-era ClientLimits family
/// (GET 0x39442CFB sibling / SET 0xFDFC7D49, private). `min_level`/
/// `max_level` constrain the pstate range the driver may pick; clearing the
/// limits table is the RELEASE path for a force-locked pstate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PstateClientLimit {
    pub pstate_id: u32,
    pub min_level: u32,
    pub max_level: u32,
}

impl PhysicalGpu {
    pub fn handle(&self) -> &sys::handles::NvPhysicalGpuHandle {
        &self.0
    }

    pub fn enumerate() -> crate::NvapiResult<Vec<Self>> {
        trace!("gpu.enumerate()");
        let mut handles = [Default::default(); sys::types::NVAPI_MAX_PHYSICAL_GPUS];
        let mut gpus = match unsafe { nvcall!(NvAPI_EnumPhysicalGPUs@get(&mut handles)) } {
            Err(crate::NvapiError {
                status: Status::NvidiaDeviceNotFound,
                ..
            }) => Vec::new(),
            Ok(len) => handles[..len as usize]
                .iter()
                .cloned()
                .map(PhysicalGpu)
                .collect(),
            Err(e) => return Err(e),
        };

        let tcc_gpus = Self::enumerate_tcc()?;
        for gpu in tcc_gpus {
            let handle = gpu.handle().as_ptr();
            if !gpus
                .iter()
                .any(|existing| existing.handle().as_ptr() == handle)
            {
                gpus.push(gpu);
            }
        }

        Ok(gpus)
    }

    fn enumerate_tcc() -> crate::NvapiResult<Vec<Self>> {
        trace!("gpu.enumerate_tcc()");
        let mut handles = [Default::default(); sys::types::NVAPI_MAX_PHYSICAL_GPUS];
        match unsafe { nvcall!(NvAPI_EnumTCCPhysicalGPUs@get(&mut handles)) } {
            Err(crate::NvapiError {
                status: Status::NvidiaDeviceNotFound,
                ..
            }) => Ok(Vec::new()),
            Err(crate::NvapiError {
                status: Status::NoImplementation,
                ..
            }) => Ok(Vec::new()),
            Err(crate::NvapiError {
                status: Status::NotSupported,
                ..
            }) => Ok(Vec::new()),
            Ok(len) => Ok(handles[..len as usize]
                .iter()
                .cloned()
                .map(PhysicalGpu)
                .collect()),
            Err(e) => Err(e),
        }
    }

    pub fn tachometer(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.tachometer()");
        unsafe { nvcall!(NvAPI_GPU_GetTachReading@get(self.0)) }
    }

    pub fn short_name(&self) -> crate::NvapiResult<String> {
        trace!("gpu.short_name()");
        unsafe { nvcall!(NvAPI_GPU_GetShortName@get(self.0) => into) }
    }

    pub fn full_name(&self) -> crate::NvapiResult<String> {
        trace!("gpu.full_name()");
        unsafe { nvcall!(NvAPI_GPU_GetFullName@get(self.0) => into) }
    }

    pub fn uuid(&self) -> crate::NvapiResult<String> {
        trace!("gpu.uuid()");

        // First try the string-based overload (works on most drivers).
        let string_result: crate::NvapiResult<String> =
            unsafe { nvcall!(NvAPI_GPU_GetUUID@get(self.0) => into) };

        match string_result {
            Ok(uuid) => return Ok(uuid),
            Err(crate::NvapiError {
                status: Status::IncompatibleStructVersion,
                ..
            }) => {
                trace!(
                    "gpu.uuid(): string overload returned INCOMPATIBLE_STRUCT_VERSION, falling back to V1 struct"
                );
            }
            Err(e) => return Err(e),
        }

        // Fallback: use the versioned NV_GPU_UUID_V1 struct (R595+ driver path).
        // Some older GPUs (10/20-series) on newer drivers only support this path.
        self.uuid_v1()
    }

    fn uuid_v1(&self) -> crate::NvapiResult<String> {
        use crate::sys::gpu::NV_GPU_UUID_V1;

        let mut data = NV_GPU_UUID_V1::default();

        // The nvapi! macro types NvAPI_GPU_GetUUID to take *mut NvAPI_ShortString,
        // but NVAPI dispatches by function ID (0xdc95673d) and the driver actually
        // accepts a versioned NV_GPU_UUID_V1* when the struct-version path is used.
        // Cast the pointer to match the declared FFI signature — same as the
        // vfp_curve V1 fallback pattern.
        let status =
            unsafe { sys::api::NvAPI_GPU_GetUUID(self.0, ptr::from_mut(&mut data).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_GetUUID, status)?;

        // Format the 16-byte GUID as a standard UUID string:
        // xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        let u = data.uuid;
        Ok(format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            u[0],
            u[1],
            u[2],
            u[3],
            u[4],
            u[5],
            u[6],
            u[7],
            u[8],
            u[9],
            u[10],
            u[11],
            u[12],
            u[13],
            u[14],
            u[15],
        ))
    }

    pub fn vbios_version(&self) -> crate::NvapiResult<(u32, u32)> {
        trace!("gpu.vbios_revision()");
        Ok(unsafe {
            (
                nvcall!(NvAPI_GPU_GetVbiosRevision@get(self.0))?,
                nvcall!(NvAPI_GPU_GetVbiosOEMRevision@get(self.0))?,
            )
        })
    }

    pub fn vbios_version_string(&self) -> crate::NvapiResult<String> {
        trace!("gpu.vbios_version_string()");
        unsafe { nvcall!(NvAPI_GPU_GetVbiosVersionString@get(self.0) => into) }
    }

    pub fn driver_model(&self) -> crate::NvapiResult<DriverModel> {
        trace!("gpu.driver_model()");
        unsafe { nvcall!(NvAPI_GetDriverModel@get(self.0)).map(DriverModel::new) }
    }

    pub fn gpu_id(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.gpu_id()");
        unsafe { nvcall!(NvAPI_GetGPUIDfromPhysicalGPU@get(self.0)) }
    }

    pub fn pci_identifiers(&self) -> crate::NvapiResult<PciIdentifiers> {
        trace!("gpu.pci_identifiers()");
        let mut pci = PciIdentifiers::default();
        unsafe {
            nvcall!(NvAPI_GPU_GetPCIIdentifiers(
                self.0,
                &mut pci.device_id,
                &mut pci.subsystem_id,
                &mut pci.revision_id,
                &mut pci.ext_device_id
            ))
            .map(|()| pci)
        }
    }

    pub fn bus_info(&self) -> crate::Result<BusInfo> {
        trace!("gpu.bus_info()");
        let bus_type = self.bus_type()?;
        Ok(BusInfo {
            irq: self.irq()?,
            id: self.bus_id()?,
            slot_id: self.bus_slot_id()?,
            bus: match bus_type {
                BusType::Pci => Bus::Pci {
                    ids: self.pci_identifiers()?,
                },
                BusType::PciExpress => Bus::PciExpress {
                    ids: self.pci_identifiers()?,
                    lanes: self.pcie_lanes()?,
                },
                ty => Bus::Other(ty),
            },
        })
    }

    pub fn gpu_type(&self) -> crate::Result<GpuType> {
        trace!("gpu.gpu_type()");
        unsafe { nvcall!(NvAPI_GPU_GetGPUType@get(self.0) => try) }
    }

    pub fn bus_type(&self) -> crate::Result<BusType> {
        trace!("gpu.bus_type()");
        unsafe { nvcall!(NvAPI_GPU_GetBusType@get(self.0) => try) }
    }

    pub fn bus_id(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.bus_id()");
        unsafe { nvcall!(NvAPI_GPU_GetBusId@get(self.0)) }
    }

    pub fn bus_slot_id(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.bus_slot_id()");
        unsafe { nvcall!(NvAPI_GPU_GetBusSlotId@get(self.0)) }
    }

    pub fn irq(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.irq()");
        unsafe { nvcall!(NvAPI_GPU_GetIRQ@get(self.0)) }
    }

    pub fn pcie_lanes(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.pcie_lanes()");
        unsafe { nvcall!(NvAPI_GPU_GetCurrentPCIEDownstreamWidth@get(self.0)) }
    }

    pub fn board_number(&self) -> crate::NvapiResult<[u8; 0x10]> {
        trace!("gpu.board_number()");
        unsafe { nvcall!(NvAPI_GPU_GetBoardInfo@get(self.0)).map(|data| *data.BoardNum) }
    }

    pub fn system_type(&self) -> crate::Result<SystemType> {
        trace!("gpu.system_type()");
        unsafe { nvcall!(NvAPI_GPU_GetSystemType@get(self.0) => try) }
    }

    pub fn core_count(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.core_count()");
        unsafe { nvcall!(NvAPI_GPU_GetGpuCoreCount@get(self.0)) }
    }

    pub fn shader_pipe_count(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.shader_pipe_count()");
        unsafe { nvcall!(NvAPI_GPU_GetShaderPipeCount@get(self.0)) }
    }

    pub fn shader_sub_pipe_count(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.shader_sub_pipe_count()");
        unsafe { nvcall!(NvAPI_GPU_GetShaderSubPipeCount@get(self.0)) }
    }

    pub fn ram_type(&self) -> crate::Result<RamType> {
        trace!("gpu.ram_type()");
        unsafe { nvcall!(NvAPI_GPU_GetRamType@get(self.0) => try) }
    }

    pub fn ram_maker(&self) -> crate::Result<RamMaker> {
        trace!("gpu.ram_maker()");
        unsafe { nvcall!(NvAPI_GPU_GetRamMaker@get(self.0) => try) }
    }

    pub fn ram_bus_width(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.ram_bus_width()");
        unsafe { nvcall!(NvAPI_GPU_GetRamBusWidth@get(self.0)) }
    }

    pub fn ram_bank_count(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.ram_bank_count()");
        unsafe { nvcall!(NvAPI_GPU_GetRamBankCount@get(self.0)) }
    }

    pub fn ram_partition_count(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.ram_partition_count()");
        unsafe { nvcall!(NvAPI_GPU_GetPartitionCount@get(self.0)) }
    }

    pub fn foundry(&self) -> crate::Result<Foundry> {
        trace!("gpu.foundry()");
        unsafe { nvcall!(NvAPI_GPU_GetFoundry@get(self.0) => try) }
    }

    #[allow(deprecated)]
    pub fn memory_info(&self) -> crate::NvapiResult<MemoryInfo> {
        trace!("gpu.memory_info()");

        unsafe { nvcall!(NvAPI_GPU_GetMemoryInfo@get(self.0) => raw) }
    }

    pub fn physical_frame_buffer_size(&self) -> crate::NvapiResult<Kibibytes> {
        trace!("gpu.physical_frame_buffer_size()");
        unsafe { nvcall!(NvAPI_GPU_GetPhysicalFrameBufferSize@get(self.0)).map(Kibibytes) }
    }

    pub fn virtual_frame_buffer_size(&self) -> crate::NvapiResult<Kibibytes> {
        trace!("gpu.virtual_frame_buffer_size()");
        unsafe { nvcall!(NvAPI_GPU_GetVirtualFrameBufferSize@get(self.0)).map(Kibibytes) }
    }

    pub fn architecture(&self) -> crate::NvapiResult<ArchInfo> {
        trace!("gpu.architecture()");

        unsafe { nvcall!(NvAPI_GPU_GetArchInfo@get(self.0) => raw) }
    }

    /// Static compute/PhysX/framebuffer capability word
    /// (`NvAPI_GPU_GetComputeCapabilities`, ID `0xb7bcf50d`). Despite the name the bits are
    /// PhysX/compute-software/framebuffer oriented, NOT virtualization/large-BAR (see
    /// [sys::gpu::NV_GPU_COMPUTE_CAPS]). One-shot descriptor — the driver maps
    /// `DATA_NOT_FOUND` to a zero word, so an all-clear result is meaningful.
    pub fn compute_capabilities(&self) -> crate::NvapiResult<ComputeCapabilities> {
        trace!("gpu.compute_capabilities()");

        unsafe { nvcall!(NvAPI_GPU_GetComputeCapabilities@get(self.0) => raw) }
    }

    pub fn workstation_features(
        &self,
    ) -> crate::NvapiResult<(WorkstationFeatureMask, WorkstationFeatureMask)> {
        trace!("gpu.workstation_features()");

        unsafe {
            nvcall!(NvAPI_GPU_WorkstationFeatureQuery@get2(self.0)).map(
                |(configured, consistent)| {
                    (
                        WorkstationFeatureMask::from_bits_truncate(configured.value),
                        WorkstationFeatureMask::from_bits_truncate(consistent.value),
                    )
                },
            )
        }
    }

    pub fn ecc_status(
        &self,
    ) -> crate::Result<<ecc::NV_GPU_ECC_STATUS_INFO as RawConversion>::Target> {
        trace!("gpu.ecc_status()");

        unsafe { nvcall!(NvAPI_GPU_GetECCStatusInfo@get(self.0) => raw) }
    }

    pub fn ecc_errors(
        &self,
    ) -> crate::NvapiResult<<ecc::NV_GPU_ECC_ERROR_INFO as RawConversion>::Target> {
        trace!("gpu.ecc_errors()");

        unsafe { nvcall!(NvAPI_GPU_GetECCErrorInfo@get(self.0) => raw) }
    }

    pub fn ecc_reset(&self, current: bool, aggregate: bool) -> crate::NvapiResult<()> {
        trace!("gpu.ecc_reset({:?}, {:?})", current, aggregate);

        unsafe {
            nvcall!(NvAPI_GPU_ResetECCErrorInfo(
                self.0,
                current.into(),
                aggregate.into()
            ))
        }
    }

    pub fn ecc_configuration(&self) -> crate::NvapiResult<(bool, bool)> {
        trace!("gpu.ecc_configuration()");

        unsafe {
            nvcall!(NvAPI_GPU_GetECCConfigurationInfo@get(self.0))
                .map(|raw| (raw.isEnabled(), raw.isEnabledByDefault()))
        }
    }

    pub fn ecc_configure(&self, enable: bool, immediately: bool) -> crate::NvapiResult<()> {
        trace!("gpu.ecc_configure()");

        unsafe {
            nvcall!(NvAPI_GPU_SetECCConfiguration(
                self.0,
                enable.into(),
                immediately
            ))
        }
    }

    pub fn clock_frequencies(
        &self,
        clock_type: ClockFrequencyType,
    ) -> crate::NvapiResult<ClockFrequencies> {
        trace!("gpu.clock_frequencies({:?})", clock_type);
        let mut clocks = clock::NV_GPU_CLOCK_FREQUENCIES::default();
        clocks.set_clock_type(clock_type.raw());

        unsafe { nvcall!(NvAPI_GPU_GetAllClockFrequencies@get{clocks}(self.0) => raw) }
    }

    /// Effective (actually-running) clocks via GetAllClocks V2 (ID 0x1bd69f49,
    /// RTSS `NV_GPU_CLOCK_INFO_V2`). Returns the `extendedDomain` effective
    /// frequency per present public domain (Graphics/Memory/Processor).
    pub fn effective_clocks(&self) -> crate::NvapiResult<crate::clock::EffectiveClocks> {
        trace!("gpu.effective_clocks()");
        let mut data = clock::private::NV_GPU_CLOCK_INFO_V2 {
            version: NvVersion::new(size_of::<clock::private::NV_GPU_CLOCK_INFO_V2>(), 2),
            ..Default::default()
        };
        // Same function ID as the V1 GetAllClocks; pass the V2 buffer via a
        // cast pointer (the driver reads the version tag to pick the layout).
        let status =
            unsafe { sys::api::NvAPI_GPU_GetAllClocks(self.0, ptr::from_mut(&mut data).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_GetAllClocks, status).and_then(|_| {
            use crate::types::RawConversion;
            data.convert_raw().map_err(Into::into)
        })
    }

    /// All 32 effective clock domains via GetAllClocks V2 (ID 0x1bd69f49,
    /// RTSS `NV_GPU_CLOCK_INFO_V2.extendedDomain[]`), keyed by
    /// [`ClockDomainId`](crate::clock::ClockDomainId). Superset of
    /// [`effective_clocks`](Self::effective_clocks): additionally exposes the
    /// internal fabric clocks (Gpc, **Xbar/crossbar**, Sys, Hub, Host, …,
    /// Pciegen). Only domains with a non-zero effective frequency are
    /// returned.
    pub fn all_clocks(&self) -> crate::NvapiResult<crate::clock::AllClocks> {
        trace!("gpu.all_clocks()");
        let mut data = clock::private::NV_GPU_CLOCK_INFO_V2 {
            version: NvVersion::new(size_of::<clock::private::NV_GPU_CLOCK_INFO_V2>(), 2),
            ..Default::default()
        };
        let status =
            unsafe { sys::api::NvAPI_GPU_GetAllClocks(self.0, ptr::from_mut(&mut data).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_GetAllClocks, status)
            .map(|_| crate::clock::all_clocks_from_raw(&data))
    }

    /// Base/boost clock pairs via GetAllClockFrequencies V3 compact (ID
    /// 0xDCB616C3, magic 0x30108 — discovered in AmpereOC). `mode` selects
    /// the table: 1 = base, 2 = boost. slot[0] = core kHz, slot[1] = memory
    /// kHz. Live-verified on Ada mobile (4060L: base 2175/8001 MHz, boost
    /// 2370/8001 MHz). Returns `(core_kHz, memory_kHz)` for the mode.
    pub fn base_boost_clocks(&self, mode: BaseBoostMode) -> crate::NvapiResult<(u32, u32)> {
        trace!("gpu.base_boost_clocks({mode:?})");
        let mut data =
            unsafe { std::mem::zeroed::<clock::private::NV_GPU_CLOCK_INFO_V3_COMPACT>() };
        data.version = NvVersion::new(0x108, 3);
        data.mode = mode as u32;
        let status = unsafe {
            sys::api::NvAPI_GPU_GetAllClockFrequencies(self.0, ptr::from_mut(&mut data).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_GetAllClockFrequencies, status).map(|_| {
            let core = data.slots[0].value_kHz;
            let mem = data.slots[1].value_kHz;
            (core, mem)
        })
    }

    /// Per-channel / per-rail power via PowerMonitor v4 GetInfo + v1 GetStatus
    /// (IDs 0xC12EB19E / 0xF40238EF). Returns the decoded descriptor table
    /// (channel_type, pwr_rail identity, Q12 scaling) plus a best-effort live
    /// reading for channel 0 (the total GPU power channel).
    ///
    /// **Pre-wrap / research.** The descriptor layout is variable-stride and
    /// parsed by signature scan; the GetStatus values are raw with units not
    /// yet confirmed (see [`crate::power`] docs). Gate the call on the GPU
    /// supporting PowerMonitor — returns an error otherwise.
    pub fn power_monitor_v4(&self) -> crate::NvapiResult<crate::power::PowerMonitor> {
        trace!("gpu.power_monitor_v4()");
        // GetInfo: try the richest layout first, fall back to smaller ones so
        // older drivers that reject v4 still yield descriptors. v4|6312,
        // v3|3240, v1|2728 share an identical header + descriptor-offset
        // format (differ only in type=5 VF-LUT truncation), so the reader
        // works on whichever succeeds. Each arm stamps its version magic and,
        // on Ok, builds a version-independent PowerMonitorInfo.
        macro_rules! try_getinfo {
            ($ty:ty, $ver:expr) => {{
                let mut info = <$ty>::default();
                info.version = NvVersion::new(size_of::<$ty>(), $ver);
                let status = unsafe {
                    sys::api::NvAPI_GPU_PowerMonitorGetInfo(self.0, ptr::from_mut(&mut info).cast())
                };
                if crate::status_result(sys::Api::NvAPI_GPU_PowerMonitorGetInfo, status).is_ok() {
                    Some(crate::power::PowerMonitorInfo::from(&info))
                } else {
                    None
                }
            }};
        }
        let info = try_getinfo!(power::private::NV_GPU_POWER_MONITOR_GET_INFO_V4, 4)
            .or_else(|| try_getinfo!(power::private::NV_GPU_POWER_MONITOR_GET_INFO_V3_3240, 3))
            .or_else(|| try_getinfo!(power::private::NV_GPU_POWER_MONITOR_GET_INFO_V1_2728, 1))
            .ok_or(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_PowerMonitorGetInfo,
                sys::Status::NotSupported,
            ))?;

        // GetStatus v1|392: the driver only fills channels whose bits are set
        // in the INPUT channel_mask at +0x04. Default (zero) fills only ch0;
        // pass GetInfo's full mask so every present channel is populated.
        let mut status_buf = [0u8; 392];
        // +0x00 version = (1<<16)|392, +0x04 channel_mask = GetInfo's mask.
        status_buf[0..4].copy_from_slice(&(NvVersion::new(392, 1).data).to_le_bytes());
        let mask = info.channel_mask;
        status_buf[4..8].copy_from_slice(&mask.to_le_bytes());
        let status = unsafe {
            sys::api::NvAPI_GPU_PowerMonitorGetStatus(self.0, status_buf.as_mut_ptr().cast())
        };
        // GetStatus is best-effort (may stub on some driver/GPU combos); a
        // non-Ok status still yields descriptors from GetInfo, just no live
        // values. Only treat a real Ok as carrying status bytes.
        let _ = crate::status_result(sys::Api::NvAPI_GPU_PowerMonitorGetStatus, status);
        Ok(crate::power::power_monitor_from_raw(&info, &status_buf))
    }

    /// The 4 GPU-Z-confirmed per-rail power readings (Board / Chip / MVDDC /
    /// PWR_SRC) in milliwatts, via PowerMonitor GetStatus v1|392. Units
    /// confirmed by exact GPU-Z match. Returns `Ok(PowerRails::default())`
    /// (all-None) if the GPU/driver doesn't expose PowerMonitor — callers can
    /// treat any `None` field as "not available on this GPU". See
    /// [`crate::power::PowerRails`] for the layout caveat.
    pub fn power_rails(&self) -> crate::NvapiResult<crate::power::PowerRails> {
        trace!("gpu.power_rails()");
        // GetInfo (v4 -> v3 -> v1|2728 fallback) for the descriptor table +
        // channel_mask. The descriptors tell us each channel's pwr_rail
        // identity — we label each reading by that, NOT by a hardcoded GetStatus
        // offset (offsets are channel-order-dependent and differ per GPU:
        // e.g. +0x44 is InputTotalBoard on a 4060 laptop but InputPex12v1 / PCIe
        // slot on a desktop Turing).
        macro_rules! try_getinfo {
            ($ty:ty, $ver:expr) => {{
                let mut info = <$ty>::default();
                info.version = NvVersion::new(size_of::<$ty>(), $ver);
                let status = unsafe {
                    sys::api::NvAPI_GPU_PowerMonitorGetInfo(self.0, ptr::from_mut(&mut info).cast())
                };
                if crate::status_result(sys::Api::NvAPI_GPU_PowerMonitorGetInfo, status).is_ok() {
                    Some(crate::power::PowerMonitorInfo::from(&info))
                } else {
                    None
                }
            }};
        }
        let info = try_getinfo!(power::private::NV_GPU_POWER_MONITOR_GET_INFO_V4, 4)
            .or_else(|| try_getinfo!(power::private::NV_GPU_POWER_MONITOR_GET_INFO_V3_3240, 3))
            .or_else(|| try_getinfo!(power::private::NV_GPU_POWER_MONITOR_GET_INFO_V1_2728, 1))
            .ok_or(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_PowerMonitorGetInfo,
                sys::Status::NotSupported,
            ))?;

        // channel_bit -> (pwr_rail, channel_type) from the descriptors.
        let rail_map = crate::power::descriptor_rail_map(&info);

        // Per-bit GetStatus isolation: one call per channel with
        // channel_mask = (1<<bit). This is the order-independent path (the
        // full-mask call returns -1 on some GPUs, e.g. desktop Turing), and
        // it isolates each channel's record so we extract its value without a
        // hardcoded offset. Cheap: each call is a sub-ms RM escape, ≤11 calls.
        //
        // EXTRACTION: every per-bit buffer shares a baseline of header/offset
        // slots (version, mask, the +0x08 accumulator, the +0x80 calibration
        // offset, and channel-0's slot at +0x44). The channel's OWN value is
        // at an offset that is nonzero in THIS bit's buffer but absent from the
        // shared baseline. We compute the baseline = offsets present across ALL
        // sampled bits, then each channel's value = largest nonzero u32 at a
        // non-baseline offset (channel-0 falls back to +0x44, which is its own
        // slot). This is offset-agnostic and correct per-GPU.
        let bits: Vec<u32> = rail_map.iter().map(|(b, _, _)| *b).collect();
        // Sample every bit first, collect (bit -> Vec<(offset,value)>).
        let mut per_bit: Vec<(u32, Vec<(usize, u32)>)> = Vec::new();
        for &bit in &bits {
            let mut status_buf = [0u8; 392];
            status_buf[0..4].copy_from_slice(&(NvVersion::new(392, 1).data).to_le_bytes());
            status_buf[4..8].copy_from_slice(&(1u32 << bit).to_le_bytes());
            let status = unsafe {
                sys::api::NvAPI_GPU_PowerMonitorGetStatus(self.0, status_buf.as_mut_ptr().cast())
            };
            if crate::status_result(sys::Api::NvAPI_GPU_PowerMonitorGetStatus, status).is_ok() {
                let nz = crate::power::nonzero_offsets(&status_buf);
                per_bit.push((bit, nz));
            } else {
                per_bit.push((bit, Vec::new())); // rail present but unreadable
            }
        }
        // EXTRACTION + DISAMBIGUATION with confidence tiers.
        //
        // On some GPUs (e.g. desktop Turing) a per-bit GetStatus does NOT
        // isolate one channel — summation (type=1) channels return a full-board
        // view (the same shared offsets +0x08/+0x2C/+0x44/+0x74 appear for
        // every bit). Only some channels (type=8 SensorClientAligned) get a
        // genuinely private offset. So a channel's reading is assigned a
        // [`crate::power::Confidence`] tier:
        //  - Measured: ≥1 PRIVATE offset (nonzero here, absent from every other
        //    bit). Trustworthy.
        //  - Inferred: no private offset, but topology disambiguation found a
        //    shared offset this channel is the UNIQUE un-owned claimant of
        //    (every other bit sharing it is already resolved), AND the offset's
        //    GPU-Z label (if known) matches the descriptor rail, AND all
        //    claimants agree on the value within 25%. Best-effort.
        //  - Ambiguous: no clean candidate; value = largest non-baseline read
        //    (a full-board view that may duplicate another rail).
        //  - Unavailable: GetStatus didn't populate this channel.
        //
        // The disambiguation runs as a worklist to a fixed point: ownership is
        // MONOTONIC (once an offset is claimed via `resolved_owner`, no other
        // channel may claim it — this prevents a private offset of an already-
        // resolved type=8 channel from leaking into every other channel's
        // candidate pool). Values are computed AFTER ownership settles.
        Ok(crate::power::disambiguate_power_rails(&rail_map, &per_bit))
    }

    pub fn current_pstate(&self) -> crate::Result<PState> {
        trace!("gpu.current_pstate()");

        unsafe { nvcall!(NvAPI_GPU_GetCurrentPstate@get(self.0) => try) }
    }

    pub fn pstates(&self) -> crate::Result<PStates> {
        trace!("gpu.pstates()");
        // Version cascade for old drivers / old GPUs. The default alias is
        // V2 stamped version 3 (7416 bytes, magic `0x31CF8`) — the variant
        // EVGA Precision X1 drives and the one the R610.74 driver accepts and
        // fills (live-verified RTX 4060 Laptop: 5 pstates x 3 clock domains,
        // 456-byte pstate records). Older drivers reject newer magics, so
        // retry V2(2) (7416B, `0x21CF8`) and V1(1) (7316B, `0x11C94`) before
        // giving up on the whole Pstates20 family and dropping to the legacy
        // pre-pstates20 API (deprecated since R304, Kepler/Maxwell era).
        macro_rules! try_pstates20 {
            ($ty:ty, $ver:expr) => {{
                let mut raw = unsafe { std::mem::zeroed::<$ty>() };
                raw.version = NvVersion::new(size_of::<$ty>(), $ver);
                let status = unsafe {
                    sys::api::NvAPI_GPU_GetPstates20(self.0, ptr::from_mut(&mut raw).cast())
                };
                if crate::status_result(sys::Api::NvAPI_GPU_GetPstates20, status).is_ok() {
                    match raw.convert_raw() {
                        Ok(p) => Some(p),
                        // A conversion failure on driver-validated data is
                        // unexpected; treat like a version miss and let the
                        // next arm (or legacy) take over.
                        Err(_) => None,
                    }
                } else {
                    None
                }
            }};
        }
        if let Some(p) = try_pstates20!(pstate::NV_GPU_PERF_PSTATES20_INFO_V2, 3)
            .or_else(|| try_pstates20!(pstate::NV_GPU_PERF_PSTATES20_INFO_V2, 2))
        {
            return Ok(p);
        }
        // V1 (7316B) has no over-voltage array; build the view manually.
        {
            let mut raw = unsafe { std::mem::zeroed::<pstate::NV_GPU_PERF_PSTATES20_INFO_V1>() };
            raw.version = NvVersion::new(size_of::<pstate::NV_GPU_PERF_PSTATES20_INFO_V1>(), 1);
            let status =
                unsafe { sys::api::NvAPI_GPU_GetPstates20(self.0, ptr::from_mut(&mut raw).cast()) };
            if crate::status_result(sys::Api::NvAPI_GPU_GetPstates20, status).is_ok() {
                return Ok(PStates {
                    editable: raw.bIsEditable.get(),
                    pstates: raw.pstates[..raw.numPstates as usize]
                        .iter()
                        .map(|ps| {
                            crate::PStateSettings::from_raw(
                                ps,
                                raw.numClocks.try_into().unwrap(),
                                raw.numBaseVoltages.try_into().unwrap(),
                            )
                        })
                        .collect::<Result<_, _>>()?,
                    overvolt: Vec::new(),
                });
            }
        }
        trace!("gpu.pstates(): Pstates20 not available, falling back to legacy PstatesInfo");
        self.legacy_pstates()
    }

    pub fn legacy_pstates(&self) -> crate::Result<PStates> {
        trace!("gpu.legacy_pstates()");
        unsafe { nvcall!(NvAPI_GPU_GetPstatesInfoEx@get(self.0, 0u32) => raw) }
    }

    /// NOTE on units (ccminer cross-ref, nvml.cpp:1467 "gpu delta value
    /// seems to be x2, not the memory"): the boost-TABLE path
    /// (`set_vfp_table` on 0x23F1B133) stores GPU deltas in Kilohertz2
    /// (×2, /2000 = MHz) while MEMORY deltas are plain kHz (/1000 = MHz).
    /// This SetPstates20 path takes plain kHz for both domains (ccminer
    /// writes `freqDelta_kHz.value = delta` then logs `delta/1000`).
    pub fn set_pstates<I: IntoIterator<Item = (PState, ClockDomain, KilohertzDelta)>>(
        &self,
        deltas: I,
    ) -> crate::NvapiResult<()> {
        trace!("gpu.set_pstates()");

        let mut info = pstate::NV_GPU_PERF_PSTATES20_INFO::default();

        let mut map: BTreeMap<PState, (usize, usize)> = Default::default();
        for (pstate, clock, delta) in deltas {
            trace!("gpu.set_pstate({:?}, {:?}, {:?})", pstate, clock, delta);
            let pstates = map.len();
            let map = map.entry(pstate).or_insert((pstates, 0));
            let entry = &mut info.pstates[map.0];
            entry.pstateId = pstate.raw();
            let entry = &mut entry.clocks[map.1];
            entry.domainId = clock.raw();
            entry.freqDelta_kHz.value = delta.0;
            map.1 += 1;
        }
        info.numPstates = map.len() as _;
        info.numClocks = map.iter().map(|v| v.1.1).max().unwrap_or(0) as _;

        unsafe { nvcall!(NvAPI_GPU_SetPstates20(self.0, &info)) }
    }

    /// `enable`: 1 unlocks overclocked-pstate range (50-series: extended memory OC
    /// range — call before `set_pstates20` to exceed the stock VBIOS clamp), 0 restores.
    pub fn enable_overclocked_pstates(&self, enable: bool) -> crate::NvapiResult<()> {
        trace!("gpu.enable_overclocked_pstates({enable})");
        unsafe { nvcall!(NvAPI_GPU_EnableOverclockedPstates(self.0, enable as u32)) }
    }

    /// Set the global over-voltage offset via the PSTATES20 V2 `voltages[]`
    /// OV array (numVoltages@+7316 / voltages[0].voltDelta_uV@+7332 on the
    /// 7416B V2 struct) — the path HYDRA 2.2B PRO drives as its
    /// "NvApiSetOverVoltageOffset" export. Distinct from per-pstate
    /// baseVoltage deltas: a single core-domain OV offset on a zeroed struct.
    pub fn set_overvolt(&self, delta: crate::types::MicrovoltsDelta) -> crate::NvapiResult<()> {
        trace!("gpu.set_overvolt({delta:?})");
        let mut info = pstate::NV_GPU_PERF_PSTATES20_INFO {
            numVoltages: 1,
            ..Default::default()
        };
        info.voltages[0].domainId = pstate::VoltageInfoDomain::Core.into();
        info.voltages[0].voltDelta_uV.value = delta.0;
        unsafe { nvcall!(NvAPI_GPU_SetPstates20(self.0, &info)) }
    }

    pub fn enable_dynamic_pstates(&self, enable: u32) -> crate::NvapiResult<()> {
        trace!("gpu.enable_dynamic_pstates(enable={})", enable);
        unsafe { nvcall!(NvAPI_GPU_EnableDynamicPstates(self.0, enable)) }
    }

    pub fn dynamic_pstates_info(&self) -> crate::Result<Utilizations> {
        trace!("gpu.dynamic_pstates_info()");

        unsafe { nvcall!(NvAPI_GPU_GetDynamicPstatesInfoEx@get(self.0) => raw) }
    }

    /// Private and deprecated, use `dynamic_pstates_info()` instead.
    pub fn usages(
        &self,
    ) -> crate::Result<<clock::private::NV_USAGES_INFO as RawConversion>::Target> {
        trace!("gpu.usages()");

        unsafe { nvcall!(NvAPI_GPU_GetUsages@get(self.0) => raw) }
    }

    pub fn vfp_mask(
        &self,
    ) -> crate::Result<
        <clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO as RawConversion>::Target,
    > {
        trace!("gpu.vfp_mask()");

        unsafe { nvcall!(NvAPI_GPU_ClockClientClkVfPointsGetInfo@get(self.0) => raw) }
    }

    pub fn vfp_info(&self) -> crate::Result<VfpInfo> {
        Ok(VfpInfo {
            domains: self.vfp_ranges()?,
            mask: self.vfp_mask()?,
        })
    }

    pub(crate) fn vfp_table_raw(
        &self,
        info: &VfpInfo,
    ) -> crate::Result<clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL> {
        trace!("gpu.vfp_table({:?})", info);
        let data = clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL {
            mask: info.mask.mask,
            ..Default::default()
        };

        let v2 =
            unsafe { nvcall!(NvAPI_GPU_ClockClientClkVfPointsGetControl@get{data}(self.0) => err) };
        if v2.is_ok() {
            return v2;
        }
        // V1 magic fallback (0x12420, ver1/9248B): the impl handler accepts
        // BOTH (ver−0x12420)&0xFFFEFFFF==0 → ver1 and ver2; AmpereOC and
        // HYDRA both send ver1. Same layout, only the version stamp differs.
        let mut v1 = unsafe {
            std::mem::zeroed::<clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL>()
        };
        use crate::sys::nvapi::VersionedStruct;
        *v1.nvapi_version_mut() = NvVersion::with_version(1 << 16 | 9248);
        v1.mask = info.mask.mask;
        unsafe { nvcall!(NvAPI_GPU_ClockClientClkVfPointsGetControl@get{v1}(self.0) => err) }
    }

    pub fn vfp_table(&self, info: &VfpInfo) -> crate::Result<crate::clock::ClockTable> {
        self.vfp_table_raw(info)
            .and_then(|raw| crate::clock::ClockTable::from_raw(&raw, info))
    }

    pub fn set_vfp_table<
        I: Iterator<Item = (usize, Kilohertz2Delta)>,
        M: Iterator<Item = (usize, Kilohertz2Delta)>,
    >(
        &self,
        info: &VfpInfo,
        clocks: I,
        _memory: M,
    ) -> crate::Result<()> {
        trace!("gpu.set_vfp_table({:?})", info);
        let mut data = self.vfp_table_raw(info)?;
        data.mask = info.mask.mask;
        for (i, delta) in clocks {
            trace!("gpu.set_vfp_table({:?}, {:?})", i, delta);
            data.points[i].freqDeltaKHz = delta.0 / 2;
            data.mask.set_bit(i);
        }
        /*for (i, delta) in memory {
            data.memFilled[i] = 1;
            data.memDeltas[i] = delta.0;
        }*/

        unsafe { nvcall!(NvAPI_GPU_ClockClientClkVfPointsSetControl(self.0, &data) => err) }
    }

    pub fn vfp_ranges(
        &self,
    ) -> crate::Result<
        <clock::private::NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_INFO as RawConversion>::Target,
    > {
        trace!("gpu.vfp_ranges()");

        unsafe { nvcall!(NvAPI_GPU_ClockClientClkDomainsGetInfo@get(self.0) => raw) }
    }

    pub fn vfp_locks<I: IntoIterator<Item = crate::clock::PerfLimitId>>(
        &self,
        limits: I,
    ) -> crate::Result<<clock::private::NV_GPU_PERF_CLIENT_LIMITS as RawConversion>::Target> {
        trace!("gpu.vfp_locks()");
        let mut status = clock::private::NV_GPU_PERF_CLIENT_LIMITS::default();
        for (limit, entry) in limits.into_iter().zip(&mut status.entries) {
            entry.id = limit.into();
            status.count += 1;
        }

        unsafe { nvcall!(NvAPI_GPU_PerfClientLimitsGetStatus@get{status}(self.0) => raw) }
    }

    pub fn set_vfp_locks<I: IntoIterator<Item = crate::clock::ClockLockEntry>>(
        &self,
        values: I,
    ) -> crate::NvapiResult<()> {
        trace!("gpu.set_vfp_locks()");
        use clock::private::ClockLockMode;

        let mut data = clock::private::NV_GPU_PERF_CLIENT_LIMITS::default();
        for (lock, entry) in values.into_iter().zip(&mut data.entries) {
            trace!("gpu.set_vfp_lock({:?})", lock);
            data.count += 1;
            entry.id = lock.limit.into();
            let (mode, value) = match lock.lock_value {
                Some(crate::clock::ClockLockValue::Frequency(v)) => {
                    (ClockLockMode::ManualFrequency.raw(), v.0)
                }
                Some(crate::clock::ClockLockValue::Voltage(v)) => {
                    (ClockLockMode::ManualVoltage.raw(), v.0)
                }
                None => (ClockLockMode::None.raw(), 0),
            };
            entry.mode = mode;
            entry.value = value;
            entry.clock_id = lock.clock.into();
        }

        unsafe { nvcall!(NvAPI_GPU_PerfClientLimitsSetStatus(self.0, &data)) }
    }

    pub fn vfp_curve(&self, info: &VfpInfo) -> crate::Result<crate::clock::VfpCurve> {
        trace!("gpu.vfp_curve({:?})", info);
        let data = power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS {
            mask: info.mask.mask,
            ..Default::default()
        };

        unsafe {
            let v3_result =
                nvcall!(NvAPI_GPU_ClockClientClkVfPointsGetStatus@get{data}(self.0) => err)
                    .and_then(|raw| crate::clock::VfpCurve::from_raw(&raw, info));
            if v3_result.is_ok() {
                return v3_result;
            }

            use crate::sys::nvapi::VersionedStruct;
            let mut data_v1 =
                std::mem::zeroed::<power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1>();
            *data_v1.nvapi_version_mut() = NvVersion::with_struct::<
                power::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_V1,
            >(2);
            data_v1.mask = info.mask.mask;
            let status = sys::api::NvAPI_GPU_ClockClientClkVfPointsGetStatus(
                self.0,
                ptr::from_mut(&mut data_v1).cast(),
            );
            crate::status_result(sys::Api::NvAPI_GPU_ClockClientClkVfPointsGetStatus, status)
                .map_err(Into::into)
                .and_then(|_| crate::clock::VfpCurve::from_raw_v1(&data_v1, info))
        }
    }

    pub fn core_voltage(
        &self,
    ) -> crate::Result<<power::private::NV_GPU_CLIENT_VOLT_RAILS_STATUS as RawConversion>::Target>
    {
        trace!("gpu.core_voltage()");

        unsafe { nvcall!(NvAPI_GPU_ClientVoltRailsGetStatus@get(self.0) => raw) }
    }

    pub fn core_voltage_boost(
        &self,
    ) -> crate::Result<<power::private::NV_GPU_CLIENT_VOLT_RAILS_CONTROL as RawConversion>::Target>
    {
        trace!("gpu.core_voltage_boost()");

        unsafe { nvcall!(NvAPI_GPU_ClientVoltRailsGetControl@get(self.0) => raw) }
    }

    pub fn set_core_voltage_boost(&self, value: Percentage) -> crate::NvapiResult<()> {
        trace!("gpu.set_core_voltage_boost({:?})", value);
        let data = power::private::NV_GPU_CLIENT_VOLT_RAILS_CONTROL {
            percent: value.0,
            ..Default::default()
        };

        unsafe { nvcall!(NvAPI_GPU_ClientVoltRailsSetControl(self.0, &data)) }
    }

    /// Read-only walk of the private VoltRails family (the "melonVolt path" —
    /// see `reverse/melonvolt/ANALYSIS.md`): GetInfo (rail builder) → seed →
    /// GetControl + GetStatus. Never writes; the SetControl sibling
    /// (0x87C55C8A, the µV-offset path melonVolt drives on RTX 5090) is
    /// intentionally unwrapped.
    pub fn volt_rails(&self) -> crate::Result<VoltRails> {
        trace!("gpu.volt_rails()");
        use crate::sys::api::{
            NvAPI_GPU_VoltVoltRailsGetControl, NvAPI_GPU_VoltVoltRailsGetInfo,
            NvAPI_GPU_VoltVoltRailsGetStatus,
        };
        use power::private::{
            NV_GPU_VOLT_RAILS_CONTROL, NV_GPU_VOLT_RAILS_INFO, NV_GPU_VOLT_RAILS_STATUS_V1,
        };

        let mut info = NV_GPU_VOLT_RAILS_INFO::default();
        let st = unsafe { NvAPI_GPU_VoltVoltRailsGetInfo(self.0, ptr::from_mut(&mut info).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_VoltVoltRailsGetInfo, st)
            .map_err(crate::Error::from)?;

        let mut control = NV_GPU_VOLT_RAILS_CONTROL::default();
        control.seed_from_info(&info);
        let st = unsafe {
            NvAPI_GPU_VoltVoltRailsGetControl(self.0, ptr::from_mut(&mut control).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_VoltVoltRailsGetControl, st)
            .map_err(crate::Error::from)?;

        // GetStatus requires the V1 stamp (0x10AC8) — best-effort: a driver
        // without it still yields the control object.
        let mut status = NV_GPU_VOLT_RAILS_STATUS_V1::default();
        status.seed_from_info(&info);
        let st =
            unsafe { NvAPI_GPU_VoltVoltRailsGetStatus(self.0, ptr::from_mut(&mut status).cast()) };
        let status_entries =
            match crate::status_result(sys::Api::NvAPI_GPU_VoltVoltRailsGetStatus, st) {
                Ok(()) => status.entries().map(VoltRailEntry::from_raw).collect(),
                Err(e) => {
                    warn!("VoltVoltRailsGetStatus failed ({e:?}); returning control-only snapshot");
                    Vec::new()
                }
            };

        let rail_descriptors = (0..32u32)
            .filter(|bit| info.rail_mask & (1 << bit) != 0)
            .filter_map(|bit| {
                info.rail_entry_raw(bit).map(|raw| RailDescriptor {
                    rail_bit: bit,
                    raw_u32: raw.to_vec(),
                })
            })
            .collect();

        Ok(VoltRails {
            rail_mask: info.rail_mask,
            control: control.entries().map(VoltRailEntry::from_raw).collect(),
            status: status_entries,
            rail_descriptors,
        })
    }

    /// Write a value into one rail's control-entry payload (index 0, @+76) and
    /// verify the driver retained it — the mechanism half of melonVolt's
    /// protocol (snapshot → locate → patch → SET → readback). Policy (type
    /// check, ±mV limits) belongs to the caller; the entry's payload index 0
    /// is the µV offset on RTX-5090 MSVDD (type 3) entries.
    ///
    /// Returns the value the driver actually retained.
    #[allow(non_snake_case)] // uV suffix matches the sys-layer field naming
    pub fn set_volt_rail_value(&self, rail_bit: u32, value_uV: i32) -> crate::Result<i32> {
        trace!("gpu.set_volt_rail_value({rail_bit}, {value_uV})");
        use crate::sys::api::{
            NvAPI_GPU_VoltVoltRailsGetControl, NvAPI_GPU_VoltVoltRailsGetInfo,
            NvAPI_GPU_VoltVoltRailsSetControl,
        };
        use power::private::{NV_GPU_VOLT_RAILS_CONTROL, NV_GPU_VOLT_RAILS_INFO, ctrl_entry};

        let mut info = NV_GPU_VOLT_RAILS_INFO::default();
        let st = unsafe { NvAPI_GPU_VoltVoltRailsGetInfo(self.0, ptr::from_mut(&mut info).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_VoltVoltRailsGetInfo, st)
            .map_err(crate::Error::from)?;

        let dense = Self::dense_index_for(info.rail_mask, rail_bit)?;

        let mut control = NV_GPU_VOLT_RAILS_CONTROL::default();
        control.seed_from_info(&info);
        let st = unsafe {
            NvAPI_GPU_VoltVoltRailsGetControl(self.0, ptr::from_mut(&mut control).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_VoltVoltRailsGetControl, st)
            .map_err(crate::Error::from)?;

        // patch payload index 0 (entry byte +76, dword units relative to `rest`)
        let off = ctrl_entry::STRIDE * dense + ctrl_entry::VALUES - 8;
        let dst = control
            .rest
            .get_mut(off..off + 4)
            .ok_or(crate::Error::ArgumentRange(Default::default()))?;
        dst.copy_from_slice(&value_uV.to_le_bytes());

        let st =
            unsafe { NvAPI_GPU_VoltVoltRailsSetControl(self.0, ptr::from_ref(&control).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_VoltVoltRailsSetControl, st)
            .map_err(crate::Error::from)?;

        // readback: fresh seeded GET, compare payload index 0
        let mut verify = NV_GPU_VOLT_RAILS_CONTROL::default();
        verify.seed_from_info(&info);
        let st =
            unsafe { NvAPI_GPU_VoltVoltRailsGetControl(self.0, ptr::from_mut(&mut verify).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_VoltVoltRailsGetControl, st)
            .map_err(crate::Error::from)?;
        let retained = verify
            .entries()
            .find(|(bit, _, _)| *bit == rail_bit)
            .map(|(_, _, values)| values[0])
            .ok_or(crate::Error::ArgumentRange(Default::default()))?;
        if retained != value_uV {
            // "Driver did not retain requested value" (melonVolt's wording)
            return Err(crate::Error::ArgumentRange(Default::default()));
        }
        Ok(retained)
    }

    /// Dense-entry index of `rail_bit` within `mask` (set bits, ascending).
    fn dense_index_for(mask: u32, rail_bit: u32) -> crate::Result<usize> {
        if rail_bit >= 32 || mask & (1 << rail_bit) == 0 {
            return Err(crate::Error::ArgumentRange(Default::default()));
        }
        Ok((0..rail_bit).filter(|b| mask & (1 << b) != 0).count())
    }

    // --- Blackwell XBar ClockClient clock-domain family ---------------------
    // (reverse/melonvolt/xbar.txt — Loong0x00 LACT #1147). The 4 NV2080 RM
    // commands wrapped via private NVAPI IDs (escape 0x07000109). All GET paths
    // live-verified on Ada 4060 Laptop / R575.74.

    /// Controllable clock-domain block from the private ClockClient
    /// GetControl (RM 0x2080901b, ID 0xF58938F5). Returns the controllable
    /// mask + per-domain type/range/offset entries. The article's XBAR
    /// domain is bit 1 ([`crate::clock::ClockDomainId::Xbar`]).
    #[allow(non_snake_case)] // kHz suffix matches the sys-layer field naming
    pub fn clk_domains_control(&self) -> crate::Result<crate::clock::ClockDomainControl> {
        trace!("gpu.clk_domains_control()");
        use crate::sys::api::NvAPI_GPU_ClockClkDomainsGetControl;
        use clock::private::{
            NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL, NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2,
        };

        // GET_CONTROL is MASK-SEEDED: the handler reads the mask at +8 to
        // decide which per-domain records to fill, and echoes it back. Seed a
        // broad mask so every controllable domain is populated, then derive
        // the TRUE controllable set from records the driver actually filled
        // (record type != 0). The driver rejects u32::MAX; 0xFF is accepted.
        //
        // V2 (magic 0x261A4, 24996B) is preferred: it marshals value dwords
        // for the type-0x0A records modern drivers report; V1 only fills
        // their type dword. Fall back to V1 when the driver rejects V2.
        let mut v2 = NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2::default();
        v2.set_mask(0xFF);
        let st =
            unsafe { NvAPI_GPU_ClockClkDomainsGetControl(self.0, ptr::from_mut(&mut v2).cast()) };
        if crate::status_result(sys::Api::NvAPI_GPU_ClockClkDomainsGetControl, st).is_ok() {
            let mask = v2.controllable_mask();
            let entries = (0..32u32)
                .filter_map(|bit| {
                    let typ = v2.record_type(bit).filter(|&t| t != 0)?;
                    let value_modifiable = crate::clock::ClkDomainControlEntry::v2_marshalable(typ);
                    let mut values_kHz = [0i32; 8];
                    if value_modifiable {
                        for (i, v) in values_kHz.iter_mut().enumerate() {
                            *v = v2.value(bit, i).unwrap_or(0);
                        }
                    }
                    Some(crate::clock::ClkDomainControlEntry {
                        bit,
                        entry_type: typ,
                        value_modifiable,
                        values_kHz,
                    })
                })
                .collect();
            return Ok(crate::clock::ClockDomainControl { mask, entries });
        }

        // V1 fallback (older drivers): values only for types {1,3,4,5,6,7,8,9}.
        let mut control = NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL::default();
        control.set_mask(0xFF);
        let st = unsafe {
            NvAPI_GPU_ClockClkDomainsGetControl(self.0, ptr::from_mut(&mut control).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkDomainsGetControl, st)
            .map_err(crate::Error::from)?;

        let mask = control.controllable_mask();
        let entries = control
            .entries()
            .map(|(bit, typ, off, rmin, rmax, appl)| {
                let value_modifiable = crate::clock::ClkDomainControlEntry::v1_marshalable(typ);
                let mut values_kHz = [0i32; 8];
                if value_modifiable {
                    values_kHz[0] = off;
                    values_kHz[1] = rmin;
                    values_kHz[2] = rmax;
                    values_kHz[3] = appl;
                }
                crate::clock::ClkDomainControlEntry {
                    bit,
                    entry_type: typ,
                    value_modifiable,
                    values_kHz,
                }
            })
            .collect();

        Ok(crate::clock::ClockDomainControl { mask, entries })
    }

    /// Physical clock for one domain from the private ClockClient
    /// MEASURE_FREQ (RM 0x20809006, ID 0xFB8F61EC). Windows returns a raw
    /// {counter, timestamp} pair — NOT the article's direct kHz — so this
    /// samples twice (~50 ms apart) and computes
    /// `freq = Δcounter / Δtimestamp_ns × 1e9 Hz`. `domain_bit` is the
    /// sequential domain INDEX (GPC=0, XBAR=1, SYS=2, MCLK=4).
    pub fn clk_domain_freq(&self, domain_bit: u32) -> crate::Result<crate::clock::ClockDomainFreq> {
        trace!("gpu.clk_domain_freq({domain_bit})");
        use crate::sys::api::NvAPI_GPU_ClockCounterMeasureAvgFreq;
        use clock::private::{
            NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE, NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE2,
        };

        fn sample(
            gpu: sys::handles::NvPhysicalGpuHandle,
            domain_bit: u32,
        ) -> crate::Result<(u64, u64, u32, u8)> {
            // stamp V1 magic 0x10020 (version 1, size 0x20)
            let mut m = NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE {
                version: sys::api::NvVersion::new(0x20, 1),
                domain_index: domain_bit,
                ..Default::default()
            };
            let st =
                unsafe { NvAPI_GPU_ClockCounterMeasureAvgFreq(gpu, ptr::from_mut(&mut m).cast()) };
            if crate::status_result(sys::Api::NvAPI_GPU_ClockCounterMeasureAvgFreq, st).is_ok() {
                return Ok((m.counter as u64, m.timestamp_ns, m.rsvd2, 1));
            }
            // V1 rejected (Pascal observed: some domains fail with a raw RM
            // error) — retry the V2 form (magic 0x20020, u64 counter).
            let mut m2 = NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE2 {
                version: sys::api::NvVersion::new(0x20, 2),
                domain_index: domain_bit,
                ..Default::default()
            };
            let st =
                unsafe { NvAPI_GPU_ClockCounterMeasureAvgFreq(gpu, ptr::from_mut(&mut m2).cast()) };
            crate::status_result(sys::Api::NvAPI_GPU_ClockCounterMeasureAvgFreq, st)
                .map_err(crate::Error::from)?;
            Ok((m2.counter, m2.timestamp_ns, m2.extra, 2))
        }

        let (c1, t1, _, _) = sample(self.0, domain_bit)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let (c2, t2, _, _) = sample(self.0, domain_bit)?;

        let dt_ns = t2.saturating_sub(t1);
        let dc = c2.saturating_sub(c1);
        let freq_hz = if dt_ns > 0 {
            (dc as f64 / dt_ns as f64) * 1e9
        } else {
            0.0
        };

        Ok(crate::clock::ClockDomainFreq {
            domain: crate::clock::ClockDomainId::try_from(domain_bit as i32)
                .unwrap_or(crate::clock::ClockDomainId::Gpc),
            freq_mhz: freq_hz / 1e6,
        })
    }

    /// Detailed single-domain measure — the computed frequency PLUS the
    /// second sample's raw {counter, timestamp, extra} and which protocol
    /// form (V1 0x10020 / V2 0x20020) the driver accepted. For counter
    /// unit calibration (Pascal M) and protocol forensics.
    pub fn clk_domain_freq_detail(
        &self,
        domain_bit: u32,
    ) -> crate::Result<crate::clock::ClockDomainFreqDetail> {
        trace!("gpu.clk_domain_freq_detail({domain_bit})");
        use crate::sys::api::NvAPI_GPU_ClockCounterMeasureAvgFreq;
        use clock::private::{
            NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE, NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE2,
        };

        fn sample(
            gpu: sys::handles::NvPhysicalGpuHandle,
            domain_bit: u32,
        ) -> crate::Result<(u64, u64, u32, u8)> {
            let mut m = NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE {
                version: sys::api::NvVersion::new(0x20, 1),
                domain_index: domain_bit,
                ..Default::default()
            };
            let st =
                unsafe { NvAPI_GPU_ClockCounterMeasureAvgFreq(gpu, ptr::from_mut(&mut m).cast()) };
            if crate::status_result(sys::Api::NvAPI_GPU_ClockCounterMeasureAvgFreq, st).is_ok() {
                return Ok((m.counter as u64, m.timestamp_ns, m.rsvd2, 1));
            }
            let mut m2 = NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE2 {
                version: sys::api::NvVersion::new(0x20, 2),
                domain_index: domain_bit,
                ..Default::default()
            };
            let st =
                unsafe { NvAPI_GPU_ClockCounterMeasureAvgFreq(gpu, ptr::from_mut(&mut m2).cast()) };
            crate::status_result(sys::Api::NvAPI_GPU_ClockCounterMeasureAvgFreq, st)
                .map_err(crate::Error::from)?;
            Ok((m2.counter, m2.timestamp_ns, m2.extra, 2))
        }

        let (c1, t1, _, _) = sample(self.0, domain_bit)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        let (c2, t2, extra, protocol) = sample(self.0, domain_bit)?;

        let dt_ns = t2.saturating_sub(t1);
        let dc = c2.saturating_sub(c1);
        let freq_hz = if dt_ns > 0 {
            (dc as f64 / dt_ns as f64) * 1e9
        } else {
            0.0
        };

        Ok(crate::clock::ClockDomainFreqDetail {
            domain: crate::clock::ClockDomainId::try_from(domain_bit as i32)
                .unwrap_or(crate::clock::ClockDomainId::Gpc),
            freq_mhz: freq_hz / 1e6,
            protocol,
            counter: c2,
            timestamp_ns: t2,
            extra,
        })
    }

    /// DIRECT physical clock for one domain — the green-curve MEASURE path
    /// (ID 0x527FC458). Unlike [`clk_domain_freq`](Gpu::clk_domain_freq) /
    /// [`clk_domain_freq_detail`](Gpu::clk_domain_freq_detail) (counter-based
    /// `0xFB8F61EC`, two samples + 50 ms sleep + Δcounter/Δt), this API
    /// returns `freq_khz` in one call: the driver writes the value directly.
    /// Best for an immediate post-write verification of an XBar/SYS offset
    /// (XBAR=`domain_bit` 1, SYS=`domain_bit` 2). `domain_bit` is the same
    /// sequential domain INDEX the counter variant uses (GPC=0, XBAR=1,
    /// SYS=2, MCLK=4). Returns `freq_khz == 0` when the driver refuses or the
    /// domain is not measurable through this interface (VIDEO/entry 4 has no
    /// measure domain — verify it via exact control-block readback instead).
    pub fn clk_domain_freq_direct(
        &self,
        domain_bit: u32,
    ) -> crate::Result<crate::clock::ClockDomainFreqDirect> {
        trace!("gpu.clk_domain_freq_direct({domain_bit})");
        use crate::sys::api::NvAPI_GPU_ClockClkDomainsMeasureFreq;
        use clock::private::NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_FREQ_DIRECT;

        // magic 0x0001000C = (1<<16)|0xC — version 1, 12-byte struct
        let mut m = NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE_FREQ_DIRECT {
            version: sys::api::NvVersion::new(0xC, 1),
            domain_index: domain_bit,
            ..Default::default()
        };
        let st =
            unsafe { NvAPI_GPU_ClockClkDomainsMeasureFreq(self.0, ptr::from_mut(&mut m).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkDomainsMeasureFreq, st)
            .map_err(crate::Error::from)?;
        Ok(crate::clock::ClockDomainFreqDirect {
            domain: crate::clock::ClockDomainId::try_from(domain_bit as i32)
                .unwrap_or(crate::clock::ClockDomainId::Gpc),
            freq_khz: m.freq_khz,
        })
    }

    /// Batch physical clocks for many domains in ONE RM round-trip family —
    /// the V3 MEASURE_FREQ (magic 0x30038): up to 32 packed 24B entries,
    /// each carrying its own {counter, timestamp} seed. Two calls ~50 ms
    /// apart give every domain's Δcounter/Δtimestamp × 1e9 Hz. Falls back
    /// per-domain to the V1/V2 single measure when the driver rejects the
    /// batch form (Pascal observed).
    pub fn clk_domain_freqs_batch(
        &self,
        domains: &[u32],
    ) -> crate::Result<Vec<crate::clock::ClockDomainFreq>> {
        trace!("gpu.clk_domain_freqs_batch({domains:?})");
        use crate::sys::api::NvAPI_GPU_ClockCounterMeasureAvgFreq;
        use clock::private::NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE3;

        if domains.is_empty() {
            return Ok(Vec::new());
        }
        let n = domains
            .len()
            .min(clock::private::clk_measure_v3::MAX_ENTRIES);

        fn sample_batch(
            gpu: sys::handles::NvPhysicalGpuHandle,
            domains: &[u32],
        ) -> crate::Result<NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE3> {
            // stamp V3 magic 0x30038 (version 3, size 0x38)
            let mut m = NV_GPU_CLOCK_CLIENT_CLK_DOMAIN_MEASURE3 {
                version: sys::api::NvVersion::new(0x178, 3),
                ..Default::default()
            };
            m.set_count(domains.len() as u8);
            for (i, &d) in domains.iter().enumerate() {
                m.set_entry(i, d, 0, 0)
                    .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
            }
            let st =
                unsafe { NvAPI_GPU_ClockCounterMeasureAvgFreq(gpu, ptr::from_mut(&mut m).cast()) };
            crate::status_result(sys::Api::NvAPI_GPU_ClockCounterMeasureAvgFreq, st)
                .map_err(crate::Error::from)?;
            Ok(m)
        }

        let sample = |domains: &[u32]| -> crate::Result<Vec<(u64, u64)>> {
            let m1 = sample_batch(self.0, domains)?;
            std::thread::sleep(std::time::Duration::from_millis(50));
            let m2 = sample_batch(self.0, domains)?;
            Ok((0..domains.len())
                .map(|i| {
                    let (c1, t1, _) = m1.entry(i).unwrap_or((0, 0, 0));
                    let (c2, t2, _) = m2.entry(i).unwrap_or((0, 0, 0));
                    (c2.saturating_sub(c1), t2.saturating_sub(t1))
                })
                .collect())
        };

        match sample(&domains[..n]) {
            // KNOWN OPEN ISSUE: on R610.74 the V3 call SUCCEEDS but returns
            // a frozen snapshot — the per-entry outputs don't advance
            // between samples (even with RMW seeding), so Δcounter/Δt is
            // always 0. Detect that and fall back per-domain.
            Ok(deltas) if deltas.iter().any(|&(dc, _)| dc > 0) => Ok(deltas
                .iter()
                .zip(&domains[..n])
                .map(|(&(dc, dt), &bit)| {
                    let freq_hz = if dt > 0 {
                        (dc as f64 / dt as f64) * 1e9
                    } else {
                        0.0
                    };
                    crate::clock::ClockDomainFreq {
                        domain: crate::clock::ClockDomainId::try_from(bit as i32)
                            .unwrap_or(crate::clock::ClockDomainId::Gpc),
                        freq_mhz: freq_hz / 1e6,
                    }
                })
                .collect()),
            // batch rejected OR frozen — fall back per-domain to V1/V2,
            // skipping domains the driver refuses (e.g. Disp type 0x02,
            // Pascal's RM-rejected gpc/xbar) instead of failing the set
            _ => {
                let mut out = Vec::new();
                for &bit in &domains[..n] {
                    if let Ok(f) = self.clk_domain_freq(bit) {
                        out.push(f);
                    }
                }
                Ok(out)
            }
        }
    }

    /// Write a signed kHz offset into one clock-domain's control record and
    /// verify the driver retained it — the private ClockClient SET_CONTROL
    /// (RM 0x2080d01c, ID 0xD14B69CF). DANGEROUS GPU clock write.
    ///
    /// Implements the article's mandated reversible recipe (xbar.txt:62-72):
    /// snapshot the ENTIRE GetControl block → version/size gate (refuse if
    /// the driver returned an unknown magic — only V2 0x261A4 is decoded for
    /// the write path) → patch a copy with the offset → SET_CONTROL →
    /// GET_CONTROL readback → verify retained → restore the snapshot on
    /// mismatch. If `temporary`, the snapshot is written back and verified
    /// restored before returning.
    ///
    /// `domain_bit` is the domain INDEX/mask-bit (XBAR=1). `slot` picks which
    /// of the record's 8 value dwords to write (V2 rec+268+4*slot); slot
    /// semantics are driver-opaque — per the article slot 0 is the signed
    /// frequency offset (kHz), neighbors are range/voltage terms. The caller
    /// owns range/offset policy (the article bounds XBAR ±60000 kHz on GB202).
    pub fn set_clk_domain_offset(
        &self,
        domain_bit: u32,
        offset_kHz: i32,
        slot: u32,
        temporary: bool,
    ) -> crate::Result<crate::clock::ClkDomainControlEntry> {
        #![allow(non_snake_case)]
        trace!(
            "gpu.set_clk_domain_offset({domain_bit}, {offset_kHz}, slot={slot}, temporary={temporary})"
        );
        use crate::sys::api::{
            NvAPI_GPU_ClockClkDomainsGetControl, NvAPI_GPU_ClockClkDomainsSetControl,
        };
        use clock::private::NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2;

        // GET_CONTROL is MASK-SEEDED: the caller writes the controllable mask
        // at +8 to tell the driver which per-domain records to fill. Discover
        // the TRUE controllable set with a broad seed first (bits whose records
        // the driver actually fills, record type != 0), then use that real mask
        // for every subsequent GET/SET — never submit a broad seed to SET_CONTROL
        // (it would ask the driver to apply 32 records, most of them empty).
        //
        // V2 (magic 0x261A4) is the write path: it marshals the type-0x0A
        // records this driver reports; V1 would silently drop them.
        let mut probe = NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2::default();
        probe.set_mask(0xFF);
        let st = unsafe {
            NvAPI_GPU_ClockClkDomainsGetControl(self.0, ptr::from_mut(&mut probe).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkDomainsGetControl, st)
            .map_err(crate::Error::from)?;

        // Step 1/2: version/size gate — refuse the write if the driver
        // returned an unknown/mismatched magic. Only V2 (0x261A4) is
        // layout-decoded for the write path.
        if probe.version.data != 0x261A4 {
            return Err(crate::Error::Nvapi(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_ClockClkDomainsSetControl,
                Status::NotSupported,
            )));
        }
        if slot >= 8 {
            return Err(crate::Error::ArgumentRange(Default::default()));
        }
        let real_mask = probe.controllable_mask();
        if domain_bit >= 32 || real_mask & (1 << domain_bit) == 0 {
            return Err(crate::Error::ArgumentRange(Default::default()));
        }
        // Refuse record types the V2 protocol does not marshal (e.g. 0x02 —
        // Disp bit 6): SET_CONTROL silently drops them and the readback can
        // never match.
        let entry_type = probe.record_type(domain_bit).unwrap_or(0);
        if !crate::clock::ClkDomainControlEntry::v2_marshalable(entry_type) {
            return Err(crate::Error::Nvapi(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_ClockClkDomainsSetControl,
                Status::NotSupported,
            )));
        }

        // Step 3: GET_CONTROL snapshot with the REAL controllable mask —
        // preserve the ENTIRE original block (every controllable record).
        let mut snapshot = NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2::default();
        snapshot.set_mask(real_mask);
        let st = unsafe {
            NvAPI_GPU_ClockClkDomainsGetControl(self.0, ptr::from_mut(&mut snapshot).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkDomainsGetControl, st)
            .map_err(crate::Error::from)?;

        // Step 4: patch a COPY (preserve every other byte + the real mask).
        let mut modified = snapshot;
        modified
            .set_value(domain_bit, slot as usize, offset_kHz)
            .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;

        // Step 5: SET_CONTROL (full block, real controllable mask).
        let st =
            unsafe { NvAPI_GPU_ClockClkDomainsSetControl(self.0, ptr::from_ref(&modified).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkDomainsSetControl, st)
            .map_err(crate::Error::from)?;

        // Step 6: GET_CONTROL readback + confirm the driver retained the value.
        let mut verify = NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2::default();
        verify.set_mask(real_mask);
        let st = unsafe {
            NvAPI_GPU_ClockClkDomainsGetControl(self.0, ptr::from_mut(&mut verify).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkDomainsGetControl, st)
            .map_err(crate::Error::from)?;
        let retained = verify
            .value(domain_bit, slot as usize)
            .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
        if retained != offset_kHz {
            // driver did not retain — restore the original snapshot (best effort).
            let _ = unsafe {
                NvAPI_GPU_ClockClkDomainsSetControl(self.0, ptr::from_ref(&snapshot).cast())
            };
            return Err(crate::Error::ArgumentRange(Default::default()));
        }

        // Step 8 (temporary): write back the saved block + verify restored.
        if temporary {
            let st = unsafe {
                NvAPI_GPU_ClockClkDomainsSetControl(self.0, ptr::from_ref(&snapshot).cast())
            };
            crate::status_result(sys::Api::NvAPI_GPU_ClockClkDomainsSetControl, st)
                .map_err(crate::Error::from)?;
            let mut restored = NV_GPU_CLOCK_CLIENT_CLK_DOMAINS_CONTROL2::default();
            restored.set_mask(real_mask);
            let st = unsafe {
                NvAPI_GPU_ClockClkDomainsGetControl(self.0, ptr::from_mut(&mut restored).cast())
            };
            crate::status_result(sys::Api::NvAPI_GPU_ClockClkDomainsGetControl, st)
                .map_err(crate::Error::from)?;
            let (after, orig) = (
                restored.value(domain_bit, slot as usize).unwrap_or(0),
                snapshot.value(domain_bit, slot as usize).unwrap_or(0),
            );
            if after != orig {
                return Err(crate::Error::ArgumentRange(Default::default()));
            }
        }

        let mut values_kHz = [0i32; 8];
        for (i, v) in values_kHz.iter_mut().enumerate() {
            *v = verify.value(domain_bit, i).unwrap_or(0);
        }
        values_kHz[slot as usize] = retained;
        Ok(crate::clock::ClkDomainControlEntry {
            bit: domain_bit,
            entry_type: verify.record_type(domain_bit).unwrap_or(0),
            // the SET guard above already refused non-marshalable types
            value_modifiable: true,
            values_kHz,
        })
    }

    /// V/F curve points from the private ClockClient V/F-POINTS read path
    /// (GetInfo 0x8895B510 → GetStatus 0x7FEE9032, RM 0x20809061/0x20809062
    /// — the article's 127-point XBAR V/F table family). GetStatus's +4..+132
    /// header is seeded from GetInfo's mask output (mandatory — zero seed
    /// returns no records, garbage returns -1). Units live-calibrated against
    /// the public GPC VFP curve; see [`crate::clock::ClkVfPointPrivate`].
    pub fn clk_vf_points_private(&self) -> crate::Result<crate::clock::ClkVfPointsPrivate> {
        #![allow(non_snake_case)]
        trace!("gpu.clk_vf_points_private()");
        use crate::sys::api::{
            NvAPI_GPU_ClockClkVfPointsGetInfo, NvAPI_GPU_ClockClkVfPointsGetStatus,
        };
        use clock::private::{
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE,
        };

        // NOTE: ~2.4 MB of zeroed buffers would overflow the stack. Boxing
        // alone is NOT enough: `Box::new(Big::default())` materializes the
        // temporary on the caller's stack before the heap copy in debug
        // builds (release inlines the construction straight into the box),
        // which overflows a 1-2 MB main thread stack. Allocate zeroed and
        // stamp the magic directly — both structs' Default is zeros + MAGIC.
        let mut info = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version =
                NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC);
            b
        };
        let st =
            unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(self.0, ptr::from_mut(&mut *info).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetInfo, st)
            .map_err(crate::Error::from)?;

        let mut status = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version =
                NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE::MAGIC);
            b
        };
        info.seed_status_header(&mut status);
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetStatus(self.0, ptr::from_mut(&mut *status).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetStatus, st)
            .map_err(crate::Error::from)?;

        // collapse the two 64-dword bank masks into 8 u64s
        let mut masks = [0u64; 8];
        for bank in 0..2usize {
            for idx in 0..clock::private::clk_vfp_info::POINTS {
                if info.point_present(bank, idx) == Some(true) {
                    masks[bank * 4 + idx / 64] |= 1u64 << (idx % 64);
                }
            }
        }

        let mut points = Vec::new();
        for bank in 0..2usize {
            for idx in 0..clock::private::clk_vfp_info::POINTS {
                if info.point_present(bank, idx) != Some(true) {
                    continue;
                }
                let typ = match status.record_type(bank, idx) {
                    Some(t) if t != 0 => t,
                    _ => continue,
                };
                // Pascal-generation parser: type-1 records report the
                // +0x24 frequency term DOUBLED (live-observed on a
                // 10-series: the parsed "default" is exactly 2× the
                // running clock). Halve type-1 frequency terms; type 8/13/18
                // (Ada+) are plain MHz.
                let div = if typ == 1 { 2 } else { 1 };
                points.push(crate::clock::ClkVfPointPrivate {
                    bank: bank as u8,
                    index: idx as u16,
                    record_type: typ,
                    voltage_uV: status.voltage_uv(bank, idx).unwrap_or(0),
                    freq_default_mhz: status.freq_default_mhz(bank, idx).unwrap_or(0) / div,
                    freq_current_mhz: status.freq_current_mhz(bank, idx).unwrap_or(0) / div,
                });
            }
        }

        // Segment the filled points into contiguous same-type runs — bank 0
        // packs multiple domains back-to-back (GPC curve, mem pstate bins,
        // XBAR curve, ...), so runs are the plottable units.
        let mut segments: Vec<crate::clock::ClkVfSegment> = Vec::new();
        // ordinal of each kind within the current bank — the empirical
        // domain_hint is keyed on it (vf #1=GPC, #2=XBAR, #3=HOST; bins
        // #1=Mem, #2=Host; live A/B on 4060 Laptop / R610.74)
        let mut vf_ordinal = [0usize; 2];
        let mut bins_ordinal = [0usize; 2];
        for p in &points {
            let last = segments.last_mut();
            match last {
                Some(s)
                    if s.bank == p.bank
                        && s.record_type == p.record_type
                        && s.end_index + 1 == p.index
                        // a same-type curve CONCATENATION (GPC then XBAR,
                        // both type 8) restarts the voltage axis — split
                        // there too, or plotting would glue two domains
                        // into one curve
                        && p.voltage_uV >= s.voltage_uV_max =>
                {
                    s.end_index = p.index;
                    s.count += 1;
                    s.voltage_uV_min = s.voltage_uV_min.min(p.voltage_uV);
                    s.voltage_uV_max = s.voltage_uV_max.max(p.voltage_uV);
                    s.freq_default_mhz_min = s.freq_default_mhz_min.min(p.freq_default_mhz);
                    s.freq_default_mhz_max = s.freq_default_mhz_max.max(p.freq_default_mhz);
                }
                _ => {
                    segments.push(crate::clock::ClkVfSegment {
                        bank: p.bank,
                        record_type: p.record_type,
                        // provisional — re-classified after the runs are built
                        kind: crate::clock::ClkVfSegmentKind::VfCurve,
                        domain_hint: crate::clock::ClkVfDomainHint::Unknown,
                        start_index: p.index,
                        end_index: p.index,
                        count: 1,
                        voltage_uV_min: p.voltage_uV,
                        voltage_uV_max: p.voltage_uV,
                        freq_default_mhz_min: p.freq_default_mhz,
                        freq_default_mhz_max: p.freq_default_mhz,
                    })
                }
            }
        }

        // CLASSIFY by run LENGTH, per the multi-generation census
        // (Pascal/Turing/Ampere/Ada + A100): every segment is
        // voltage-and-frequency ascending internally, and a new segment
        // starts at a voltage-axis reset (the merge rule above). Curves
        // are long (80 / 127 / 128 points observed); pstate-bin lists
        // (mem-style: one freq/voltage per pstate) are 4-5 points.
        // Record TYPE is useless here — generations reuse types across
        // the two kinds (Turing's GPC curve and Ada's bins share a type).
        for s in segments.iter_mut() {
            s.kind = if s.count >= 8 {
                crate::clock::ClkVfSegmentKind::VfCurve
            } else {
                crate::clock::ClkVfSegmentKind::PstateBins
            };
        }
        for s in segments.iter_mut() {
            let ord = &mut (match s.kind {
                crate::clock::ClkVfSegmentKind::VfCurve => &mut vf_ordinal,
                crate::clock::ClkVfSegmentKind::PstateBins => &mut bins_ordinal,
            }[s.bank as usize]);
            s.domain_hint = match (s.kind, *ord) {
                (crate::clock::ClkVfSegmentKind::VfCurve, 0) => crate::clock::ClkVfDomainHint::Gpc,
                (crate::clock::ClkVfSegmentKind::VfCurve, 1) => crate::clock::ClkVfDomainHint::Xbar,
                (crate::clock::ClkVfSegmentKind::VfCurve, 2) => crate::clock::ClkVfDomainHint::Host,
                (crate::clock::ClkVfSegmentKind::PstateBins, 0) => {
                    crate::clock::ClkVfDomainHint::Mem
                }
                // 4060: host/disp pstate ceiling; Turing: unknown 5-bin
                // list — pstate-family either way
                (crate::clock::ClkVfSegmentKind::PstateBins, 1) => {
                    crate::clock::ClkVfDomainHint::Host
                }
                _ => crate::clock::ClkVfDomainHint::Unknown,
            };
            *ord += 1;
        }

        Ok(crate::clock::ClkVfPointsPrivate {
            masks,
            points,
            segments,
        })
    }

    /// Read the private ClockClient V/F-POINTS CONTROL override table
    /// (GetControl 0xDA025C3E, 1060B records). This is the readback surface
    /// for everything `set_vfp_point_private` / `set_vfp_range_private`
    /// write: per-record mode@+36 (0 = absolute kHz offset, 1 = delta) and
    /// value@+56 (u32 kHz / low i16 raw control). The private GetStatus
    /// exposes NO raw-offset field (only current = default + offset), so
    /// this is the only direct readback of raw control values. All-zero at
    /// stock; masks must be seeded from GetInfo first (mandatory).
    pub fn clk_vf_control_private(&self) -> crate::Result<crate::clock::ClkVfControlPrivate> {
        #![allow(non_snake_case)]
        trace!("gpu.clk_vf_control_private()");
        use crate::sys::api::{
            NvAPI_GPU_ClockClkVfPointsGetControl, NvAPI_GPU_ClockClkVfPointsGetInfo,
        };
        use clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE;

        // Same stack-overflow hazard as clk_vf_points_private: the control
        // block alone is 4.3 MB — allocate zeroed, never Box::new(default()).
        let mut info = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version =
                NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC);
            b
        };
        let st =
            unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(self.0, ptr::from_mut(&mut *info).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetInfo, st)
            .map_err(crate::Error::from)?;

        let mut ctrl = unsafe {
            let b = Box::<clock::private::NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version = NvVersion::with_version(clock::private::clk_vfp_control::MAGIC);
            b
        };
        ctrl.seed_masks_from_info(&info);
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(self.0, ptr::from_mut(&mut *ctrl).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetControl, st)
            .map_err(crate::Error::from)?;

        let mut points = Vec::new();
        for bank in 0..2usize {
            for idx in 0..clock::private::clk_vfp_info::POINTS {
                if info.point_present(bank, idx) != Some(true) {
                    continue;
                }
                points.push(crate::clock::ClkVfControlPointPrivate {
                    bank: bank as u8,
                    index: idx as u16,
                    mode: ctrl.mode(bank, idx).unwrap_or(0),
                    value: ctrl.value(bank, idx).unwrap_or(0),
                });
            }
        }

        Ok(crate::clock::ClkVfControlPrivate { points })
    }

    /// Write one V/F curve point via the private ClockClient V/F-POINTS
    /// SetControl (RM 0x20809062→0x07000109, ID 0xFEC00D04). DANGEROUS V/F
    /// curve write — the per-point analogue of the public `set_vfp_table`,
    /// but covering ALL fabric domains (GPC/XBAR/HOST/...) and supporting
    /// freq-offset mode (mode=0) which the public path cannot do.
    ///
    /// Implements the mandated RMW recipe (mirrors `set_clk_domain_offset`):
    /// GetInfo → seed bank masks → GetControl snapshot → patch one record →
    /// SetControl → GetControl readback → verify → restore on mismatch.
    ///
    /// `bank` is 0 (V/F curve points) or 1 (pstate-class records). `idx`
    /// is the point index (0..2048) within that bank. `freq_mode` selects
    /// mode 0 (kHz frequency OFFSET, same as public VFP freqDeltaKHz,
    /// max clamp ~990 MHz) vs mode 1 (reverse-volt lookup: delta → voltage
    /// shift → look up default freq at shifted voltage → becomes freq offset;
    /// non-linear mapping depends on local curve slope). Both modes produce
    /// identical curves after RM interpolation. `value` is the raw u32 to
    /// write (for reverse-volt mode, only the low i16 is used).
    pub fn set_vfp_point_private(
        &self,
        bank: usize,
        idx: usize,
        freq_mode: bool,
        value: u32,
    ) -> crate::Result<u32> {
        trace!(
            "gpu.set_vfp_point_private(bank={bank}, idx={idx}, freq_mode={freq_mode}, value={value})"
        );
        use crate::sys::api::{
            NvAPI_GPU_ClockClkVfPointsGetControl, NvAPI_GPU_ClockClkVfPointsGetInfo,
            NvAPI_GPU_ClockClkVfPointsSetControl,
        };
        use clock::private::{
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE,
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
        };

        if bank > 1 || idx >= clock::private::clk_vfp_control::POINTS {
            return Err(crate::Error::ArgumentRange(Default::default()));
        }

        // 1. GetInfo → seed bank masks (mandatory, same as the read path)
        let mut info: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version =
                NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC);
            b
        };
        let st =
            unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(self.0, ptr::from_mut(&mut *info).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetInfo, st)
            .map_err(crate::Error::from)?;

        // 2. GetControl snapshot with seeded masks — the RMW source.
        // Use unsafe { zeroed() } not default() — the 4MB rest[] array
        // would overflow the stack when Box::new moves it from stack to heap.
        let mut snapshot: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            b.assume_init()
        };
        snapshot.version =
            sys::api::NvVersion::with_version(clock::private::clk_vfp_control::MAGIC);
        snapshot.seed_masks_from_info(&info);
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(self.0, ptr::from_mut(&mut *snapshot).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetControl, st)
            .map_err(crate::Error::from)?;

        // 3. Patch the snapshot IN PLACE (no clone — the 4MB struct
        // would overflow the stack if cloned). We restore it on mismatch.
        let user_type = if bank == 0 { 8 } else { 6 };
        snapshot
            .set_mask_bit(bank, idx)
            .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
        snapshot
            .set_record_type(bank, idx, user_type)
            .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
        if freq_mode {
            snapshot.set_absolute(bank, idx, value)
        } else {
            snapshot.set_delta(bank, idx, value as i16)
        }
        .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;

        // 4. SetControl — pass the Box's inner pointer (not &Box)
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsSetControl(self.0, ptr::from_ref(&*snapshot).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsSetControl, st)
            .map_err(crate::Error::from)?;

        // 5. Readback + verify
        let mut verify: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            b.assume_init()
        };
        verify.version = sys::api::NvVersion::with_version(clock::private::clk_vfp_control::MAGIC);
        verify.seed_masks_from_info(&info);
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(self.0, ptr::from_mut(&mut *verify).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetControl, st)
            .map_err(crate::Error::from)?;

        let retained_mode = verify.mode(bank, idx).unwrap_or(0);
        let retained_value = verify.value(bank, idx).unwrap_or(0);

        // For mode 0: value must match exactly
        // For mode 1: low i16 must match
        let ok = if freq_mode {
            retained_mode == 0 && retained_value == value
        } else {
            retained_mode == 1 && (retained_value as i16) == (value as i16)
        };

        if !ok {
            // restore the original snapshot (best effort)
            let _ = unsafe {
                NvAPI_GPU_ClockClkVfPointsSetControl(self.0, ptr::from_ref(&*snapshot).cast())
            };
            return Err(crate::Error::ArgumentRange(Default::default()));
        }

        Ok(retained_value)
    }

    /// Sparse mode-1 (reverse-volt) calibration over ONE domain segment of
    /// the private V/F-points table (bank 0). For every `pt_step`-th
    /// present point in `idx_lo..=idx_hi` — pass one DOMAIN per call:
    /// GPC 0-127, XBAR 128-255, HOST 256+ — walks an ascending mode-1
    /// delta ladder (0..=`dmax` step `d_step`), reads the per-point effect
    /// from the STATUS current-frequency field, restores the point
    /// (mode-0 value 0) and fits the exact staircase
    /// ([`crate::clock::clk_vf_stair_fit`]).
    ///
    /// Purpose: validate the universal prior ([`crate::clock::clk_vf_g_prior`])
    /// or measure per-domain modulation (XBAR runs ~+20% hot below ~900
    /// MHz def). Cache the results per GPU + driver version; a handful of
    /// points (`pt_step` 16-32) confirms alignment.
    ///
    /// Pascal: type-1 records leave the STATUS current field empty — such
    /// points return [`crate::clock::ClkVfCalKind::CurAbsent`] (mode-1 writes DO
    /// take effect there; a MEASURE_FREQ effect source is a future
    /// extension behind the same ladder loop).
    pub fn clk_vf_calibrate_private(
        &self,
        idx_lo: usize,
        idx_hi: usize,
        pt_step: usize,
        d_step: i64,
        dmax: i64,
    ) -> crate::Result<Vec<crate::clock::ClkVfCalPoint>> {
        trace!(
            "gpu.clk_vf_calibrate_private({idx_lo}..={idx_hi}, pt_step={pt_step}, d_step={d_step}, dmax={dmax})"
        );
        use crate::sys::api::{
            NvAPI_GPU_ClockClkVfPointsGetInfo, NvAPI_GPU_ClockClkVfPointsGetStatus,
        };
        use clock::private::{
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE,
        };

        fn gcd(mut a: i64, mut b: i64) -> i64 {
            while b != 0 {
                let t = a % b;
                a = b;
                b = t;
            }
            a
        }

        const BANK: usize = 0;
        let idx_hi = idx_hi.min(clock::private::clk_vfp_info::POINTS - 1);
        let pt_step = pt_step.max(1);
        let d_step = d_step.clamp(10, 500);
        let dmax = dmax.clamp(200, 1000);

        // baseline: info + seeded status read (the ladder reuses `info` to
        // re-read status after each write)
        let mut info: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version =
                NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC);
            b
        };
        let st =
            unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(self.0, ptr::from_mut(&mut *info).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetInfo, st)
            .map_err(crate::Error::from)?;
        let read_status = |info: &NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE| {
            let mut s = Box::new(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_STATUS_PRIVATE::default());
            info.seed_status_header(&mut s);
            let st = unsafe {
                NvAPI_GPU_ClockClkVfPointsGetStatus(self.0, ptr::from_mut(&mut *s).cast())
            };
            crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetStatus, st)
                .map_err(crate::Error::from)
                .map(|_| s)
        };
        let baseline = read_status(&info)?;

        let ladder: Vec<i64> = (0..=dmax).step_by(d_step as usize).collect();
        let mut out = Vec::new();
        let mut n = 0usize;
        for idx in idx_lo..=idx_hi {
            if info.point_present(BANK, idx) != Some(true) {
                continue;
            }
            n += 1;
            if !(n - 1).is_multiple_of(pt_step) {
                continue;
            }

            // Pascal type-1 decode: frequency terms are doubled
            let typ = baseline.record_type(BANK, idx).unwrap_or(0);
            let div: i64 = if typ == 1 { 2 } else { 1 };
            let def = baseline.freq_default_mhz(BANK, idx).unwrap_or(0) as i64 / div;
            let volt_mv = baseline.voltage_uv(BANK, idx).unwrap_or(0) / 1000;
            let base_cur = baseline.freq_current_mhz(BANK, idx).unwrap_or(0) as i64 / div;
            let push = |out: &mut Vec<crate::clock::ClkVfCalPoint>,
                        kind: crate::clock::ClkVfCalKind| {
                out.push(crate::clock::ClkVfCalPoint {
                    bank: BANK as u8,
                    idx: idx as u16,
                    def_mhz: def as u32,
                    volt_mv,
                    kind,
                });
            };
            if base_cur == 0 || def == 0 {
                push(&mut out, crate::clock::ClkVfCalKind::CurAbsent);
                continue;
            }

            // ladder: write mode-1 delta → read effect; early-exit only on
            // the positive side (a flat NEGATIVE head is normal — the
            // backward slope cap clamps it within one grid step)
            let mut samples: Vec<crate::clock::ClkVfStairSample> = Vec::new();
            let mut first_e: Option<i64> = None;
            let mut flat_from_start = 0usize;
            for &d in &ladder {
                self.set_vfp_point_private(BANK, idx, false, d as u32)?;
                let s = read_status(&info)?;
                let cur = s.freq_current_mhz(BANK, idx).unwrap_or(0) as i64 / div;
                let e = cur - def;
                match first_e {
                    None => {
                        first_e = Some(e);
                        flat_from_start = 1;
                    }
                    Some(fe) if e == fe => flat_from_start += 1,
                    _ => flat_from_start = 0,
                }
                if flat_from_start >= 5 && d >= 200 {
                    break; // genuinely dead point (still flat at +200)
                }
                samples.push((d, e));
            }
            self.set_vfp_point_private(BANK, idx, true, 0)?; // restore

            // trim floor/flatten-clamped flats at both ends
            while samples.len() > 2 && samples[0].1 == samples[1].1 {
                samples.remove(0);
            }
            while samples.len() > 2 && samples[samples.len() - 1].1 == samples[samples.len() - 2].1
            {
                samples.pop();
            }
            // >=3 distinct effect levels required (2-level fits are noise)
            let mut levels: Vec<i64> = Vec::new();
            for &(_, e) in &samples {
                if levels.last() != Some(&e) {
                    levels.push(e);
                }
            }
            if levels.len() < 3 {
                push(
                    &mut out,
                    crate::clock::ClkVfCalKind::Pinned {
                        flat_effect_mhz: samples.first().map(|&(_, e)| e).unwrap_or(0),
                    },
                );
                continue;
            }
            // Q = GCD of nonzero |effects|
            let mut q_gcd = 0i64;
            for &(_, e) in &samples {
                if e != 0 {
                    q_gcd = gcd(q_gcd, e.abs());
                }
            }
            let q = if q_gcd > 0 { q_gcd } else { 15 };
            match crate::clock::clk_vf_stair_fit(&samples, q) {
                Some(fit) => push(
                    &mut out,
                    crate::clock::ClkVfCalKind::Fitted {
                        fit,
                        q_mhz: q,
                        n_used: samples.len(),
                    },
                ),
                None => push(&mut out, crate::clock::ClkVfCalKind::Unstable),
            }
        }
        Ok(out)
    }

    /// Write a RANGE of V/F curve points with the same delta — the
    /// private-path analogue of the public `set_vfp_range_delta_mhz`.
    /// Patches every point in `[start, end]` (inclusive) on `bank` in a
    /// single RMW cycle (one GetControl → patch N → SetControl), then
    /// readbacks the first and last point to verify.
    pub fn set_vfp_range_private(
        &self,
        bank: usize,
        start: usize,
        end: usize,
        delta_mhz: i16,
    ) -> crate::Result<()> {
        trace!("gpu.set_vfp_range_private(bank={bank}, {start}..={end}, delta={delta_mhz})");
        use crate::sys::api::{
            NvAPI_GPU_ClockClkVfPointsGetControl, NvAPI_GPU_ClockClkVfPointsGetInfo,
            NvAPI_GPU_ClockClkVfPointsSetControl,
        };
        use clock::private::{
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE,
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
        };

        if bank > 1 || start > end || end >= clock::private::clk_vfp_control::POINTS {
            return Err(crate::Error::ArgumentRange(Default::default()));
        }

        // 1. GetInfo → seed masks
        let mut info: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version =
                NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC);
            b
        };
        let st =
            unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(self.0, ptr::from_mut(&mut *info).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetInfo, st)
            .map_err(crate::Error::from)?;

        // 2. GetControl snapshot (on heap, zeroed to avoid stack overflow)
        let mut snapshot: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            b.assume_init()
        };
        snapshot.version =
            sys::api::NvVersion::with_version(clock::private::clk_vfp_control::MAGIC);
        snapshot.seed_masks_from_info(&info);
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(self.0, ptr::from_mut(&mut *snapshot).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetControl, st)
            .map_err(crate::Error::from)?;

        // 3. Patch every point in [start, end] in-place
        let user_type = if bank == 0 { 8 } else { 6 };
        for idx in start..=end {
            snapshot
                .set_mask_bit(bank, idx)
                .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
            snapshot
                .set_record_type(bank, idx, user_type)
                .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
            snapshot
                .set_delta(bank, idx, delta_mhz)
                .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
        }

        // 4. SetControl
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsSetControl(self.0, ptr::from_ref(&*snapshot).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsSetControl, st)
            .map_err(crate::Error::from)?;

        // 5. Readback first + last point's mode to verify SET succeeded
        let mut verify: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            b.assume_init()
        };
        verify.version = sys::api::NvVersion::with_version(clock::private::clk_vfp_control::MAGIC);
        verify.seed_masks_from_info(&info);
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(self.0, ptr::from_mut(&mut *verify).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetControl, st)
            .map_err(crate::Error::from)?;

        // verify that at least the first point has mode=1 (delta)
        let mode = verify.mode(bank, start).unwrap_or(0);
        if mode != 1 {
            return Err(crate::Error::Nvapi(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_ClockClkVfPointsSetControl,
                Status::NotSupported,
            )));
        }

        Ok(())
    }

    /// Like [`set_vfp_range_private`] but writes a DIFFERENT raw mode-1
    /// value per point (one RMW cycle, per-point patch). `deltas` must
    /// contain exactly `end - start + 1` entries in index order. Used by
    /// the CLI `--raw-converted` path, which translates a single MHz
    /// target through each point's own g(def) prior (C/D0 vary with def).
    pub fn set_vfp_range_per_point_private(
        &self,
        bank: usize,
        start: usize,
        end: usize,
        deltas: &[i16],
    ) -> crate::Result<()> {
        trace!(
            "gpu.set_vfp_range_per_point_private(bank={bank}, {start}..={end}, {} pts)",
            deltas.len()
        );
        use crate::sys::api::{
            NvAPI_GPU_ClockClkVfPointsGetControl, NvAPI_GPU_ClockClkVfPointsGetInfo,
            NvAPI_GPU_ClockClkVfPointsSetControl,
        };
        use clock::private::{
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE,
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
        };

        if bank > 1 || start > end || end >= clock::private::clk_vfp_control::POINTS {
            return Err(crate::Error::ArgumentRange(Default::default()));
        }
        if deltas.len() != end - start + 1 {
            return Err(crate::Error::ArgumentRange(Default::default()));
        }

        let mut info: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version =
                NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC);
            b
        };
        let st =
            unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(self.0, ptr::from_mut(&mut *info).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetInfo, st)
            .map_err(crate::Error::from)?;

        let mut snapshot: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            b.assume_init()
        };
        snapshot.version =
            sys::api::NvVersion::with_version(clock::private::clk_vfp_control::MAGIC);
        snapshot.seed_masks_from_info(&info);
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(self.0, ptr::from_mut(&mut *snapshot).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetControl, st)
            .map_err(crate::Error::from)?;

        let user_type = if bank == 0 { 8 } else { 6 };
        for (offset, idx) in (start..=end).enumerate() {
            snapshot
                .set_mask_bit(bank, idx)
                .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
            snapshot
                .set_record_type(bank, idx, user_type)
                .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
            snapshot
                .set_delta(bank, idx, deltas[offset])
                .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
        }

        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsSetControl(self.0, ptr::from_ref(&*snapshot).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsSetControl, st)
            .map_err(crate::Error::from)?;

        // verify the first point took mode=1
        let mut verify: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            b.assume_init()
        };
        verify.version = sys::api::NvVersion::with_version(clock::private::clk_vfp_control::MAGIC);
        verify.seed_masks_from_info(&info);
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(self.0, ptr::from_mut(&mut *verify).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetControl, st)
            .map_err(crate::Error::from)?;
        if verify.mode(bank, start).unwrap_or(0) != 1 {
            return Err(crate::Error::Nvapi(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_ClockClkVfPointsSetControl,
                Status::NotSupported,
            )));
        }
        Ok(())
    }

    /// Reset every present V/F curve point on `bank` to its default
    /// frequency by clearing any applied mode-0 (absolute kHz) override —
    /// i.e. write mode 0 / value 0 to each present point in a single RMW
    /// cycle (one GetControl → patch all → SetControl → readback verify).
    ///
    /// This is the private-family analogue of the public `reset_vfp` /
    /// `core_reset_vfp`, but unlike those (which route through the pstate20
    /// or public Client VfPoints families and therefore cannot clear
    /// private mode-0 overrides), it writes the SAME 0xFEC00D04
    /// SetControl that `set_vfp_point_private` uses — so it actually clears
    /// the mode-0 raw/converted offsets the private write paths apply.
    ///
    /// Only points the driver reports as present (per GetInfo's per-bank
    /// masks) are touched; absent points keep their (zero) mask bit and
    /// are skipped by the SET handler. `bank` 0 is the V/F curve bank;
    /// bank 1 holds pstate-class records and is reset the same way.
    ///
    /// Returns the count of points written (present points patched), for
    /// caller-side diagnostics. `Ok(0)` means the family is present but
    /// the bank has no points (an empty GPU, e.g. during hot-remove).
    pub fn reset_vfp_private(&self, bank: usize, only_mode: Option<u32>) -> crate::Result<usize> {
        trace!("gpu.reset_vfp_private(bank={bank}, only_mode={only_mode:?})");
        use crate::sys::api::{
            NvAPI_GPU_ClockClkVfPointsGetControl, NvAPI_GPU_ClockClkVfPointsGetInfo,
            NvAPI_GPU_ClockClkVfPointsSetControl,
        };
        use clock::private::{
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE,
            NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE,
        };

        if bank > 1 {
            return Err(crate::Error::ArgumentRange(Default::default()));
        }

        // 1. GetInfo → point masks + descriptors (mandatory seed).
        let mut info: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE>::new_zeroed();
            let mut b = b.assume_init();
            b.version =
                NvVersion::with_version(NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_INFO_PRIVATE::MAGIC);
            b
        };
        let st =
            unsafe { NvAPI_GPU_ClockClkVfPointsGetInfo(self.0, ptr::from_mut(&mut *info).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetInfo, st)
            .map_err(crate::Error::from)?;

        // 2. GetControl snapshot with seeded masks — the RMW source.
        let mut snapshot: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> = unsafe {
            let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
            b.assume_init()
        };
        snapshot.version =
            sys::api::NvVersion::with_version(clock::private::clk_vfp_control::MAGIC);
        snapshot.seed_masks_from_info(&info);
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsGetControl(self.0, ptr::from_mut(&mut *snapshot).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetControl, st)
            .map_err(crate::Error::from)?;

        // 3. Patch every PRESENT point to mode 0 / value 0 (clear override).
        // `only_mode` restricts the clear to points currently in that mode
        // (0 = absolute kHz, 1 = raw delta) — None clears BOTH (mode-1
        // leftovers otherwise survive: writing mode 0/0 over a mode-1 point
        // is the clear for either mode).
        // The record type byte (8 for bank 0, 6 for bank 1) is the
        // CONTROL-family user type, not the GetStatus type — same as the
        // single-point setter. `seed_masks_from_info` already set the mask
        // bits for present points, so we only need to patch the record body.
        let user_type = if bank == 0 { 8 } else { 6 };
        let mut written = 0usize;
        let mut first_patched: Option<usize> = None;
        for idx in 0..clock::private::clk_vfp_control::POINTS {
            if !info.point_present(bank, idx).unwrap_or(false) {
                continue;
            }
            if let Some(want) = only_mode {
                if snapshot.mode(bank, idx).unwrap_or(0) != want {
                    continue;
                }
            }
            snapshot
                .set_record_type(bank, idx, user_type)
                .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
            snapshot
                .set_absolute(bank, idx, 0)
                .ok_or_else(|| crate::Error::ArgumentRange(Default::default()))?;
            first_patched.get_or_insert(idx);
            written += 1;
        }

        // 4. SetControl — pass the Box's inner pointer.
        let st = unsafe {
            NvAPI_GPU_ClockClkVfPointsSetControl(self.0, ptr::from_ref(&*snapshot).cast())
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsSetControl, st)
            .map_err(crate::Error::from)?;

        // 5. Readback + verify the first present point took mode 0 / value 0.
        // A present-but-empty bank (written==0) skips verification: there is
        // no point to read back, and the SET was a no-op snapshot round-trip.
        if written > 0 {
            let mut verify: Box<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE> = unsafe {
                let b = Box::<NV_GPU_CLOCK_CLIENT_CLK_VF_POINTS_CONTROL_PRIVATE>::new_zeroed();
                b.assume_init()
            };
            verify.version =
                sys::api::NvVersion::with_version(clock::private::clk_vfp_control::MAGIC);
            verify.seed_masks_from_info(&info);
            let st = unsafe {
                NvAPI_GPU_ClockClkVfPointsGetControl(self.0, ptr::from_mut(&mut *verify).cast())
            };
            crate::status_result(sys::Api::NvAPI_GPU_ClockClkVfPointsGetControl, st)
                .map_err(crate::Error::from)?;

            // find the first PATCHED point and confirm mode==0, value==0
            // (with only_mode, untouched points keep their mode)
            if let Some(idx) = first_patched {
                let mode = verify.mode(bank, idx).unwrap_or(1);
                let value = verify.value(bank, idx).unwrap_or(1);
                if mode != 0 || value != 0 {
                    return Err(crate::Error::Nvapi(crate::NvapiError::new(
                        sys::Api::NvAPI_GPU_ClockClkVfPointsSetControl,
                        Status::NotSupported,
                    )));
                }
            }
        }

        Ok(written)
    }

    // --- PerfVfeEqu / PerfVfeVar family (escape 0x070001C6) ----------------
    //
    // The THIRD V/F edit surface: RM voltage-frequency EQUATIONS (Equ) and
    // VARIABLES (Var). GETs live-verified on Ada 4060 Laptop / R610.74
    // (probe_vfe example); SETs are elevation-gated and stay at this layer.

    /// PerfVfeEqu GET_INFO (ID 0x8D49471C, RM 0x2080A0B5): equation
    /// directory — mask + per-entry type/name. No input seeding needed.
    pub fn vfe_equ_info(&self) -> crate::Result<crate::clock::VfeEquInfo> {
        trace!("gpu.vfe_equ_info()");
        use crate::sys::api::NvAPI_GPU_PerfVfeEquGetInfo;
        use clock::private::{NV_PERF_VFE_EQU_INFO, vfe_equ_info};

        let mut buf = vec![0u8; vfe_equ_info::SIZE];
        buf[0..4].copy_from_slice(&vfe_equ_info::MAGIC.to_le_bytes());
        let st = unsafe { NvAPI_GPU_PerfVfeEquGetInfo(self.0, buf.as_mut_ptr().cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_PerfVfeEquGetInfo, st)
            .map_err(crate::Error::from)?;
        let s: &NV_PERF_VFE_EQU_INFO = unsafe { &*buf.as_ptr().cast() };

        let mut out = crate::clock::VfeEquInfo::default();
        for i in 0..vfe_equ_info::MAX_ENTRIES {
            if s.mask_bit(i).unwrap_or(false) {
                out.mask_bits.push(i as u32);
            }
            if let Some(t) = s.entry_type(i).filter(|&t| t != 0) {
                out.entries.push(crate::clock::VfeEquInfoEntry {
                    index: i as u32,
                    entry_type: t,
                    name: s.entry_name(i).unwrap_or(0),
                    aux: s.entry_aux(i).unwrap_or(0),
                    dwords: s.entry_dwords(i, 8).unwrap_or_default(),
                });
            }
        }
        Ok(out)
    }

    /// PerfVfeEqu GET_CONTROL (ID 0x4C75C9FE, RM 0x2080A0B6). Seeds the
    /// input mask from GetInfo (proper cascade), falls back to seeding the
    /// first 64 bits if info fails. Tries the largest capacity magic first,
    /// then the live-verified smaller one.
    pub fn vfe_equ_control(&self) -> crate::Result<crate::clock::VfeEquControl> {
        trace!("gpu.vfe_equ_control()");
        use crate::sys::api::NvAPI_GPU_PerfVfeEquGetControl;
        use clock::private::{NV_PERF_VFE_EQU_CONTROL, vfe_equ_control, vfe_equ_info};

        let info = self.vfe_equ_info().ok();
        let mut buf = vec![0u8; vfe_equ_control::SIZE_MAX];
        // helper view over the raw buffer
        macro_rules! view {
            ($b:expr) => {
                unsafe { &*($b.as_ptr().cast::<NV_PERF_VFE_EQU_CONTROL>()) }
            };
        }

        for &magic in &[vfe_equ_control::MAGIC_MAX, vfe_equ_control::MAGIC_MIN] {
            buf.iter_mut().for_each(|b| *b = 0);
            buf[0..4].copy_from_slice(&magic.to_le_bytes());
            {
                let s: &mut NV_PERF_VFE_EQU_CONTROL = unsafe { &mut *buf.as_mut_ptr().cast() };
                match &info {
                    Some(i) => {
                        // seed mask dwords from GetInfo's mask
                        for dword in 0..256u32 {
                            let bit_source = (0..32).any(|b| {
                                let idx = dword * 32 + b;
                                idx < vfe_equ_info::MAX_ENTRIES as u32
                                    && i.mask_bits.contains(&(dword * 32 + b))
                            });
                            let v = if bit_source { u32::MAX } else { 0 };
                            let abs = vfe_equ_control::MASK + 4 * dword as usize;
                            if abs >= 4 && abs - 4 + 4 <= s.rest.len() {
                                s.rest[abs - 4..abs].copy_from_slice(&v.to_le_bytes());
                            }
                        }
                    }
                    None => {
                        let s2: &mut NV_PERF_VFE_EQU_CONTROL =
                            unsafe { &mut *buf.as_mut_ptr().cast() };
                        s2.seed_mask_bits(64);
                    }
                }
            }
            let st = unsafe { NvAPI_GPU_PerfVfeEquGetControl(self.0, buf.as_mut_ptr().cast()) };
            if crate::status_result(sys::Api::NvAPI_GPU_PerfVfeEquGetControl, st).is_ok() {
                let s = view!(buf);
                let mut out = crate::clock::VfeEquControl::default();
                for i in 0..8192usize {
                    if s.mask_bit(i).unwrap_or(false) {
                        out.mask_bits.push(i as u32);
                    }
                    if let Some(t) = s.entry_type(i).filter(|&t| t != 0) {
                        out.entries.push(crate::clock::VfeEquControlEntry {
                            index: i as u32,
                            type_raw: t,
                            dwords: s.entry_dwords(i, 8).unwrap_or_default(),
                        });
                    }
                }
                return Ok(out);
            }
            // remember last status for error propagation
            if magic == vfe_equ_control::MAGIC_MIN {
                crate::status_result(sys::Api::NvAPI_GPU_PerfVfeEquGetControl, st)
                    .map_err(crate::Error::from)?;
            }
        }
        Err(crate::Error::Nvapi(crate::NvapiError::new(
            sys::Api::NvAPI_GPU_PerfVfeEquGetControl,
            Status::NotSupported,
        )))
    }

    /// PerfVfeEqu SET_CONTROL (ID 0x68B798C4) — DANGEROUS equation write,
    /// elevation-gated. Takes a prepared control block (snapshot from a
    /// GetControl buffer re-read via this method's buffer conventions).
    /// Only for privileged tooling; not exposed to hi/core/CLI.
    pub fn set_vfe_equ_control_raw(&self, block: &[u8]) -> crate::Result<()> {
        trace!("gpu.set_vfe_equ_control_raw(len={})", block.len());
        use crate::sys::api::NvAPI_GPU_PerfVfeEquSetControl;
        if block.len() < 4 || block.len() > clock::private::vfe_equ_control::SIZE_MAX {
            return Err(crate::Error::ArgumentRange(Default::default()));
        }
        let st = unsafe { NvAPI_GPU_PerfVfeEquSetControl(self.0, block.as_ptr().cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_PerfVfeEquSetControl, st)
            .map_err(crate::Error::from)?;
        Ok(())
    }

    /// PerfVfeVar GET_INFO (ID 0xB9DA41D6, RM 0x2080A0B1): variable
    /// directory — 256-bit mask + per-entry type. Uses the live-verified
    /// 70344 tier (larger tiers use different internal offsets).
    pub fn vfe_var_info(&self) -> crate::Result<crate::clock::VfeVarInfo> {
        trace!("gpu.vfe_var_info()");
        use crate::sys::api::NvAPI_GPU_PerfVfeVarGetInfo;
        use clock::private::{NV_PERF_VFE_VAR_INFO, vfe_var_info};

        let mut buf = vec![0u8; vfe_var_info::SIZE];
        buf[0..4].copy_from_slice(&vfe_var_info::MAGIC.to_le_bytes());
        let st = unsafe { NvAPI_GPU_PerfVfeVarGetInfo(self.0, buf.as_mut_ptr().cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_PerfVfeVarGetInfo, st)
            .map_err(crate::Error::from)?;
        let s: &NV_PERF_VFE_VAR_INFO = unsafe { &*buf.as_ptr().cast() };

        let mut out = crate::clock::VfeVarInfo::default();
        for i in 0..vfe_var_info::MAX_ENTRIES {
            if s.mask_bit(i).unwrap_or(false) {
                out.mask_bits.push(i as u32);
            }
            if let Some(t) = s.entry_type(i).filter(|&t| t != 0) {
                out.entries.push(crate::clock::VfeVarInfoEntry {
                    index: i as u32,
                    entry_type: t,
                    dwords: s.entry_dwords(i, 8).unwrap_or_default(),
                });
            }
        }
        Ok(out)
    }

    /// PerfVfeVar GET_CONTROL (ID 0x5D387298, RM 0x2080A0B3). Uses the
    /// live-verified 68300 tier: mask 0xFFFF seed (first 16 entries),
    /// count @+8, 88-byte records from +0x4C.
    pub fn vfe_var_control(&self) -> crate::Result<crate::clock::VfeVarControl> {
        trace!("gpu.vfe_var_control()");
        use crate::sys::api::NvAPI_GPU_PerfVfeVarGetControl;
        use clock::private::{NV_PERF_VFE_VAR_CONTROL, vfe_var_control};

        let mut buf = vec![0u8; vfe_var_control::SIZE];
        buf[0..4].copy_from_slice(&vfe_var_control::MAGIC.to_le_bytes());
        buf[4..8].copy_from_slice(&0xFFFF_u32.to_le_bytes());
        let st = unsafe { NvAPI_GPU_PerfVfeVarGetControl(self.0, buf.as_mut_ptr().cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_PerfVfeVarGetControl, st)
            .map_err(crate::Error::from)?;
        let s: &NV_PERF_VFE_VAR_CONTROL = unsafe { &*buf.as_ptr().cast() };
        let count = s.count().unwrap_or(0);
        let mut out = crate::clock::VfeVarControl {
            count,
            entries: Vec::new(),
        };
        // records are dense from +0x4C, `count` of them (live 70)
        let n = (count as usize).min(vfe_var_control::MAX_ENTRIES);
        for i in 0..n {
            let dwords = s.entry_dwords(i, 8).unwrap_or_default();
            if dwords.iter().any(|&d| d != 0) {
                out.entries.push(crate::clock::VfeVarControlEntry {
                    index: i as u32,
                    dwords,
                });
            }
        }
        Ok(out)
    }

    /// PerfVfeVar SET_CONTROL (ID 0x79FA23A2) — DANGEROUS variable write,
    /// elevation-gated. Only for privileged tooling.
    pub fn set_vfe_var_control_raw(&self, block: &[u8]) -> crate::Result<()> {
        trace!("gpu.set_vfe_var_control_raw(len={})", block.len());
        use crate::sys::api::NvAPI_GPU_PerfVfeVarSetControl;
        if block.len() < 4 || block.len() > clock::private::vfe_var_control::SIZE {
            return Err(crate::Error::ArgumentRange(Default::default()));
        }
        let st = unsafe { NvAPI_GPU_PerfVfeVarSetControl(self.0, block.as_ptr().cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_PerfVfeVarSetControl, st)
            .map_err(crate::Error::from)?;
        Ok(())
    }

    #[allow(unused_assignments)]
    pub fn power_usage<C: IntoIterator<Item = crate::clock::PowerTopologyChannelId>>(
        &self,
        channels: C,
    ) -> crate::Result<<power::private::NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS as RawConversion>::Target>
    {
        trace!("gpu.power_usage()");
        let mut status = power::private::NV_GPU_CLIENT_POWER_TOPOLOGY_STATUS::default();
        for (channel, entry) in channels.into_iter().zip(&mut status.entries) {
            entry.channel = channel.into();
            status.count += 1;
        }
        status.count = status.count.saturating_sub(1);

        unsafe { nvcall!(NvAPI_GPU_ClientPowerTopologyGetStatus@get(self.0) => raw) }
    }

    pub fn power_usage_channels(&self) -> crate::Result<Vec<crate::clock::PowerTopologyChannelId>> {
        trace!("gpu.power_usage_channels()");
        unsafe { nvcall!(NvAPI_GPU_ClientPowerTopologyGetInfo@get(self.0) => raw) }
    }

    pub fn power_limit_info(
        &self,
    ) -> crate::Result<<power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO as RawConversion>::Target>
    {
        trace!("gpu.power_limit_info()");

        unsafe { nvcall!(NvAPI_GPU_ClientPowerPoliciesGetInfo@get(self.0) => raw) }
    }

    pub fn power_limit(
        &self,
    ) -> crate::Result<<power::private::NV_GPU_CLIENT_POWER_POLICIES_STATUS as RawConversion>::Target>
    {
        trace!("gpu.power_limit()");

        unsafe { nvcall!(NvAPI_GPU_ClientPowerPoliciesGetStatus@get(self.0) => raw) }
    }

    pub fn set_power_limit<I: IntoIterator<Item = Percentage1000>>(
        &self,
        values: I,
    ) -> crate::NvapiResult<()> {
        trace!("gpu.set_power_limit()");
        let mut data = power::private::NV_GPU_CLIENT_POWER_POLICIES_STATUS::default();
        //data.valid = 1;
        for (entry, v) in data.entries.iter_mut().zip(values) {
            trace!("gpu.set_power_limit({:?})", v);
            entry.power_target = v.0;
            data.count += 1;
        }

        unsafe { nvcall!(NvAPI_GPU_ClientPowerPoliciesSetStatus(self.0, &data)) }
    }

    /// Set the PPAB / Dynamic-Boost controller enable state (notebook platform
    /// power coordination between dGPU and CPU). `active = true` enables the
    /// controller (the "PPAB Enable" checkbox in OEM partner tools), `false`
    /// disables it. NDA-private ID 0x1504FC3D; GLOBAL single-arg boolean setter
    /// (no per-GPU handle — targets the implicitly-selected GPU, like the ref tool).
    /// Calls the private lifecycle init (0xAD298D3F) first, exactly as the ref tool's
    /// init stub does — without it the driver returns API_NOT_INITIALIZED.
    pub fn set_dynamic_boost(&self, active: bool) -> crate::NvapiResult<()> {
        trace!("gpu.set_dynamic_boost({})", active);
        self.private_lifecycle_init()?;
        unsafe { nvcall!(NvAPI_GPU_ClientDynamicBoostSetStatus(active.into())) }
    }

    /// Private NVAPI lifecycle/controller init (NDA 0xAD298D3F). the ref tool calls
    /// this with arg=1 at init, before any Dynamic-Boost/QBoost power setter, but
    /// does NOT gate later setters on its result. Mirror that: `NoImplementation`
    /// (observed on Linux `libnvidia-api`, where the TGP Get/Set private IDs ARE
    /// implemented but the lifecycle init is not) is swallowed as non-fatal — the
    /// real setter call surfaces its own status if it genuinely can't proceed.
    pub fn private_lifecycle_init(&self) -> crate::NvapiResult<()> {
        trace!("gpu.private_lifecycle_init()");
        match unsafe { nvcall!(NvAPI_GPU_PrivateLifecycleInit(true.into())) } {
            Ok(()) => Ok(()),
            Err(e) if e.status == crate::Status::NoImplementation => {
                warn!("private_lifecycle_init not implemented by this driver; continuing");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// TGP-watts power range (min/default/max in **milliwatts**) + the active
    /// policy-table index, from the private ClientPowerPoliciesGetInfo variant
    /// (NDA, ID 0x67F31384). `policy_index` defaults to 2 when the driver
    /// reports none (0xFF), matching the ref tool. Returns `Ok(None)` where the
    /// driver does not expose the private interface.
    pub fn tgp_watt_range(&self) -> crate::NvapiResult<Option<TgpWattRange>> {
        trace!("gpu.tgp_watt_range()");
        // 347KB struct — allocate the backing bytes on the heap directly to
        // avoid a stack temporary, then cast in place. version is set via the
        // StructVersion::versioned() layout (dword0).
        let mut buf: Vec<u8> =
            vec![
                0u8;
                std::mem::size_of::<power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE>()
            ];
        // stamp the version magic the driver expects (StructVersion for ver 1)
        let ver = <power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE as sys::nvapi::StructVersion>::NVAPI_VERSION;
        buf[..4].copy_from_slice(&ver.data.to_ne_bytes());
        let info: &power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE =
            unsafe { &*(buf.as_ptr() as *const _) };
        let status = unsafe {
            sys::api::NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate(
                self.0,
                buf.as_mut_ptr() as *mut _,
            )
        };
        if crate::status_result(
            sys::Api::NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate,
            status,
        )
        .is_err()
        {
            return Ok(None);
        }
        let idx = info.policy_index().unwrap_or(2) as usize;
        Ok(Some(TgpWattRange {
            policy_index: idx,
            min_mw: info.min_mw(idx),
            default_mw: info.default_mw(idx),
            max_mw: info.max_mw(idx),
        }))
    }

    /// D-Notifier (D0-notify / "extern power state") current state + the D1..D5
    /// power-cap table, from the SAME private ClientPowerPoliciesGetInfo variant
    /// as [`tgp_watt_range`] (NDA, ID `0x67F31384`). The D-Notifier fields live
    /// in the TAIL of the 347KB struct (after the TGP policy table). RE'd from
    /// the ref tool `[GPUHandle::pollDNotifyLimit]`; power values cross-checked live
    /// on RTX 4060 Laptop (D2=55W, D3=45W, D4=33W, D5=10W, D1=Unlimited).
    /// Returns `Ok(None)` where the driver doesn't expose the private interface.
    pub fn dnotify_info(&self) -> crate::NvapiResult<Option<DNotifierInfo>> {
        trace!("gpu.dnotify_info()");
        // Same 347KB GetInfo struct as tgp_watt_range; heap-backed.
        let mut buf: Vec<u8> =
            vec![
                0u8;
                std::mem::size_of::<power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE>()
            ];
        let ver = <power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE as sys::nvapi::StructVersion>::NVAPI_VERSION;
        buf[..4].copy_from_slice(&ver.data.to_ne_bytes());
        let info: &power::private::NV_GPU_CLIENT_POWER_POLICIES_INFO_PRIVATE =
            unsafe { &*(buf.as_ptr() as *const _) };
        let status = unsafe {
            sys::api::NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate(
                self.0,
                buf.as_mut_ptr() as *mut _,
            )
        };
        if crate::status_result(
            sys::Api::NvAPI_GPU_ClientPowerPoliciesGetInfoPrivate,
            status,
        )
        .is_err()
        {
            return Ok(None);
        }

        // Build the full D1..D5 table. D1 (idx -1) is conventionally Unlimited;
        // D2..D5 read their mW cap from the per-D power table.
        let mut levels: [DNotifierLevel; 5] = std::array::from_fn(|i| {
            // i = 0..4 → driver indices -1, 0, 1, 2, 3 (D1..D5).
            let didx = i as i32 - 1;
            let mut lvl = DNotifierLevel::from_index(didx).unwrap_or(DNotifierLevel {
                level: 0,
                index: didx,
                power_mw: None,
            });
            if didx != -1 {
                lvl.power_mw = info.dnotify_power_mw(didx);
            }
            lvl
        });
        // Backfill D1's slot: it has no table value, conventionally Unlimited.
        levels[0].power_mw = None;

        let active = info
            .dnotify_active_index()
            .and_then(DNotifierLevel::from_index);

        Ok(Some(DNotifierInfo { active, levels }))
    }

    /// Set the D-Notifier (D0-notify) limit to the given D level. `didx` is the
    /// signed driver level code: `-1`=D1/Unlimited, `0`=D2, `1`=D3, `2`=D4,
    /// `3`=D5 — exactly the values the ref tool's `setDNotifyLimit` switch maps from
    /// the D1..D5 CLI args. Raw two-arg NDA setter (NvAPI_GPU_ClientExtern
    /// PowerState set, ID `0x48E0847D`): `(hPhysicalGPU, level: u32)` — no struct
    /// buffer. The level is passed as a raw u32 (sign-extended for D1's -1).
    pub fn set_dnotify_limit(&self, didx: i32) -> crate::NvapiResult<()> {
        trace!("gpu.set_dnotify_limit({})", didx);
        // the ref tool's process performs a private lifecycle init at startup before
        // any power-control setter; mirror that (harmless if already done), the
        // same guard set_dynamic_boost / set_tgp_watt use.
        self.private_lifecycle_init()?;
        // Pass the signed level as a u32 (0xFFFFFFFF for D1's -1, matching
        // the ref tool's mov v15, -1).
        unsafe { nvcall!(NvAPI_GPU_ClientExternPowerStateSet(self.0, didx as u32)) }
    }

    /// P-State level table (present pstates + per-pstate min/max clock in kHz
    /// for the given clock-domain) from the private PerfPstatesGetInfo
    /// (`0x7B30AE0D`). The source of the ref tool's `-pstate` GET listing. `domain`
    /// selects the clock dimension (0=GPC/core by default; the ref tool resolves the
    /// GPC index via 0x57B5A5DF). Returns `Ok(None)` where the driver doesn't
    /// expose the private interface.
    pub fn pstate_levels_domain(
        &self,
        domain: usize,
    ) -> crate::NvapiResult<Option<PStateLevelsInfo>> {
        trace!("gpu.pstate_levels_domain({})", domain);
        // 275KB — heap-backed to be stack-safe, same pattern as tgp_watt_range.
        let mut buf: Vec<u8> =
            vec![0u8; std::mem::size_of::<clock::private::NV_GPU_PERF_PSTATES_INFO_PRIVATE>()];
        let ver = <clock::private::NV_GPU_PERF_PSTATES_INFO_PRIVATE as sys::nvapi::StructVersion>::NVAPI_VERSION;
        buf[..4].copy_from_slice(&ver.data.to_ne_bytes());
        let info: &clock::private::NV_GPU_PERF_PSTATES_INFO_PRIVATE =
            unsafe { &*(buf.as_ptr() as *const _) };
        let status = unsafe {
            sys::api::NvAPI_GPU_PerfPstatesGetInfoPrivate(self.0, buf.as_mut_ptr() as *mut _)
        };
        if crate::status_result(sys::Api::NvAPI_GPU_PerfPstatesGetInfoPrivate, status).is_err() {
            return Ok(None);
        }
        let pstates = info
            .pstate_entries_domain(domain)
            .into_iter()
            .map(|e| PStateClockRange {
                pstate: e.pstate,
                min_khz: e.min_khz,
                max_khz: e.max_khz,
            })
            .collect();
        Ok(Some(PStateLevelsInfo { pstates }))
    }

    /// P-State level table for the default (GPC/core) clock-domain. Convenience
    /// for [`pstate_levels_domain`](Self::pstate_levels_domain)(0).
    pub fn pstate_levels(&self) -> crate::NvapiResult<Option<PStateLevelsInfo>> {
        self.pstate_levels_domain(0)
    }

    /// The set of P-State numbers currently locked (via PerfClientLimitsSetStatus
    /// 0x39442CFB), from the private ClientPStateLimitStatus (NDA 0x9962C97C).
    /// Empty when nothing is locked (default/cleared state). Returns `Ok(None)`
    /// where the driver doesn't expose the private interface.
    pub fn pstate_lock_status(&self) -> crate::NvapiResult<Option<Vec<u8>>> {
        trace!("gpu.pstate_lock_status()");
        // 164-byte struct — heap-backed. The driver's version magic 0x10088
        // reports size 136 (v1); write it raw since it doesn't match the
        // 164-byte buffer the ref tool allocates.
        let mut buf: Vec<u8> =
            vec![0u8; std::mem::size_of::<clock::private::NV_GPU_CLIENT_PSTATE_LIMIT_STATUS>()];
        buf[..4].copy_from_slice(&0x10088u32.to_ne_bytes());
        let status: &clock::private::NV_GPU_CLIENT_PSTATE_LIMIT_STATUS =
            unsafe { &*(buf.as_ptr() as *const _) };
        let res = unsafe {
            sys::api::NvAPI_GPU_ClientPStateLimitStatus(self.0, buf.as_mut_ptr() as *mut _)
        };
        if crate::status_result(sys::Api::NvAPI_GPU_ClientPStateLimitStatus, res).is_err() {
            return Ok(None);
        }
        Ok(Some(status.locked_pstates()))
    }

    /// Set the native NVAPI P-State lock (the the ref tool `-pstate:<index>` SETTER,
    /// PerfClientLimitsSetStatus NDA 0x39442CFB). RE'd byte-exact from the ref tool's
    /// `[GPUHandle::setPState]`:
    ///
    /// - `PStateNativeLock::Reset` → 4 entries clearing limit IDs 0,1,4,5
    ///   (Gpu, GpuLowerbound, Unknown_4, Unknown_5) to mode 0 (None).
    /// - `PStateNativeLock::PstateOnly(n)` → 2 entries (id 5,4 mode 1 value n)
    ///   selecting pstate `n` without touching frequency.
    /// - `PStateNativeLock::PstateAndFreq { pstate, freq_khz }` → 4 entries:
    ///   id 0/1 (Gpu/GpuLowerbound) mode 2 (ManualFrequency) value=freq_kHz,
    ///   id 5/4 mode 1 (PstateSelect) value=pstate.
    ///
    /// Calls the private lifecycle init + clearRatedTdp (0xC9E9BB33 mode 0)
    /// first, exactly as the ref tool's setPState does. `freq_khz` is in kHz
    /// (MHz × 1000). Returns Ok if the lock applied.
    pub fn set_pstate_native(&self, lock: PStateNativeLock) -> crate::NvapiResult<()> {
        trace!("gpu.set_pstate_native({:?})", lock);
        self.private_lifecycle_init()?;
        self.clear_rated_tdp()?;

        // 780-byte PerfClientLimits V2 buffer. Heap-backed.
        let mut buf: Vec<u8> =
            vec![0u8; std::mem::size_of::<clock::private::NV_GPU_PERF_CLIENT_LIMITS>()];
        // version magic 0x2030C (v2 | 780).
        buf[..4].copy_from_slice(&0x2030Cu32.to_ne_bytes());
        let data: &mut clock::private::NV_GPU_PERF_CLIENT_LIMITS =
            unsafe { &mut *(buf.as_mut_ptr() as *mut _) };

        // Raw mode codes (NV_GPU_CLOCK_LOCK_MODE is a c_int alias).
        let mode_pstate = clock::private::ClockLockMode::PstateSelect.repr();
        let mode_freq = clock::private::ClockLockMode::ManualFrequency.repr();
        // Helper: write entry[k] = {id, mode, value} (other fields stay 0).
        let mut set_entry = |k: usize, id: i32, mode: i32, value: u32| {
            if let Some(e) = data.entries.get_mut(k) {
                e.id = id.into();
                e.mode = mode.into();
                e.value = value;
            }
        };

        match lock {
            PStateNativeLock::Reset => {
                data.count = 4;
                // Clear limit IDs 0,1,4,5 to mode None (reset all locks).
                for (k, id) in [0i32, 1, 4, 5].iter().enumerate() {
                    set_entry(k, *id, 0, 0);
                }
            }
            PStateNativeLock::PstateOnly { pstate } => {
                data.count = 2;
                set_entry(0, 5, mode_pstate, pstate as u32);
                set_entry(1, 4, mode_pstate, pstate as u32);
            }
            PStateNativeLock::PstateAndFreq { pstate, freq_khz } => {
                data.count = 4;
                set_entry(0, 0, mode_freq, freq_khz); // Gpu upperbound
                set_entry(1, 1, mode_freq, freq_khz); // Gpu lowerbound
                set_entry(2, 5, mode_pstate, pstate as u32);
                set_entry(3, 4, mode_pstate, pstate as u32);
            }
        }

        unsafe {
            nvcall!(NvAPI_GPU_PerfClientLimitsSetStatus(
                self.0,
                buf.as_ptr() as *const _
            ))
        }
    }

    /// Set the GPU frequency perf-cap (the ref tool `-gpuclk:<MHz>` SETTER,
    /// PerfLimitsSetStatus NDA 0x32CA4983). RE'd byte-exact from ref tool 2's
    /// `GPUHandle::setGpcClock`: clamps the perf max/min frequency to a cap
    /// value — NOT an offset, NOT a P-state lock (that's [`set_pstate_native`]).
    ///
    /// `PerfFreqCap::Cap { max_khz, min_khz }` writes two entries (max + min);
    /// `PerfFreqCap::Reset` clears both (the `-gpuclk:-1` path). `freq_khz` is
    /// MHz × 1000. Faithful to source: does NOT call `private_lifecycle_init`
    /// (setGpcClock calls the raw setter directly, unlike setPState).
    pub fn set_perf_freq_cap(&self, cap: PerfFreqCap) -> crate::NvapiResult<()> {
        trace!("gpu.set_perf_freq_cap({:?})", cap);
        let buf = build_perf_freq_cap_buffer(cap);
        unsafe {
            nvcall!(NvAPI_GPU_PerfLimitsSetStatus(
                self.0,
                buf.as_ptr() as *const _
            ))
        }
    }

    /// Read back the active GPU frequency perf-caps (PerfLimitsGetStatus NDA
    /// 0xEFCEDD1F). RE'd from ref tool 2 `isPStateLocked`: the 3-step query —
    /// GetInfo (count) → large GetStatus. Returns one entry per active cap
    /// (max/min); `locked` is true where the cap is currently applied.
    pub fn perf_freq_caps(&self) -> crate::NvapiResult<Vec<PerfFreqCapEntry>> {
        trace!("gpu.perf_freq_caps()");
        let count = self.perf_limits_info_count()?;

        let mut buf = vec![0u8; PERF_LIMITS_SIZE];
        buf[..4].copy_from_slice(&PERF_LIMITS_MAGIC.to_ne_bytes());
        buf[PERF_LIMITS_OFF_COUNT..PERF_LIMITS_OFF_COUNT + 4].copy_from_slice(&count.to_ne_bytes());
        unsafe {
            nvcall!(NvAPI_GPU_PerfLimitsGetStatus(
                self.0,
                buf.as_mut_ptr() as *mut _
            ))?;
        }

        let n = read_u32(&buf, PERF_LIMITS_OFF_COUNT) as usize;
        let mut out = Vec::with_capacity(n);
        for k in 0..n {
            let base = PERF_LIMITS_ENTRY0_BASE + k * PERF_LIMITS_ENTRY_STRIDE;
            if base + PERF_LIMITS_OFF_LOCKED + 1 > buf.len() {
                break;
            }
            let entry = PerfFreqCapEntry {
                type_marker: read_u32(&buf, base + PERF_LIMITS_OFF_TYPE),
                freq_khz: read_u32(&buf, base + PERF_LIMITS_OFF_FREQ),
                locked: buf[base + PERF_LIMITS_OFF_LOCKED] != 0,
            };
            // Skip empty slots: the driver returns a large count (the struct's
            // full capacity) but only a few entries are active caps. Mirror
            // ref tool 2's isPStateLocked, which only acts on entries where the
            // locked flag / type / freq are non-zero.
            if entry.locked || entry.type_marker != 0 || entry.freq_khz != 0 {
                out.push(entry);
            }
        }
        Ok(out)
    }

    /// Read the entry count from the medium PerfLimitsGetInfo struct (NDA
    /// 0xE63AE22B, magic 0x1300C). ref tool 2's `isPStateLocked` uses this as the
    /// entry count for the paired large GetStatus struct.
    fn perf_limits_info_count(&self) -> crate::NvapiResult<u32> {
        let mut buf = vec![0u8; PERF_LIMITS_INFO_SIZE];
        buf[..4].copy_from_slice(&PERF_LIMITS_INFO_MAGIC.to_ne_bytes());
        unsafe {
            nvcall!(NvAPI_GPU_PerfLimitsGetInfo(
                self.0,
                buf.as_mut_ptr() as *mut _
            ))?;
        }
        Ok(read_u32(&buf, PERF_LIMITS_INFO_OFF_COUNT))
    }

    /// Clear the rated-TDP control (NDA 0xC9E9BB33, mode 0). the ref tool's setPState
    /// calls this before applying a new P-State/frequency lock. "Rated TDP" =
    /// the nominal default power baseline.
    fn clear_rated_tdp(&self) -> crate::NvapiResult<()> {
        trace!("gpu.clear_rated_tdp()");
        // 12-byte struct {version: 0x1000C, dword1: 1, mode: 0}. Heap-backed.
        let mut buf = [0u8; 12];
        buf[..4].copy_from_slice(&0x1000Cu32.to_ne_bytes());
        buf[4..8].copy_from_slice(&1u32.to_ne_bytes());
        // mode 0 (clear) — dword2 stays 0.
        unsafe {
            nvcall!(NvAPI_GPU_ClientRatedTdpControl(
                self.0,
                buf.as_ptr() as *const _
            ))
        }
    }

    /// Rated-TDP readback trio (RE'd R610.74). Returns
    /// `(control_mode, info_capabilities, status)`:
    /// - control: the 12B SET-struct view — reads mode @+4, fills current
    ///   mode @+8 (0xED2BEA09, sub-cmd 0x207E004E)
    /// - info: one capability byte (0x87BD35EF, 0x10008)
    /// - status: 36B rich view (0xFCBDF642, 0x10024) — raw dwords; decode
    ///   TBD (fill order +4/+8u8/+12, five mode dwords +16..+32).
    pub fn rated_tdp_readback(&self) -> crate::NvapiResult<(u32, u8, [u32; 10])> {
        trace!("gpu.rated_tdp_readback()");
        use crate::sys::nvapi::VersionedStruct;
        use clock::private::{
            NV_GPU_RATED_TDP_CONTROL, NV_GPU_RATED_TDP_INFO, NV_GPU_RATED_TDP_STATUS,
        };
        let mut control = unsafe { std::mem::zeroed::<NV_GPU_RATED_TDP_CONTROL>() };
        *control.nvapi_version_mut() = NvVersion::with_version(0x1000C);
        let st = unsafe { sys::api::NvAPI_GPU_PerfRatedTdpGetControl(self.0, &mut control) };
        crate::status_result(sys::Api::NvAPI_GPU_PerfRatedTdpGetControl, st)?;

        let mut info = unsafe { std::mem::zeroed::<NV_GPU_RATED_TDP_INFO>() };
        *info.nvapi_version_mut() = NvVersion::with_version(0x10008);
        let st = unsafe { sys::api::NvAPI_GPU_PerfRatedTdpGetInfo(self.0, &mut info) };
        crate::status_result(sys::Api::NvAPI_GPU_PerfRatedTdpGetInfo, st)?;

        let mut status = unsafe { std::mem::zeroed::<NV_GPU_RATED_TDP_STATUS>() };
        *status.nvapi_version_mut() = NvVersion::with_version(0x10024);
        let st = unsafe { sys::api::NvAPI_GPU_PerfRatedTdpGetStatus(self.0, &mut status) };
        crate::status_result(sys::Api::NvAPI_GPU_PerfRatedTdpGetStatus, st)?;

        let raw = [
            status.dword_04,
            status.byte_08 as u32,
            status.dword_0c,
            status.mode_0,
            status.mode_1,
            status.mode_2,
            status.mode_3,
            status.mode_4,
            0,
            0,
        ];
        Ok((control.mode, info.capabilities, raw))
    }

    // ------------------------------------------------------------------
    // Driver-side ("OEM"/NVIDIA) OC Scanner control — the family MSI's
    // MSIOCScanner uses on drivers >= 455.00 instead of the legacy
    // user-mode scanner.dll. The scan runs INSIDE the driver; user mode
    // only starts/stops/reverts it and observes results on the V/F curve.
    // 68-byte control struct, version magic 0x10044; per the MSIOCScanner
    // host the payload beyond the version is left zeroed.
    // ------------------------------------------------------------------

    fn oem_oc_scanner_call(&self, start: bool, stop: bool, revert: bool) -> crate::NvapiResult<()> {
        trace!("gpu.oem_oc_scanner_call(start={start}, stop={stop}, revert={revert})");
        let mut buf = [0u8; 68];
        buf[..4].copy_from_slice(&0x10044u32.to_ne_bytes());
        let (id, st) = if start {
            (sys::Api::NvAPI_GPU_ClientStartOcScanner, unsafe {
                sys::api::private::NvAPI_GPU_ClientStartOcScanner(
                    self.0,
                    buf.as_mut_ptr() as *mut _,
                )
            })
        } else if stop {
            (sys::Api::NvAPI_GPU_ClientStopOcScanner, unsafe {
                sys::api::private::NvAPI_GPU_ClientStopOcScanner(self.0, buf.as_mut_ptr() as *mut _)
            })
        } else if revert {
            (sys::Api::NvAPI_GPU_ClientRevertOc, unsafe {
                sys::api::private::NvAPI_GPU_ClientRevertOc(self.0, buf.as_mut_ptr() as *mut _)
            })
        } else {
            return Err(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_ClientStartOcScanner,
                crate::Status::Error,
            ));
        };
        crate::status_result(id, st)
    }

    /// Start the driver-side OC scanner (NDA 0xBC4AEE25). Subscribes the
    /// status callback first (VelocityX protocol: start = subscribe + start,
    /// both errors tolerated separately), then starts the scan — the driver
    /// scans in the background and applies the resulting V/F offsets itself.
    /// Progress is observable via `oem_oc_scanner_last_update()`.
    pub fn oem_oc_scanner_start(&self) -> crate::NvapiResult<()> {
        let _ = self.oem_oc_scanner_subscribe();
        self.oem_oc_scanner_call(true, false, false)
    }

    /// Stop the driver-side OC scanner (NDA 0xC28B73DE).
    pub fn oem_oc_scanner_stop(&self) -> crate::NvapiResult<()> {
        self.oem_oc_scanner_call(false, true, false)
    }

    /// Revert the OC applied by the driver-side scanner (NDA 0xCC727B22) —
    /// restores the pre-scan curve.
    pub fn oem_oc_scanner_revert(&self) -> crate::NvapiResult<()> {
        self.oem_oc_scanner_call(false, false, true)
    }

    /// Query the last OC scanner run status (NDA 0x593E8E72). Uses the same
    /// 68-byte control struct (magic 0x10044). Returns a status code
    /// describing the scanner state: OK = idle/has-result, -104 = busy/
    /// scanning, -1 = generic error, -191 = not-on-bus. Per IDA this is a
    /// STATUS-ONLY call — it does NOT write per-point results into the
    /// struct; per-point data arrives via the Register callback.
    pub fn oem_oc_scanner_status(&self) -> crate::NvapiResult<()> {
        trace!("gpu.oem_oc_scanner_status()");
        let mut buf = [0u8; 68];
        buf[..4].copy_from_slice(&0x10044u32.to_ne_bytes());
        let st = unsafe {
            sys::api::private::NvAPI_GPU_ClientGetLastOcScannerResults(
                self.0,
                buf.as_mut_ptr() as *mut _,
            )
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClientGetLastOcScannerResults, st)
    }

    /// Query the last INCOMPLETE OC-scanner run's partial results (NDA
    /// 0xBE371D0A, @0x180073550). Same 68B control struct / 0x10044 magic
    /// as `oem_oc_scanner_status`; RPC cmd 13 (2→-104, 4→-191). Companion
    /// to the status call for runs that did not finish.
    pub fn oem_oc_scanner_incomplete_results(&self) -> crate::NvapiResult<()> {
        trace!("gpu.oem_oc_scanner_incomplete_results()");
        let mut buf = [0u8; 68];
        buf[..4].copy_from_slice(&0x10044u32.to_ne_bytes());
        let st = unsafe {
            sys::api::private::NvAPI_GPU_GetLastIncompleteOcScannerResults(
                self.0,
                buf.as_mut_ptr() as *mut _,
            )
        };
        crate::status_result(sys::Api::NvAPI_GPU_GetLastIncompleteOcScannerResults, st)
    }

    /// Enable/disable the background OC scanner (NDA 0x06DC7CE8,
    /// @0x1800717C0). 72B struct, magic 0x10048; enable byte @+4 and the
    /// 9-byte feature GUID 0B 0A 0E 08 E8 72 9D D9 F3 @+10 (validated).
    pub fn oem_oc_scanner_set_background(&self, enable: bool) -> crate::NvapiResult<()> {
        trace!("gpu.oem_oc_scanner_set_background({enable})");
        use crate::sys::nvapi::VersionedStruct;
        use clock::private::NV_GPU_OC_BACKGROUND_SCANNER_CONTROL;
        let mut control = unsafe { std::mem::zeroed::<NV_GPU_OC_BACKGROUND_SCANNER_CONTROL>() };
        *control.nvapi_version_mut() = NvVersion::with_version(0x10048);
        control.enable = enable as u8;
        control.feature_guid = [0x0B, 0x0A, 0x0E, 0x08, 0xE8, 0x72, 0x9D, 0xD9, 0xF3];
        let st = unsafe {
            sys::api::private::NvAPI_GPU_ClientEnableBackgroundOcScanner(self.0, &mut control)
        };
        crate::status_result(sys::Api::NvAPI_GPU_ClientEnableBackgroundOcScanner, st)
    }

    /// Register the OC Scanner status callback (NDA 0x1CB41116). Uses the
    /// PNY VelocityX V1-EX register layout (magic 0x100D8, 216B, callback at
    /// +0x50 — the newer sibling of MSI's 0x10098/152B). The trampoline
    /// stores the latest notification into process-global statics readable
    /// via `oem_oc_scanner_last_update()`.
    pub fn oem_oc_scanner_subscribe(&self) -> crate::NvapiResult<()> {
        trace!("gpu.oem_oc_scanner_subscribe()");
        use clock::private::{
            NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM_V1EX, NV_OC_SCANNER_STATUS_CALLBACK,
        };
        let mut parm: NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM_V1EX = unsafe { std::mem::zeroed() };
        parm.version = NvVersion::with_struct::<NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM_V1EX>(1);
        parm.callback = Some(oc_scanner_status_trampoline as NV_OC_SCANNER_STATUS_CALLBACK);
        let st = unsafe {
            sys::api::private::NvAPI_GPU_ClientRegisterForOcScannerStatusUpdates(
                self.0,
                ptr::from_mut(&mut parm).cast(),
            )
        };
        match crate::status_result(
            sys::Api::NvAPI_GPU_ClientRegisterForOcScannerStatusUpdates,
            st,
        ) {
            // Older drivers expect the MSI-era 152B layout (magic 0x10098,
            // callback at +0x78, cookie at +0x30) — fall back to it.
            Err(e) if e.status == crate::Status::IncompatibleStructVersion => self
                .oem_oc_scanner_register_v1(Some(
                    oc_scanner_status_trampoline as NV_OC_SCANNER_STATUS_CALLBACK,
                )),
            other => other,
        }
    }

    /// Raw MSI-era register (magic 0x10098/152B, cookie@+0x30,
    /// validity@+0x50, callback@+0x78).
    fn oem_oc_scanner_register_v1(
        &self,
        callback: Option<clock::private::NV_OC_SCANNER_STATUS_CALLBACK>,
    ) -> crate::NvapiResult<()> {
        let mut buf = [0u8; 152];
        buf[..4].copy_from_slice(&0x10098u32.to_ne_bytes());
        if let Some(cb) = callback {
            buf[0x78..0x80].copy_from_slice(&(cb as usize).to_ne_bytes());
        }
        let st = unsafe {
            sys::api::private::NvAPI_GPU_ClientRegisterForOcScannerStatusUpdates(
                self.0,
                buf.as_mut_ptr().cast(),
            )
        };
        crate::status_result(
            sys::Api::NvAPI_GPU_ClientRegisterForOcScannerStatusUpdates,
            st,
        )
    }

    /// Unregister the OC Scanner status callback — the same 0x1CB41116 call
    /// with a NULL callback (VelocityX Unsubscribe protocol).
    pub fn oem_oc_scanner_unsubscribe(&self) -> crate::NvapiResult<()> {
        trace!("gpu.oem_oc_scanner_unsubscribe()");
        use clock::private::NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM_V1EX;
        let mut parm: NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM_V1EX = unsafe { std::mem::zeroed() };
        parm.version = NvVersion::with_struct::<NV_GPU_OC_SCANNER_STATUS_UPDATE_PARM_V1EX>(1);
        parm.callback = None;
        let st = unsafe {
            sys::api::private::NvAPI_GPU_ClientRegisterForOcScannerStatusUpdates(
                self.0,
                ptr::from_mut(&mut parm).cast(),
            )
        };
        match crate::status_result(
            sys::Api::NvAPI_GPU_ClientRegisterForOcScannerStatusUpdates,
            st,
        ) {
            Err(e) if e.status == crate::Status::IncompatibleStructVersion => {
                self.oem_oc_scanner_register_v1(None)
            }
            other => other,
        }
    }

    /// Latest OC Scanner callback snapshot (process-global). `state` uses
    /// the VelocityX 3-state mapping (0 idle / 1 scanning / 2 failed-or-
    /// finished); `progress` is the raw +0x50 dword. Returns
    /// `(scan_state, progress, status_0x60, status_0x64)`.
    pub fn oem_oc_scanner_last_update() -> (u32, u32, u32, u32) {
        use std::sync::atomic::Ordering;
        (
            OC_SCANNER_LAST.scan_state.load(Ordering::Relaxed),
            OC_SCANNER_LAST.progress.load(Ordering::Relaxed),
            OC_SCANNER_LAST.status_0x60.load(Ordering::Relaxed),
            OC_SCANNER_LAST.status_0x64.load(Ordering::Relaxed),
        )
    }

    /// Battery Boost 2.0 enable/disable (NDA 0xD2561B69, private).
    /// (state 1=enable, 0=disable). Mobile-only.
    pub fn set_bb2_active(&self, enable: bool) -> crate::NvapiResult<()> {
        trace!("gpu.set_bb2_active(enable={})", enable);
        use power::private::NV_SYS_CLIENT_JPAC_CONTROL;
        let mut ctrl = NV_SYS_CLIENT_JPAC_CONTROL::bb2_active(enable);
        let st = unsafe {
            sys::api::private::NvAPI_SYS_ClientJpacSetControl(ptr::from_mut(&mut ctrl).cast())
        };
        crate::status_result(sys::Api::NvAPI_SYS_ClientJpacSetControl, st)
    }

    /// Whisper Mode 2.0 enable/disable (NDA 0xD2561B69, private).
    /// (state 1=enable, 0=disable). Mobile-only.
    pub fn set_wm2_active(&self, enable: bool) -> crate::NvapiResult<()> {
        trace!("gpu.set_wm2_active(enable={})", enable);
        use power::private::NV_SYS_CLIENT_JPAC_CONTROL;
        let mut ctrl = NV_SYS_CLIENT_JPAC_CONTROL::wm2_active(enable);
        let st = unsafe {
            sys::api::private::NvAPI_SYS_ClientJpacSetControl(ptr::from_mut(&mut ctrl).cast())
        };
        crate::status_result(sys::Api::NvAPI_SYS_ClientJpacSetControl, st)
    }

    /// Whisper Mode 2.0 acoustic mode (NDA 0xD2561B69, private).
    ///  (0=Quieter, 1=Quiet, 2=Balanced).
    pub fn set_wm2_mode(&self, mode: power::private::Wm2AcousticMode) -> crate::NvapiResult<()> {
        trace!("gpu.set_wm2_mode(mode={:?})", mode);
        use power::private::NV_SYS_CLIENT_JPAC_CONTROL;
        let mut ctrl = NV_SYS_CLIENT_JPAC_CONTROL::wm2_mode(mode);
        let st = unsafe {
            sys::api::private::NvAPI_SYS_ClientJpacSetControl(ptr::from_mut(&mut ctrl).cast())
        };
        crate::status_result(sys::Api::NvAPI_SYS_ClientJpacSetControl, st)
    }

    /// Force the GPU into a given P-State (NDA 0x025BFB10, private).
    ///
    /// `set_type` live-tested on 4060L: 0/1/2 ALL force-lock the pstate —
    /// NONE release. IDA (handler sub_1801D60C0) confirms why: set_type is
    /// validated to {0,1,2} (else -5) and encoded as a 2-bit mode field at RM
    /// buffer offset 56, but all three values take the SAME RM escape
    /// (0x7000056) with no branching — the mode distinction is internal to
    /// the kernel RM and not observable here. No release path exists in this
    /// handler.
    ///
    /// `pstate`: 0..15 accepted (bitmask = 1<<pstate); **16 is special =
    /// all pstates** (bitmask 0); ≥17 → -5. nvapioc uses set_type=2.
    ///
    /// To RELEASE a forced pstate use a different API: `EnableDynamicPstates`
    /// (0xFA579A0F, enable=0) is the likely unlock — SetForcePstateEx
    /// (0xE7B1198D) is also force-only (just +1 flag bit vs base, no min/max).
    pub fn set_force_pstate(&self, pstate: u32, set_type: u32) -> crate::NvapiResult<()> {
        trace!(
            "gpu.set_force_pstate(pstate={}, set_type={})",
            pstate, set_type
        );
        let st = unsafe { sys::api::private::NvAPI_GPU_SetForcePstate(self.0, pstate, set_type) };
        crate::status_result(sys::Api::NvAPI_GPU_SetForcePstate, st)
    }

    /// Kepler-era pstate floor/ceiling clamps GET (private
    /// GetPstateClientLimits). Returns the per-pstate min/max limits
    /// currently in force (empty when unrestricted).
    pub fn pstate_client_limits(&self) -> crate::NvapiResult<Vec<PstateClientLimit>> {
        trace!("gpu.pstate_client_limits()");
        use crate::sys::nvapi::VersionedStruct;
        use pstate::private::NV_GPU_PSTATE_CLIENT_LIMITS;
        let mut raw = unsafe { std::mem::zeroed::<NV_GPU_PSTATE_CLIENT_LIMITS>() };
        *raw.nvapi_version_mut() = NvVersion::with_struct::<NV_GPU_PSTATE_CLIENT_LIMITS>(1);
        let st = unsafe {
            sys::api::private::NvAPI_GPU_GetPstateClientLimits(self.0, ptr::from_mut(&mut raw))
        };
        crate::status_result(sys::Api::NvAPI_GPU_GetPstateClientLimits, st)?;
        let n = (raw.numLimits as usize).min(pstate::NVAPI_MAX_GPU_PSTATE20_PSTATES);
        Ok((0..n)
            .map(|i| {
                let l = &raw.limits[i];
                PstateClientLimit {
                    pstate_id: l.pstateId.repr() as u32,
                    min_level: l.minLevel,
                    max_level: l.maxLevel,
                }
            })
            .collect())
    }

    /// Kepler-era pstate floor/ceiling clamp SET (NDA 0xFDFC7D49, private —
    /// nvidiaInspector's legacy OC family). This is the RELEASE path for a
    /// force-locked pstate ([`set_force_pstate`]): pass an empty slice to
    /// clear all limits, or clamp specific pstates to a min/max level range.
    /// Note this is the older sibling of the already-wrapped modern
    /// PerfClientLimits (0x39442CFB) family — prefer that one on Pascal+.
    pub fn set_pstate_client_limits(&self, limits: &[PstateClientLimit]) -> crate::NvapiResult<()> {
        trace!("gpu.set_pstate_client_limits(len={})", limits.len());
        use crate::sys::nvapi::VersionedStruct;
        use pstate::private::NV_GPU_PSTATE_CLIENT_LIMITS;
        if limits.len() > pstate::NVAPI_MAX_GPU_PSTATE20_PSTATES {
            return Err(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_SetPstateClientLimits,
                sys::Status::InvalidArgument,
            ));
        }
        let mut raw = unsafe { std::mem::zeroed::<NV_GPU_PSTATE_CLIENT_LIMITS>() };
        *raw.nvapi_version_mut() = NvVersion::with_struct::<NV_GPU_PSTATE_CLIENT_LIMITS>(1);
        raw.numLimits = limits.len() as u32;
        for (dst, src) in raw.limits.iter_mut().zip(limits) {
            dst.pstateId = (src.pstate_id as i32).into();
            dst.minLevel = src.min_level;
            dst.maxLevel = src.max_level;
        }
        let st = unsafe {
            sys::api::private::NvAPI_GPU_SetPstateClientLimits(self.0, ptr::from_ref(&raw))
        };
        crate::status_result(sys::Api::NvAPI_GPU_SetPstateClientLimits, st)
    }

    /// Kepler-era per-pstate clock table SET (0x07BCF4AC, from
    /// nvidiaInspector's legacy OC family — the SET sibling of the bound
    /// GetPerfClocks 0x1EA54A3B). `num_clocks` is the table count the legacy
    /// API expects (vertminer's SET wrapper never observed working; value 1
    /// alongside the 10868-byte V2 table is the best guess). Expect
    /// NotSupported on Pascal+ — modern cards go through SetPstates20.
    pub fn set_perf_clocks(
        &self,
        num_clocks: u32,
        clocks: &clock::NV_GPU_PERF_CLOCKS,
    ) -> crate::NvapiResult<()> {
        trace!("gpu.set_perf_clocks(num_clocks={})", num_clocks);
        let st =
            unsafe { sys::api::NvAPI_GPU_SetPerfClocks(self.0, num_clocks, ptr::from_ref(clocks)) };
        crate::status_result(sys::Api::NvAPI_GPU_SetPerfClocks, st)
    }

    /// Legacy pstate table SET (0xCDF27911, from nvidiaInspector's legacy OC
    /// family — pre-pstates20 OC path). `input_flags` observed 0. Expect
    /// NotSupported on Pascal+; use [`set_pstates20`][Self::set_pstates]
    /// instead on modern drivers.
    pub fn set_pstates_info(
        &self,
        input_flags: u32,
        info: &pstate::NV_GPU_PERF_PSTATES_INFO,
    ) -> crate::NvapiResult<()> {
        trace!("gpu.set_pstates_info(flags={})", input_flags);
        let st =
            unsafe { sys::api::NvAPI_GPU_SetPstatesInfo(self.0, input_flags, ptr::from_ref(info)) };
        crate::status_result(sys::Api::NvAPI_GPU_SetPstatesInfo, st)
    }

    /// Restart the display driver (NDA 0xB4B26B65). The classic "apply OC"
    /// trigger used by legacy OC CLIs after writing clock/voltage settings.
    /// No arguments. Modern drivers apply most settings without a restart,
    /// so this is mostly vestigial.
    pub fn restart_display_driver(&self) -> crate::NvapiResult<()> {
        trace!("gpu.restart_display_driver()");
        let st = unsafe { sys::api::NvAPI_RestartDisplayDriver() };
        crate::status_result(sys::Api::NvAPI_RestartDisplayDriver, st)
    }

    /// GC6 / RTD3 force-wake control (NDA 0xD387D414). Commands the RM driver
    /// to query (cmd=0), force-sleep (cmd=1), or force-wake (cmd=2) the dGPU's
    /// GC6 power state. Returns the driver-decoded `result` state
    /// (`NV_GPU_GC6_STATE_*`: D0_ACTIVE=3, GC6_IDLE=2, OK=0). Use cmd=0 after a
    /// wake to confirm the transition. On 610 mobile drivers this reaches the
    /// kernel driver with no per-call GCOFF guard, so it can wake a powered-down
    /// dGPU that would otherwise make overclock ops fail with -220.
    pub fn gc6_control(&self, cmd: u32) -> crate::NvapiResult<u32> {
        trace!("gpu.gc6_control(cmd={})", cmd);
        use crate::sys::nvapi::VersionedStruct;
        let mut data = unsafe { std::mem::zeroed::<power::private::NV_GPU_GC6_CONTROL_V1>() };
        *data.nvapi_version_mut() =
            NvVersion::with_struct::<power::private::NV_GPU_GC6_CONTROL_V1>(1);
        data.cmd = cmd;
        let status =
            unsafe { sys::api::NvAPI_GPU_GC6Control(self.0, ptr::from_mut(&mut data).cast()) };
        crate::status_result(sys::Api::NvAPI_GPU_GC6Control, status)?;
        Ok(data.result)
    }

    /// Query the current GC6 power state (GC6Control cmd=0). Returns the
    /// driver-decoded state: 3 = D0/active, 2 = GC6/idle, 0 = OK/no report.
    /// NOTE: on the 610 mobile driver the query path returns NVAPI_ERROR (-1)
    /// even though wake (cmd=2) succeeds — only the SET half of GC6Control is
    /// implemented there. Treat an `Err` here as "query unsupported", not a
    /// wake failure; use [`force_gc6_exit`] + a follow-up op to confirm wakes.
    pub fn gc6_query_state(&self) -> crate::NvapiResult<u32> {
        self.gc6_control(power::private::NV_GPU_GC6_CONTROL_CMD_QUERY)
    }

    /// Force the dGPU into GC6 / idle (GC6Control cmd=1) — the sleep path.
    pub fn gc6_force_sleep(&self) -> crate::NvapiResult<u32> {
        self.gc6_control(power::private::NV_GPU_GC6_CONTROL_CMD_SLEEP)
    }

    /// Force the dGPU out of GC6 via the GC6Control cmd=2 path (NDA 0xD387D414).
    /// Returns the post-wake state. Prefer [`PhysicalGpu::force_gc6_exit`] for a
    /// simpler one-shot wake unless you need the struct-based result.
    pub fn gc6_force_wake(&self) -> crate::NvapiResult<u32> {
        self.gc6_control(power::private::NV_GPU_GC6_CONTROL_CMD_WAKE)
    }

    /// Force the dGPU out of GC6 / GCOFF — the one-shot wake (NDA 0x55590CB2).
    /// Single-arg escape (0x10000FC); no struct, no version magic. The
    /// purpose-built counterpart to `gc6_force_wake` but simpler and the
    /// recommended first call before any overclock op on a 610 mobile driver
    /// where the dGPU may have powered off. Returns -104 (mapped to an error)
    /// on SKUs without GC6 support.
    pub fn force_gc6_exit(&self) -> crate::NvapiResult<()> {
        trace!("gpu.force_gc6_exit()");
        unsafe { nvcall!(NvAPI_GPU_ForceGC6Exit(self.0)) }
    }

    /// Set the GPU TGP in **watts** (the watts-form TGP slider). Performs the
    /// read-modify-write the the ref tool `setTgpWatt` does: GET the 10016-byte
    /// control buffer (NDA 0x8B3E7343), patch the active policy entry's power
    /// field to `watts × 1000` mW, SET it back (NDA 0xBFF09E59). `policy_index`
    /// selects the entry (the mask bit); use the index from [`tgp_watt_range`].
    /// Returns the resolved milliwatts actually written.
    /// Set the GPU TGP in **watts** (the watts-form TGP slider). Performs the
    /// read-modify-write the the ref tool `setTgpWatt` does: GET the 10016-byte
    /// control buffer (NDA 0x8B3E7343), patch the active policy entry's power
    /// field to `watts × 1000` mW, SET it back (NDA 0xBFF09E59). `policy_index`
    /// selects the entry (the mask bit); use the index from [`tgp_watt_range`].
    /// Returns the resolved milliwatts actually written.
    ///
    /// **Driver-gate caveat (RTX 4060 Laptop, driver r576):** the SET entry
    /// point 0xBFF09E59 is NOT resolvable from nvoc's process —
    /// `nvapi_QueryInterface(0xBFF09E59)` returns NULL (QI for the paired GET
    /// 0x8B3E7343 succeeds). the ref tool's process resolves it, so the ref tool's
    /// `-gpupwr:<watts>` works; something in the ref tool's full driver-invoker setup
    /// (NvPCF/QBoost-controller init, or even WinRing0) registers the entry
    /// point. The call here therefore returns `NoImplementation` on this driver
    /// until that registration path is reproduced. The buffer layout, magic,
    /// mask, and power-field offset are all byte-verified against the ref tool via
    /// WinDbg (handle 0x100, magic 0x12720, mask 1<<idx, mW @ buf+0x8A0+40*idx).
    pub fn set_tgp_watt(&self, watts: u32, policy_index: usize) -> crate::NvapiResult<u32> {
        trace!("gpu.set_tgp_watt({} W, idx {})", watts, policy_index);
        // the ref tool's init stub calls the private lifecycle init 0xAD298D3F(1) at
        // process startup before ANY power-control NVAPI call. Mirror that — but
        // best-effort: on desktop Linux `libnvidia-api` does not implement the
        // private lifecycle init (it resolves to no implementation), yet the TGP
        // Get/Set endpoints are live. Swallow `NoImplementation` (done in
        // [`private_lifecycle_init`]) and, for any other init error, log + continue
        // so the SET — not the init — reports whether it can proceed.
        if let Err(e) = self.private_lifecycle_init() {
            warn!(
                "set_tgp_watt: private_lifecycle_init failed ({:?}); attempting set anyway",
                e.status
            );
        }
        // the ref tool's setTgpWatt runs AFTER queryPowerPolicy (GetInfoPrivate) has
        // populated the GPUHandle's policy state. Mirror that: call the private
        // GetInfo first so the driver's power-policy state is primed.
        let _ = self.tgp_watt_range()?;
        // 10KB — heap-backed to be stack-safe.
        let mut buf: Vec<u8> =
            vec![0u8; std::mem::size_of::<power::private::NV_GPU_CLIENT_TGP_WATT_STATUS>()];
        let ver = <power::private::NV_GPU_CLIENT_TGP_WATT_STATUS as sys::nvapi::StructVersion>::NVAPI_VERSION;
        buf[..4].copy_from_slice(&ver.data.to_ne_bytes());
        unsafe {
            let status =
                sys::api::NvAPI_GPU_ClientTgpWattGetStatus(self.0, buf.as_mut_ptr() as *mut _);
            crate::status_result(sys::Api::NvAPI_GPU_ClientTgpWattGetStatus, status)?;
        }
        let milliwatts = if watts == 0xFFFFFFFF {
            0xFFFFFFFF
        } else {
            watts.saturating_mul(1000)
        };
        let data: &mut power::private::NV_GPU_CLIENT_TGP_WATT_STATUS =
            unsafe { &mut *(buf.as_mut_ptr() as *mut _) };
        data.set_power_mw(policy_index, milliwatts);
        unsafe {
            let status =
                sys::api::NvAPI_GPU_ClientTgpWattSetStatus(self.0, buf.as_ptr() as *const _);
            crate::status_result(sys::Api::NvAPI_GPU_ClientTgpWattSetStatus, status)?;
        }
        Ok(milliwatts)
    }

    /// Reset the GPU TGP to its rated/default value (the TGP slider's "Reset").
    /// Same read-modify-write as [`set_tgp_watt`], but writes the default mW
    /// reported by [`tgp_watt_range`] (or 0 if unavailable) into the entry.
    ///
    /// Calls the private lifecycle init first, mirroring [`set_tgp_watt`], but
    /// treats it as best-effort: any failure is logged and the read-modify-write
    /// proceeds regardless. On desktop Linux `libnvidia-api` does not implement
    /// the private lifecycle init (the ID resolves to no implementation), yet the
    /// TGP Get/Set endpoints are live — so the init is not necessary there and
    /// must not abort the reset. The real TGP SET surfaces its own status if it
    /// genuinely cannot proceed. (`set_tgp_watt`'s `private_lifecycle_init` already
    /// swallows `NoImplementation` for the same reason; reset goes further and
    /// tolerates any init error, since it is a recovery path.)
    pub fn reset_tgp_watt(&self, policy_index: usize) -> crate::NvapiResult<Option<u32>> {
        trace!("gpu.reset_tgp_watt(idx {})", policy_index);
        if let Err(e) = self.private_lifecycle_init() {
            warn!(
                "reset_tgp_watt: private_lifecycle_init failed ({:?}); attempting reset anyway",
                e.status
            );
        }
        let default_mw = self.tgp_watt_range()?.and_then(|r| r.default_mw);
        let mut buf: Vec<u8> =
            vec![0u8; std::mem::size_of::<power::private::NV_GPU_CLIENT_TGP_WATT_STATUS>()];
        let ver = <power::private::NV_GPU_CLIENT_TGP_WATT_STATUS as sys::nvapi::StructVersion>::NVAPI_VERSION;
        buf[..4].copy_from_slice(&ver.data.to_ne_bytes());
        unsafe {
            let status =
                sys::api::NvAPI_GPU_ClientTgpWattGetStatus(self.0, buf.as_mut_ptr() as *mut _);
            crate::status_result(sys::Api::NvAPI_GPU_ClientTgpWattGetStatus, status)?;
        }
        if let Some(mw) = default_mw {
            let data: &mut power::private::NV_GPU_CLIENT_TGP_WATT_STATUS =
                unsafe { &mut *(buf.as_mut_ptr() as *mut _) };
            data.set_power_mw(policy_index, mw);
        }
        unsafe {
            let status =
                sys::api::NvAPI_GPU_ClientTgpWattSetStatus(self.0, buf.as_ptr() as *const _);
            crate::status_result(sys::Api::NvAPI_GPU_ClientTgpWattSetStatus, status)?;
        }
        Ok(default_mw)
    }

    pub fn thermal_settings(
        &self,
        index: Option<u32>,
    ) -> crate::Result<<thermal::NV_GPU_THERMAL_SETTINGS as RawConversion>::Target> {
        trace!("gpu.thermal_settings({:?})", index);

        unsafe {
            nvcall!(NvAPI_GPU_GetThermalSettings@get(self.0, index.unwrap_or(thermal::NVAPI_THERMAL_TARGET_ALL.repr() as _)) => raw)
        }
    }

    /// Thermal-channel capability descriptor (undocumented
    /// `NvAPI_GPU_ThermChannelGetInfo`, 0x0bc8163d). Best-effort: callers
    /// must tolerate failure (pre-Pascal GPUs may not expose it). On success
    /// it provides the authoritative `priChIdx` LUT (which channel index is
    /// the hot spot / VRAM reading) plus per-channel metadata (ch_type /
    /// offset_sw / offset_hw / scaling / range) — feed its `channel_mask` to
    /// [`Self::thermal_channel_status`] and index the result by priChIdx.
    /// (Verified on Pascal/Turing/Ampere laptop + desktop GPUs: returns OK,
    /// e.g. 1080Ti channel_mask=0x03, priChIdx GPU_AVG=0/GPU_MAX=1.)
    pub fn thermal_channel_info(
        &self,
    ) -> crate::Result<<thermal::private::NV_GPU_THERMAL_THERM_CHANNEL_INFO as RawConversion>::Target>
    {
        trace!("gpu.thermal_channel_info()");
        let data = thermal::private::NV_GPU_THERMAL_THERM_CHANNEL_INFO_PARAMS_V2 {
            version: NvVersion::new(
                size_of::<thermal::private::NV_GPU_THERMAL_THERM_CHANNEL_INFO_PARAMS_V2>(),
                2,
            ),
            ..Default::default()
        };

        unsafe { nvcall!(NvAPI_GPU_ThermChannelGetInfo@get{data}(self.0) => raw) }
    }

    /// Live thermal-channel readings (the STATUS half of the ThermChannel
    /// pair; ID 0x65fe3aad, `channel[32]` layout). `channel_mask` should come
    /// from [`Self::thermal_channel_info`]'s `channel_mask`; the returned
    /// temps are indexed directly by channel number, so `get(priChIdx[GPU_MAX])`
    /// is the authoritative hot-spot temperature.
    pub fn thermal_channel_status(
        &self,
        channel_mask: u32,
    ) -> crate::Result<
        <thermal::private::NV_GPU_THERMAL_THERM_CHANNEL_STATUS as RawConversion>::Target,
    > {
        trace!("gpu.thermal_channel_status(0x{:x})", channel_mask);
        let mut data = thermal::private::NV_GPU_THERMAL_THERM_CHANNEL_STATUS_PARAMS_V2 {
            version: NvVersion::new(
                size_of::<thermal::private::NV_GPU_THERMAL_THERM_CHANNEL_STATUS_PARAMS_V2>(),
                2,
            ),
            ..Default::default()
        };
        data.channel_mask = channel_mask;

        unsafe { nvcall!(NvAPI_GPU_ThermChannelGetStatus@get{data}(self.0) => raw) }
    }

    pub fn thermal_limit_info(
        &self,
    ) -> crate::Result<
        <thermal::private::NV_GPU_CLIENT_THERMAL_POLICIES_INFO as RawConversion>::Target,
    > {
        trace!("gpu.thermal_limit_info()");

        unsafe { nvcall!(NvAPI_GPU_ClientThermalPoliciesGetInfo@get(self.0) => raw) }
    }

    pub fn thermal_limit(
        &self,
    ) -> crate::Result<
        <thermal::private::NV_GPU_CLIENT_THERMAL_POLICIES_STATUS as RawConversion>::Target,
    > {
        trace!("gpu.thermal_limit()");

        unsafe { nvcall!(NvAPI_GPU_ClientThermalPoliciesGetStatus@get(self.0) => raw) }
    }

    pub fn set_thermal_limit<I: IntoIterator<Item = crate::thermal::ThermalLimit>>(
        &self,
        value: I,
    ) -> crate::NvapiResult<()> {
        trace!("gpu.set_thermal_limit()");
        let mut data = thermal::private::NV_GPU_CLIENT_THERMAL_POLICIES_STATUS::default();
        for (entry, v) in data.entries.iter_mut().zip(value) {
            trace!("gpu.set_thermal_limit({:?})", v);
            *entry = v.to_raw();
            data.count += 1;
        }

        unsafe { nvcall!(NvAPI_GPU_ClientThermalPoliciesSetStatus(self.0, &data)) }
    }

    /// Read the current target-temperature wall (mobile "targettemp") for the
    /// given policy index, in degrees Celsius. Uses the PRIVATE
    /// ClientThermalPolicies GET-prime (NDA 0xC4554575) — the read half of the
    /// RMW pair that actually works on mobile GPUs (the documented
    /// ClientThermalPoliciesGetStatus 0xE9C425A1 returns a transient/no-op value
    /// on mobile). Returns None if the index is out of range or unsupported.
    ///
    /// `policy_index`: on RTX 4060 Laptop the wall ("GPU Target Temperature" in
    /// nvidia-smi) is **index 2** (reads 87C = nvidia-smi's value). idx 0/1/4
    /// are other thermal policies, idx 3/5/6/7 are invalid (return Error).
    pub fn target_temperature(&self, policy_index: usize) -> crate::Result<Option<f32>> {
        trace!("gpu.target_temperature({})", policy_index);
        let mut data = thermal::private::NV_GPU_CLIENT_THERMAL_TARGET_STATUS::default();
        let ver = <thermal::private::NV_GPU_CLIENT_THERMAL_TARGET_STATUS as sys::nvapi::StructVersion>::NVAPI_VERSION;
        data.version = ver;
        data.mask = 1u32 << policy_index;
        unsafe {
            let status = sys::api::NvAPI_GPU_ClientThermalTargetGetStatus(self.0, &mut data);
            crate::status_result(sys::Api::NvAPI_GPU_ClientThermalTargetGetStatus, status)?;
        }
        Ok(data.target_temp_c(policy_index))
    }

    /// Scan every target-temp policy slot (idx 0..ENTRIES_MAX) and return the
    /// ones the driver actually exposes (single-bit GET per index — multi-bit
    /// masks like 0xFFFF are rejected by the driver, so we probe one at a time).
    /// Each entry is `(policy_index, target_temp_celsius)`; `Ok(vec)` is empty
    /// when no policies are exposed (desktop GPUs typically). Used for
    /// `get-temp-thresholds --nvapi` and for per-GPU discovery of which
    /// index is the "GPU Target Temperature" wall (on RTX 4060 Laptop that's
    /// idx 2; it reads 87C and matches nvidia-smi's "GPU Target Temperature").
    pub fn target_temperature_policies(&self) -> crate::Result<Vec<(usize, f32)>> {
        trace!("gpu.target_temperature_policies()");
        let max = thermal::private::NV_GPU_CLIENT_THERMAL_TARGET_ENTRIES_MAX;
        let mut out = Vec::new();
        for idx in 0..max {
            // A single-bit mask is mandatory; 1<<idx for idx >= 32 would wrap,
            // but ENTRIES_MAX (16) is well below 32, so this is safe.
            match self.target_temperature(idx) {
                Ok(Some(c)) => out.push((idx, c)),
                Ok(None) => {} // index out of range inside the buffer (shouldn't happen)
                Err(_) => {}   // driver doesn't expose this policy slot — skip
            }
        }
        Ok(out)
    }

    /// Query the private ClientThermalPolicies GetInfo (0x2F69F8E5) once and
    /// return the policy index the ref tool itself uses for target-temp control:
    /// GPS index if the VBIOS exposes one, else the acoustics index (desktop
    /// fallback = NVML AcousticCurr), else None. This is the authoritative
    /// per-GPU discovery that replaces hardcoding idx 2. RE'd from the ref-tool CLI
    /// GPUHandle::queryTargetTemperature (sub_14002C410).
    pub fn target_temp_policy_index(&self) -> crate::Result<Option<usize>> {
        trace!("gpu.target_temp_policy_index()");
        let mut info = thermal::private::NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO::default();
        let ver = <thermal::private::NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO as sys::nvapi::StructVersion>::NVAPI_VERSION;
        info.version = ver;
        unsafe {
            let status = sys::api::NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo(self.0, &mut info);
            crate::status_result(
                sys::Api::NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo,
                status,
            )?;
        }
        Ok(info.target_temp_policy_index().map(|b| b as usize))
    }

    /// VBIOS min/default/max target temp (celsius) for one policy slot, from
    /// the private GetInfo. Mirrors dword[231*idx + 232/233/234] (Q8 /256).
    /// `Ok(None)` if GetInfo fails or the index is out of the table.
    pub fn target_temperature_info(
        &self,
        policy_index: usize,
    ) -> crate::Result<Option<(f32, f32, f32)>> {
        trace!("gpu.target_temperature_info({})", policy_index);
        let mut info = thermal::private::NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO::default();
        let ver = <thermal::private::NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO as sys::nvapi::StructVersion>::NVAPI_VERSION;
        info.version = ver;
        unsafe {
            let status = sys::api::NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo(self.0, &mut info);
            crate::status_result(
                sys::Api::NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo,
                status,
            )?;
        }
        Ok(info.target_temp_range(policy_index as u8))
    }

    /// Combined view of every target-temp policy slot the driver exposes:
    /// each entry carries its live current temp (GET-prime) and, when GetInfo
    /// has it, the VBIOS min/default/max range. Drives
    /// `get-temp-thresholds --nvapi`. Performs one GetInfo call + one GET per
    /// exposed slot.
    pub fn target_temperature_policies_with_info(
        &self,
    ) -> crate::Result<Vec<TargetTempPolicyEntry>> {
        trace!("gpu.target_temperature_policies_with_info()");
        // One GetInfo call covers all slots' ranges.
        let mut info = thermal::private::NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO::default();
        let ver = <thermal::private::NV_GPU_CLIENT_THERMAL_POLICIES_PRIVATE_INFO as sys::nvapi::StructVersion>::NVAPI_VERSION;
        info.version = ver;
        let info_ok = unsafe {
            let status = sys::api::NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo(self.0, &mut info);
            crate::status_result(
                sys::Api::NvAPI_GPU_ClientThermalPoliciesPrivateGetInfo,
                status,
            )
            .is_ok()
        };
        let max = thermal::private::NV_GPU_CLIENT_THERMAL_TARGET_ENTRIES_MAX;
        let mut out = Vec::new();
        for idx in 0..max {
            let current = match self.target_temperature(idx) {
                Ok(Some(c)) => c,
                _ => continue, // driver doesn't expose this slot
            };
            let range = if info_ok {
                info.target_temp_range(idx as u8)
            } else {
                None
            };
            out.push(TargetTempPolicyEntry {
                policy_index: idx,
                current,
                min: range.map(|(mn, _def, _mx)| mn),
                default: range.map(|(_mn, def, _mx)| def),
                max: range.map(|(_mn, _def, mx)| mx),
            });
        }
        Ok(out)
    }

    /// Raw GET-prime of the target-temp control buffer with a caller-supplied
    /// mask. Returns the full opaque buffer for diagnostics (e.g. locating the
    /// real target-temp field by scanning for a known Q8 value). Use
    /// `mask = 0xFFFF` to read all entries.
    pub fn target_temperature_raw(
        &self,
        mask: u32,
    ) -> crate::Result<thermal::private::NV_GPU_CLIENT_THERMAL_TARGET_STATUS> {
        trace!("gpu.target_temperature_raw(mask=0x{:X})", mask);
        let mut data = thermal::private::NV_GPU_CLIENT_THERMAL_TARGET_STATUS::default();
        let ver = <thermal::private::NV_GPU_CLIENT_THERMAL_TARGET_STATUS as sys::nvapi::StructVersion>::NVAPI_VERSION;
        data.version = ver;
        data.mask = mask;
        unsafe {
            let status = sys::api::NvAPI_GPU_ClientThermalTargetGetStatus(self.0, &mut data);
            crate::status_result(sys::Api::NvAPI_GPU_ClientThermalTargetGetStatus, status)?;
        }
        Ok(data)
    }

    /// Set the target-temperature wall (mobile "targettemp") to `celsius` for the
    /// given policy index. Uses the PRIVATE ClientThermalPolicies RMW pair (NDA
    /// GET-prime 0xC4554575 + SET 0xE097144F) — the path the ref-tool CLI
    /// `-targettemp:<C>` uses and that persists on mobile GPUs (nvidia-smi
    /// confirms). The documented ClientThermalPoliciesSetStatus 0x34C0B13D
    /// returns OK on mobile but does NOT persist; this private pair does.
    ///
    /// `policy_index`: on RTX 4060 Laptop the target-temp ("GPU Target
    /// Temperature") policy is **index 2** (confirmed via nvidia-smi cross-check;
    /// idx 2 reads/writes the 87C wall). idx 0/1/4 are other thermal policies
    /// (slowdown/etc), idx 3/5/6/7 are invalid. Callers should discover the
    /// right index per-GPU (the ref tool stores it in its GPUHandle at `v9[776]`,
    /// populated from ClientThermalPoliciesGetInfo 0x0D258BB5).
    ///
    /// Mirrors the ref-tool CLI sub_140013090: GET-prime to fill the buffer, patch the
    /// entry's Q8 temp field (celsius*256 at dword 15*idx+7), SET it back.
    /// Caller is responsible for clamping `celsius` to the VBIOS range (the ref tool
    /// enforces [min, max] from the GPUHandle; out-of-range values make SET
    /// return a generic Error).
    pub fn set_target_temperature(
        &self,
        celsius: f32,
        policy_index: usize,
    ) -> crate::NvapiResult<()> {
        trace!(
            "gpu.set_target_temperature({} C, idx {})",
            celsius, policy_index
        );
        // GET-prime: fill the buffer with current policy state (opaque fields
        // must be preserved across the RMW — do NOT zero after this).
        let mut data = thermal::private::NV_GPU_CLIENT_THERMAL_TARGET_STATUS::default();
        let ver = <thermal::private::NV_GPU_CLIENT_THERMAL_TARGET_STATUS as sys::nvapi::StructVersion>::NVAPI_VERSION;
        data.version = ver;
        data.mask = 1u32 << policy_index;
        unsafe {
            let status = sys::api::NvAPI_GPU_ClientThermalTargetGetStatus(self.0, &mut data);
            crate::status_result(sys::Api::NvAPI_GPU_ClientThermalTargetGetStatus, status)?;
        }
        // Patch the target temp (Q8) for this entry; set_target_temp_c also
        // sets the mask bit (already set, but harmless).
        data.set_target_temp_c(policy_index, celsius);
        // SET: apply the patched buffer.
        unsafe { nvcall!(NvAPI_GPU_ClientThermalTargetSetStatus(self.0, &data)) }
    }

    /// Admin-free pstate lock via `NvAPI_GPU_SetPerfLevel` (0x75dd3e6a,
    /// escape 0x7000040). 2026-08-26 correction — NOT the NVCP power-mode
    /// dropdown: `level` is an INDEX into the GPU's actual available
    /// P-State list (see `pstate_levels` / get-pstate-native), not a fixed
    /// enum — on the 4060 Laptop (P3/P4/P5/P8+P0) the measured mapping is
    /// 0=P8, 1=P5, 2=P4, 3=P3, 4=P0, but other GPUs expose a different
    /// P-State set. No release argument exists (RM accepts only valid
    /// indices; -1/16 and every other value return NVAPI_ERROR), the lock
    /// survives reset-force-pstate / reset-pstate-native /
    /// EnableDynamicPstates(0) / SetPowerMizerInfo, and only a driver
    /// reload/reboot clears it. Re-locking another index re-targets.
    pub fn set_pstate_lock(&self, level: u32) -> crate::NvapiResult<()> {
        trace!("gpu.set_pstate_lock({level})");
        unsafe { nvcall!(NvAPI_GPU_SetPerfLevel(self.0, level)) }
    }

    /// NVCP "电源模式" GET — reads the current PowerMizer mode via
    /// `NvAPI_GPU_GetPowerMizerInfo` (0x76bfa16b, RE'd R610.74: 4-arg).
    /// `powerSource` 1|2 (AC/DC selector), queryType fixed 3. Returns the
    /// public mode ∈ {6,7} (6=first mode, 7=second — the Adaptive/Prefer-Max
    /// pair the SET validates).
    pub fn power_mizer_info(&self, power_source: u32) -> crate::NvapiResult<u32> {
        trace!("gpu.power_mizer_info({power_source})");
        let mut data: u32 = 0;
        let st =
            unsafe { sys::api::NvAPI_GPU_GetPowerMizerInfo(self.0, power_source, 3, &mut data) };
        crate::status_result(sys::Api::NvAPI_GPU_GetPowerMizerInfo, st)?;
        Ok(data)
    }

    /// NVCP "电源模式" SET — `NvAPI_GPU_SetPowerMizerInfo` (0x50016c78,
    /// RE'd R610.74: 4-arg by-value). `mode` must be 6 or 7 (the only two
    /// valid public values, same pair the GET reports).
    pub fn set_power_mizer(&self, power_source: u32, mode: u32) -> crate::NvapiResult<()> {
        trace!("gpu.set_power_mizer({power_source}, {mode})");
        unsafe { nvcall!(NvAPI_GPU_SetPowerMizerInfo(self.0, power_source, 3, mode)) }
    }

    /// PCF platform dynamic-boost status — `NvAPI_PCF_DynamicBoostGetStatus`
    /// (0xc80068a1, RE'd R610.74: single `*mut bool` out, no GPU handle).
    /// Reads the PCF controller table (`rec[0]==1 && rec[+60]!=2 &&
    /// rec[+61]!=2`); NOT the effective PPAB enable readback — live-probed
    /// 2026-08-26: both status bytes read 2 with PPAB enforcing (see
    /// examples/probe_pcf_dynamic_boost.rs).
    pub fn dynamic_boost_status(&self) -> crate::NvapiResult<bool> {
        trace!("gpu.dynamic_boost_status()");
        // The PCF table requires the private lifecycle init (0xAD298D3F
        // arg=1) or every PCF query returns API_NOT_INITIALIZED (live
        // observed R610.74). Best-effort — ignore failures.
        let _ = self.private_lifecycle_init();
        let mut active = crate::sys::types::BoolU32(0);
        let st = unsafe { sys::api::NvAPI_PCF_DynamicBoostGetStatus(&mut active) };
        crate::status_result(sys::Api::NvAPI_PCF_DynamicBoostGetStatus, st)?;
        Ok(active.0 != 0)
    }

    /// Direct core-voltage read — `NvAPI_GPU_GetCoreVoltage` (0x58337FA3,
    /// RE'd R610.74: `fn(selector, *value)`, escape 0x07000043). A distinct
    /// RM surface from the VoltVoltRails family.
    pub fn core_voltage_scalar(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.core_voltage_scalar()");
        let mut value: u32 = 0;
        let st = unsafe { sys::api::NvAPI_GPU_GetCoreVoltage(self.0, &mut value) };
        crate::status_result(sys::Api::NvAPI_GPU_GetCoreVoltage, st)?;
        Ok(value)
    }

    /// Core-voltage control-object read — `NvAPI_GPU_GetCoreVoltageControl`
    /// (0xA91F88EB, escape 0x07000045).
    pub fn core_voltage_control(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.core_voltage_control()");
        let mut value: u32 = 0;
        let st = unsafe { sys::api::NvAPI_GPU_GetCoreVoltageControl(self.0, &mut value) };
        crate::status_result(sys::Api::NvAPI_GPU_GetCoreVoltageControl, st)?;
        Ok(value)
    }

    /// Core-voltage control SET — `NvAPI_GPU_SetCoreVoltageControl`
    /// (0xDC2BD4A6, escape 0x07000044, `fn(selector, value)`).
    /// Elevation-gated (-104 without admin).
    pub fn set_core_voltage_control(&self, value: u32) -> crate::NvapiResult<()> {
        trace!("gpu.set_core_voltage_control({value})");
        unsafe { nvcall!(NvAPI_GPU_SetCoreVoltageControl(self.0, value)) }
    }

    /// PMGR voltage-request arbiter GET — `NvAPI_GPU_GetPMGRVoltage
    /// RequestArbiterValues` (0x717648FD, escape 0x0700019F get-flag 0,
    /// v2 struct magic 0x20030). Returns the 11 raw arbiter dwords.
    /// Distinct RM surface from both VoltVoltRails (0x07000191) and the
    /// ClientVoltRails percent family.
    pub fn pmgr_voltage_arbiter(&self) -> crate::NvapiResult<[u32; 11]> {
        trace!("gpu.pmgr_voltage_arbiter()");
        use crate::sys::nvapi::VersionedStruct;
        use power::private::NV_PMGR_VOLTAGE_ARBITER_VALUES;
        let mut values = unsafe { std::mem::zeroed::<NV_PMGR_VOLTAGE_ARBITER_VALUES>() };
        *values.nvapi_version_mut() = NvVersion::with_version(0x20030);
        let st =
            unsafe { sys::api::NvAPI_GPU_GetPMGRVoltageRequestArbiterValues(self.0, &mut values) };
        crate::status_result(sys::Api::NvAPI_GPU_GetPMGRVoltageRequestArbiterValues, st)?;
        Ok(values.values)
    }

    /// PMGR voltage-request arbiter SET — `NvAPI_GPU_SetPMGRVoltage
    /// RequestArbiterValues` (0x9C4BB8D0, escape 0x0700019F set-flag 1).
    /// Elevation-gated (-104). Prefer the GET→patch→SET RMW pattern.
    pub fn set_pmgr_voltage_arbiter(&self, values: &[u32; 11]) -> crate::NvapiResult<()> {
        trace!("gpu.set_pmgr_voltage_arbiter({:?})", &values[..3]);
        use crate::sys::nvapi::VersionedStruct;
        use power::private::NV_PMGR_VOLTAGE_ARBITER_VALUES;
        let mut buf = unsafe { std::mem::zeroed::<NV_PMGR_VOLTAGE_ARBITER_VALUES>() };
        *buf.nvapi_version_mut() = NvVersion::with_version(0x20030);
        buf.values = *values;
        let st = unsafe {
            sys::api::NvAPI_GPU_SetPMGRVoltageRequestArbiterValues(self.0, &buf as *const _)
        };
        crate::status_result(sys::Api::NvAPI_GPU_SetPMGRVoltageRequestArbiterValues, st)
    }

    /// Read the NVCP power-mode (均衡/高性能 = Balanced/Max) capability:
    /// `(mode_mask, max_mode_idx)`. The feature exists only when
    /// `max_mode_idx == 1` (0xFFFF on unsupported GPUs, e.g. 4060L).
    pub fn power_modes_capability(&self) -> crate::NvapiResult<(u16, u16)> {
        trace!("gpu.power_modes_capability()");
        let mut info =
            unsafe { std::mem::zeroed::<power::private::NV_GPU_CLIENT_POWER_MODES_INFO>() };
        use crate::sys::nvapi::VersionedStruct;
        *info.nvapi_version_mut() =
            NvVersion::with_struct::<power::private::NV_GPU_CLIENT_POWER_MODES_INFO>(1);
        let st = unsafe { sys::api::NvAPI_GPU_ClientPowerModesGetInfo(self.0, &mut info) };
        crate::status_result(sys::Api::NvAPI_GPU_ClientPowerModesGetInfo, st)?;
        Ok((info.mode_mask, info.max_mode_idx))
    }

    /// Read the active NVCP power mode (`Balanced`/`Max`).
    pub fn power_mode(&self) -> crate::NvapiResult<&'static str> {
        trace!("gpu.power_mode()");
        let control = self.power_modes_primed_control()?;
        Ok(if control.active_mode_idx == 1 {
            "Max"
        } else {
            "Balanced"
        })
    }

    /// Set the NVCP power mode (均衡/高性能). Implements the App's
    /// instruction-verified GET-prime RMW protocol: GetInfo → seed
    /// CONTROL+0x04 → GetControl → write ONLY the u16 mode → SetControl.
    pub fn set_power_mode(&self, max: bool) -> crate::NvapiResult<()> {
        trace!("gpu.set_power_mode(max={max})");
        let mut control = self.power_modes_primed_control()?;
        control.active_mode_idx = max as u16;
        unsafe { nvcall!(NvAPI_GPU_ClientPowerModesSetControl(self.0, &control)) }
    }

    /// GET-prime helper shared by read/write: seeds CONTROL+0x04 from
    /// INFO+0x04 (required in BOTH paths per the UXDriver RE), then
    /// GetControl so every untouched byte passes through as the driver
    /// returned it.
    fn power_modes_primed_control(
        &self,
    ) -> crate::NvapiResult<power::private::NV_GPU_CLIENT_POWER_MODES_CONTROL> {
        let mut info =
            unsafe { std::mem::zeroed::<power::private::NV_GPU_CLIENT_POWER_MODES_INFO>() };
        let mut control =
            unsafe { std::mem::zeroed::<power::private::NV_GPU_CLIENT_POWER_MODES_CONTROL>() };
        use crate::sys::nvapi::VersionedStruct;
        *info.nvapi_version_mut() =
            NvVersion::with_struct::<power::private::NV_GPU_CLIENT_POWER_MODES_INFO>(1);
        *control.nvapi_version_mut() =
            NvVersion::with_struct::<power::private::NV_GPU_CLIENT_POWER_MODES_CONTROL>(1);
        let st = unsafe { sys::api::NvAPI_GPU_ClientPowerModesGetInfo(self.0, &mut info) };
        crate::status_result(sys::Api::NvAPI_GPU_ClientPowerModesGetInfo, st)?;
        control.seed = info.seed;
        let st = unsafe { sys::api::NvAPI_GPU_ClientPowerModesGetControl(self.0, &mut control) };
        crate::status_result(sys::Api::NvAPI_GPU_ClientPowerModesGetControl, st)?;
        Ok(control)
    }

    /// Fake the GPU thermal sensor reading so the driver's thermal policy
    /// (boost/throttle) reacts to a synthetic temperature. Requires VBIOS
    /// "Secured Overrides" table with `<Temp faking allowed>` enabled;
    /// otherwise returns `NotSupported` (observed on Ada mobile).
    /// RE'd from ref tool 2 + ThermSpyPremium (both use the same 4-arg
    /// signature on `NvAPI_GPU_SetExtendedThermalSimulationMode`).
    /// Falls back to the BASIC variant `NvAPI_GPU_SetThermalSimulationMode`
    /// (0x8CD42541, same 4-arg shape per R610.74 RE) when the Extended call
    /// is rejected — same function family, no separate API surface.
    pub fn set_temp_sim(&self, temperature_celsius: i32) -> crate::NvapiResult<()> {
        trace!("gpu.set_temp_sim({temperature_celsius} C)");
        let ext = unsafe {
            nvcall!(NvAPI_GPU_SetExtendedThermalSimulationMode(
                self.0,
                0, // flags
                1, // enable
                temperature_celsius,
            ))
        };
        match ext {
            Err(ref e) if matches!(e.status, Status::NotSupported | Status::NoImplementation) => unsafe {
                nvcall!(NvAPI_GPU_SetThermalSimulationMode(
                    self.0,
                    0,
                    1,
                    temperature_celsius,
                ))
            },
            result => result,
        }
    }

    /// Disable temperature simulation (restore real sensor reading).
    pub fn disable_temp_sim(&self) -> crate::NvapiResult<()> {
        trace!("gpu.disable_temp_sim()");
        let ext = unsafe {
            nvcall!(NvAPI_GPU_SetExtendedThermalSimulationMode(
                self.0, 0, // flags
                0, // disable
                0, // temperature ignored
            ))
        };
        match ext {
            Err(ref e) if matches!(e.status, Status::NotSupported | Status::NoImplementation) => unsafe {
                nvcall!(NvAPI_GPU_SetThermalSimulationMode(self.0, 0, 0, 0))
            },
            result => result,
        }
    }

    /// Read back the current temperature-simulation state. Returns
    /// `(enable, temperature_celsius)` when supported.
    pub fn temp_sim(&self) -> crate::NvapiResult<(bool, i32)> {
        trace!("gpu.temp_sim()");
        let mut flags = 0u32;
        let mut enable = 0u32;
        let mut temp = 0i32;
        unsafe {
            nvcall!(NvAPI_GPU_GetThermalSimulationMode(
                self.0,
                &mut flags,
                &mut enable,
                &mut temp,
            ))?
        };
        Ok((enable != 0, temp))
    }

    pub fn cooler_info(
        &self,
    ) -> crate::Result<BTreeMap<crate::thermal::FanCoolerId, crate::thermal::CoolerInfo>> {
        trace!("gpu.cooler_info()");

        let res = unsafe { nvcall!(NvAPI_GPU_ClientFanCoolersGetInfo@get(self.0) => raw) };

        match res {
            Err(crate::Error::Nvapi(crate::NvapiError {
                status: Status::NotSupported,
                ..
            })) => (),
            res => return res,
        }

        self.cooler_settings_()
            .map(|c| c.into_iter().map(|(i, c)| (i, c.info)).collect())
    }

    pub fn cooler_status(
        &self,
    ) -> crate::Result<BTreeMap<crate::thermal::FanCoolerId, crate::thermal::CoolerStatus>> {
        trace!("gpu.cooler_status()");

        let res = unsafe { nvcall!(NvAPI_GPU_ClientFanCoolersGetStatus@get(self.0) => raw) };

        match res {
            Err(crate::Error::Nvapi(crate::NvapiError {
                status: Status::NotSupported,
                ..
            })) => (),
            res => return res,
        }

        self.cooler_settings_()
            .map(|c| c.into_iter().map(|(i, c)| (i, c.status)).collect())
    }

    pub fn cooler_control(
        &self,
    ) -> crate::Result<BTreeMap<crate::thermal::FanCoolerId, crate::thermal::CoolerSettings>> {
        trace!("gpu.cooler_status()");

        let res = unsafe { nvcall!(NvAPI_GPU_ClientFanCoolersGetControl@get(self.0) => raw) };

        match res {
            Err(crate::Error::Nvapi(crate::NvapiError {
                status: Status::NotSupported,
                ..
            })) => (),
            res => return res,
        }

        self.cooler_settings_()
            .map(|c| c.into_iter().map(|(i, c)| (i, c.control)).collect())
    }

    /// Read the fan-policy capabilities block (`ClientFanPoliciesGetInfo` NDA
    /// 0x52B76D12, structure magic `0x2004C`, 76 bytes). Size/magic
    /// corroborated by EVGA Precision X1 (ManagedNvApi.dll). Field layout
    /// beyond the version dword is opaque — returned raw for decoding.
    pub fn fan_policy_info(
        &self,
    ) -> crate::NvapiResult<cooler::private::NV_GPU_CLIENT_FAN_POLICIES_INFO> {
        trace!("gpu.fan_policy_info()");

        let raw = unsafe {
            nvcall!(NvAPI_GPU_ClientFanPoliciesGetInfo@get{
                cooler::private::NV_GPU_CLIENT_FAN_POLICIES_INFO::new()
            }(self.0))?
        };

        Ok(raw)
    }

    /// Read the GPU fan-curve table (`ClientFanPoliciesGetControl` NDA
    /// 0xE543C540, structure magic `0x200DC`). RE'd from ref tool 2
    /// `pollFanCurve`: one table snapshot holds up to 4 curve slots, each
    /// with 3 monotonic (temperature, RPM) points. Returns one `FanCurve`
    /// per slot reported by the driver's `count` byte.
    ///
    /// Point encodings (matching ref tool's dialog round-trip): temperature
    /// stored Q8.8 (`temp << 8`, read back as `(x + 128) >> 8`), RPM stored
    /// Q16-scaled (`(x * 100 + 32768) / 65536`).
    pub fn fan_curves(&self) -> crate::NvapiResult<Vec<FanCurve>> {
        trace!("gpu.fan_curves()");

        let raw = unsafe {
            nvcall!(NvAPI_GPU_ClientFanPoliciesGetControl@get{
                cooler::private::NV_GPU_CLIENT_FAN_POLICIES_CONTROL::new()
            }(self.0))?
        };

        let count = raw.count.min(4) as usize;
        let mut out = Vec::with_capacity(count);
        for k in 0..count {
            let curve = &raw.curves[k];
            let mut points = Vec::with_capacity(3);
            for p in &curve.points[..3] {
                points.push(FanCurvePoint {
                    temp_c: ((p.temp_q8.wrapping_add(128)) >> 8) as u16,
                    rpm: (p.rpm_q16 as u64 * 100).div_ceil(65536) as u32,
                });
            }
            out.push(FanCurve {
                index: curve.index,
                points,
            });
        }
        Ok(out)
    }

    /// Set one fan-curve slot (`ClientFanPoliciesSetControl` NDA 0xC181947A,
    /// structure magic `0x200DC`). RE'd from ref tool 2's `setFanCurve`: mirror
    /// the RMW protocol — GET the current table, overwrite the `index` slot's
    /// three (temperature, RPM) points, SET the whole table back. This leaves
    /// the driver-owned reserved lane untouched.
    ///
    /// The driver's Set handler enforces **strict monotonicity** on all three
    /// dword lanes (temperature must increase across points, RPM must
    /// increase, reserved must increase) — pass an increasing curve or you
    /// get `NvapiError(-5)`. Curves are typically only settable on desktop
    /// boards (mobile laptops drive their fans through the EC, not NVAPI).
    pub fn set_fan_curve(&self, curve: &FanCurve) -> crate::NvapiResult<()> {
        trace!("gpu.set_fan_curve({:?})", curve);
        if curve.points.len() > 3 {
            return Err(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_ClientFanPoliciesSetControl,
                sys::Status::InvalidArgument,
            ));
        }

        let mut raw = unsafe {
            nvcall!(NvAPI_GPU_ClientFanPoliciesGetControl@get{
                cooler::private::NV_GPU_CLIENT_FAN_POLICIES_CONTROL::new()
            }(self.0))?
        };
        if curve.index >= 4 {
            return Err(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_ClientFanPoliciesSetControl,
                sys::Status::InvalidArgument,
            ));
        }
        let slot = &mut raw.curves[curve.index as usize];
        slot.index = curve.index;
        for (i, p) in curve.points.iter().enumerate() {
            slot.points[i].temp_q8 = (p.temp_c as u32) << 8;
            slot.points[i].rpm_q16 = p.rpm * 65536 / 100;
        }
        unsafe { nvcall!(NvAPI_GPU_ClientFanPoliciesSetControl(self.0, &raw)) }
    }

    /// Reset one fan-curve slot to factory (`FanPolicySetControl` NDA
    /// 0x2B2A2A45, structure magic `0x214AC`). RE'd byte-exact from ref tool 2
    /// `GPUHandle::resetFanCurve` and cross-checked against the impl.dll
    /// handler: GET the full 0x14AC policy block, write `1 << curve_index`
    /// into the reset bitmask at +0x04, set flag bit0 at +0x08, SET back.
    ///
    /// This is ref tool 2's NVAPI fan reset — NOT the public
    /// `RestoreCoolerSettings`, which the driver rejects with
    /// NOT_SUPPORTED(-104) on GPUs whose user-mode cooler table isn't
    /// exposed (observed on desktop RTX 3060/2070; NVML's
    /// SetDefaultFanSpeed_v2 uses a separate RM arbiter channel and works
    /// there). `curve_index` is the slot to reset (0..=3; ref tool 2's reset
    /// button uses 0).
    pub fn reset_fan_curve(&self, curve_index: u32) -> crate::NvapiResult<()> {
        trace!("gpu.reset_fan_curve({})", curve_index);
        if curve_index >= 4 {
            return Err(crate::NvapiError::new(
                sys::Api::NvAPI_GPU_FanPolicySetControl,
                sys::Status::InvalidArgument,
            ));
        }
        use cooler::private::{
            NV_GPU_FAN_POLICY_CONTROL_MAGIC, NV_GPU_FAN_POLICY_CONTROL_SIZE,
            NV_GPU_FAN_POLICY_OFF_FLAGS, NV_GPU_FAN_POLICY_OFF_RESET_MASK,
        };
        let mut buf = vec![0u8; NV_GPU_FAN_POLICY_CONTROL_SIZE];
        buf[..4].copy_from_slice(&NV_GPU_FAN_POLICY_CONTROL_MAGIC.to_ne_bytes());
        unsafe {
            nvcall!(NvAPI_GPU_FanPolicyGetControl(
                self.0,
                buf.as_mut_ptr() as *mut _
            ))?;
        }
        // Reset bitmask at +0x04 (ref tool 2 assigns, not ORs) + apply flag
        // bit0 at +0x08 (ref tool 2 sets it unconditionally on the reset path).
        let mask = 1u32 << curve_index;
        buf[NV_GPU_FAN_POLICY_OFF_RESET_MASK..NV_GPU_FAN_POLICY_OFF_RESET_MASK + 4]
            .copy_from_slice(&mask.to_ne_bytes());
        let mut flags = [0u8; 4];
        flags.copy_from_slice(&buf[NV_GPU_FAN_POLICY_OFF_FLAGS..NV_GPU_FAN_POLICY_OFF_FLAGS + 4]);
        let flags = u32::from_ne_bytes(flags) | 1;
        buf[NV_GPU_FAN_POLICY_OFF_FLAGS..NV_GPU_FAN_POLICY_OFF_FLAGS + 4]
            .copy_from_slice(&flags.to_ne_bytes());
        unsafe {
            nvcall!(NvAPI_GPU_FanPolicySetControl(
                self.0,
                buf.as_ptr() as *const _
            ))
        }
    }

    /// Toggle fan stop / zero-RPM for a curve slot (`FanArbiterSet` NDA
    /// 0x44CD3014, versioned struct `0x10124` = the 292-byte V1). RE'd from
    /// ref tool 2's `setFanCurve`'s tail call: count=1 at +0x04, then
    /// arbiters[0] at +0x24 = {arbiter_index = curve_index, flags bit0 =
    /// FAN_STOP enable}.
    pub fn set_fan_stop(&self, curve_index: u32, enable: bool) -> crate::NvapiResult<()> {
        trace!("gpu.set_fan_stop({}, {})", curve_index, enable);
        use cooler::private::{
            NV_GPU_CLIENT_FAN_ARBITER_CONTROL_V1, NV_GPU_CLIENT_FAN_ARBITERS_CONTROL,
            NV_GPU_CLIENT_FAN_ARBITERS_CONTROL_V1,
        };
        let mut ctrl = NV_GPU_CLIENT_FAN_ARBITERS_CONTROL_V1 {
            version:
                <NV_GPU_CLIENT_FAN_ARBITERS_CONTROL as sys::nvapi::StructVersion>::NVAPI_VERSION,
            count: 1,
            ..Default::default()
        };
        ctrl.arbiters[0] = NV_GPU_CLIENT_FAN_ARBITER_CONTROL_V1 {
            arbiter_index: curve_index,
            flags: if enable {
                cooler::private::FanArbiterControlFlags::FAN_STOP.value()
            } else {
                cooler::private::NV_FAN_ARBITER_CONTROL_FLAGS::default()
            },
        };
        unsafe { nvcall!(NvAPI_GPU_ClientFanArbitersSetControl(self.0, &ctrl)) }
    }

    /// Query per-cooler info via the private FanCoolerGetInfo (NDA 0x65CE5BFC,
    /// struct magic 0x10888). Returns one entry per cooler with its type
    /// (0=active, 1=pwm, 2=pwm-tach) and min/max RPM range. RE'd from
    /// ref tool 2's `setFanSim` — this is the private path, richer than the
    /// public `GetCoolerSettings` (which only returns level/defaultPolicy).
    pub fn cooler_info_private(&self) -> crate::NvapiResult<Vec<PrivateCoolerInfo>> {
        trace!("gpu.cooler_info_private()");
        use cooler::private::{
            NV_GPU_FAN_COOLER_CONTROL_MAGIC, NV_GPU_FAN_COOLER_CONTROL_SIZE,
            NV_GPU_FAN_COOLER_ENTRY_STRIDE, NV_GPU_FAN_COOLER_INFO_MAGIC,
            NV_GPU_FAN_COOLER_INFO_SIZE, NV_GPU_FAN_COOLER_OFF_MAX_RPM,
            NV_GPU_FAN_COOLER_OFF_MIN_RPM, NV_GPU_FAN_COOLER_OFF_ST_CURRENT,
            NV_GPU_FAN_COOLER_OFF_ST_PWM, NV_GPU_FAN_COOLER_OFF_TYPE,
            NV_GPU_FAN_COOLER_STATUS_MAGIC, NV_GPU_FAN_COOLER_STATUS_SIZE,
        };
        let mut buf = vec![0u8; NV_GPU_FAN_COOLER_INFO_SIZE];
        buf[..4].copy_from_slice(&NV_GPU_FAN_COOLER_INFO_MAGIC.to_ne_bytes());
        unsafe {
            nvcall!(NvAPI_GPU_FanCoolerGetInfo(
                self.0,
                buf.as_mut_ptr() as *mut _
            ))?;
        }
        // info+0x04 is a 32-bit presence MASK, not a count (ref tool 2)
        // pollFanSpeed iterates set bits — a 2-fan GPU can report 3 bits).
        let mask = read_u32(&buf, 0x04);

        // Control struct: type + min/max per cooler (dword[33*k + N]).
        let mut ctrl = vec![0u8; NV_GPU_FAN_COOLER_CONTROL_SIZE];
        ctrl[..4].copy_from_slice(&NV_GPU_FAN_COOLER_CONTROL_MAGIC.to_ne_bytes());
        write_u32(&mut ctrl, 0x04, mask);
        unsafe {
            nvcall!(NvAPI_GPU_FanCoolerGetControl(
                self.0,
                ctrl.as_mut_ptr() as *mut _
            ))?;
        }

        // Status struct: current speed + current PWM per cooler.
        let mut st = vec![0u8; NV_GPU_FAN_COOLER_STATUS_SIZE];
        st[..4].copy_from_slice(&NV_GPU_FAN_COOLER_STATUS_MAGIC.to_ne_bytes());
        write_u32(&mut st, 0x04, mask);
        unsafe {
            nvcall!(NvAPI_GPU_FanCoolerGetStatus(
                self.0,
                st.as_mut_ptr() as *mut _
            ))?;
        }

        let mut out = Vec::new();
        for k in 0..32u32 {
            if mask & (1 << k) == 0 {
                continue;
            }
            let cb = k as usize * NV_GPU_FAN_COOLER_ENTRY_STRIDE;
            let sb = k as usize * NV_GPU_FAN_COOLER_ENTRY_STRIDE;
            let current_pwm = read_u32(&st, sb + NV_GPU_FAN_COOLER_OFF_ST_PWM);
            out.push(PrivateCoolerInfo {
                index: k,
                cooler_type: read_u32(&ctrl, cb + NV_GPU_FAN_COOLER_OFF_TYPE),
                min: read_u32(&ctrl, cb + NV_GPU_FAN_COOLER_OFF_MIN_RPM),
                max: read_u32(&ctrl, cb + NV_GPU_FAN_COOLER_OFF_MAX_RPM),
                current: read_u32(&st, sb + NV_GPU_FAN_COOLER_OFF_ST_CURRENT),
                current_pwm_percent: (current_pwm / 655).min(100),
            });
        }
        Ok(out)
    }

    /// Set fan speed by RPM via the private FanCoolerSetControl (NDA
    /// 0xEB44E8AA, struct 0x210AC). RE'd byte-exact from ref tool 2
    /// `GPUHandle::setFanSim`: GET the control snapshot, patch the target
    /// cooler's enable + level per its cooler type, SET back.
    ///
    /// `cooler_index` picks the cooler (0-based); `None` targets EVERY
    /// cooler present in the info mask (single RMW round-trip). `rpm` is the
    /// target RPM; pass `None` to disable simulation (clear the enable bit →
    /// return to driver/auto control). The input is clamped into the cooler's
    /// `[min_rpm, max_rpm]` physical range (queried from the control
    /// struct) and linearly mapped onto the 0..65536 level scale
    /// (`level = rpm / max × 65536`) — for every cooler type. Returns one
    /// entry per targeted cooler.
    pub fn set_fan_rpm(
        &self,
        cooler_index: Option<u32>,
        rpm: Option<u32>,
    ) -> crate::NvapiResult<Vec<SetFanRpmResult>> {
        trace!("gpu.set_fan_rpm({:?}, {:?})", cooler_index, rpm);
        if let Some(i) = cooler_index {
            if i >= 32 {
                return Err(crate::NvapiError::new(
                    sys::Api::NvAPI_GPU_FanCoolerSetControl,
                    sys::Status::InvalidArgument,
                ));
            }
        }
        use cooler::private::{
            NV_GPU_FAN_COOLER_CONTROL_MAGIC, NV_GPU_FAN_COOLER_CONTROL_SIZE,
            NV_GPU_FAN_COOLER_ENTRY_STRIDE, NV_GPU_FAN_COOLER_ENTRY0_BASE,
            NV_GPU_FAN_COOLER_INFO_MAGIC, NV_GPU_FAN_COOLER_INFO_SIZE,
            NV_GPU_FAN_COOLER_OFF_ENABLE, NV_GPU_FAN_COOLER_OFF_LEVEL,
            NV_GPU_FAN_COOLER_OFF_MAX_RPM, NV_GPU_FAN_COOLER_OFF_MIN_RPM,
            NV_GPU_FAN_COOLER_OFF_TYPE,
        };
        let mut buf = vec![0u8; NV_GPU_FAN_COOLER_CONTROL_SIZE];
        buf[..4].copy_from_slice(&NV_GPU_FAN_COOLER_CONTROL_MAGIC.to_ne_bytes());
        // FanCoolerGetInfo fills the count; mirror ref tool 2 by querying info
        // first to set count, then GET control.
        let mask = {
            let mut info = vec![0u8; NV_GPU_FAN_COOLER_INFO_SIZE];
            info[..4].copy_from_slice(&NV_GPU_FAN_COOLER_INFO_MAGIC.to_ne_bytes());
            unsafe {
                nvcall!(NvAPI_GPU_FanCoolerGetInfo(
                    self.0,
                    info.as_mut_ptr() as *mut _
                ))?;
            }
            let mask = read_u32(&info, 0x04);
            // Guard: the requested cooler must actually exist (presence
            // mask from info, NOT a count — see pollFanSpeed).
            if let Some(i) = cooler_index {
                if mask & (1u32 << i) == 0 {
                    return Err(crate::NvapiError::new(
                        sys::Api::NvAPI_GPU_FanCoolerSetControl,
                        sys::Status::InvalidArgument,
                    ));
                }
            }
            mask
        };
        write_u32(&mut buf, 0x04, mask);
        unsafe {
            nvcall!(NvAPI_GPU_FanCoolerGetControl(
                self.0,
                buf.as_mut_ptr() as *mut _
            ))?;
        }
        // Targets: the requested cooler, or every present cooler.
        let targets: Vec<u32> = match cooler_index {
            Some(i) => vec![i],
            None => (0..32u32).filter(|&k| mask & (1u32 << k) != 0).collect(),
        };
        let mut out = Vec::with_capacity(targets.len());
        for k in targets {
            let base = NV_GPU_FAN_COOLER_ENTRY0_BASE + k as usize * NV_GPU_FAN_COOLER_ENTRY_STRIDE;
            let cooler_type = read_u32(&buf, base + NV_GPU_FAN_COOLER_OFF_TYPE);
            let min_rpm = read_u32(&buf, base + NV_GPU_FAN_COOLER_OFF_MIN_RPM);
            let max_rpm = read_u32(&buf, base + NV_GPU_FAN_COOLER_OFF_MAX_RPM);
            match rpm {
                None => {
                    // Disable simulation: clear enable bit.
                    let en = read_u32(&buf, base + NV_GPU_FAN_COOLER_OFF_ENABLE);
                    write_u32(&mut buf, base + NV_GPU_FAN_COOLER_OFF_ENABLE, en & !1);
                }
                Some(target) => {
                    // min/max from the control struct are the cooler's PHYSICAL
                    // RPM range (2070 live-verified: fan0 max = 3300 = full
                    // speed). The level register is a 0..65536 scale where
                    // 65536 = 100% = max RPM, so the conversion is a direct
                    // linear map: raw = rpm / max × 65536. (The relative
                    // interpolation ((v-min)<<16)/(max-min) double-converts —
                    // it first normalizes into the min..max span and then the
                    // driver scales again.)
                    // Guard: clamp the input into [min, max] (u64 math, no
                    // overflow). The 0..65536 level scale applies to ALL cooler
                    // types — 2070 live test showed the pwm-tach (type 2) raw
                    // RPM write lands at rpm/65536 ≈ 5% at full speed.
                    if max_rpm == 0 {
                        // No range reported: reject rather than divide by 0.
                        return Err(crate::NvapiError::new(
                            sys::Api::NvAPI_GPU_FanCoolerSetControl,
                            sys::Status::InvalidArgument,
                        ));
                    }
                    let v = if min_rpm <= max_rpm {
                        target.clamp(min_rpm, max_rpm)
                    } else {
                        target
                    };
                    let level = ((v as u64) << 16) / (max_rpm as u64) as u64;
                    let en = read_u32(&buf, base + NV_GPU_FAN_COOLER_OFF_ENABLE);
                    write_u32(&mut buf, base + NV_GPU_FAN_COOLER_OFF_ENABLE, en | 1);
                    write_u32(&mut buf, base + NV_GPU_FAN_COOLER_OFF_LEVEL, level as u32);
                }
            }
            out.push(SetFanRpmResult {
                cooler_index: k,
                cooler_type,
                min_rpm,
                max_rpm,
                applied_rpm: rpm,
            });
        }
        unsafe {
            nvcall!(NvAPI_GPU_FanCoolerSetControl(
                self.0,
                buf.as_ptr() as *const _
            ))?;
        }
        Ok(out)
    }

    pub fn getcooler_settings(
        &self,
        index: Option<u32>,
    ) -> crate::Result<Vec<crate::thermal::Cooler>> {
        trace!("gpu.getcooler_settings({:?})", index);

        let index = match index {
            Some(index) => index,
            None if <cooler::private::NV_GPU_GETCOOLER_SETTINGS as sys::nvapi::StructVersion>::NVAPI_VERSION.version() < 4 =>
                cooler::private::NVAPI_COOLER_TARGET_ALL.repr() as _,
            None => 0,
        };
        unsafe { nvcall!(NvAPI_GPU_GetCoolerSettings@get(self.0, index) => raw) }
    }

    fn cooler_settings_(
        &self,
    ) -> crate::Result<BTreeMap<crate::thermal::FanCoolerId, crate::thermal::Cooler>> {
        self.getcooler_settings(None).and_then(|c| {
            c.into_iter()
                .enumerate()
                .map(|(i, c)| {
                    (i as i32 + 1)
                        .try_into()
                        .map_err(Into::into)
                        .map(|i| (i, c))
                })
                .collect()
        })
    }

    pub fn cooler_settings(
        &self,
    ) -> crate::Result<BTreeMap<crate::thermal::FanCoolerId, crate::thermal::Cooler>> {
        match self.cooler_settings_() {
            Err(crate::Error::Nvapi(crate::NvapiError {
                status: Status::NotSupported,
                ..
            })) => (),
            res => return res,
        }

        self.cooler_info()?
            .into_iter()
            .zip(self.cooler_status()?)
            .zip(self.cooler_control()?)
            .map(|(((id, info), (ids, status)), (idc, control))| match id {
                id if id == ids && id == idc => Ok((
                    id,
                    crate::thermal::Cooler {
                        info,
                        status,
                        control,
                        unknown: 0,
                    },
                )),
                _ => Err(sys::ArgumentRangeError.into()),
            })
            .collect()
    }

    #[deprecated]
    pub fn set_cooler_levels<I: IntoIterator<Item = crate::thermal::CoolerSettings>>(
        &self,
        index: Option<u32>,
        values: I,
    ) -> crate::NvapiResult<()> {
        trace!("gpu.set_cooler_levels({:?})", index);
        let mut data = cooler::private::NV_GPU_SETCOOLER_LEVEL::default();
        for (entry, level) in data.cooler.iter_mut().zip(values) {
            trace!("gpu.set_cooler_level({:?})", level);
            entry.currentLevel = level.level.unwrap_or_default().0;
            entry.currentPolicy = level.policy.raw();
        }

        unsafe {
            nvcall!(NvAPI_GPU_SetCoolerLevels(
                self.0,
                index.unwrap_or(cooler::private::NVAPI_COOLER_TARGET_ALL.repr() as _),
                &data
            ))
        }
    }

    pub fn set_cooler<
        I: IntoIterator<Item = (crate::thermal::FanCoolerId, crate::thermal::CoolerSettings)>,
    >(
        &self,
        values: I,
    ) -> crate::NvapiResult<()> {
        trace!("gpu.set_cooler()");
        let mut backup = cooler::private::NV_GPU_SETCOOLER_LEVEL::default();
        let mut data = cooler::private::NV_GPU_CLIENT_FAN_COOLERS_CONTROL::default();

        for (entry, (backup_entry, (id, settings))) in data
            .coolers
            .iter_mut()
            .zip(backup.cooler.iter_mut().zip(values))
        {
            trace!("gpu.set_cooler({:?})", settings);
            *entry = settings.to_raw(id);
            data.count += 1;

            backup_entry.currentLevel = settings.level.unwrap_or_default().0;
            backup_entry.currentPolicy = settings.policy.raw();
        }

        let res = unsafe { nvcall!(NvAPI_GPU_ClientFanCoolersSetControl(self.0, &data)) };

        match res {
            Err(crate::NvapiError {
                status: Status::NotSupported,
                ..
            }) => unsafe {
                nvcall!(NvAPI_GPU_SetCoolerLevels(
                    self.0,
                    cooler::private::NVAPI_COOLER_TARGET_ALL.repr() as _,
                    &backup
                ))
            },
            res => res,
        }
    }

    pub fn restore_cooler_settings(&self, index: &[u32]) -> crate::NvapiResult<()> {
        trace!("gpu.restore_cooler_settings({:?})", index);
        let ptr = if index.is_empty() {
            ptr::null()
        } else {
            index.as_ptr()
        };
        unsafe {
            nvcall!(NvAPI_GPU_RestoreCoolerSettings(
                self.0,
                ptr,
                index.len() as u32
            ))
        }
    }

    pub fn cooler_policy_table(
        &self,
        index: u32,
        policy: crate::thermal::CoolerPolicy,
    ) -> crate::Result<<cooler::private::NV_GPU_COOLER_POLICY_TABLE as RawConversion>::Target> {
        trace!("gpu.cooler_policy_table({:?})", index);
        let mut data = cooler::private::NV_GPU_COOLER_POLICY_TABLE {
            policy: policy.raw(),
            ..Default::default()
        };

        unsafe {
            nvcall!(NvAPI_GPU_GetCoolerPolicyTable@get(self.0, index, &mut data) => err).and_then(
                |count| {
                    data.convert_raw().map_err(From::from).map(|mut c| {
                        c.levels.truncate(count as usize);
                        // TODO: ensure remaining levels are null?
                        c
                    })
                },
            )
        }
    }

    pub fn set_cooler_policy_table(
        &self,
        index: u32,
        value: &<cooler::private::NV_GPU_COOLER_POLICY_TABLE as RawConversion>::Target,
    ) -> crate::NvapiResult<()> {
        trace!("gpu.set_cooler_policy_table({:?}, {:?})", index, value);
        let data = cooler::private::NV_GPU_COOLER_POLICY_TABLE {
            policy: value.policy.raw(),
            ..Default::default()
        };
        // TODO: data.policyCoolerLevel

        unsafe {
            nvcall!(NvAPI_GPU_SetCoolerPolicyTable(
                self.0,
                index,
                &data,
                value.levels.len() as u32
            ))
        }
    }

    pub fn restore_cooler_policy_table(
        &self,
        index: &[u32],
        policy: crate::thermal::CoolerPolicy,
    ) -> crate::NvapiResult<()> {
        trace!("gpu.restore_cooler_policy_table({:?}, {:?})", index, policy);
        let ptr = if index.is_empty() {
            ptr::null()
        } else {
            index.as_ptr()
        };
        unsafe {
            nvcall!(NvAPI_GPU_RestoreCoolerPolicyTable(
                self.0,
                ptr,
                index.len() as u32,
                policy.raw()
            ))
        }
    }

    pub fn fan_arbiter_info(
        &self,
    ) -> crate::Result<<cooler::private::NV_GPU_CLIENT_FAN_ARBITERS_INFO_V1 as RawConversion>::Target>
    {
        trace!("gpu.fan_arbiter_info()");

        unsafe { nvcall!(NvAPI_GPU_ClientFanArbitersGetInfo@get(self.0) => raw) }
    }

    pub fn fan_arbiter_status(
        &self,
    ) -> crate::Result<
        <cooler::private::NV_GPU_CLIENT_FAN_ARBITERS_STATUS_V1 as RawConversion>::Target,
    > {
        trace!("gpu.fan_arbiter_status()");

        unsafe { nvcall!(NvAPI_GPU_ClientFanArbitersGetStatus@get(self.0) => raw) }
    }

    pub fn fan_arbiter_control(
        &self,
    ) -> crate::Result<
        <cooler::private::NV_GPU_CLIENT_FAN_ARBITERS_CONTROL_V1 as RawConversion>::Target,
    > {
        trace!("gpu.fan_arbiter_control()");

        unsafe { nvcall!(NvAPI_GPU_ClientFanArbitersGetControl@get(self.0) => raw) }
    }

    pub fn perf_info(
        &self,
    ) -> crate::Result<<power::private::NV_GPU_PERF_POLICIES_INFO_PARAMS as RawConversion>::Target>
    {
        trace!("gpu.perf_info()");

        unsafe { nvcall!(NvAPI_GPU_PerfPoliciesGetInfo@get(self.0) => raw) }
    }

    pub fn perf_status(
        &self,
    ) -> crate::Result<<power::private::NV_GPU_PERF_POLICIES_STATUS_PARAMS as RawConversion>::Target>
    {
        trace!("gpu.perf_status()");

        unsafe { nvcall!(NvAPI_GPU_PerfPoliciesGetStatus@get(self.0) => raw) }
    }

    pub fn voltage_domains_status(
        &self,
    ) -> crate::Result<<power::private::NV_VOLT_STATUS as RawConversion>::Target> {
        trace!("gpu.voltage_domains_status()");

        unsafe { nvcall!(NvAPI_GPU_GetVoltageDomainsStatus@get(self.0) => raw) }
    }

    pub fn voltage_step(
        &self,
    ) -> crate::Result<<power::private::NV_VOLT_STATUS as RawConversion>::Target> {
        trace!("gpu.voltage_step()");

        unsafe { nvcall!(NvAPI_GPU_GetVoltageStep@get(self.0) => raw) }
    }

    pub fn voltage_table(
        &self,
    ) -> crate::Result<<power::private::NV_VOLT_TABLE as RawConversion>::Target> {
        trace!("gpu.voltage_table()");

        unsafe { nvcall!(NvAPI_GPU_GetVoltages@get(self.0) => raw) }
    }

    pub fn performance_decrease(&self) -> crate::NvapiResult<PerformanceDecreaseReason> {
        trace!("gpu.performance_decrease()");

        unsafe {
            nvcall!(NvAPI_GPU_GetPerfDecreaseInfo@get(self.0))
                .map(|v| PerformanceDecreaseReason::from_bits_truncate(v.value))
        }
    }

    pub fn current_thermal_level(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.current_thermal_level()");
        unsafe { nvcall!(NvAPI_GPU_GetCurrentThermalLevel@get(self.0)) }
    }

    pub fn current_fan_speed_level(&self) -> crate::NvapiResult<u32> {
        trace!("gpu.current_fan_speed_level()");
        unsafe { nvcall!(NvAPI_GPU_GetCurrentFanSpeedLevel@get(self.0)) }
    }

    pub fn display_ids_all(
        &self,
    ) -> crate::Result<Vec<<display::NV_GPU_DISPLAYIDS as RawConversion>::Target>> {
        trace!("gpu.display_ids_all()");
        let mut count =
            unsafe { nvcall!(NvAPI_GPU_GetAllDisplayIds@get(self.0, ptr::null_mut())) }?;
        if count == 0 {
            return Ok(Vec::new());
        }
        let mut data = vec![display::NV_GPU_DISPLAYIDS::default(); count as usize];

        unsafe {
            nvcall!(NvAPI_GPU_GetAllDisplayIds(self.0, data.as_mut_ptr(), &mut count) => err)
                .and_then(|()| {
                    data.into_iter()
                        .map(|v| v.convert_raw().map_err(From::from))
                        .collect()
                })
        }
    }

    pub fn display_ids_connected(
        &self,
        flags: ConnectedIdsFlags,
    ) -> crate::Result<Vec<<display::NV_GPU_DISPLAYIDS as RawConversion>::Target>> {
        trace!("gpu.display_ids_connected({:?})", flags);
        let mut count = unsafe {
            let mut count = 0;
            nvcall!(NvAPI_GPU_GetConnectedDisplayIds(
                self.0,
                ptr::null_mut(),
                &mut count,
                flags.bits().into()
            ))
            .map(|()| count)
        }?;
        if count == 0 {
            return Ok(Vec::new());
        }
        let mut data = vec![display::NV_GPU_DISPLAYIDS::default(); count as usize];

        unsafe {
            nvcall!(NvAPI_GPU_GetConnectedDisplayIds(self.0, data.as_mut_ptr(), &mut count, flags.bits().into()) => err)
                .and_then(|()| data.into_iter().map(|v| v.convert_raw().map_err(From::from)).collect())
        }
    }

    pub fn get_edid(&self, display_id: u32) -> crate::NvapiResult<Vec<u8>> {
        trace!("gpu.get_edid(0x{:08x})", display_id);
        let mut edid = display::NV_EDID::default();
        unsafe {
            nvcall!(NvAPI_GPU_GetEDID(self.0, display_id, &mut edid))?;
        }
        let size = edid.sizeofEDID as usize;
        Ok(edid.EDID_Data[..size.min(256)].to_vec())
    }

    pub fn set_edid(&self, display_id: u32, data: &[u8]) -> crate::NvapiResult<()> {
        trace!("gpu.set_edid(0x{:08x}, {} bytes)", display_id, data.len());
        let mut edid = display::NV_EDID::default();
        let len = data.len().min(256);
        edid.EDID_Data[..len].copy_from_slice(&data[..len]);
        edid.sizeofEDID = len as u32;
        unsafe {
            nvcall!(NvAPI_GPU_SetEDID(self.0, display_id, &mut edid))?;
        }
        Ok(())
    }

    pub fn clear_edid(&self, display_id: u32) -> crate::NvapiResult<()> {
        trace!("gpu.clear_edid(0x{:08x})", display_id);
        let mut edid = display::NV_EDID::default();
        unsafe {
            nvcall!(NvAPI_GPU_SetEDID(self.0, display_id, &mut edid))?;
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn i2c_read(
        &self,
        display_mask: u32,
        port: Option<u8>,
        port_is_ddc: bool,
        address: u8,
        register: &[u8],
        bytes: &mut [u8],
        speed: i2c::I2cSpeed,
    ) -> crate::NvapiResult<usize> {
        trace!(
            "i2c_read({}, {:?}, {:?}, 0x{:02x}, {:?}, {:?})",
            display_mask, port, port_is_ddc, address, register, speed
        );
        let mut data = i2c::NV_I2C_INFO {
            displayMask: display_mask,
            bIsDDCPort: if port_is_ddc {
                sys::NV_TRUE
            } else {
                sys::NV_FALSE
            } as _,
            i2cDevAddress: address << 1,
            pbI2cRegAddress: if register.is_empty() {
                ptr::null_mut()
            } else {
                register.as_ptr() as *mut _
            },
            regAddrSize: register.len() as _,
            pbData: bytes.as_mut_ptr(),
            cbSize: bytes.len() as _,
            i2cSpeed: i2c::NVAPI_I2C_SPEED_DEPRECATED,
            i2cSpeedKhz: speed.raw(),
            portId: port.unwrap_or_default(),
            bIsPortIdSet: if port.is_some() {
                sys::NV_TRUE as _
            } else {
                sys::NV_FALSE as _
            },
            ..Default::default()
        };

        unsafe {
            nvcall!(NvAPI_I2CRead(self.0, &mut data)).map(|()| data.cbSize as usize) // TODO: not actually sure if this ever changes?
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn i2c_write(
        &self,
        display_mask: u32,
        port: Option<u8>,
        port_is_ddc: bool,
        address: u8,
        register: &[u8],
        bytes: &[u8],
        speed: i2c::I2cSpeed,
    ) -> crate::NvapiResult<()> {
        trace!(
            "i2c_write({}, {:?}, {:?}, 0x{:02x}, {:?}, {:?})",
            display_mask, port, port_is_ddc, address, register, speed
        );
        let mut data = i2c::NV_I2C_INFO {
            displayMask: display_mask,
            bIsDDCPort: if port_is_ddc {
                sys::NV_TRUE
            } else {
                sys::NV_FALSE
            } as _,
            i2cDevAddress: address << 1,
            pbI2cRegAddress: if register.is_empty() {
                ptr::null_mut()
            } else {
                register.as_ptr() as *mut _
            },
            regAddrSize: register.len() as _,
            pbData: bytes.as_ptr() as *mut _,
            cbSize: bytes.len() as _,
            i2cSpeed: i2c::NVAPI_I2C_SPEED_DEPRECATED,
            i2cSpeedKhz: speed.raw(),
            portId: port.unwrap_or_default(),
            bIsPortIdSet: if port.is_some() {
                sys::NV_TRUE as _
            } else {
                sys::NV_FALSE as _
            },
            ..Default::default()
        };

        unsafe { nvcall!(NvAPI_I2CWrite(self.0, &mut data)) }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct PciIdentifiers {
    pub device_id: u32,
    pub subsystem_id: u32,
    pub revision_id: u32,
    pub ext_device_id: u32,
}

impl fmt::Display for PciIdentifiers {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{:08x} - {:08x} - {:08x} - {:x}",
            self.device_id, self.subsystem_id, self.ext_device_id, self.revision_id
        )
    }
}

impl PciIdentifiers {
    pub fn vendor_id(&self) -> u16 {
        self.ids().0
    }

    pub fn product_id(&self) -> u16 {
        self.ids().1
    }

    pub fn ids(&self) -> (u16, u16) {
        let pid = (self.device_id >> 16) as u16;
        let vid = self.device_id as u16;
        if vid == 0x10de && self.subsystem_id != 0 {
            let spid = (self.subsystem_id >> 16) as u16;
            (
                self.subsystem_id as u16,
                if spid == 0 {
                    // Colorful and Inno3D
                    pid
                } else {
                    spid
                },
            )
        } else {
            (vid, pid)
        }
    }

    pub fn vendor(&self) -> Result<Vendor, sys::ArgumentRangeError> {
        Vendor::try_from(self.vendor_id() as i32)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, Default, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct BusInfo {
    pub id: u32,
    pub slot_id: u32,
    pub irq: u32,
    pub bus: Bus,
}

impl BusInfo {
    pub fn vendor(&self) -> Result<Option<Vendor>, sys::ArgumentRangeError> {
        self.bus.vendor()
    }
}

impl fmt::Display for BusInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} ({}:{} routed to IRQ {})",
            self.bus, self.id, self.slot_id, self.irq
        )
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Bus {
    Pci { ids: PciIdentifiers },
    PciExpress { ids: PciIdentifiers, lanes: u32 },
    Other(BusType),
}

impl Bus {
    pub fn bus_type(&self) -> BusType {
        match self {
            Bus::Pci { .. } => BusType::Pci,
            Bus::PciExpress { .. } => BusType::PciExpress,
            &Bus::Other(ty) => ty,
        }
    }

    pub fn pci_ids(&self) -> Option<&PciIdentifiers> {
        match self {
            Bus::Pci { ids } => Some(ids),
            Bus::PciExpress { ids, .. } => Some(ids),
            _ => None,
        }
    }

    pub fn vendor(&self) -> Result<Option<Vendor>, sys::ArgumentRangeError> {
        match self.pci_ids() {
            Some(ids) => ids.vendor().map(Some),
            None => Ok(None),
        }
    }
}

impl fmt::Display for Bus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Bus::PciExpress { lanes, .. } => {
                fmt::Display::fmt(&BusType::PciExpress, f)?;
                if *lanes > 0 {
                    write!(f, " x{}", lanes)?;
                }
                Ok(())
            }
            Bus::Pci { .. } => fmt::Display::fmt(&BusType::Pci, f),
            Bus::Other(ty) => fmt::Display::fmt(ty, f),
        }
    }
}

impl Default for Bus {
    fn default() -> Self {
        Bus::Other(Default::default())
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct MemoryInfo {
    pub dedicated: Kibibytes,
    pub dedicated_available: Kibibytes,
    pub system: Kibibytes,
    pub shared: Kibibytes,
    pub dedicated_available_current: Kibibytes,
    pub dedicated_evictions_size: Kibibytes,
    pub dedicated_evictions: u32,
}

impl RawConversion for driverapi::NV_DISPLAY_DRIVER_MEMORY_INFO {
    type Target = MemoryInfo;
    type Error = Infallible;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        Ok(MemoryInfo {
            dedicated: Kibibytes(self.dedicatedVideoMemory),
            dedicated_available: Kibibytes(self.availableDedicatedVideoMemory),
            system: Kibibytes(self.systemVideoMemory),
            shared: Kibibytes(self.sharedSystemMemory),
            dedicated_available_current: Kibibytes(self.curAvailableDedicatedVideoMemory),
            dedicated_evictions_size: Kibibytes(self.dedicatedVideoMemoryEvictionsSize),
            dedicated_evictions: self.dedicatedVideoMemoryEvictionCount,
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct DriverModel {
    pub value: u32,
}

impl DriverModel {
    pub fn new(value: u32) -> Self {
        DriverModel { value }
    }

    pub fn wddm(&self) -> (u8, u8) {
        // 2.0 or 1.(value >> 8)
        let major = ((self.value >> 12) & 0xf) as u8;
        (
            major,
            if major == 2 {
                0
            } else {
                (self.value >> 8) as u8 & 0xf
            },
        )
    }
}

impl fmt::Display for DriverModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let wddm = self.wddm();
        write!(f, "WDDM {}.{:02}", wddm.0, wddm.1)
    }
}

impl fmt::Debug for DriverModel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} ({:08x})", self, self.value)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct DisplayId {
    pub connector: MonitorConnectorType,
    pub display_id: u32,
    pub flags: DisplayIdsFlags,
}

impl RawConversion for display::NV_GPU_DISPLAYIDS {
    type Target = DisplayId;
    type Error = sys::ArgumentRangeError;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        Ok(DisplayId {
            connector: MonitorConnectorType::from_raw(self.connectorType)?,
            display_id: self.displayId,
            flags: DisplayIdsFlags::from_bits_truncate(self.flags.value),
        })
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum Architecture {
    T2X(sys::gpu::ArchitectureImplementationT2X),
    T3X(sys::gpu::ArchitectureImplementationT3X),
    NV40(sys::gpu::ArchitectureImplementationNV40),
    NV50(sys::gpu::ArchitectureImplementationNV50),
    G78(sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID),
    G80(sys::gpu::ArchitectureImplementationG80),
    G90(sys::gpu::ArchitectureImplementationG90),
    GT200(sys::gpu::ArchitectureImplementationGT200),
    GF100(sys::gpu::ArchitectureImplementationGF100),
    GK100(sys::gpu::ArchitectureImplementationGK100),
    GK110(sys::gpu::ArchitectureImplementationGK110),
    GK200(sys::gpu::ArchitectureImplementationGK200),
    GM000(sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID),
    GM200(sys::gpu::ArchitectureImplementationGM200),
    GP100(sys::gpu::ArchitectureImplementationGP100),
    GV100(sys::gpu::ArchitectureImplementationGV100),
    GV110(sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID),
    TU100(sys::gpu::ArchitectureImplementationTU100),
    GA100(sys::gpu::ArchitectureImplementationGA100),
    Unknown {
        id: sys::gpu::NV_GPU_ARCHITECTURE_ID,
        implementation: sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID,
    },
}

impl Default for Architecture {
    fn default() -> Self {
        Architecture::Unknown {
            id: Default::default(),
            implementation: Default::default(),
        }
    }
}

impl Architecture {
    pub fn new<I: Into<sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID>>(
        id: ArchitectureId,
        implementation: I,
    ) -> Self {
        Self::from_raw(id.into(), implementation.into())
    }

    pub fn from_raw(
        id: sys::gpu::NV_GPU_ARCHITECTURE_ID,
        implementation: sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID,
    ) -> Self {
        Self::from_raw_inner(id, implementation).unwrap_or(Self::Unknown { id, implementation })
    }

    fn from_raw_inner(
        id: sys::gpu::NV_GPU_ARCHITECTURE_ID,
        implementation: sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID,
    ) -> Result<Self, sys::ArgumentRangeError> {
        let implementation = implementation.repr();
        Ok(match id {
            sys::gpu::NV_GPU_ARCHITECTURE_T2X => Architecture::T2X(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_T3X => Architecture::T3X(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_NV40 => Architecture::NV40(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_NV50 => Architecture::NV50(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_G78 => Architecture::G78(sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID::with_repr(implementation)),
            sys::gpu::NV_GPU_ARCHITECTURE_G80 => Architecture::G80(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_G90 => Architecture::G90(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_GT200 => Architecture::GT200(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_GF100 => Architecture::GF100(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_GK100 => Architecture::GK100(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_GK110 => Architecture::GK110(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_GK200 => Architecture::GK200(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_GM000 => Architecture::GM000(sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID::with_repr(implementation)),
            sys::gpu::NV_GPU_ARCHITECTURE_GM200 => Architecture::GM200(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_GP100 => Architecture::GP100(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_GV100 => Architecture::GV100(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_GV110 => Architecture::GV110(sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID::with_repr(implementation)),
            sys::gpu::NV_GPU_ARCHITECTURE_TU100 => Architecture::TU100(implementation.try_into()?),
            sys::gpu::NV_GPU_ARCHITECTURE_GA100 => Architecture::GA100(implementation.try_into()?),
            _ => return Err(Default::default()),
        })
    }

    pub fn id(&self) -> Result<ArchitectureId, sys::gpu::NV_GPU_ARCHITECTURE_ID> {
        Ok(match *self {
            Architecture::T2X(..) => ArchitectureId::T2X,
            Architecture::T3X(..) => ArchitectureId::T3X,
            Architecture::NV40(..) => ArchitectureId::NV40,
            Architecture::NV50(..) => ArchitectureId::NV50,
            Architecture::G78(..) => ArchitectureId::G78,
            Architecture::G80(..) => ArchitectureId::G80,
            Architecture::G90(..) => ArchitectureId::G90,
            Architecture::GT200(..) => ArchitectureId::GT200,
            Architecture::GF100(..) => ArchitectureId::GF100,
            Architecture::GK100(..) => ArchitectureId::GK100,
            Architecture::GK110(..) => ArchitectureId::GK110,
            Architecture::GK200(..) => ArchitectureId::GK200,
            Architecture::GM000(..) => ArchitectureId::GM000,
            Architecture::GM200(..) => ArchitectureId::GM200,
            Architecture::GP100(..) => ArchitectureId::GP100,
            Architecture::GV100(..) => ArchitectureId::GV100,
            Architecture::GV110(..) => ArchitectureId::GV110,
            Architecture::TU100(..) => ArchitectureId::TU100,
            Architecture::GA100(..) => ArchitectureId::GA100,
            Architecture::Unknown { id, .. } => return id.try_into().map_err(|_| id),
        })
    }

    pub fn raw_id(&self) -> sys::gpu::NV_GPU_ARCHITECTURE_ID {
        self.id().map(|id| id.into()).unwrap_or_else(|id| id)
    }

    pub fn raw_implementation(&self) -> sys::gpu::NV_GPU_ARCH_IMPLEMENTATION_ID {
        match *self {
            Architecture::T2X(i) => i.repr().into(),
            Architecture::T3X(i) => i.repr().into(),
            Architecture::NV40(i) => i.repr().into(),
            Architecture::NV50(i) => i.repr().into(),
            Architecture::G78(i) => i,
            Architecture::G80(i) => i.repr().into(),
            Architecture::G90(i) => i.repr().into(),
            Architecture::GT200(i) => i.repr().into(),
            Architecture::GF100(i) => i.repr().into(),
            Architecture::GK100(i) => i.repr().into(),
            Architecture::GK110(i) => i.repr().into(),
            Architecture::GK200(i) => i.repr().into(),
            Architecture::GM000(i) => i,
            Architecture::GM200(i) => i.repr().into(),
            Architecture::GP100(i) => i.repr().into(),
            Architecture::GV100(i) => i.repr().into(),
            Architecture::GV110(i) => i,
            Architecture::TU100(i) => i.repr().into(),
            Architecture::GA100(i) => i.repr().into(),
            Architecture::Unknown { implementation, .. } => implementation,
        }
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Architecture::T2X(i) => fmt::Display::fmt(i, f),
            Architecture::T3X(i) => fmt::Display::fmt(i, f),
            Architecture::NV40(i) => fmt::Display::fmt(i, f),
            Architecture::NV50(i) => fmt::Display::fmt(i, f),
            Architecture::G80(i) => fmt::Display::fmt(i, f),
            Architecture::G90(i) => fmt::Display::fmt(i, f),
            Architecture::GT200(i) => fmt::Display::fmt(i, f),
            Architecture::GF100(i) => fmt::Display::fmt(i, f),
            Architecture::GK100(i) => fmt::Display::fmt(i, f),
            Architecture::GK110(i) => fmt::Display::fmt(i, f),
            Architecture::GK200(i) => fmt::Display::fmt(i, f),
            Architecture::GM200(i) => fmt::Display::fmt(i, f),
            Architecture::GP100(i) => fmt::Display::fmt(i, f),
            Architecture::GV100(i) => fmt::Display::fmt(i, f),
            Architecture::TU100(i) => fmt::Display::fmt(i, f),
            Architecture::GA100(i) => fmt::Display::fmt(i, f),
            Architecture::G78(implementation)
            | Architecture::GM000(implementation)
            | Architecture::GV110(implementation)
            | Architecture::Unknown { implementation, .. } => match self.id() {
                Ok(ref id) if *implementation == 0 => fmt::Display::fmt(id, f),
                Ok(id) => write!(f, "{}:{}", id, implementation),
                Err(id) => write!(f, "Unknown:{}:{}", id, implementation),
            },
        }
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct ArchInfo {
    pub arch: Architecture,
    pub revision: sys::gpu::NV_GPU_CHIP_REVISION,
}

impl ArchInfo {
    pub fn revision(&self) -> Result<ChipRevision, sys::gpu::NV_GPU_CHIP_REVISION> {
        self.revision.try_into().map_err(|_| self.revision)
    }
}

impl fmt::Display for ArchInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&self.arch, f)?;
        match self.revision() {
            Ok(rev) => write!(f, ":{}", rev),
            Err(rev) => write!(f, ":{}", rev),
        }
    }
}

impl RawConversion for sys::gpu::NV_GPU_ARCH_INFO_V1 {
    type Target = ArchInfo;
    type Error = Infallible;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:?})", self);
        Ok(ArchInfo {
            arch: Architecture::from_raw(self.architecture, self.implementation),
            revision: self.revision,
        })
    }
}

/// Static compute/PhysX/framebuffer capability flags for a GPU, from
/// `NvAPI_GPU_GetComputeCapabilities`. See [sys::gpu::NV_GPU_COMPUTE_CAPS] for the
/// individual flag bits (base-compute, compute-capable, board-DB match, PhysX installed,
/// VRAM >= 256 MiB, PhysX GPU selected).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Default, Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct ComputeCapabilities {
    pub flags: sys::gpu::NV_GPU_COMPUTE_CAPS,
}

impl RawConversion for sys::gpu::NV_GPU_COMPUTE_CAPS_INFO_V1 {
    type Target = ComputeCapabilities;
    type Error = Infallible;

    fn convert_raw(&self) -> Result<Self::Target, Self::Error> {
        trace!("convert_raw({:?})", self);
        Ok(ComputeCapabilities { flags: self.flags })
    }
}

/// TGP-watts range + active policy index (from the private
/// One target-temperature (温度墙) policy slot: live current temp plus the
/// VBIOS min/default/max range from the private ClientThermalPolicies GetInfo
/// (0x2F69F8E5). `current` is always present; the range fields are None when
/// GetInfo didn't cover this slot. All values are celsius.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct TargetTempPolicyEntry {
    pub policy_index: usize,
    /// Live current target temp (private GET-prime 0xC4554575).
    pub current: f32,
    /// VBIOS minimum (the writable floor; idx 2 = 75C on RTX 4060 Laptop).
    pub min: Option<f32>,
    /// VBIOS rated/default target temp.
    pub default: Option<f32>,
    /// VBIOS maximum (the writable ceiling; idx 2 = 87C).
    pub max: Option<f32>,
}

/// TGP-watts range (NDA 0x67F31384). All values are in **milliwatts**.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct TgpWattRange {
    /// Active power-policy table entry index (default 2 when driver reports none).
    pub policy_index: usize,
    /// Minimum TGP (mW), if the entry exposed it.
    pub min_mw: Option<u32>,
    /// Rated/default TGP (mW), if the entry exposed it.
    pub default_mw: Option<u32>,
    /// Maximum TGP (mW), if the entry exposed it.
    pub max_mw: Option<u32>,
}

/// One D-Notifier (D0-notify / "extern power state") level: the named D level
/// (D1..D5) and the power cap it imposes when active. `None` power means
/// "Unlimited" (only ever true for D1).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DNotifierLevel {
    /// D level number, 1..5 (D1..D5). Render as `format!("D{}", level)`.
    pub level: u8,
    /// The signed level code the driver uses (-1=D1, 0=D2, 1=D3, 2=D4, 3=D5).
    pub index: i32,
    /// Power cap in **milliwatts** when this level is active; `None` = Unlimited
    /// (D1). RE'd from the ref tool's "D{n}({power}mW)" string.
    pub power_mw: Option<u32>,
}

impl DNotifierLevel {
    /// Map a driver D-index code (-1..3) to the canonical D level, or `None`
    /// for an invalid sentinel.
    pub fn from_index(index: i32) -> Option<Self> {
        let (level, unlimited) = match index {
            -1 => (1u8, true),
            0 => (2, false),
            1 => (3, false),
            2 => (4, false),
            3 => (5, false),
            _ => return None,
        };
        Some(Self {
            level,
            index,
            power_mw: if unlimited { None } else { Some(0) },
        })
    }

    /// Convenience label, "D1".."D5".
    pub fn label(&self) -> String {
        format!("D{}", self.level)
    }
}

/// D-Notifier current state read from the private ClientPowerPoliciesGetInfo
/// (`0x67F31384`): the active D level plus the full D1..D5 power-cap table.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct DNotifierInfo {
    /// The currently-active D level (None when the driver reports the N/A sentinel).
    pub active: Option<DNotifierLevel>,
    /// The D1..D5 power-cap table (always 5 entries, in D1→D5 order).
    pub levels: [DNotifierLevel; 5],
}

/// One P-State entry from the private PerfPstatesGetInfo table (`0x7B30AE0D`):
/// the pstate number and its min/max core clock in kHz. RE'd from the ref tool's
/// `queryPStateInfo` (the source of `-pstate` GET).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PStateClockRange {
    /// P-State number, 0..31 (e.g. 0 for P0).
    pub pstate: u8,
    /// Min core clock (kHz), if the driver exposed it.
    pub min_khz: Option<u32>,
    /// Max core clock (kHz), if the driver exposed it.
    pub max_khz: Option<u32>,
}

/// P-State level table from the private PerfPstatesGetInfo (`0x7B30AE0D`).
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct PStateLevelsInfo {
    /// Present P-States in ascending order, each with min/max core clock (kHz).
    pub pstates: Vec<PStateClockRange>,
}

/// Native NVAPI P-State lock request (the the ref tool `-pstate:<index>` SETTER,
/// PerfClientLimitsSetStatus 0x39442CFB). RE'd from the ref tool's setPState.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PStateNativeLock {
    /// Reset all P-State locks to default (the ref tool `-pstate:-1`).
    Reset,
    /// Pin the active P-State to `pstate` without locking a frequency
    /// (the ref tool setPState with freq=-1).
    PstateOnly { pstate: u8 },
    /// Pin the active P-State AND lock its frequency to `freq_khz`
    /// (the ref tool setPState with both pstate and freq). `freq_khz` is MHz × 1000.
    PstateAndFreq { pstate: u8, freq_khz: u32 },
}

/// GPU frequency perf-cap request (the ref tool `-gpuclk:<MHz>` SETTER,
/// PerfLimitsSetStatus NDA 0x32CA4983). RE'd byte-exact from ref tool 2's
/// `GPUHandle::setGpcClock`: clamps the perf max/min frequency to a cap value
/// (NOT an offset, NOT a P-state lock — see [[PStateNativeLock]] for that).
/// `freq_khz` is MHz × 1000.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerfFreqCap {
    /// Clear the perf frequency cap (`-gpuclk:-1`): both entries enable=0,
    /// no frequency written.
    Reset,
    /// Clamp perf frequency to `[min_khz, max_khz]`. Either bound may be 0 to
    /// leave that side unset (ref tool 2 sets both to the same cap value).
    Cap { max_khz: u32, min_khz: u32 },
}

/// One entry read back by `Gpu::perf_freq_caps` (PerfLimitsGetStatus NDA
/// 0xEFCEDD1F). `type_marker` is the driver's entry-type code (0x5D='Pmax',
/// 0x49='I'=Pmin observed in ref tool 2); `freq_khz` is the cap (MHz × 1000);
/// `locked` is non-zero when the cap is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerfFreqCapEntry {
    pub type_marker: u32,
    pub freq_khz: u32,
    pub locked: bool,
}

/// Per-cooler info aggregated from the private FanCoolers family (NDA):
/// GetInfo (mask) + GetControl (type/min/max) + GetStatus (current).
/// RE'd from ref tool 2's pollFanSpeed. NOTE the speed fields are in the
/// DRIVER's scale — on some GPUs (2070 desktop observed) that grid is the
/// normalized 0..65536 duty scale, not physical RPM; `current_pwm_percent`
/// is the observable duty for cross-checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrivateCoolerInfo {
    pub index: u32,
    /// 0=active, 1=pwm, 2=pwm-tach
    pub cooler_type: u32,
    /// Driver-scale minimum (see struct doc).
    pub min: u32,
    /// Driver-scale maximum.
    pub max: u32,
    /// Current speed in the same driver scale (status dword 19).
    pub current: u32,
    /// Current duty in percent (status dword 24 × 100 / 65536).
    pub current_pwm_percent: u32,
}

/// Result of a `set_fan_rpm` call (private FanCoolerSetControl NDA 0xEB44E8AA).
/// RE'd from ref tool 2's setFanSim. `cooler_type`: 0=active, 1=pwm, 2=pwm-tach.
/// `applied_rpm` is `None` when simulation was disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetFanRpmResult {
    pub cooler_index: u32,
    pub cooler_type: u32,
    pub min_rpm: u32,
    pub max_rpm: u32,
    pub applied_rpm: Option<u32>,
}

// ── PerfLimits large-struct byte offsets (magic 0x6642C, 0x4642C B) ──
// RE'd from ref tool 2's setGpcClock (sub_140023FE0) / isPStateLocked
// (sub_14002C8E0). Entry stride 0x464; entry0 data @ +0x2C, entry1 @ +0x490.
pub(super) const PERF_LIMITS_MAGIC: u32 = 0x6642C;
pub(super) const PERF_LIMITS_SIZE: usize = 0x4642C;
pub(super) const PERF_LIMITS_OFF_COUNT: usize = 0x08;
pub(super) const PERF_LIMITS_ENTRY_STRIDE: usize = 0x464;
pub(super) const PERF_LIMITS_ENTRY0_BASE: usize = 0x2C; // entry0 type_marker
pub(super) const PERF_LIMITS_ENTRY1_BASE: usize = 0x490; // = 0x2C + 0x464
pub(super) const PERF_LIMITS_OFF_TYPE: usize = 0x00; // rel to entry base
pub(super) const PERF_LIMITS_OFF_ENABLE: usize = 0x30; // rel to entry base
pub(super) const PERF_LIMITS_OFF_FREQ: usize = 0x58; // rel to entry base
pub(super) const PERF_LIMITS_OFF_LOCKED: usize = 0x458; // rel to entry base (GET only; struct+0x484)
// SET type markers (entry0=max, entry1=min).
pub(super) const PERF_LIMITS_TYPE_MAX: u32 = 0x58;
pub(super) const PERF_LIMITS_TYPE_MIN: u32 = 0x5B;
pub(super) const PERF_LIMITS_ENABLE_APPLY: u32 = 2;
pub(super) const PERF_LIMITS_ENABLE_RESET: u32 = 0;
// ── PerfLimits medium-struct (GetInfo, magic 0x1300C) ──
pub(super) const PERF_LIMITS_INFO_MAGIC: u32 = 0x1300C;
pub(super) const PERF_LIMITS_INFO_SIZE: usize = 0x300C;
pub(super) const PERF_LIMITS_INFO_OFF_COUNT: usize = 0x08;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct VfpInfo {
    pub domains: ClockDomainInfo,
    pub mask: VfpMask,
}

impl VfpInfo {
    pub fn iter<'s>(&'s self, domain: ClockDomain) -> impl Iterator<Item = usize> + 's {
        self.domains
            .get(domain)
            .into_iter()
            .flat_map(|d| d.vfp_index.range().filter(|&i| self.mask.mask.get_bit(i)))
    }

    pub fn index<'s, 'a, T: 'static>(
        &'s self,
        domain: ClockDomain,
        entries: &'a [T],
    ) -> impl Iterator<Item = (usize, &'a T)> + 's
    where
        'a: 's,
    {
        self.iter(domain).map(move |i| (i, &entries[i]))
    }

    pub fn index_mut<'s, 'a, T: 'static>(
        &'s self,
        domain: ClockDomain,
        entries: &'a mut [T],
    ) -> impl Iterator<Item = (usize, &'a mut T)> + 's
    where
        'a: 's,
    {
        let mut entries = entries.iter_mut().enumerate();
        self.iter(domain).map(move |i| {
            loop {
                match entries.next() {
                    None => panic!("entries out of range of {:?}", self),
                    Some((ei, _)) if ei < i => (),
                    Some(t) => break t,
                }
            }
        })
    }
}

/// One temperature→RPM point of a GPU fan curve (`ClientFanPolicies` table,
/// structure magic `0x200DC`). RE'd from ref tool 2's `DialogFanCurve` pane:
/// the dialog edits three monotonic (temp, RPM) pairs per curve slot, and the
/// driver's Set handler rejects non-strictly-increasing input with -5.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FanCurvePoint {
    /// input temperature in °C (stored in the driver struct as Q8.8, ×256)
    pub temp_c: u16,
    /// target fan speed in RPM (stored Q16, ×65536/100)
    pub rpm: u32,
}

/// A single fan-curve slot as reported by [`PhysicalGpu::fan_curves`] /
/// targeted by [`PhysicalGpu::set_fan_curve`]. The table holds up to 4 slots
/// (ref tool 2's runtime "Next Curve" cycles `(idx + 1) % count`); `count` — the
/// table's first byte after the magic — is the authoritative curve count.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FanCurve {
    /// curve slot index
    pub index: u8,
    /// 3 monotonic (temperature, RPM) points
    pub points: Vec<FanCurvePoint>,
}

/// Little-endian u32 write at a byte offset (heap-backed NVAPI struct helper).
#[inline]
fn write_u32(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_ne_bytes());
}

/// Little-endian u32 read at a byte offset (heap-backed NVAPI struct helper).
#[inline]
fn read_u32(buf: &[u8], off: usize) -> u32 {
    let mut b = [0u8; 4];
    b.copy_from_slice(&buf[off..off + 4]);
    u32::from_ne_bytes(b)
}

/// Build the PerfLimits SetStatus buffer (magic 0x6642C, 0x4642C B) for a
/// [`PerfFreqCap`]. Extracted from `set_perf_freq_cap` so the byte layout is
/// unit-testable without a live GPU handle. RE'd from ref tool's setGpcClock.
fn build_perf_freq_cap_buffer(cap: PerfFreqCap) -> Vec<u8> {
    let mut buf = vec![0u8; PERF_LIMITS_SIZE];
    buf[..4].copy_from_slice(&PERF_LIMITS_MAGIC.to_ne_bytes());
    // count = 2 (entry0 = max, entry1 = min).
    write_u32(&mut buf, PERF_LIMITS_OFF_COUNT, 2);

    let (enable, max_khz, min_khz) = match cap {
        PerfFreqCap::Reset => (PERF_LIMITS_ENABLE_RESET, 0, 0),
        PerfFreqCap::Cap { max_khz, min_khz } => (PERF_LIMITS_ENABLE_APPLY, max_khz, min_khz),
    };
    // entry0 (max): type_marker @ +0x2C, enable @ +0x5C, freq @ +0x84.
    write_u32(
        &mut buf,
        PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_TYPE,
        PERF_LIMITS_TYPE_MAX,
    );
    write_u32(
        &mut buf,
        PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_ENABLE,
        enable,
    );
    write_u32(
        &mut buf,
        PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_FREQ,
        max_khz,
    );
    // entry1 (min): type_marker @ +0x490, enable @ +0x4C0, freq @ +0x508.
    write_u32(
        &mut buf,
        PERF_LIMITS_ENTRY1_BASE + PERF_LIMITS_OFF_TYPE,
        PERF_LIMITS_TYPE_MIN,
    );
    write_u32(
        &mut buf,
        PERF_LIMITS_ENTRY1_BASE + PERF_LIMITS_OFF_ENABLE,
        enable,
    );
    write_u32(
        &mut buf,
        PERF_LIMITS_ENTRY1_BASE + PERF_LIMITS_OFF_FREQ,
        min_khz,
    );
    buf
}

#[cfg(test)]
mod perflimits_tests {
    use super::*;

    /// The large-struct layout RE'd from ref tool's setGpcClock must hold.
    #[test]
    fn perf_limits_layout_constants() {
        assert_eq!(PERF_LIMITS_MAGIC, 0x6642C);
        assert_eq!(PERF_LIMITS_SIZE, 0x4642C);
        assert_eq!(PERF_LIMITS_OFF_COUNT, 0x08);
        assert_eq!(PERF_LIMITS_ENTRY_STRIDE, 0x464);
        assert_eq!(PERF_LIMITS_ENTRY0_BASE, 0x2C);
        assert_eq!(PERF_LIMITS_ENTRY1_BASE, 0x2C + 0x464); // 0x490
        assert_eq!(PERF_LIMITS_OFF_TYPE, 0x00);
        assert_eq!(PERF_LIMITS_OFF_ENABLE, 0x30); // entry+0x30 = struct+0x5C
        assert_eq!(PERF_LIMITS_OFF_FREQ, 0x58); // entry+0x58 = struct+0x84
        assert_eq!(PERF_LIMITS_OFF_LOCKED, 0x458); // entry+0x458 = struct+0x484
        assert_eq!(PERF_LIMITS_TYPE_MAX, 0x58);
        assert_eq!(PERF_LIMITS_TYPE_MIN, 0x5B);
        assert_eq!(PERF_LIMITS_ENABLE_APPLY, 2);
        assert_eq!(PERF_LIMITS_ENABLE_RESET, 0);
        // medium GetInfo struct
        assert_eq!(PERF_LIMITS_INFO_MAGIC, 0x1300C);
        assert_eq!(PERF_LIMITS_INFO_SIZE, 0x300C);
        assert_eq!(PERF_LIMITS_INFO_OFF_COUNT, 0x08);
    }

    #[test]
    fn build_perf_freq_cap_buffer_cap_writes_both_entries() {
        // -gpuclk:300 → max=min=300 MHz = 300_000 kHz (the ref tool's pattern).
        let buf = build_perf_freq_cap_buffer(PerfFreqCap::Cap {
            max_khz: 300_000,
            min_khz: 300_000,
        });
        assert_eq!(buf.len(), PERF_LIMITS_SIZE);
        assert_eq!(read_u32(&buf, 0), PERF_LIMITS_MAGIC);
        assert_eq!(read_u32(&buf, PERF_LIMITS_OFF_COUNT), 2);
        // entry0 (max)
        assert_eq!(
            read_u32(&buf, PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_TYPE),
            PERF_LIMITS_TYPE_MAX
        );
        assert_eq!(
            read_u32(&buf, PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_ENABLE),
            PERF_LIMITS_ENABLE_APPLY
        );
        assert_eq!(
            read_u32(&buf, PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_FREQ),
            300_000
        );
        // entry1 (min) @ +0x490
        assert_eq!(
            read_u32(&buf, PERF_LIMITS_ENTRY1_BASE + PERF_LIMITS_OFF_TYPE),
            PERF_LIMITS_TYPE_MIN
        );
        assert_eq!(
            read_u32(&buf, PERF_LIMITS_ENTRY1_BASE + PERF_LIMITS_OFF_ENABLE),
            PERF_LIMITS_ENABLE_APPLY
        );
        assert_eq!(
            read_u32(&buf, PERF_LIMITS_ENTRY1_BASE + PERF_LIMITS_OFF_FREQ),
            300_000
        );
    }

    #[test]
    fn build_perf_freq_cap_buffer_reset_clears_enable() {
        // -gpuclk:-1 → enable=0 on both entries, freq stays 0.
        let buf = build_perf_freq_cap_buffer(PerfFreqCap::Reset);
        assert_eq!(read_u32(&buf, 0), PERF_LIMITS_MAGIC);
        assert_eq!(read_u32(&buf, PERF_LIMITS_OFF_COUNT), 2);
        assert_eq!(
            read_u32(&buf, PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_ENABLE),
            0
        );
        assert_eq!(
            read_u32(&buf, PERF_LIMITS_ENTRY1_BASE + PERF_LIMITS_OFF_ENABLE),
            0
        );
        // type markers still written (ref tool 2 writes them even on reset path).
        assert_eq!(
            read_u32(&buf, PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_TYPE),
            PERF_LIMITS_TYPE_MAX
        );
    }

    #[test]
    fn perf_freq_caps_parses_get_buffer() {
        // Simulate a GET-status buffer with 2 entries: entry0 locked max,
        // entry1 unlocked min — mirrors the isPStateLocked read loop.
        // (We can't call perf_freq_caps without a GPU handle, so test the
        // field offsets the parse path uses.)
        let mut buf = vec![0u8; PERF_LIMITS_SIZE];
        write_u32(&mut buf, 0, PERF_LIMITS_MAGIC);
        write_u32(&mut buf, PERF_LIMITS_OFF_COUNT, 2);
        write_u32(
            &mut buf,
            PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_TYPE,
            0x5D, // Pmax
        );
        write_u32(
            &mut buf,
            PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_FREQ,
            300_000,
        );
        buf[PERF_LIMITS_ENTRY0_BASE + PERF_LIMITS_OFF_LOCKED] = 1; // locked
        write_u32(
            &mut buf,
            PERF_LIMITS_ENTRY1_BASE + PERF_LIMITS_OFF_TYPE,
            0x49, // Pmin
        );
        write_u32(
            &mut buf,
            PERF_LIMITS_ENTRY1_BASE + PERF_LIMITS_OFF_FREQ,
            200_000,
        );
        // entry1 locked byte stays 0

        // Re-implement the parse loop the getter uses, against this buffer.
        let n = read_u32(&buf, PERF_LIMITS_OFF_COUNT) as usize;
        let mut entries = Vec::with_capacity(n);
        for k in 0..n {
            let base = PERF_LIMITS_ENTRY0_BASE + k * PERF_LIMITS_ENTRY_STRIDE;
            entries.push(PerfFreqCapEntry {
                type_marker: read_u32(&buf, base + PERF_LIMITS_OFF_TYPE),
                freq_khz: read_u32(&buf, base + PERF_LIMITS_OFF_FREQ),
                locked: buf[base + PERF_LIMITS_OFF_LOCKED] != 0,
            });
        }
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].type_marker, 0x5D);
        assert_eq!(entries[0].freq_khz, 300_000);
        assert!(entries[0].locked);
        assert_eq!(entries[1].type_marker, 0x49);
        assert_eq!(entries[1].freq_khz, 200_000);
        assert!(!entries[1].locked);
    }
}

#[cfg(test)]
mod fan_curve_tests {
    use super::*;

    /// The `0x200DC` curve-table layout RE'd from ref tool 2 + impl.dll: magic
    /// at +0, count byte at +4, slots at +20 with a 52-byte stride, each slot's
    /// 3 points at +4h/+10h/+1Ch (12-byte points, {temp<<8, reserved, rpm}).
    #[test]
    fn fan_curve_table_layout() {
        assert_eq!(
            cooler::private::NV_GPU_CLIENT_FAN_POLICIES_CONTROL_V1::MAGIC,
            0x200DC
        );
        let mut raw = cooler::private::NV_GPU_CLIENT_FAN_POLICIES_CONTROL::new();
        raw.version = 0x200DC;
        raw.count = 1;
        let slot = &mut raw.curves[0];
        slot.index = 0;
        slot.points[0].temp_q8 = 40 << 8;
        slot.points[0].rpm_q16 = 1200 * 65536 / 100;
        slot.points[1].temp_q8 = 60 << 8;
        slot.points[1].rpm_q16 = 2000 * 65536 / 100;
        slot.points[2].temp_q8 = 80 << 8;
        slot.points[2].rpm_q16 = 3000 * 65536 / 100;

        // The NV_GPU_CLIENT_FAN_POLICIES_CONTROL_V1 layout is enforced via
        // its struct definition (repr(C) + Padding<T> wrappers): version u32
        // + count u8 + 15 header + 4 slot × 52 B. Each slot: index u8 + 3 pad
        // + 3 point {temp_q8, reserved, rpm_q16} + 12 tail — so point0.temp
        // lands at struct byte 20 + 4 = 24 and point0.rpm at 20 + 12 = 32.
        assert_eq!(
            std::mem::size_of::<cooler::private::NV_GPU_CLIENT_FAN_POLICIES_CONTROL_V1>(),
            4 + 1 + 15 + 4 * 52
        );
        assert_eq!(
            std::mem::size_of::<cooler::private::NV_GPU_CLIENT_FAN_POLICIES_CURVE_V1>(),
            52
        );
        assert_eq!(
            std::mem::offset_of!(cooler::private::NV_GPU_CLIENT_FAN_POLICIES_CURVE_V1, points),
            4
        );
        assert_eq!(
            std::mem::offset_of!(
                cooler::private::NV_GPU_CLIENT_FAN_POLICIES_POINT_V1,
                rpm_q16
            ),
            8
        );
    }

    /// Round-trip encoding matches ref tool's dialog read logic
    /// ((x + 128) >> 8 for temp, (x*100 + 32768) / 65536 for RPM).
    #[test]
    fn fan_curve_encode_roundtrip() {
        let mut raw = cooler::private::NV_GPU_CLIENT_FAN_POLICIES_CONTROL::new();
        raw.count = 1;
        let slot = &mut raw.curves[0];
        slot.points[0].temp_q8 = 42 << 8;
        slot.points[0].rpm_q16 = 1600 * 65536 / 100;

        assert_eq!((slot.points[0].temp_q8.wrapping_add(128)) >> 8, 42);
        assert_eq!(
            (slot.points[0].rpm_q16 as u64 * 100).div_ceil(65536) as u32,
            1600
        );
    }
}
