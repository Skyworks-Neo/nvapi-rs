# nvapi-macros

Proc-macros for the `nvapi` / `nvapi-sys` crates. Not useful standalone.

- `nvenum!` / `nvbits!` / `nvenum_display!` — generate the typed enum /
  bitflags types with their FFI alias (`NvValue<T>` newtypes, ABI-equivalent
  to the bare `c_int` / `u32` aliases they replace)
- `nvstruct!` — FFI struct definitions with zerocopy derives, alignment
  padding (`#[nv_align]`), an opt-out for fields zerocopy rejects
  (`#[nv_unchecked]`), and a generated `zeroed()` constructor
- `nvinherit!` / `#[nv_inherit]` — shared struct field blocks
- `nvversion!` — version-dword stamping (`StructVersion` impls, size pins)

See the `sys` crate for the actual NVAPI bindings.
