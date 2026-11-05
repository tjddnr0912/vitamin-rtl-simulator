(§5.b MEM-GUARD · BLOCKED) (no guard on process memory):
* symptom: no process memory guard
* fix: RSS watchdog + `--max-mem`
* status: BLOCKED (macOS FFI is unsafe)
