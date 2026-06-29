# 18 · 병렬/GPU 가속 분석

> 2026-06-05 코드 기반 분석. 결론: **이벤트구동 RTL sim 코어는 GPU 비적합**; CPU word化+SIMD와 컴파일드 백엔드가 실질 가속 경로.

> **⚠️ 2026-06-07 실측 갱신 (아래 [§실측](#실측-2026-06-07--예측-vs-측정) 참조):** Stage C 바이트코드 VM 구현 후 `/usr/bin/sample`
> 프로파일링 결과, 본 문서가 지목한 **두 병목(eval 트리워크 디스패치·`Value` 힙할당) 예측이 1차 측정엔 모두 빗나갔다.** 진짜 지배
> 비용은 **bit-serial bit-by-bit 처리**(net read/write·shift·resize — 인터프리터·VM **공유** 경로)였고, 이를 word化/inline으로
> 정리해 **누적 ~6x**(eval-heavy 2781→461ms). **다만 2차 재평가(2026-06-07 오후):** "eval = ~2-4%·native-eval
> 저ROI"는 *eval-light 벤치마크의 산물*이었다 — 연산자수 스윕으로 eval 비중이 식 복잡도에 선형(K=16 **70%**,
> K=32 **82%**)임을 측정. **native-eval ROI는 워크로드 의존**: 식-바운드 RTL엔 고ROI(설계당 ~2-3x), 스케줄러-
> 바운드엔 저ROI. 향후 방향: [`../ROADMAP.md`](../ROADMAP.md).

## 요약 판정

| 방향 | 평가 | 이유 |
|---|---|---|
| GPU(Metal/CUDA/일반) 코어 엔진 | ❌ 비권장 | 이벤트구동 RTL은 분기발산·희소활성·시간인과·포인터추적으로 GPU-적대적 |
| CPU word-level + SIMD(NEON/AVX) | ✅ 실질 이득, 저위험 | 4-state 비트연산이 현재 bit-by-bit → word化만으로 ~64×, SIMD 추가. **실측: net I/O·shift·resize·read까지 word化/inline 확장 → 누적 ~6x** |
| 멀티코어 PDES(timestep 내) | ❎ **조건부 NO-GO (2026-06-11 연구 종결)** | 결정성은 차단 요인 아님(보존 설계 존재) — 진짜 제약은 워크로드 폭(W=1~8)·직렬 잔류 ~20%(Amdahl 상한 T4 ≈2.5x)·공수. [§PDES 타당성 연구](#pdes-타당성-연구-2026-06-11--p4-t4-종결) |
| 컴파일드 백엔드(코드젠) | ⚠️ **워크로드 의존** (식-바운드 고ROI / 스케줄러-바운드 저ROI) | **2차 재평가:** eval 비중은 식 복잡도에 선형(K=16 70%·K=32 82%). 식-바운드(ALU·crypto·깊은 조합)엔 native-eval ~2-3x; 스케줄러-바운드엔 작음. 고위험·다세션, P5 게이트가 정확성 상쇄. [§실측 native-eval 재평가](#native-eval-재평가-2026-06-07-오후--위-저roi-판정-정정) |
| stimulus-parallel GPU(Monte-Carlo) | 별개 제품 | branch-free cycle-based 엔진 신규 필요 |

## 왜 이벤트구동 RTL은 GPU-적대적인가
1. **분기 발산**: 프로세스마다 데이터의존 if/case/loop → GPU SIMT warp 발산 = 처리량 붕괴.
2. **희소 활성도**: 매 timestep 변한 net/process만 재평가(극소수). GPU는 조밀·균일 작업을 원함.
3. **시간 인과성**: 시각 T는 T-1 의존 → timestep *간* 병렬 불가. timestep *내* 독립 프로세스만(곧 희소·발산).
4. **포인터 추적**: IR이 index-edge arena → 재귀 트리 평가 = uncoalesced 메모리.
5. **세밀 동기화**: NBA·델타·계층화 리전 = 배리어/atomic.
6. 상용(VCS/Xcelium/Questa) 전부 CPU. GPU sim 연구는 cycle-based 대규모 게이트넷·stimulus 병렬(회귀팜) 같은 *다른* 문제 대상.

## 코드에서 찾은 병렬화 기회

### ⭐ (1) 4-state 비트연산 word化 + SIMD — 최우선 — ✅ **구현 완료(2026-06-05)**
- (이전) `eval.rs` `bitwise()`/`BitNot`/6 리덕션이 **bit-by-bit** (`for i in 0..w { f(get_vu(i), …) }`, 64bit AND가 64회).
- (구현) val/unk 2-plane **word-parallel 공식**으로 교체 — `value.rs`의 `and_w`/`or_w`/`xor_w`/`xnor_w`/`not_w` + `eval.rs`의 `reduce_word`/`RedKind`.
  AND: known-0 = `(~av&~au)|(~bv&~bu)`, known-1 = `(~au&av)&(~bu&bv)`, rv=known1·ru=`~known0&~known1`.
  라스트 부분워드는 `low_mask`로 마스킹(not_w/xnor_w가 high 0&0→1). per-bit `*1`은 `#[cfg(test)]` 오라클로 보존(`word_vs_bit_parity`가 4×4 입력×NOT을 bit-exact 대조).
- 효과: 64비트당 64회→1회 + 브랜치리스 → LLVM 자동벡터화(NEON/AVX). wide 버스 큰 이득, 좁은 설계 무손해(1 word).
- **`std::simd` 미도입:** portable_simd는 **nightly 전용**이라 MSRV-1.82 stable + `--locked` 3-OS 바이트동일 핀과 충돌. 안정 u64 워드 루프가 이미 SIMD-친화형(64-lane/워드)이며 LLVM이 자동벡터화하므로 명시 SIMD 불요. 도입하려면 nightly 또는 `wide` 크레이트가 필요한데 둘 다 핵심 불변식을 깸 → 의도적 제외.
- 비교(`eval.rs` relational/eq)는 산술 레인(64/128bit 정수)이라 별개 경로 — 워드化 대상 아님.

### (2) for-loop copy block (사용자 지목)
- const-bound for/repeat는 elaborate서 UNROLL(straight-line, cap). 펼친 바디는 순차 실행.
- 일반 병렬화는 iteration 의존성 분석 필요(blocking `=` 시퀀스). 거대 메모리 init/`$readmemh`만 데이터병렬이나 GPU 런치 오버헤드가 패배 → CPU SIMD/threads 적합.

### (3) timestep 내 프로세스 병렬 (PDES)
- `sched.rs` active `batch`를 `for r in batch` 순차. 공유 net 쓰기 동기화 + `tie`(선언순) 결정성 충돌 → 결정적 merge 필요(큰 엔지니어링).
- **→ 2026-06-11 타당성 연구로 종결(조건부 NO-GO)** — 측정·설계 스케치·재진입 조건 = [§PDES 타당성 연구](#pdes-타당성-연구-2026-06-11--p4-t4-종결).

### (4) 대용량 메모리/init
- `state.rs:412` `expand_init`, VCD 대용량 배열 덤프 — 데이터병렬이나 일회성·폭 제한. CPU SIMD 적합.

## 권고 로드맵 (GPU-free 우선)
1. ✅ **완료(2026-06-05)**: 비트연산/reduction word化 (안정 u64; std::simd는 nightly 충돌로 제외, LLVM 자동벡터화로 흡수).
2. **중기·진짜 가속**: 컴파일드 백엔드(IR→네이티브 코드젠). 아래 §컴파일드 백엔드 참조 — vitamin은 **2a(컴파일드 이벤트구동)부터**.
3. **장기·선택**: 멀티코어 PDES(결정성 재설계) 또는 stimulus-parallel GPU(별개 모드).
4. **GPU 코어 엔진: 권장 안 함**.

## 컴파일드 백엔드 — 두 갈래 ("컴파일드" ≠ "사이클기반")

상용 시뮬레이터 계보로 보면 "컴파일드"와 "사이클기반"은 **직교**한다:

| | 인터프리티드 | 컴파일드 |
|---|---|---|
| **이벤트구동 (full 4-state)** | Cadence Verilog-XL, **현재 vitamin** | Synopsys VCS, Cadence Xcelium, Siemens Questa |
| **사이클기반 (보통 2-state)** | (드묾) | Verilator |

- **VCS**(*Verilog **C**ompiled-code **S**imulator*)가 컴파일드의 원조 — Verilog-XL(인터프리티드 레퍼런스)을 속도로 밀어냄. Cadence도 NC-Verilog(**N**ative **C**ompiled)→Incisive→Xcelium으로 컴파일드 전환. **상용 사인오프 시뮬레이터는 컴파일드이되 이벤트구동·4-state·타이밍을 그대로 유지**(글리치/X/Z 정확).
- **Verilator**는 컴파일드 **+ 사이클기반(2-state 기본)** — 인트라사이클 스케줄링을 버리고 클럭당 일괄 평가 → 10~100×지만 미세 타이밍/일부 4-state 포기(사인오프 부적합).

### vitamin 경로
- **2a. 컴파일드 이벤트구동 (VCS/Xcelium 길) — 먼저.** 기존 이벤트 커널·`val`/`unk` 4-state·word化(①)를 **그대로 두고** 프로세스 바디(BB의 Stmt/Expr)만 네이티브(Rust) 코드로 lowering. eval-디스패치/트리워크/Value 힙할당 제거. **의미 100% 보존**(인터프리터가 골든), 중간 가속. vitamin의 이벤트구동 코어와 자연 정합.
- **2b. 사이클기반 컴파일드 (Verilator 길) — 별도 공격적 모드.** combinational rank 정적 스케줄 + 클럭당 일괄 평가. 최대 가속이나 합성가능 서브셋·사이클 의미 제약. 2a 이후 옵트인 모드로.

## 결정 기록 — 코드젠 substrate (P0a/P0b · 2026-06-06)

> 컴파일드 백엔드 선결 체크리스트(`docs/REMAINING_WORK.md` Stage B)의 P0a/P0b를 확정한다. 출처: 워크플로 `wzeyxgedk` 6-매핑/3-비평 + 사용자 결정(2026-06-06).

### P0a — target form = **바이트코드 VM** (확정)

세 후보를 프로젝트 hard-constraint(cargo-only · `build.rs` 금지 · MSRV-1.82 핀 · `--locked` 3-OS 바이트동일) 기준으로 평가:

| 형식 | 속도 | 신규 의존 | 결정성 핀 | 판정 |
|---|---|---|---|---|
| 바이트코드 VM | ~2-5× | 없음(순수 Rust) | ✅ 전부 보존 | **선택** |
| 네이티브 Rust 방출 | 10-100× | 런타임 rustc/cc + libloading | ⚠️ host LLVM 재증명 필요 | 탈락(핀 충돌) |
| 타입드 IR-2 | ~3-8× | 없음 | ✅ 보존 | 차선(작업량 대비 이득 작음) |

**근거:** 네이티브 방출은 헤드라인 10-100×지만 런타임 호스트 툴체인 의존을 도입해 cargo-only·헤르메틱 `.velab→VCD`·3-OS 결정성 핀과 정면충돌한다(`build.rs`조차 금지하는 저장소에 런타임 `rustc`는 모순). vitamin의 goal #1은 *결정성*(README)이므로, 그 정체성을 깨지 않으면서 `doc-18:63`의 두 실측 병목 — **eval 트리워크 디스패치**(`run_process`의 `Stmt/Terminator` 재귀 + `EvalCtx::eval_ctx` 재귀)와 **`Value` 힙할당**(`Vec<u64>` per 연산) — 을 제거하는 바이트코드 VM을 선택한다. 인터프리터는 항상-가용 레퍼런스로 잔류, 바이트코드는 opt-in 가속 모드.

**바이트코드에서 "compile-time constant"(P10) 표현:** 정적 width/sign·폴드된 index/width/count는 **상수 풀(immediate operand 또는 const-pool 인덱스)** 로 인코딩 — 별도 노드 타입(IR-2)이나 Rust 리터럴(emit-Rust)이 아니라 op의 즉치 피연산자. P11의 shallow-fold·사이트별 fallback은 바이트코드 *컴파일 시점*에 한 번 계산해 immediate로 굳힌다.

### P0b — compile+load 메커니즘 = **N/A (in-process 바이트코드 인터프리터)**

바이트코드 VM은 런타임 코드 생성·로드가 없다. `.vu`/`.velab` 산출물·`vita`/`vrun` 실행경로·헤르메틱 계약 전부 무변경. P0b의 "런타임 rustc / cdylib+dlopen / static dispatch" 분기는 발생하지 않음(기록상 N/A). **결정성 따름정리:** VM opcode는 `value.rs`/`eval.rs`의 *동일한* 4-state·f64 프리미티브를 디스패치하므로(재구현 아님), float 포맷·산술 축이 인터프리터와 byte-for-byte 일치 — P3의 부담을 구조적으로 축소한다.

### 골든 영향 = 없음

바이트코드·VM·backend seam은 전부 `sim_ir::SimIr` 밖(SimOpts 사이드테이블/별도 크레이트). `schema_hash::<SimIr>()` 루트 불변(이후 2026-06-10 런타임-delay re-freeze로 format_version 4 — doc-17). P15의 kernel-ABI 버전은 `format_version`과 **독립** 필드로 별도 게이트.

## P3 — float/host-toolchain 결정성 계약 (2026-06-06)

> 컴파일드 백엔드의 정당성은 "3-OS 바이트동일 유지"인데, **float 경로가 최대 미고정 축**이다. 바이트코드 substrate(P0a) 덕분에 위험은 구조적으로 축소되지만(VM이 동일 함수 호출), 그 *reuse-only 규칙*을 계약으로 동결한다.

**동결된 float-path 표면 (바이트코드 경로는 재구현 금지 · verbatim 재사용 · no fast-math):**

| 함수 | 파일 | 결정성 근거 |
|---|---|---|
| `dec_field_width(n)` | builtins.rs:436 | ≤128은 u128 정수; >128만 `n·LOG10_2` f64 — 유일한 float-multiply(컬럼폭 힌트). 양 경로 동일 함수 |
| `fmt_dec` (real arm) | builtins.rs:454 | `x.round() as i64` saturating; NaN→0 |
| `fmt_real`(`%f`) | builtins.rs:480 | Rust `{:.*}` (libm 아님) |
| `fmt_real_e`(`%e`) | builtins.rs:499 | Rust `{:.p$e}` + 2자리 지수 패딩 |
| `format_g`(`%g`) | builtins.rs:520 | Rust `{:e}`로 지수 도출(**log10 의도적 회피** — libm transcendental은 3-OS 바이트동일 아님), ±0.0 canon |
| `Value::{from_f64,to_f64}`·`real_to_int_round` | value.rs:255,269,303 | int↔real `as f64`/round-half-away |

**계약:** 위 함수는 인터프리터와 바이트코드 VM이 *동일 인스턴스*를 호출한다(opcode가 별도 float 로직을 갖지 않음). 따라서 `%f/%e/%g/%t/%d-on-real` 및 >128bit `%d` 폭이 두 경로 byte-for-byte 일치 — 이는 P5(컴파일드==인터프리티드)로 강제되고, **단일-OS 체크인 골든**(`float_format_determinism_golden`, end_to_end.rs)이 cross-OS 재현성을 잠근다(모든 OS가 동일 리터럴 매칭 = cross-OS diff와 등가). CI는 ubuntu+macos에서 같은 골든을 돌려 OS별 발산 시 해당 leg 실패.

**잔여(릴리스 신뢰도, 코드젠 비차단):** CI에 OS-간 산출물 직접 diff 잡(별도 leg 출력 비교)은 골든-리터럴 방식으로 이미 등가 달성 — 추가는 nice-to-have.

## P8 — 커널-콜 순서 / 샘플링-모먼트 계약 (2026-06-06)

> 컴파일드 바디가 인터프리터와 byte-for-byte 일치하려면, "언제 무엇을 읽고/쓰는가"가 **계약**이어야 한다. 아래 7 모먼트를 동결한다 — P5 바이트-diff가 *버그 vs 의도*인지 분류하는 기준이자, Stage C VM이 반드시 재현(또는 위임)해야 할 목록.
>
> **핵심 구조적 사실:** 이 모먼트 대부분은 **커널측**(`write_lvalue`/`schedule_nba`/`propagate_changes`/`emit_vcd_change`)에 있다. 컴파일드 바디는 `Kernel` trait(P7b)로 *동일* 커널 메서드를 호출하므로 이들을 **공짜로** 재현한다. VM이 보존해야 할 유일한 바디측 불변식은 **문장 실행 순서**(→ `schedule_nba`가 텍스트 순서로 호출되어 `nba_seq`가 같게 부여됨)이다.

| # | 모먼트 | 위치 | 분류 | VM 의무 |
|---|---|---|---|---|
| 1 | cont-assign **선언순** fixpoint | sched.rs `settle_cont_assigns`:190 | 인터프리터-only | cont-assign은 프로세스 바디 아님(P13 전까지 항상 인터프리트) → VM 무관 |
| 2 | NBA: LHS 인덱스 **schedule-time 샘플** + `nba_seq` **정렬 적용** | sched.rs `schedule_nba`:802 / `apply_nba`:554 | 커널측 + **바디순서** | VM은 `k_schedule_nba`를 **문장 텍스트 순서**로 호출만 하면 됨(순서가 nba_seq) |
| 3 | blocking offset **statement-time 해석** | exec.rs `compute_effect`(P7a) | 바디(READ phase) | VM이 write 전에 `k_resolve_lvalue_offsets` 호출 — 이미 StmtEffect에 캡처 |
| 4 | in-body `@(sig)` **arm-snapshot**(Level만 arm=Some) | sched.rs `suspend_on`:791 | 인터프리터-only | `@`=Wait=suspend → **non-codegen**(P9 제외) → VM 무관 |
| 5 | delayed `assign #d` **last_ca 변화 키잉** | sched.rs:218 | 인터프리터-only | delayed cont-assign(P13) → VM 무관 |
| 6 | `propagate_changes` **prev-refresh-LAST** | sched.rs:658 | 커널측 | 스케줄러 내부 — 양 backend 공통, VM 무관 |
| 7 | **eager per-write VCD** 방출(글리치 충실) | state.rs `write_chunk`:386 | 커널측 | `k_write_lvalue`→`emit_vcd_change` 공유측(P4) → VM 공짜 재현 |

**결론:** 7 모먼트 중 VM이 *능동적으로* 책임지는 것은 **#2/#3 (NBA 순서·blocking offset)** 뿐이며, 둘 다 "문장을 인터프리터와 같은 순서로 실행"하면 자동 충족된다(StmtEffect가 #3을 캡처, 텍스트순 실행이 #2를 보장). 나머지(#1/#4/#5 인터프리터-only, #6/#7 커널측)는 backend-중립이다. 이 계약이 깨지면 P5가 즉시 red.

**corpus 커버리지(P6):** #2=`nba_sample`(`a[i]<=v; i=i+1`), #3=`mem_oob`/배열 쓰기, #7=`multi_write_glitch`(동일 delta 3회 쓰기), #1/#5=`cont_assign_mixed`(연속할당+클럭 프로세스 혼재). #4(in-body `@`)는 non-codegen이라 코드젠 corpus 비대상(인터프리터 433 테스트가 커버).

## 실측 (2026-06-07) — 예측 vs 측정

> Stage C C1·C2 구현 후 **측정 기반 최적화**. 도구: `/usr/bin/sample`(macOS 내장, sudo 불요) + `tests/perf_baseline.rs`
> (`#[ignore]`, `--ignored --nocapture`). 워크로드: eval-dominated 설계(`always @(posedge clk)` 내부 heavy `for` 루프).
> 각 fix는 bit-exact(441 tests + iverilog 차분 11이 스펙). **이 절은 위 §요약판정/§코드기회의 예측을 측정으로 정정한다.**

### 예측이 빗나간 지점

본 문서(§codegen 기회·§요약판정)는 두 병목을 지목했다 — **eval 트리워크 디스패치**와 **`Value` 힙할당**. 첫 프로파일은 둘 다 *지배적이 아님*을 보였다:

| doc-18 예측 | 1차 측정 결과 | 정정 |
|---|---|---|
| eval 트리워크 디스패치 = 주 병목 → native-eval(코드젠)이 답 | `eval_ctx` **~1.5%** (eval-light 벤치) | 워크로드 의존 — 식-바운드는 70-82%(아래 재평가). 1차 "저ROI"는 과도 |
| `Value` 힙할당(`Vec<u64>`/op) = 주 병목 | inline-Value 1차 실험 **~0** | 더 큰 비용에 *가려져* 있었음 |
| (미지목) | **`write_lvalue`+`set_vu`+`slice_word` = ~50%** | 진짜 #1 = **bit-serial net I/O** |

### 측정 주도 4 라운드 — 누적 ~6x

| R | fix | 무엇을 | eval-heavy | 누적 |
|---|---|---|---|---|
| — | C2 baseline | VM이 eval을 커널에 위임(=interp 동률) | 2781 ms | — |
| 1 | **word化 net I/O** | `state.rs` `slice_word`·`write_lvalue`·`write_chunk` per-bit→u64 워드 | 1274 ms | 2.18x |
| 2 | **word化 shift/resize** | `value.rs` `shr_fill`/`shl_grow`/`resize` per-bit→multi-word | 948 ms | 2.9x |
| 3 | **inline-Value** | `Value.val/unk` `Vec<u64>`→`Words`(≤128 inline, >128 heap) | 618 ms | 4.5x |
| 4 | **read/mask 정리** | `mask_top` resize 길이가드 + `read_net` inline 직접 read(transient `BitPacked` 제거) | 461 ms | **~6.0x** |

codegen-heavy(스케줄러 dominated) 196→61ms(~3.2x). VM eval-heavy 2699→385ms(0.84x interp). **모든 윈이 인터프리터·VM 공유 경로** — backend-전용 아님.

### native-eval 재평가 (2026-06-07 오후) — 위 "저ROI" 판정 정정

위 "eval = ~1.5% → native-eval 저ROI"는 **eval-LIGHT 벤치마크의 산물**이었다(`EVAL_HEAVY`는 문장당
연산자 ~3개). eval 비용은 *식 복잡도에 선형*이라 단일 벤치 한 점으로 일반화할 수 없다. **연산자수
스윕**(release, 1M 문장, K = 문장당 `acc` 피연산자):

| K (ops/stmt) | 시간(s) | eval 비중 |
|---|---|---|
| 1 | 0.45 | ~13% |
| 4 | 0.62 | ~37% |
| 8 | 0.85 | ~55% |
| 16 | 1.32 | ~70% |
| 32 | 2.25 | ~82% |

선형 적합 `t ≈ 0.39 s(고정) + 0.058 s × K` (R²≈1). **고정항 0.39 s** = net write + 루프 제어 + 문장
디스패치(= VM이 제거하는 부분, 그래서 VM은 eval-light에서만 이득). **피연산자당 58 ns** = read_net +
binop. **net-read ≈ literal**(0.45≈0.44 … 2.24≈2.22)이므로 58 ns는 read_net이 아니라 **binop의 Value
생성 + `eval_ctx` 디스패치**가 지배 — 환원불가 u64 ALU는 ~1 ns. ⇒ **58 ns 중 ~57 ns가 인터프리테이션
오버헤드**로 레지스터 기반 native-eval이 제거 가능(4-state 마스킹 감안 보수적 **4-6x on eval**).

**정정된 판정 — native-eval ROI는 워크로드 의존:**
- **식-바운드 RTL**(넓은 ALU·CRC/crypto 데이터패스·깊은 조합 cone): eval 55-82% → native-eval
  **고ROI**(설계당 ~2-3x 전망). `EXPR_HEAVY`(K=16) **VM 0.92x** — 문장 컴파일은 식-바운드에 거의
  무력, native-eval이 유일한 레버.
- **클럭/스케줄러-바운드 RTL**(`CODEGEN_HEAVY`): eval 작음 → native-eval **저ROI**, 스케줄러 축이 답.

즉 당초 doc-18의 "코드젠이 진짜 가속 경로"는 **식-바운드 한정으로는 옳았다**(1차 "저ROI" 정정은 너무
강했다). 비용 = 4-state 레지스터 머신(val+unk 평면·width/sign 마스킹·X/Z 전파·>128bit heap fallback)으로
고위험·다세션 — 단 **P5 차분 게이트(compiled==interp byte동일) + iverilog 오라클이 정확성 리스크를 이미
대폭 상쇄**한다(native-eval을 안전하게 시도 가능). 영구 회귀: `perf_baseline.rs` `EXPR_HEAVY`.

**✅ C4-lite 구현 (2026-06-08) — 예측 실현.** 위 전략에 따라 native-eval 첫 증분 랜딩(`native_eval.rs`,
VM 전용). **≤64bit 정수 서브셋**(Const·scalar Signal·Add/Sub/Mul·BitAnd/Or/Xor/Xnor·BitNot/Plus/Minus)을
codegen-able 바디 assign RHS에서 post-order 레지스터 프로그램으로 컴파일(노드당 `Value` 미생성), 그 외는
`try_compile`이 `None`→`eval_ctx` fallback. 측정(release, best-of-5):

| 벤치 | C2 VM/interp | **C4-lite VM/interp** |
|---|---|---|
| `EXPR_HEAVY` (식-바운드, K=16) | 0.92x | **0.42x (≈2.3x)** |
| `EVAL_HEAVY` (혼합, ~3 ops/stmt) | ~0.84x | **0.77x** |
| `CODEGEN_HEAVY` (스케줄러-바운드) | ~0.97x | 0.94x (불변) |

식-바운드 ~2-3x 예측이 실측으로 실현, 클럭-바운드는 eval 비병목이라 불변 — 워크로드-의존 ROI 확정. 정확성:
인터프리터=오라클(leaf는 `read_net`+`resize_keep_sign` 재사용), 산술은 X/Z poison+u64 wrapping(w≤64 sign-무관),
bitwise는 `value::*_w` 동일 primitive; native_eval 오라클 대조 단위 8 + backend_equiv native teeth 5 + 72-design
P5 차분 + iverilog 차분(460 green). frozen sim-ir 0줄 변경. follow-on(비교/시프트/Div·Mod/리덕션/ternary/concat/
>64bit/real)은 `../ROADMAP.md` §C.

### native-eval 구조 증분 (2026-06-10) — select/concat/replicate, 구조-바운드 ≈2.8x

C4-lite follow-on 2탄: **구조 트리오**가 native 서브셋에 합류 — bit/part `Select`(동적 offset,
X/Z-offset·OOR→X), `Concat`(`(hi<<lo_w)|lo` 좌측 fold = 오라클의 top-down fill), `Replicate`
(const count). 전부 자연폭 unsigned (`resize_keep_sign(w,false)` = 플레인 zero-extend — 레지스터
상위비트가 이미 0이라 **공짜**). ⭐함정: 구조 op의 오라클 root 스탬프는 signed ctx에서도
`signed=false`(`resize_keep_sign(w, false)`) — root_signed를 root 노드 종류로 분기(실사용 ctx에선
미발화나, 임의 ctx 호출자 대비 오라클 정합). 신규 `STRUCT_HEAVY` 벤치(문장당 select 4 + concat 3 +
replicate 1): **VM 0.36x interp(≈2.8x)** — 증분 전엔 select/concat 노드 하나가 전체 식을 오라클로
bail시켜 ~0.9x였던 영역. 검증: 오라클 대조 단위 6(동적 offset X·OOR·합성 select-of-concat 포함) +
teeth `native_select_concat_repl`(iverilog 라이브 일치 "5c 5c 5c 0 5c5c 5c5c xx") + P5 차분.
잔여 lane: >64bit·real·array-indexed Signal·sysfunc.

### 스케줄러축 라운드 1 (2026-06-10) — 클럭-바운드 ≈1.85x

`CODEGEN_HEAVY`(클럭-바운드: 20k 사이클 × 5 NBA, native-eval이 못 움직이던 케이스)를
`/usr/bin/sample`로 프로파일: **top-of-stack의 ~45%가 malloc/free** — 스케줄러-바운드의 실체는
"스케줄링 알고리즘"이 아니라 **타임스텝당 고정 heap churn**이었다. 제거한 할당원(전부 interp·VM
공유 경로, 순서/내용 불변이라 byte-identical by construction):

| 할당원 | 빈도 | 수리 |
|---|---|---|
| `snapshot_prev`의 derived-Clone(`prev = cur.clone()`) | 매 타임스텝 × 전체 넷 × Vec 2개 | per-field `Vec::clone_from`(capacity 재사용) |
| `propagate_changes` (c) prev 갱신 `cur.clone()` | 매 델타 × 변경 넷 | 동일 — split-borrow `clone_from` |
| `propagate_changes` `changed_nets`/`edges` Vec | 매 델타 | take/restore 스크래치 필드 |
| run-loop `cur.active` take→drop / `apply_nba` take→drop | 매 델타 / 매 NBA flush | drain + capacity 반납 |
| wheel 버킷 `BTreeMap::remove`가 Vec drop | 시뮬 시각마다 | `bucket_pool` 재활용 |
| `run_process`의 `stmts.clone()`/`term.clone()`/`Stmt::clone()` | **블록 활성화·문장 실행마다** | `&'ir SimIr` reborrow로 제자리 참조(클론 0) |
| `StmtEffect` 소유 `Lvalue`/`args` 클론 | 문장 실행마다 | `StmtEffect<'s>` 차용(NBA만 1클론 잔존) |
| `resolve_lvalue_offsets`의 `Vec<(u32,u32)>` | 대입 실행마다 | `Offsets` 인라인 enum(≤2청크 무할당, 초과만 spill) |
| `cur_scope` String 클론 | 블록 활성화마다 | 비교 후 `clone_from` |

측정(release, best-of-5): `CODEGEN_HEAVY` interp **61.8→33.4 ms (≈1.85x)** / VM **56.0→30.3 ms (≈1.85x)**;
부수 효과 `EVAL_HEAVY` interp 497→390 ms(≈1.27x — 루프 내 blocking 대입의 효과/offsets 클론 제거),
`EXPR_HEAVY` VM 0.42x 유지. 최종 프로파일에서 **malloc/free가 top-of-stack 목록에서 소멸** — 잔여는
eval축(interp `mask_top`/`resize`/`read_net` 정규화, VM native-eval이 우회)과 per-delta 전체-넷 스캔
(`cur != prev` memcmp — 넷 수 선형이라 대형 디자인용 다음 구조 단계 = dirty-list, 쓰기 경로 훅 +
정렬로 byte-identity 유지 필요). 571 green, 골든 byte 불변.

### 스케줄러축 라운드 2 (2026-06-10) — dirty-list + snapshot 제거, 다(多)넷 ≈19.7x

신규 `NETS_HEAVY` 벤치(512 idle reg + 2-net clk/카운터 churn, 20k 사이클): **305 ms** — 같은 churn의
8-net 디자인(33 ms) 대비 9배, 즉 **idle 넷이 매 델타·매 타임스텝 과세**당하고 있었다. 두 단계 수리:

1. **dirty-list 스윕** — `propagate_changes`의 per-delta 전체-넷 `cur != prev` 스캔을
   write_chunk 깔때기의 `note_change` 마킹(+ flag dedup)으로 교체. 스윕은 sort 후 `cur != prev`
   필터(A→B→A 왕복 제거) — 정렬이 구 스캔의 ascending 순서를 복원해 **byte-identical**.
   건전성: `prev`의 유일한 writer가 step (c)와 생성자(둘 다 prev=cur)이므로 변경 넷은 반드시
   마킹돼 있다. → 305→**107 ms**.
2. **`snapshot_prev` 삭제** — 시간 전진마다 전체 넷 cur→prev 복사는 위 불변식에 의해
   **증명 가능한 no-op**(안정점에서 prev==cur). 그런데 O(nets)/timestep 비용. 삭제 →
   107→**15.5 ms**. 합계 **≈19.7x**; 8-net 벤치들은 불변(noise 내), 617 green(byte-compare
   스위트 전부 통과).

⭐교훈: "스케줄러-바운드"의 두 번째 절반은 **idle-넷 세금**이었다 — 라운드1(allocator), 라운드2
(O(nets) 패스 2개). ~~대형 디자인 스케일링의 다음 후보는 net_to_edge/waiter 쪽 자료구조.~~
**→ 측정으로 종결(2026-06-10, `perf_nets_scaling` 프로브):** idle-net 수를 512→2048→8192로
스윕해도 wall-clock **평탄**(~15-17ms, 노이즈권) — R2의 dirty-list+snapshot 제거가 idle-net
세금을 전부 걷어냈고, net_to_edge/waiter 워크는 변경 넷×waiter 수에 비례(부하 비례 = 세금
아님)함이 확인됐다. 다넷 자료구조 추가 작업은 **무근거 — 항목 폐기**, 프로브는 영구 회귀
계기로 잔류(스케일링 퇴행 시 즉시 드러남).

### native-eval C6 lane (2026-06-10) — array-indexed + 65..=128-bit wide, 식-바운드 적용폭 확대

C4-lite의 마지막 큰 lane 2개가 합류 (`native_eval.rs` dual-stack 설계):

1. **array-indexed Signal** (`mem[i]` RHS 읽기) — `LoadIndexed`/`WLoadIndexed` op. 인덱스는
   self-determined 컴파일(>64-bit 인덱스는 bail), 런타임 변환은 오라클과 동일한
   `to_u64→u32::try_from→u32::MAX OOR sentinel` 체인(X/Z 인덱스·OOR → all-X 읽기,
   wrap 없음). `read_net(net, Some(idx)).resize_keep_sign` verbatim.
2. **wide lane (65..=128bit)** — **별도 u128-pair 스택**(`WIDE_STACK=8`)에 2-word 레지스터.
   narrow 스택/op은 한 줄도 안 바뀌어 기존 lane 성능 보존(단일 스택 32B-slot화는 narrow에
   세금). 지원: `WConst`/`WLoad*`/`WArith`(unsigned u128 lane = 오라클 정확 재현)/`WBitwise`
   (`*_w` 공식이 비트-병렬이라 u128 그대로)/`WNot`/`WNeg`/`WCmp`(i128 부호확장 비교)/
   `WEqNe`/`WCaseEqNe`/`WShl`/`WShr`(128 가드+MSB pair fill)/`WDivMod`(unsigned)/
   `WTernary`(cond wide/narrow 양쪽)/`WReduce`/`WLogNot`. narrow 1-bit 생산자가 wide ctx에
   먹힐 땐 `Promote`(zero-extend 브리지). **bail(오라클 잔류): signed >64 arith/divmod**(오라클이
   X-poison하는 영역 — 보수적 제외), wide shift-amount,
   wide LogBin 피연산자, >128bit, real, sysfunc. *(wide 구조 트리오는 v6 ④에서 랜딩 — 아래.)*

### native-eval v6 ④ (2026-06-11) — wide 구조 트리오 + real lane 측정-폐기

**wide 구조 트리오 랜딩**: `WSelect`/`WConcatPair`/`WRepl`이 select/concat/replicate를
65..=128bit까지 — 혼합-스택 op(narrow offset/part가 wide base/acc와 합성, 결과는 자연폭>64
iff wide 스택), 오라클 계약 동일(X/Z offset=sel_w X·OOB bit=X·in-range는 2-word shift 1회).
신규 벤치 `WIDE_STRUCT_HEAVY`(100-bit select/concat/replicate 핫루프): **VM 0.44x interp
(≈2.3x)** — 트리오 전엔 wide 구조 노드가 전체 식을 bail시켜 ~1x였던 형. >128 bail 핀 유지.

**real lane 측정-폐기**: 신규 `REAL_HEAVY` 프로브(f64 산술 핫루프) — **VM 0.90x**. 식 평가는
오라클-bound지만 real 산술은 합성가능 RTL 핫패스에 사실상 부재(테스트벤치 영역)라 제3의 f64
레지스터 파일이 지불하지 않음. 프로브는 영구 계기로 잔류(워크로드가 바뀌면 재평가).

**has_xz/to_u128 검사-폐기**: inline-Value 라운드에서 이미 word-parallel(마스크된 word 로드,
bit 루프 0) — §A 미세 항목 종결.

신규 벤치(release, best-of-5): **`WIDE_HEAVY`(100-bit EXPR_HEAVY 형) VM 0.59x interp(≈1.7x)** ·
**`MEM_HEAVY`(문장마다 동적 `mem[p]`/`mem[q]` 읽기) VM 0.72x(≈1.4x)**; 기존 lane 불변
(`EXPR_HEAVY` 0.45x·`STRUCT_HEAVY` 0.36x·클럭-바운드 0.93x). ⭐함정: wide 스택을 32-slot으로
잡으면 **run()마다 1KB zero-init**이 narrow 프로그램에도 과세(no-unsafe 정책상 MaybeUninit
불가) — 8-slot(256B)로 줄이니 전 lane 회복(wide 식 깊이는 post-order 좌fold라 2–3 peak,
초과는 컴파일 시 오라클 bail). 검증: 오라클 대조 단위 12(2-word carry/wrap·X-word1-poison·
부호 경계 bit99·OOR/X 인덱스) + bail teeth 7 + backend_equiv teeth 3(100-bit 10-op 정확값
witness·고워드 X-poison·indexed 3종) + iverilog 차분 2 디자인(`diff_wide_lane_c6`/
`diff_indexed_expr_reads`). frozen sim-ir 0줄, 골든 불변, 635 green.

### 교훈 (방법론)

1. **병목은 양파.** 표면층(bit-serial) 제거 → 재측정 → 그 밑(alloc) → 또 그 밑(정규화/transient-alloc). 한 번 측정으로 끝나지 않음.
2. **"실패한" 실험도 선행 최적화 후 재시도 가치.** inline-Value: 1차 ~0(net-write per-bit 루프가 alloc 가리고 Deref 오버헤드 상쇄) → 그 루프 word化 후 3차 1.55x.
3. **공유 경로 > backend-전용.** interp·VM 둘 다 빨라지고 위험 낮음.
4. **`std::simd` 여전히 미도입**(§(1)과 동일 이유: nightly/MSRV-1.82/3-OS 충돌). u64 워드 루프를 LLVM이 자동벡터화.
5. **"스케줄러-바운드"의 절반은 allocator-바운드.** (2026-06-10) 알고리즘 교체 없이 타임스텝당 고정
   할당만 제거해도 클럭-바운드 1.85x — derived `Clone::clone_from`가 재할당이라는 함정(`Vec::clone_from`은
   재사용)과 `mem::take` 후 소비가 capacity를 버린다는 함정이 반복 패턴.

## PDES 타당성 연구 (2026-06-11 · P4-T4 종결)

> **결론 먼저: 조건부 NO-GO.** byte-identical 결정성은 차단 요인이 **아니다** — 보존 설계가 존재한다(아래 스케치).
> 실제 차단 요인 셋: ①워크로드 폭(현 corpus는 활성 배치 W=1~8 — 이득 0~손해) ②엔진 직렬 잔류 몫 ~20%
> (Amdahl 상한 T=4 ≈2.5x·T=8 ≈3.3x·∞ 5x) ③상당한 엔진 스레딩 공사 대비 측정 상한 2~3x. 재진입 조건은 말미에 핀.
> 종전 "결정성 invariant와 정면충돌이라 불가" 프레임(본 문서 구판·감사)은 **과했다** — 이 연구로 정정한다.

### 측정 — 프로브 3종 (`perf_baseline.rs` `perf_pdes_*`, 영구 계기 · Apple Silicon 10-core)

1. **τ (per-delta 디스패치 왕복)** — `perf_pdes_sync_cost`
   - naive `thread::scope` spawn/join: **31µs(T2) ~ 93µs(T8)/delta** → 즉사(델타당 일이 µs 단위인데 디스패치가 수십 µs).
   - 상주 풀 + spin barrier(generation scatter·countdown gather): **294ns(T2) / 471ns(T4) / 2.0µs(T8)**.
     T8 급증 = P-core 초과분이 E-core 스핀 — 실설계는 T를 P-core 수로 클램프해야 함.
2. **g (엔진 활성화 입도)** — `perf_pdes_engine_grain`: W개의 독립 4-NBA flop `always` 블록(이상적 최대 병렬 케이스)
   - 한계비용 **~700ns/activation**(W=16→1024 수렴 707→686ns; W=1은 1351ns — 고정 슬롯 오버헤드 포함).
   - `/usr/bin/sample`(release+debuginfo) 자가시간 분류: **병렬화 가능 ~78-82%**(eval_binary_ctx 282·mask_top 244·
     eval_ctx 242·resize 216·read_net 187·NBA 캡처 k_schedule_nba 54·Value 말록류 ~188 등 — 전부 per-process 작업)
     vs **직렬 잔류 ~18-22%**(apply_nba 측 write_chunk 95·write_lvalue 83·propagate_changes 105·정렬 등 — 커밋/전파 phase).
3. **BSP mock (디스패치 측 스피드업 매트릭스)** — `perf_pdes_bsp_mock`: 상주 풀+정적 chunk 소유+직렬 commit pass,
   설계 스케치와 동일한 결정적 디스패치 형상. 실측 grain 보정 후(T=4 기준):

   | grain(실측) | W=8 | W=64 | W=512 | W=4096 |
   |---|---|---|---|---|
   | ~28ns | 0.39x | 1.56x | 2.96x | 3.51x |
   | ~195ns | 1.59x | 3.09x | 3.61x | 3.69x |
   | ~890ns | 2.93x | 3.61x | 3.72x | 3.95x |

   T=8은 W×g가 클 때만 우세(최대 5.1x @ g890·W4096) — 저부하에선 E-core 스핀이 역효과(0.11x까지).

**합성 추정(이상적 wide-synchronous, g≈700ns):** 디스패치 측과 Amdahl 상한의 min → **W≥64에서 ≈2~2.5x(T4), ≈3x(T8)**.
W≤8 소형 바디 ≤1.6x~손해, **W=1(corpus의 테스트벤치형 전부)은 병렬 대상 자체가 없음.**

### byte-identical 보존 설계 스케치 — BSP-per-delta v1 ("가능하나 비싸다"의 근거)

- **적격 클래스**: suspend-free + 쓰기 전부 NBA인 프로세스(= P9 분류 재사용, "pure-eval"). NBA-pure끼리는 같은 델타 내
  상호 가시성이 원천 차단(쓰기가 NBA 리전에 착지)이라 **read/write set 충돌 분석 없이 병렬 안전**.
- **run-splitting**: blocking-writer/fork/dyn-heap 터치 프로세스가 배치를 분할 — 경계 사이 pure-eval 구간만 산개,
  분할자는 배치 순서 그대로 직렬 실행(순차 가시성 보존 by construction).
- **per-process 로그**: NBA 캡처=(batch_idx, intra_seq) 키 머지(현 전역 seq와 동일 전순서) · `$display`/`$strobe`/`$monitor`
  등록=프로세스별 버퍼→batch 순 flush(순차 출력은 프로세스별 연속이므로 byte 동일) · `$finish`류=최소 batch_idx 승자
  +이후 인덱스 로그 폐기(순차의 mid-batch 중단 재현 — pure-eval은 상태 직접 쓰기가 없어 폐기=무료).
- **dirty-list**: per-thread 수집→머지→기존 정렬(스케줄러 R2가 이미 정렬 기반이라 자연 합류). wheel 스케줄은
  pure-eval에 없음(suspend-free라 mid-body delay/`@` 부재).
- **엔진 공사 비용(왜 비싼가)**: `!Send` 해체(`Box<dyn Write>`/`LogSink`/`Cell`/Rc `vm_cache` 9곳→per-worker),
  EvalCtx/native 스택 per-worker 복제, corpus 전체 `--threads 1` vs N byte-diff 게이트(T1 인프라 재사용 가능).
  v2(NBA apply 자체를 disjoint-net 병렬화)는 직렬 몫을 ~10%로 낮춰 상한 ~10x까지 열 수 있으나
  propagate(엣지 검출·웨이커 정렬·리전 제어)는 본질적으로 순서 민감 — v1 범위 밖.

### 판정 + 재진입 조건 (핀)

- **지금 구현하지 않는다(NO-GO).** 측정 상한 2~3x는 "지속적 W≥64 + eval-지배 바디" 워크로드에서만 실현되는데
  현 corpus/Phase-1 사용자 워크로드에 부재. 기계 비용(스레딩 공사+결정성 머지+게이트 유지)이 조건부 이득을 상회.
- **재진입 조건**: 실사용 디자인이 (a) 시뮬 시간 대부분에서 활성 배치 폭 **W ≥ ~64**, (b) 활성화당 병렬-phase grain
  **≥ ~200ns**(eval-지배 바디)를 보일 때 BSP v1 착수 — 기대 2~3x(T=P-core 클램프). 그 미만은 mock이 ≤1.6x,
  Amdahl+머지 상수가 잠식. 진입 시 위 스케치 + T1식 byte-diff 게이트가 출발점.

향후 과제·전략 결정(VM 동결 vs native-eval vs 인프라)은 [`../ROADMAP.md`](../ROADMAP.md).
