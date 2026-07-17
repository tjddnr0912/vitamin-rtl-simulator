//! Dev-only `separate-bins` shim (doc-03): a standalone `vrun` binary sharing
//! the multicall code path — `argv[0]` is "vrun", so dispatch is identical.
fn main() {
    cli::driver_main()
}
