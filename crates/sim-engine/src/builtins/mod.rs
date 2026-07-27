//! $-task handlers (inlined for v1; HOOK: extract to hdl-builtins post-v1).
//! Handles $dumpfile/$dumpvars/$dumpoff/$dumpon/$dumpall → vcd-writer,
//! $display/$write/$monitor/$strobe formatting → stdout sink, $finish/$stop.

use std::io::Write;

use sim_ir::SysTaskId;
use vcd_writer::{IdCode, ScopeType};

use crate::eval::NetReader;
use crate::sched::Scheduler;
use crate::state::{vcd_var_reference_decl, vcd_var_type, FmtCapture, MonitorState, SimState};
use crate::value::Value;

// ---- split parts (mechanical refactor) ----
mod crv_draw;
mod dispatch;
mod queues_io;
mod radix_arr;
mod render;
pub(crate) use crv_draw::*;
pub(crate) use dispatch::*;
pub(crate) use queues_io::*;
pub(crate) use radix_arr::*;
pub(crate) use render::*;

/// Control-flow signal back to the executor.
pub(crate) enum Ctl {
    Continue,
    Finish,
    Stop,
    /// Runtime `$fatal` (RunFatal): abort the run with `ExitClass::Fatal`.
    Fatal,
}
