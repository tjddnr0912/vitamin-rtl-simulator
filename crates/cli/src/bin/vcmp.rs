//! Dev-only `separate-bins` shim (doc-03): a standalone `vcmp` binary sharing
//! the multicall code path — `argv[0]` is "vcmp", so dispatch is identical.
fn main() {
    cli::driver_main()
}
