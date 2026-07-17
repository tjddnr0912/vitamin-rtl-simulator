//! Dev-only `separate-bins` shim (doc-03): a standalone `velab` binary sharing
//! the multicall code path — `argv[0]` is "velab", so dispatch is identical.
fn main() {
    cli::driver_main()
}
