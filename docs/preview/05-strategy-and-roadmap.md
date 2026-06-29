# 05 · 전략 · 로드맵 (SystemVerilog-first)

## 전략 요지

IEEE 1800(SystemVerilog)은 IEEE 1364(Verilog)를 흡수한 상위 호환 표준이다. 따라서 SV RTL 서브셋을 구현하면 Verilog-2005 RTL 전체가 자동으로 커버된다 — 별도 Verilog 프론트엔드는 필요 없다. 단일 SV 프론트엔드로 두 언어를 동시에 지원하는 것이 이 전략의 핵심이다.

VHDL(IEEE 1076)은 다른 언어이므로 별도 프론트엔드(lexer/parser/elaborator)가 필요하다. 그러나 elaborate 이후 단계 — sim-ir, sim-engine, hdl-builtins, vcd-writer — 는 언어 중립 설계이므로 SV 경로와 공유한다. VHDL 프론트엔드는 공유 IR 위에 얹는 구조다.

---

## Phase 1 — MVP

**범위:** SV 합성가능 RTL 서브셋. Verilog-2005 RTL 전체를 포함한다.

**산출물:** preprocess → lex → parse → elaborate → event-driven sim → VCD. 백엔드는 인터프리터 방식(IR-walking).

`timescale` 정밀도와 VCD 생성은 MVP 1일차부터 지원한다. 정확성 없이 속도만 높이는 방향은 택하지 않는다.

**system tasks 핵심 셋 (Phase 1):**

- 출력: `$display` / `$write` / `$monitor` / `$strobe` (형식 변형 포함)
- 시간 조회: `$time` / `$realtime`
- 시뮬레이션 제어: `$finish` / `$stop`
- VCD dump 패밀리: `$dumpfile` / `$dumpvars` / `$dumpon` / `$dumpoff` / `$dumpall`

**Phase 1 마일스톤:**

1. 단일 모듈 + `always`/`always_ff`/`always_comb`/`always_latch` 블록 / clock 토글 + `$display` 정상 동작
2. 다계층 모듈 + parameter resolution
3. `$dumpvars` 호출로 VCD 파일 생성
4. Icarus Verilog와 차등검증 PASS (신호값·천이 시각 일치)

**Phase 1 인프라 트랙 (코어와 병행 — Phase 1 완료 게이트):**

- **단계 산출물 직렬화** — `vita-artifact`(+`vita-artifact-derive` proc-macro): `work/` 라이브러리 + `.velab` 스냅샷 + 구조적 schema 해시(D2). (→ §14)
- **filelist 전개기** — `-f`/`-F` 재귀 중첩, `+incdir+`/`+define+` 집계. (→ §14 §3.1)
- **진단/로깅** — `diag`(단일 진단 렌더) + `vita-log`(transcript·로그파일 tee·severity·exit-code). (→ §13)
- **에러 코드 카탈로그** — 안정 `MsgCode` + §15 레퍼런스 + CI 1:1 동기 게이트.
- **건전성 게이트 (Phase 1 PASS 조건):** RULE V(stale 스냅샷 `vrun` 거부), RULE S(디렉티브 carryover 해시), API(typed PreprocInputs/ElabInputs)가 동작·테스트돼야 한다.

---

## Phase 2 — SV 확장

**범위:** Phase 1 RTL 서브셋에서 SV 고유 구문으로 확장.

**주요 언어 기능:** `interface` / `modport`, `package`, `struct` / `enum` / `typedef`, `foreach`, `unique` / `priority` (구조적 SV)

