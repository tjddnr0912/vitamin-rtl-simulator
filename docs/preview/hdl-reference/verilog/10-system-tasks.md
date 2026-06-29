# 10 · System Tasks (Verilog 관점)

Verilog (IEEE 1364-2005)의 표준 시스템 태스크/함수 개요. 자세한 카테고리별 참조는
`../system-tasks/`로 cross-link.

---

## Verilog에 포함된 시스템 태스크 카테고리

| 카테고리 | 주요 항목 | 상세 |
|---|---|---|
| Display & I/O | `$display`, `$write`, `$monitor`, `$strobe` + `b/o/h` 접미사 | [../system-tasks/01-display-io.md](../system-tasks/01-display-io.md) |
| File I/O | `$fopen`, `$fclose`, `$fwrite`, `$fdisplay`, `$fread` | [../system-tasks/02-file-io.md](../system-tasks/02-file-io.md) |
| Memory load | `$readmemb`, `$readmemh` | [../system-tasks/03-memory-load.md](../system-tasks/03-memory-load.md) |
| Sim control | `$finish`, `$stop` | [../system-tasks/04-simulation-control.md](../system-tasks/04-simulation-control.md) |
| Time | `$time`, `$stime`, `$realtime` | [../system-tasks/05-time-functions.md](../system-tasks/05-time-functions.md) |
| Conversion | `$signed`, `$unsigned`, `$rtoi`, `$itor`, `$bitstoreal`, `$realtobits` | [../system-tasks/06-conversion.md](../system-tasks/06-conversion.md) |
| VCD dump | `$dumpfile`, `$dumpvars`, `$dumpon`, `$dumpoff`, `$dumpall`, `$dumpflush`, `$dumplimit` | [../system-tasks/10-vcd-dump.md](../system-tasks/10-vcd-dump.md) |
| Random | `$random`, `$dist_*` | [../system-tasks/09-random.md](../system-tasks/09-random.md) |

## SV 전용 (Verilog 미포함)

`$urandom`/`$urandom_range`, `$past`/`$rose`/`$fell`/`$stable`/`$changed` (assertion
sampling), `$bits`/`$clog2`/`$countones`, `$value$plusargs`/`$test$plusargs`는
IEEE 1800 (SystemVerilog) 확장이다. Verilog-2005 단독 환경에서는 사용 불가.

자세한 내용은 `../system-tasks/` 폴더 전체와
`../systemverilog/08-functions-tasks.md` 참조.

## 합성 가능 여부

❌ 모든 시스템 태스크/함수는 **비합성**이다. 시뮬레이션 및 검증 전용.
합성 도구는 `$display`, `$monitor` 등 시스템 태스크를 무시하거나 경고를 낸다.
RTL 코드 안에 시스템 태스크를 넣어야 한다면 `` `ifndef SYNTHESIS `` / `` `endif ``
로 감싸는 것이 표준 관행이다.

## Sources

- IEEE 1364-2005 §17 (system tasks and functions)
- ../system-tasks/ 폴더 전체 (카테고리별 상세)
