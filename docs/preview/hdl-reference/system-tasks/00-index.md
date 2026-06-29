# 00 · System Tasks · Functions 인덱스

`$`로 시작하는 표준 빌트인은 본 프로젝트 `hdl-builtins` 크레이트가 구현한다.

## 카테고리 인덱스

| # | 파일 | 주요 항목 | Phase |
|---|---|---|---|
| 01 | [display-io](01-display-io.md) | `$display`/`$write`/`$monitor`/`$strobe` + b/o/h 변형 | 1 |
| 02 | file-io | WRITE: `$fopen`/`$fclose`/`$fwrite`/`$fdisplay`(+b/o/h, MCD)/`$sformat`/`$sformatf` ✅; READ `$fread`/`$fscanf`/`$fgets`/`$sscanf`(+`$feof`/`$fgetc`) ✅ (v9 — blocking-assign rhs 전용); `$fmonitor`/`$fstrobe`는 미구현(silent-degrade) | 2 (READ/WRITE 완료) |
| 03 | memory-load | `$readmemb`/`$readmemh` ✅; `$writememb`/`$writememh` ✅ (v9) | 2 (완료) |
| 04 | [simulation-control](04-simulation-control.md) | `$finish`/`$stop` ✅; `$exit` ✅ (v9 — `$finish` 별칭) | 1 (완료) |
| 05 | [time-functions](05-time-functions.md) | `$time`/`$realtime`/`$stime` ✅ | 1 |
| 06 | conversion | `$signed`/`$unsigned`/`$rtoi`/`$itor`/`$bitstoreal`/`$realtobits` ✅ (`$shortrealtobits`/`$bitstoshortreal` 미구현) | 2 (완료) |
| 07 | bit-vector | `$bits`/`$clog2`/`$countones`/`$countbits`/`$onehot`/`$onehot0`/`$isunknown` | 2 |
| 08 | math | `$pow`/`$ln`/`$log10`/`$exp`/`$sqrt`/`$sin`/`$cos`/`$tan` 등 | 2 |
| 09 | random | `$random`/`$urandom`/`$urandom_range`/`$dist_*` | 2 |
| 10 | vcd-dump | `$dumpfile`/`$dumpvars`/`$dumpon`/`$dumpoff`/`$dumpall`/`$dumpflush`/`$dumplimit` | 1 |
| 11 | assertion-sampling | `$past`/`$rose`/`$fell`/`$stable`/`$changed`/`$sampled`/`$assertoff`/`$asserton`/`$assertkill` | 2 |
| 12 | introspection | `$typename`/`$cast`/`$isunbounded`/`$size`/`$left`/`$right`/`$low`/`$high`/`$increment` | 2 |
| 13 | misc | `$value$plusargs`/`$test$plusargs`/`$system` 등 | 2 |

## Phase 1 핵심 셋 (MVP 1일차부터)

다음 태스크는 가장 단순한 RTL 테스트벤치를 실행하기 위한 최소 요건이다.

- **display**: `$display`, `$write`, `$monitor`, `$strobe`
- **time**: `$time`, `$realtime`
- **control**: `$finish`, `$stop`
- **dump**: `$dumpfile`, `$dumpvars`, `$dumpon`, `$dumpoff`, `$dumpall`

이 셋이 없으면 Hello-World 수준의 시뮬레이션도 완료 확인이 불가능하다.
`$finish` 없이는 시뮬이 끝나지 않고, `$display` 없이는 결과를 볼 수 없다.

## Phase 2 확장 (대부분 구현 완료)

Phase 1 이후 파일 I/O(WRITE family), 메모리 초기화(`$readmem*`), 비트 연산 함수
(`$bits`/`$countones`/`$onehot`/`$isunknown`), 난수 생성(`$random`/`$urandom`/`$urandom_range`,
IEEE Annex N 핀)을 추가 완료. **v9(2026-06-18) 일괄 추가**: 파일 READ family
(`$fread`/`$fscanf`/`$fgets`/`$sscanf`/`$feof`/`$fgetc`)·`$writememb/h`·`$countbits`·
introspection(`$typename`/`$cast`/`$size`/`$left`/`$right` 등)·`$changed`/`$sampled`·
`$exit`(=`$finish` 별칭)·`$monitoron/off`·`$dist_uniform`. 수학 transcendentals
(`$ln`/`$exp`/`$sqrt`/`$sin` 등)·비-uniform `$dist_*`·`$system`·`$assertcontrol`만
미구현(pure-Rust libm 결정성 핀 보류 등 → loud-reject까지 deferred). 각 카테고리는 별도 파일로
관리하며, 위 인덱스의 파일 이름으로 cross-link된다.

## Phase 3 / 후속 (현행 구현 상태)

- **SV assertion sampling 구현(2026-06-14, Phase-3)**: `$past`/`$rose`/`$fell`/`$stable`는
  concurrent assert 서브셋과 함께 `rewrite_sampled`(signal당 공유 prev-reg) 경로로 구현 완료.
  hand-IEEE 핀(iverilog 13.0이 SVA + $past류를 거부 → 오라클 부재). `$changed`/`$sampled`,
  assertion control(`$assertoff`/`$asserton`/`$assertkill`)도 **구현 완료(✅, SVA-REST)** — `$assertoff`/`$assertkill`은
  전역 fire-gate로 후속 fire 억제, `$asserton`은 재활성화(scoped 폼은 loud).
- **미구현 (deferred)**: 확장 VCD 태스크(`$dumpports*`) 및 FST 포맷은 현재 비목표.
  표준 준수 범위가 합성 가능 RTL 서브셋을 넘어선 검증 전용 기능은 별도 로드맵에서 결정한다.

## Sources

- 본 spec §9 (Phase별 system tasks 목록)
- IEEE 1800-2017 §20, IEEE 1364-2005 §17
- research-log: [system-tasks-display-time-2026-05-28.md](../../research-log/system-tasks-display-time-2026-05-28.md)