> `always_comb`/`always_ff`/`always_latch`는 합성가능 RTL이므로 **Phase 1로 이동**한다. 1차 근거는 06 엔진 스펙의 Phase-1 예제·auto-sensitivity 동작이며, `W-ELAB-ALWCOMBORDER`(W3046, §15 부록 A의 MVP-SIM 인벤토리 코드)도 이를 전제한다. Phase 2에는 구조적 SV만 남는다.
>
> **✅ Phase 2 전 항목 완료(2026-06-12, format_version 7).** `interface`/`modport`·`package`/`import`/`pkg::`·`struct`/`enum`/`typedef`·`foreach`·`unique`/`priority`·full `string`·동적 배열/queue/연관 배열 전부 IN 승격. 파일 I/O·`$readmemb/h`·`$random`(Annex N)·plusargs·bit-vector(`$bits`/`$countones`/`$onehot(0)`/`$isunknown`)도 함께 승격. **이후 v9(2026-06-18)에서 추가 승격**: 파일 READ 패밀리(`$fread`/`$fscanf`/`$fgets`/`$sscanf`/`$feof`/`$fgetc`)·`$writememb/h`·`$countbits`·introspection(`$typename`/`$cast`/`$size`/`$left`/`$right`/`$low`/`$high`/`$increment`/`$dimensions`/`$isunbounded`)·`$changed`/`$sampled`·`$exit`(=`$finish` 별칭)·`$monitoron/off`·`$dist_uniform`. **잔여 컷(loud)**: math transcendentals(N6)·비-uniform `$dist_*`(`$dist_normal`/`$dist_exponential` 등)·`$system`·`$assertcontrol`.

**system tasks 확장 셋 (Phase 2):**

- 파일 I/O: ✅ `$fopen` / `$fclose` / `$fwrite` / `$fdisplay`(구현, v7 — b/o/h·MCD) · `$sformat` / `$sformatf`(구현) · `$fread` / `$fscanf` / `$fgets` / `$sscanf` / `$feof` / `$fgetc`(✅ 구현, v9 — blocking-assign rhs)
- 메모리 로드: ✅ `$readmemh` / `$readmemb`(구현, v7) · `$writememh` / `$writememb`(✅ 구현, v9)
- 변환: `$signed` / `$unsigned` / `$rtoi` / `$itor` / `$bitstoreal` / `$realtobits`
- 비트벡터: ✅ `$bits` / `$clog2` / `$countones` / `$onehot` / `$onehot0` / `$isunknown`(구현, v7) · `$countbits`(✅ 구현, v9)
- 수학: `$pow`(정수=구현) · `$ln` / `$log10` / `$exp` / `$sqrt` / `$sin` / `$cos` / `$tan`(*math transcendentals — N6, pure-Rust libm 3-OS 결정성 핀 대기로 loud reject*)
- random: ✅ `$random`(IEEE Annex N) / `$urandom` / `$urandom_range`(구현, v7 — 자체 계약) · `$dist_uniform`(✅ 구현, v9) · 비-uniform `$dist_*`(`$dist_normal`/`$dist_exponential` 등, 후속·loud)
- assertion 샘플링: ✅ `$past` / `$rose` / `$fell` / `$stable` / `$changed` / `$sampled`(*전부 구현 — prev-reg desugar, Phase-3 SVA 트랙*)
- introspection: ✅ `$typename` / `$cast` / `$size` / `$left` / `$right` / `$low` / `$high` / `$increment` / `$dimensions` / `$isunbounded`(✅ 구현, v9 — const-fold/태스크·함수 양형)
- 기타: ✅ `$value$plusargs` / `$test$plusargs`(구현) · `$exit`(✅ 구현, v9 — `$finish` 별칭) · `$system`(후속·loud)

**Phase 2 마일스톤:**

1. ✅ `package` + `typedef` 정상 동작 (v7)
2. ✅ `interface`로 신호 그룹 전달
3. ✅ assertion 샘플링 함수(`$past` / `$rose` / `$fell` / `$stable`) 동작 — 구현 완료(Phase-3 SVA 트랙)

---

## Phase 3 — VHDL

> **현 상태(2026-06):** Phase 3은 **SVA(SystemVerilog Assertions) 서브셋 트랙으로 먼저 진입·완료**됐다
> (format_version 8, 순수 IR-0 desugar — 단일/다중-클럭 concurrent assert·전 시퀀스 연산자(`##n`/`##[m:n]`/`##[m:$]`/`[*n]`/`[*m:n]`/`throughout`/`[->n]`/`[=n]`/`within`)·sampled
> value func·named property/sequence+formal args·property-level `and`/`or`·recursive property(tail)·cross-clock N-clock chains·generate-scope·disable iff·module-level assert property·action block; 상세 = `docs/ROADMAP.md` §4.3 #6).
> **이어 frame-call(automatic/recursive 콜스택, B-track 2026-06-17)·repeat-event NBA(N1)·계층 read-only 이름 참조(N3/N3.1)도 완료**(전부 순수 IR-0).
> **faithful deferred immediate asserts**(`assert #0`=Observed·`assert final`=Reactive)도 구현(22탄). 잔여 SVA(loud, 후속) = multi-term cross-clock segment lane·sequence local variable(N2c=조건부 defer)·outer-`|=>` prop-ref skew 고급형.
> **clocking block은 조건부 NO-GO**(N4 — Preponed/sampled-value 스케줄링 리전 부재로 `@(cb); x=cb.sig`서 1-cycle lag silent-wrong, 스파이크 후 revert).
> 아래 VHDL 프론트엔드는 Phase-3의 **후속** 트랙이며 아직 미착수다.

**범위:** IEEE 1076 프론트엔드를 별도로 구축하되, elaborate 이후 공유 IR 위에 얹는다.

재사용 대상: sim-ir, sim-engine, hdl-builtins, vcd-writer — Phase 1/2와 동일 경로.

**빌트인 패키지:** `std_logic_1164` / `numeric_std`는 VHDL 프론트엔드가 인식하는 빌트인으로 처리한다.

**Phase 3 마일스톤:**

1. `entity` / `architecture` + signal assignment 동작
2. `process` + `wait` 동작
3. IEEE 패키지(`std_logic_1164`, `numeric_std`) 빌트인 동작

---

## 상시 횡단

단계 구분 없이 모든 Phase에 걸쳐 지속 관리하는 항목:

- **timescale 정밀도** — 서로 다른 `timescale` 모듈 혼재 시에도 전역 시간축이 어긋나지 않음
- **VCD 생성** — RTL dump 태스크 호출 기반, golden diff 회귀
- **차등검증** — Icarus Verilog / Verilator와 신호값·천이 시각 비교
- **진단 품질** — 소스 위치 포함 오류 메시지, 사용자가 직접 수정 가능한 수준
- **system tasks 컴플라이언스 코퍼스** — 범주별 최소 1개 케이스 전수 통과

---

## 후속 (여력 시)

현재는 비목표이나 IR 경계를 열어두고 후속 단계에서 검토한다:

- **컴파일드/JIT 백엔드** — `sim-ir` 경계를 재사용해 Verilator 계열 성능 확보
- **FST 파형** — LZ4 압축 기반, 대용량 설계 대응
- **SV assertion 영역 확장** — Preponed / Observed / Reactive / Postponed (program block용)
- **확장 VCD** — `$dumpports*` 지원

---

## 리스크 / 의존성

**SV 범위 통제:** SystemVerilog 표준은 방대하다. Phase 1을 합성가능 RTL 서브셋으로 엄격히 제한하지 않으면 MVP 범위가 무한 확장된다. 기능 추가 압력에 대해 Phase 경계를 명시적으로 유지해야 한다.

**차등검증 도구 간 의미 차이:** Icarus Verilog와 Verilator는 표준의 미묘한 부분에서 구현이 다를 수 있다. 두 도구가 충돌할 때는 IEEE LRM(1차 표준)을 최종 권위로 삼는다.

**system tasks 비결정 영역:** `$urandom`의 seed·스트림 관리, `$random`의 분포, `$readmemh`의 주소 해석 등 도구마다 구현이 다른 영역이 있다. 이 항목들은 "표준 정의 + Icarus 의미"를 기준으로 명시적으로 문서화하고, 컴플라이언스 코퍼스에 해당 케이스를 포함한다.

---

## Sources

- 본 spec §9 (로드맵) — `docs/superpowers/specs/2026-05-26-vitamin-rtl-simulator-design.md`
- 본 spec §2 (목표/비목표), §5 (아키텍처), §8 (검증 전략), §14 (리스크) — 상동
