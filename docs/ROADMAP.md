# ROADMAP — 잔여 과제 (vitamin)

> **이 문서 = 전방(남은 것)-전용.** 완료 항목의 상세 로그(§4.5.x)는 [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)에, **Phase A~D 실행 기록(§5.1-x · 슬라이스 59건)은 [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md)** 에, 옛 §번호(구 §0~§7) 원문은 [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md)에 있다(셋 다 §번호 보존). 이력 내러티브 = [DEVLOG.md](DEVLOG.md), 상위 스냅샷 = [REMAINING_WORK.md](REMAINING_WORK.md), 실행 큐 = `LOOPROMPT.md` NEXT(로컬 dev-meta), SPEC 정본 = `docs/preview/`.
>
> **기준선(2026-08-25)**: format_version **29** · **5,987 tests green** · **워크로드 코퍼스 8/10** · 3-OS CI green · MsgCode **68** · **MSRV 1.85** · 기본 백엔드 **native**. 완료 슬라이스의 한-줄 요약과 상세는 **전부** [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(상단 인덱스 · `#### 4.5.<N>` 검색)에 있다 — 이 문서는 전방 전용이므로 완료 서사를 두지 않는다(옛 헤더의 34-슬라이스 요약 체인은 2026-08-19 에 트림 · 전부 ARCHIVE 인덱스와 중복이었다).
>
> **운용 규칙**: 완료 항목은 **즉시 이 문서에서 제거**하고 ARCHIVE로 옮긴다 — 취소선 잔류가 이 파일을 106KB까지 불린 원인이다(잔여가 남은 항목만 "RESOLVED(§x·상세=ARCHIVE) — 잔여 …" 한 줄로 유지).  슬라이스 완료 시 → 상세 로그를 ARCHIVE "완료 슬라이스 로그"에 append(§4.5.x 양식·최신이 위), 이 문서의 해당 잔여 항목 삭제. 신규 발굴은 아래 해당 섹션에 1줄로 추가.


## 요약 (스캔용)

| 순 | § | 주제 | 항목 | 오라클 | 키워드 |
|---:|---|---|---:|:--:|---|
| **1** | §0 | correct-support 승격 큐 | 6 | ✓ 4/6 | **T1 잔여까지 전부 완료(§4.5.222~227)** · **T2-14 `-G` override 도 완료**(§4.5.313 one-shot + §4.5.314 staged — 잔여 = `-pvalue+` 별칭 · `-P<path>=` · 합성-해시 헤더 필드[format bump]) · **T2-8 real 정수문맥도 완료(§4.5.381)** — 잔여는 전부 오라클 분열이거나 별 축 · 남은 것 = T2 잔여(리터럴 절단·PART select) + T3-13 `case inside` |
| **2** | §2 | Silent-wrong 잔여 | 38 | ✓ 8 | ~~자기결정 위치 셋 · bound/count lane~~(§4.5.343/344 해소) · **런타임 mixed-real 넓힘 · genif cond** · package real · 구조적 지연 |
| **3** | §6 | G2 OBS 트랙 | 6단계 | 내부 3-way | OBS-2 sva.jsonl → OBS-1 잔여 → R-L4 → OBS-4/5/6 |
| **4** | §3 | Loud→supported 후보 | 30 | ✓ 대부분 | ✅ **3판 클리어 라운드 완료**(1~10+P1 · ARCHIVE §4.5.342) · string/heap · 함수/formal · 소형 큐 · VCD fidelity |
| **5** | §4 | SVA / 검증 honest-loud | 6 | 일부 無 | empty-match 융합 · N2c · prop-ref skew · N4 clocking · class down-cast |
| **★** | §5 | **✅✅ Phase A~D 전부 완료 — 다음은 [§5.2 재개 지점](#★★★★-52-재개-지점--세션이-끊겼다면-여기부터-2026-08-17)** | — | 실측 | ⭐⭐⭐ **A** 커버리지 **100.00%**(거부 0) · **B** 제품 표면이 **native 하나**(`oracle` feature) · **C** interp = 테스트 도구(성능 최적화 영구 제외) · **D** 성능: 벤치 **10/10 에서 native < vm**(착수 때 셋에서 졌고 최악 **2.52×**). ⭐⭐ **코드젠(cranelift)은 지어서·배선해서·재서 기각** — 런의 **~38% 가 shim** 이고 천장은 op 디스패치 **8.9~11.3%**(§5.1-be). ✅ **스케줄러 축은 측정됐다**(2026-08-18: 스케줄러 29.0% + 그 축의 할당 5.8% ≈ 35% 미최적화 · §5.2). **실행 기록 전문 = [ARCHIVE_PHASE_A-D](ROADMAP_ARCHIVE_PHASE_A-D.md)** |
| — | §7 | 조건부 / 장기 | 4 | — | BACKEND · VHDL · VCD-EXT · MVP-CUT (정확성과 직교) |
| — | §8 | 비계획 | 1 | — | 영구 비목표(DEFPARAM·IMPLICIT-NET·OOS) |

> 🔴 = 열린 silent-wrong(정본 최우선). 취소선/RESOLVED 항목은 **잔여가 있을 때만** 한 줄로 남고 상세는 ARCHIVE에만 둔다.
>
> **순서 주의**: 정본 우선순위(§1)는 `① 오라클 있는 CRITICAL silent-wrong > ② loud→supported`인데, 위 표의 1·2위는 **오너 지시로 §0(②)이 §2(①) 앞**에 있다. §0를 먼저 해도 §2의 ①-급이 사라진 것은 아니다.

> **§4.5.275 후속(잔여 ①만 · ② 는 §4.5.277/278 로 해소 — 상세=ARCHIVE)**: output-formal 호출의 eval-order 수리가 **클래스/2-세그먼트 메서드 본문**을 해소하지 못해 `y = c.m(x) + f(x)` 는 정직한 loud — `call_effect` 가 그 본문에 `Inert` 를 증명할 수 있으면 correct-support(오라클 有: iverilog task 쌍둥이 `y=16 x=6`). 잔여-of-278 = 프레임 **함수** 본문 안의 output-formal 호출(함수는 `Expr::Call` 로 진입해 자기 소유의 call terminator 가 없다 — 진단이 그 이유를 말한다).

> **§4.5.276 후속(잔여 ①만 · ② repeat self-width 절단은 §4.5.342 클리어 6번으로 해소)**: **루프 trip-count 증명이 리터럴만 받는다** — 식별자를 허용하려면 fold 가 net-aware 여야 하는데 `walk_scopes_key_shadowed` 의 계약이 `const_eval_in_scope` 도달 consumer 의 opt-in 을 금지한다(순서 의존 → 과거에 generate body 를 조용히 삭제). 그래서 `for (int j=0; j<NN; j++)`(localparam 경계)는 정직한 loud. 승격 전제 = 순서 무관 이름 집합 또는 §4.5.342-P1 식 span 논증.

## 0. correct-support 승격 큐 (2026-07-25 전수 재그라운딩 — **오너 지시로 최상위**)

> **목적**: 지금까지 correct-or-loud로 **LOUD 유지**한 항목 중 *실제로 구현 가능한 것*을 골라 correct-support로 올린다. 아래는 §3/§4 전체를 훑고 **12개 후보를 iverilog로 직접 재현**해 (a)아직 loud인지 (b)오라클이 있는지 확인한 결과다. 재확인에서 **4건이 stale**로 드러나 아래 "정정" 항목으로 이동했다.
>
> **순위 근거**: 전부 iverilog 오라클 有 = §1 우선순위 ②(additive·저위험). ~~T1은 한 가족이라 머신러리가 공유된다~~ — **§4.5.222 실측이 이 전제를 기각했다**(아래 T1 머리말). T2는 서로 독립적이라 개별 슬라이스.

**~~T1 — string-array~~ 전부 RESOLVED + 발굴 잔여 4건도 RESOLVED**(§4.5.219/220/222~228 · 근인은 "한 가족" 전제와 달리 4갈래[geometry·bound·hierarchical·per-activation/SoA] — 큐를 묶을 때 근인을 측정하지 않으면 이렇게 된다 · 상세=ARCHIVE).

**의도적 loud(갭 아님)**: fixed 배열에 `new[]`(iverilog도 거부) · multi-dim partial 인덱스 `s[0]`(iverilog도 거부·조용한 오원소 방지) · cross-type SoA whole-element 복사(멤버 대응 보장 없음).

**오라클 주의 — iverilog 결함 7건(vita가 IEEE 정답)**: ① string **배열 원소**의 `.len()`이 문자열 길이가 아니라 **배열 크기**를 낸다(`string s[5]; s[0]="abcdefg"` → iverilog 5, vita 7; 같은 텍스트를 스칼라에 넣으면 iverilog도 7). ② 동시 fork 활성화가 automatic string 배열을 공유한다(`A!` 대신 `A!!`). ③ 같은 fd 에 `$fmonitor` 를 두 번 걸면 **누적**해 둘 다 찍는다 — 자기 자신의 싱글턴 `$monitor` 와 모순(vita 는 destination 별 replace). ④ 빈 string **배열 원소**를 `%s` 로 찍으면 공백 1칸(스칼라는 빈 문자열). ⑤ **`$clog2` 의 signed-wrap 인자를 32비트 정수로 승격 후 unsigned 로 읽는다**(`$clog2(4'sd7+4'sd1)` → 32) — §20.8.1 "treated as an unsigned value" 는 인자 **자기 폭의 비트 패턴** 읽기라 정답은 3(verilator 5.050·vita 런타임·상수 도메인 모두 3 · §4.5.343).  ⑥ **`$itor` 가 인자를 32비트 컨테이너로 절단한다**(`$itor(64'h1_0000_0008)` → 8 · unsigned 와 signed `longint` 가 **같은 8** 이라 부호 축이 아님을 스스로 증명 · vita·verilator = 4294967304 · §4.5.361). ⑦ **string 관계연산자가 항상 1 을 낸다**(`s="ab"` 일 때 `s<"ab"`·`s<"aa"`·`s<"zz"` **전부 1** — 셋이 동시에 참일 수 없다 ⇒ 이 축에서 iverilog 는 오라클이 아니다 · vita·verilator = `0 0 1` · §4.5.361). 전부 회귀 테스트로 핀 고정.

**T2 — 독립 항목 (오라클 ✓·각자 전용 슬라이스)**

8. ~~`real` const-fold 전면 미지원~~ **RESOLVED**(§4.5.232 · generate 스코프/제어식 §4.5.241/242 · **실수→정수 문맥 §4.5.381** · 상세=ARCHIVE). §4.5.381 이 폭/range bound·replication count·정수-선언 localparam 을 **명시 변환 경계**(`int'()`/`$clog2`/`$rtoi`/선언 타입)에서 열었다 — §4.5.232 가 철회한 i64-twin 과 달리 **leaf 가 아니라 소비자**에서 변환한다(그 철회 주석이 *"routing those sites through the real domain, which is its own slice"* 라고 이 슬라이스를 지목해 뒀다). **잔여(전부 loud)**: ⓐ **암시 변환** (`logic [R-1:0]`·`{R{1'b1}}`) = **오라클 분열**(폭은 verilator 거부/iverilog 3, count 는 iverilog 거부/verilator 3) ⇒ **비목표** ⓑ 무타입 localparam 의 real 값(§6.20.2 상 그건 **real 파라미터**다 — 반올림하면 §4.5.232 가 철회한 그 silent-wrong) ⓒ **실수 override**(`#(.R(2.5))` · override 채널이 i64) ⓓ `1.0/0.0` (real 도메인이 비유한을 의도적으로 거절) ⓔ 실수 미정의 연산자(`R<<1` — iverilog 거부/verilator 6 = **분열**) ⓕ `localparam time T = R*2.0`(`param_decl_width_opt` 가 `time` 을 무타입 가지로 흘린다 · 그 함수는 **8자리 공유**) ⓖ 상수함수 본문의 `$rtoi`(모듈 스코프 resolver 라 **shadow 위험** ⇒ env 를 아는 walk 로 옮겨야 한다) ⓗ `int'(real'(R))` 중첩. generate case 의 real scrutinee 는 iverilog 도 거부라 **비목표**(§4.5.243 핀).
9. ~~generate/interface 스코프 string decl-init~~ **RESOLVED**(§4.5.228) — 근인은 `allow_string_init` 플래그가 아니라 decl-time 쓰기가 모듈 스코프 pending 리스트로 새던 것. queue/dyn decl-init·generate 내 block-local 도 같이 열림.
10. ~~sized-literal enum label → enum-method~~ **RESOLVED**(§4.5.234) · ~~상수 이름 라벨~~ **RESOLVED**(§4.5.379). ⚠️ **§4.5.379 의 census 가 이 줄의 남은 절반을 반박했다** — *"sized-literal"* 은 이미 돈다(`enum bit[7:0] { A = 8'hFF }` 와 `.name`/`.first`/`.next`/`.num` 전부 정확). 실제로 막혀 있던 건 **이름을 쓰는 라벨**(`A = L`·`L+1`·`L*2`)이고, 그러면 enum 이 통째로 `enum_defs` 에 안 들어가 **모든 메서드**가 오도적인 *"hierarchical function call"* 로 loud 였다 ⇒ 파서의 `const_locals` 를 라벨 폴드가 묻게 했다. **잔여 둘, 둘 다 loud**: ⓐ **`parameter` 라벨은 접으면 안 된다** — 실측상 인스턴스 override 가 **라벨 값을 바꾼다**(`m #(.K(9))` 에서 iverilog 가 10/`first=9`) 이고 파서는 override 전에 돈다 ⇒ 근본 해소는 **enum-method desugar 를 elaborate 로 옮기는 것**(아키텍처) ⓑ **sized-literal `localparam`**(`localparam L = 8'h5`)은 `const_locals` 가 **decimal 만** 기록해 안 접힌다(그 표는 generate 인덱스와 공유라 생산자를 넓히면 **다른 소비자**가 움직인다 — 별도 항목). 파서 폴드가 **절단이 필요한 리터럴**을 거부하는 것(unsized `'h1FFFFFFFF`·mis-sized `4'hFF`)은 그대로이고 **두 오라클도 거부**한다.
11. ~~음수 range bound~~ **RESOLVED**(§4.5.228) — plain net·multi-packed inner(**후자는 silent 였다**)·배열 원소·VCD `$var` 범위까지. 잔여 = **PART select**(`x[1:-2]`, 정직한 loud·바운드 접기가 unsigned) · **포트/formal**(warn+clamp 유지·의도적 opt-in 비대칭).

14. ~~elaborate 단계 파라미터 override — `-G<name>=<val>`~~ **RESOLVED**(§4.5.313 one-shot · **§4.5.314 staged**
    — 상세=ARCHIVE). `-G`/`--param` 이 `vita` 와 `velab`(`-L` compose 포함)에 적용되고, `vcmp`/`vrun` 은
    wrong-stage loud 거부이며, `.f` typed bucket 과 `-v` effective echo 의 `params` 행도 있다.
    **잔여 3건:**
    - **`-pvalue+<name>=<val>`** — 미구현(`grep -rn pvalue crates/` = 0건). `-G` 의 별칭 철자이므로 argv 파싱만.
    - **`-P<path>=<val>`**(계층 경로) — 미구현. defparam 이 아직 direct-child 한정이라 같은 제약을 물려받는다.
    - **`.velab` 합성 해시 입력 등록**(doc-14 §RULE B). `-G` 는 지금 **RULE-V upstream 다이제스트에 안 들어간다** —
      섞었더니 `vrun --upstream` 이 바뀌지 않은 `.vu` 를 두고 `E9003 digest changed` 를 내는 **거짓 stale** 이
      됐다(그 필드는 업스트림 입력의 다이제스트지 합성 입력의 것이 아니다). RULE B 를 지키려면 **헤더에
      자기 필드**가 필요하고 그것은 `format_version` bump 다. 지금 상태의 관측 가능한 결과 = 같은 `.vu` 에서
      서로 다른 `-G` 로 만든 두 `.velab` 이 **헤더 128바이트가 동일**하고 어떤 게이트도 구분하지 못한다(본문은
      다르므로 값은 맞다 — 위험은 provenance 뿐이고, `velab` 이 elaborate 를 건너뛰는 경로는 없다).

**T3 — 전제조건 필요 (즉시 착수 대상 아님)**

12. ~~`$fmonitor`/`$fstrobe`~~ **RESOLVED**(§4.5.228·format 25) — 동결 `Monitor`/`Strobe` id 재사용 + `file_directed_stmts` 사이드카. 모니터는 **destination 별**로 유지된다.
13. `case (x) inside {…}` — **no-oracle**(iverilog 13.0이 `case inside`/`inside` op/array reduction 전부 거부) → hand-IEEE + 내부 차분.

**정정 — 재그라운딩에서 stale로 판명(§3에서 삭제, 비목표)**

- ~~package `function string`~~ · ~~package control-flow 함수~~ — 둘 다 **이미 동작**(`hi` / `9` 확인).
- ~~generate 스코프 queue/dyn decl-init~~ — queue **이미 동작**(`4` 확인). 잔여는 string decl-init뿐(위 9번).
- ~~`always @(*)` string concat 조용히 drop~~ — **오진**. vita는 `[abcd]`로 **정확**하고 iverilog가 빈 문자열을 낸다. 명시 `@(a,b)`는 vita가 정직하게 loud. §4.5.217 리포트의 이 줄은 잘못 기록된 것.

> **주의(정직한 순위 고지)**: 프로젝트 정본 우선순위(§1)는 **① 오라클 있는 CRITICAL silent-wrong > ② loud→supported**다. (아래 §0-B가 ①-급으로 걸어 두었던 **inner NET vs outer PARAM shadow**는 **§4.5.246에서 RESOLVED** — 2026-07-30 확인.)

## 0-B. NEXT — 재개할 deep-defer follow-on

> **재개할 deep-defer 항목 없음** — round-18 8-가족(§4.5.213) · fork-in-frame(§4.5.214) · inner-NET shadow(§4.5.246/247) 전부 RESOLVED(상세=ARCHIVE). 남은 것은 아래 소형 follow-on 또는 §5.2 큐.

> 소형 follow-on(correct-or-loud loud 유지): void-cast of output-formal fn(`void'(getnext())`)·frame-formal array를 nested hier로 forward(OUTPUT/INOUT)·param/call leaf size-cast(`8'(P*a)`, §4.5.212 잔여)·**fork-in-frame 잔여(§4.5.214, 전부 Minor/safe)**: `fork_arms_self_contained`의 resolve-time 재-walk 중복 제거·공유 `enter_task_frame` arm에 load-bearing comment 보강·forking task를 호출하는 fork arm의 elaborate-time reject(현재는 F4004 tie-cap runtime guard로 안전하나 clean E3009가 더 명확)·same-instant zero-delay sibling visibility가 differential-미검증(iverilog 자체가 이 케이스서 스케줄링 특이).

## 0-C. 남은 대형 항목 3건 — 착수 판단표 (§4.5.244 실측)

> 소형·중형 잔여가 §4.5.229~243 으로 소진돼, 남은 것은 **전부 선행조건이 있는 대형 항목**이다. 각각의 **비용·payoff·선행조건**을 실측해 두었으니 다음 착수는 이 표에서 고르면 된다(크기 재추정 금지 — §4.5.233/240 의 교훈).

| 항목 | 비용 | payoff | 선행조건 / 함정 |
|---|---|---|---|
| **A. 파일위치 함수군**(`$ftell`/`$fseek`/`$rewind`/`$ferror`) | **format_version bump 확정** | 中(테스트벤치 파일 I/O) | 신규 `SysFuncId` 변종 = **frozen-root 변경** → SimIr 스키마해시·canonical·RON 골든 **전부 재핀** + 전 `.velab` 무효화. **§4.5.228 의 사이드카 우회는 불가** — `$fmonitor` 는 `$monitor` 의 destination 변종이라 기존 id 재사용이 됐지만, `$ftell`/`$rewind` 는 **의미가 겹치는 기존 id 가 없다**(실측). `$feof`/`$fgetc`/`$ungetc` 는 이미 id 가 있으니 **그것들만 쓰는 범위**라면 bump 없이 가능. |
| **B. literal 파싱 공유 크레이트**(§4.5.234 안 ①) | 中~大(559줄 이동 + 어댑터 + 전 리터럴 재검증) | **小** — 현재 파서가 거부하는 형태는 절단 리터럴(`4'hFF` in `[3:0]`)과 unsized+`s` 인데 **iverilog 도 절단 형태를 거부**한다. 즉 **능력 이득이 거의 없고** 얻는 것은 *두-술어 위험의 구조적 제거*. | `literal.rs` 가 `sim_ir::{BitPacked,ConstRepr,ConstVal}` 에 의존 → 그대로 옮기면 **hdl-parser 가 sim-ir 을 보게 된다**(레이어링 역전). 올바른 분해 = **digit→bits(중립) / ConstVal 패킹(IR)** 2단 분리. |
| ~~C. inner NET shadow~~ **RESOLVED §4.5.246** | — | — | 남은 대형 = A(파일위치군·format bump) · B(literal 크레이트·payoff 작음) |

> **권장 순서 = C > A > B.** C 만이 ①-급이고(§1 우선순위 룰), A 는 format bump 를 감수할 가치가 있을 때, B 는 능력 이득이 거의 없으므로 **다른 이유(두-술어 제거)가 우선순위를 얻을 때**만.

## 1. 착수 우선순위 — **원칙만 여기 · 현재 큐는 [§5.2 재개 지점](#★★★★-52-재개-지점--세션이-끊겼다면-여기부터-2026-08-17)**

> ⚠️⚠️ **정본이 둘이면 하나는 반드시 썩는다** — 그 일이 실제로 일어났다(2026-08-18 발견).
> 이 절의 옛 "NEXT 큐" 0번은 `③층 S1d-4a` 였는데 **2026-08-03 에 완료**됐고(§4.5.292) 그 뒤
> **Phase A~D 가 전부 끝났다**. 붙어 있던 오너 지시(*"성능 T 단계가 아래 4단계 위에 올라간다"*,
> 2026-08-03)는 **이행되어 소멸했다** — T0 완료 · **T1~T3 은 ③층에 흡수**되어 Phase A~D 로 끝났고,
> 그 뒤 **§5.2 가 성능 축을 사다리 아래로 놓았다**(2026-08-17: 수확 체감 · 코드젠 기각 ⇒ 다음은 정확성 큐).
> ⚠️ **`T4` 만 남는다** — ③층과 무관한 국소 결함(함수 지역 배열 원소 쓰기 **514 ns** vs iverilog **24 ns**)
> 이라 **기회 슬라이스**로 유지한다(근거표 = [ARCHIVE_PHASE_A-D §5.0-b](ROADMAP_ARCHIVE_PHASE_A-D.md)).
> 세션이 끊긴 뒤 이 절을 먼저 읽으면 **끝난 일을 다시 시작한다.**
>
> ⇒ **이 절은 "어떤 종류를 먼저 하나"(시간 불변 원칙)만 갖는다. "지금 무엇을 하나"(현재 큐)는
> [§5.2](#★★★★-52-재개-지점--세션이-끊겼다면-여기부터-2026-08-17) 하나뿐이다.** 새 큐를 여기 만들지 마라.

**우선순위 원칙 (시간 불변)**

1. **오라클 있는 CRITICAL silent-wrong** (§2에서 선정) — 항상 최우선. *정확성이 이 저장소의 최상위 원칙이다.*
2. **오라클 있는 loud→supported** (§3 · additive=저위험). ⚠️ *"오라클이 없다"* 는 미루는 이유가 **아니다** — 없으면 hand-IEEE 로 짓는다.
3. **전제조건 충족된 honest-loud 승격** (§0 승격 큐 · §4~§5).
4. **G2 OBS 슬라이스** (§6).

⚠️ **성능은 이 사다리 위에 올라오지 않는다**(2026-08-17 이후). Phase D 가 벤치 10/10 에서 native<vm 을
만들고 코드젠을 기각한 뒤, 남은 성능 축은 **vita 가 일부러 안 하는 것**(2-state 저장소 좁히기·levelize)과
**없는 축**(멀티코어)뿐이다 — 셋 다 정확성 계약을 거래하거나 아키텍처를 바꾸는 **오너 판정 사안**이다.
유일한 미측정이던 축(스케줄러)은 **2026-08-18 에 쟀고**(self 29.0% + 그 축의 할당 5.8%) §5.2 4번 행에 있다 — 거기서도 사다리 아래다.

**각 절이 답하는 질문** — §0 승격 큐 · §2 silent-wrong 잔여 · §3 loud→supported · §4 SVA ·
§5 perf/하드닝(**실행 기록은 [ARCHIVE_PHASE_A-D](ROADMAP_ARCHIVE_PHASE_A-D.md)**) · §6 OBS ·
§7 조건부 · §8 비목표. ⚠️ **§2·§3 은 주제별 묶음이지 착수 순서가 아니다** — 각 절 머리말의 착수표를 따른다.

## 2. Silent-wrong 잔여 (1건 제외 전부 pre-existing·baseline 동일 — deep defer 또는 기록됨)

> **⚠️ 이 절은 주제별 묶음이지 착수 순서가 아니다.** 착수 순서는 바로 아래 표를 따른다 —
> 위에서부터 읽으면 **선행조건(트리 전역 AST self-폭 패스)에 막힌 캐스트 뭉치**로 먼저 간다.
>
> **다음 착수 순서 (2026-08-21 갱신 · 옛 1~3 은 §4.5.343 · 그 다음 1·2 앞머리는 §4.5.345 · 런타임 mixed-real 은 §4.5.349 · 음수 상수 소비자는 §4.5.350 · **net 선언 초기화 fill 폭은 §4.5.353 으로 RESOLVED**)**
>
> ⭐ **§4.5.353 의 교훈이 이 표에 그대로 적용된다**: 큐 항목은 *"누군가 부딪힌 한 모양"* 이고 클래스가 아니다.
> 착수 첫 행동은 구현이 아니라 **코드에서 출발하는 census** 다(§4.5.350 은 2→3, §4.5.353 은 1→3).
>
> | # | 항목 | 왜 이 순서인가 |
> |---|---|---|
> | ~~**1**~~ | ✅ **RESOLVED §4.5.355 — 계층 타깃의 fill 폭**(2026-08-21) | 큐엔 **셋**으로 적혀 있었고 census 가 **일곱**(+ 적대 프로브가 `'x`/`'z`·task 스코프까지 **열**)임을 보였다. ⭐ 고침은 새 폭 규칙이 아니라 **같은 `ir_lvalue_width` 를 chunk 가 진짜가 된 뒤 한 번 더 묻는 것**. 상세=ARCHIVE §4.5.355 |
> | ~~**1e**~~ | ✅ **RESOLVED §4.5.360 — 선행조건이 "스코프 스냅샷"이 아니라 문자열 하나였다**(2026-08-22) | §4.5.357 이 남긴 4칸(`u.a = c ? '1 : 1'b0`)의 선행조건은 *"resolve 에서 쓸 수 있는 lowering 스코프"* 였는데, 재보니 vita 의 이름 해석은 **`cur_prefix` 에서 바깥으로 걷는 walk + elaboration 내내 살아 있는 FQ 테이블**이다 ⇒ 스냅샷 = **prefix 문자열 하나** · ⚠️ 재-lower 는 **읽기 전용 검증 뒤에만**(미해결 이름은 진단을 이미 뱉어서 되돌릴 수 없다 — 틀린 값이 거짓 loud 로 바뀌면 회귀) · 70칸 residue **4 → 0** · 상세=ARCHIVE §4.5.360 |
> | ~~**1d**~~ | ✅ **CLOSED §4.5.361 — 셋 다 no-op 이고, 그게 측정된 결론이다**(2026-08-22) | ⭐⭐ **분열은 "판정 대기"가 아니라 "어느 오라클이 그 자리에서 오라클이 아닌지"의 문제였다.** ⓐ `s <= 1'b1` — iverilog 가 **여기서 오라클이 아니다**: `s<"ab"`·`s<"aa"`·`s<"zz"` 를 **전부 1** 이라 답한다(폭·부호·변환이 하나도 없는 string↔string 비교인데 셋이 동시에 참일 수 없다) ⇒ 그 **1** 은 정보가 없다. vita=verilator, 전 칸 일치 ⇒ **고칠 것 없음** · ⓑ `'1 ** r` — ⚠️⚠️ **고쳤다가 측정이 기각했다**: 근거는 vita 자신의 불일치(`'1 ** r`=480 인데 같은 all-ones 를 fill 없이 쓴 `(4'd15+4'd1) ** r`=0)였고 그 불일치는 진짜지만, 288칸 PRE-3-way 가 iverilog 일치 **267 → 247**(옮겨간 35칸이 **전부 멀어짐**)을 냈다 ⇒ **되돌림**(differential > soundness 규칙이 작동한 자리) · ⭐ 그리고 진단이 뒤집혔다 — iverilog 는 `(4'd15+4'd1) ** r` 을 **1024** 로 읽는다 = 대입 문맥이 **밑수에 닿는다** ⇒ vita 의 불일치는 fill 경로가 문맥을 **더 줘서**가 아니라 non-fill 경로가 **덜 줘서**다 ⇒ **§2-1f 로 이관** · ⓒ `%` on real — 두 툴이 실행하되 **서로 다른 답**을 낸다(iverilog fmod 1.5 / verilator 0) = 정의된 의미가 아니라 미정의 구석의 서명. 불법 코드엔 loud 가 최상단 ⇒ **§3 갭이 아니다** · 판정 4건 전부 `oracle_split_rulings.rs` 로 핀 고정(미래 스윕이 "고쳐서" silent-wrong 을 만들지 못하게) |
> | ~~**1f**~~ | ✅ **RESOLVED §4.5.362 — real 지수는 밑수의 문맥을 회수한다**(2026-08-22) | ⚠️⚠️ **§4.5.361 이 이 행에 적어 둔 진단이 거꾸로였다**(그 슬라이스가 iverilog 한 칸만 보고 뒤집었다) — 결함은 non-fill 경로가 덜 주는 게 아니라 **fill 경로가 대입 폭을 준다**는 것 · ⭐⭐ 실격 질문은 하나였다: **대입 폭이 없는 곳에 같은 식을 보내라** — `real x = ('1+4'h0) ** r` 는 **셋 다 871.4213**, §11.4.9 가 연산자의 뜻으로 정의한 `$pow(('1+4'h0), r)` 도 **셋 다 871.4213** ⇒ iverilog 의 480 은 **목적지가 밑수로 새어 든 것**이고, 자기가 15 라고 답한 밑수의 기준이 될 수 없다 · ⭐ 규칙은 이미 `+`·`-`·`*`·`/` 가 지키고 있었다(같은 네 밑수가 `+ r` 에서 **4·18·2·18** = 형제 폭 반영) — `**` 만 한 값(480)으로 뭉갰다 · 게이트 = **연산자형 vs `$pow` 형** 192쌍 **115 → 192** · ⚠️ 순수-정수 `**` 384칸 **바이트 동일**(Table 11-21 의 나머지 절반: 정수 지수면 밑수는 여전히 문맥-결정) · ⚠️ 적대 리뷰가 **이웃 가드를 그대로 베낀 것**을 잡았다(`!expr_contains_fill(rhs)` — fill 은 real 이 못 되지만 **fill 을 품은 식은 real 이 된다**: `('1+4'h0) ** (r + '1)`) · 상세=ARCHIVE §4.5.362 |
> | ~~**🆕 A**~~ | ✅ **RESOLVED 2026-08-29 — 그리고 그게 더 큰 규칙 하나를 드러냈다** | §11.8.1 상 **한쪽이 unsigned 면 비교 전체가 unsigned** 이고 그 부호가 양쪽 피연산자로 **내려간다** ⇒ 서명된 피연산자는 zero-extend 되고 그 안의 `>>>` 는 **논리 시프트로 강등**된다. vita 는 부호를 안 내려보낸다 · **최소 재현(2 연산자)**: `reg signed [7:0] b = 8'shB3; (b >>> 4) > 8'd100` ⇒ **vita 1 / iverilog 0 / verilator 0**(exit 0 · 진단 0) · ⭐ **분리됐다**: 같은 식의 `> 8'sd100`(양쪽 signed) 은 **셋 다 0** 이고 `$signed(b >>> 4)` 는 **셋 다 −5** ⇒ 시프트도 부호도 아니라 **비교가 부호를 전파하는 방향** 하나 · 발견 = §4.5.391 의 differential 렌즈 fuzz(깊은 트리 1,130설계 중 5건이 같은 가족) · ⚠️ **wprog 와 무관**(native·vm·interp 가 전부 같은 답) ⇒ 표적은 `eval_binary_ctx` 의 비교 arm 이 `pair_signed` 를 **읽기만 하고 아래로 안 주는** 것 · ⚠️ 착수 전 census 필수 — `<`·`<=`·`>`·`>=`·`==`·`!=`·`===`·`!==` 여덟 연산자 × 부호 조합 넷 × `>>>`·`$signed`·단항 `-` 같은 부호민감 피연산자 | · ✅ **고침 = `AShr` arm 이 `eff_signed`(= self && ctx)를 쓴다**(`eval_core` + tier-2 `native_eval` **복사본 둘** · `wprog` 는 균일-부호 게이트가 이 가족을 먼저 거절해 무관) · 303칸 census **70 → 0 · 오라클 분열 0** · ⚠️⚠️ **그런데 이 수정이 만든 correct→silent-wrong 을 적대 리뷰가 BLOCKING 으로 잡았다**: 프레임 인자 copy-in 이 **formal 의 부호**를 문맥 부호로 넘긴다(§13.5.1 은 bind 를 대입으로 만들고 §11.8.3 은 **폭만** 빌려주며 §11.8.1 은 부호가 좌변에 의존하지 않는다고 한다 — 실측: iverilog 에서 formal 의 선언 부호는 **효과 0**, `fu16(8'shf7)`==`fs16(8'shf7)`==`fff7`). 옛 `AShr` 이 `ctx_signed` 를 **무시해서** 우연히 면역이었고, 고치자 `au(b>>>2)` 가 **4294967276 → 44** · ⭐⭐ **퍼널이 셋이고 리뷰 라운드마다 하나씩 나왔다** — 둘을 고친 뒤 soundness 가 12자리 census 로 *"둘이면 충분"* 이라 했는데 **differential 이 세 번째를 측정으로 찾았다**(`task_frames.rs` 의 `Terminator::Call` — **프레임 본문 안에서** 부른 **비-suspendable** 태스크. 콜리에 `$display` 하나만 있어도 이미 고친 경로로 라우팅된다) ⇒ **자리를 세는 census 는 콜리의 성질이 고르는 자리를 놓친다** · 라운드 3 재측정 **~9,200칸 · 회귀 0 · 백엔드 분열 0 · 코퍼스 바이트 동일** · 곁수확 = `inline_formal_bind.rs` 의 문서화된 갭(`ff(8'shf7)` `000000f7` → **`0000fff7`**)과 그 옆 pre-existing 셋도 닫혔다 · ⭐ 부산물 판정: signed **default 인자**에서 iverilog 가 **자기모순**(같은 default·같은 formal 인데 함수 `000000b3` / 태스크 `ffffffb3` · 자기 plain-assign 쌍둥이는 태스크 쪽과 일치) ⇒ 여기서 iverilog 는 오라클이 아니다 · vita 는 verilator 와 일치하고 **내부적으로 균일**해졌다 ⇒ `oracle_split_rulings.rs` 에 핀 |
> | **🆕 B** | ⭐ **`>>>` 의 채움 규칙이 elaborate 상수 폴더와 `case` 영역엔 아직 안 갔다**(2026-08-29 · **pre-existing** · 🆕 A 를 고치며 적대 리뷰가 census 로 찾음) | 🆕 A 는 런타임 세 실행기에서 닫혔지만 **복사본 둘이 남았다** · ⓐ **elaborate 상수 폴더** — `const_fn.rs:162` 의 `AShr => Some(a >> b)` 는 시그니처에 부호 자체가 없고 wide 쌍둥이 `const_wide.rs:308` 은 **왼쪽 피연산자 규칙**을 그대로 쓴다 ⇒ **같은 식이 한 설계 안에서 두 답**: `localparam [31:0] L1=(B>>>2)+8'd0` 이 **4294967276** 인데 런타임 쌍둥이는 **44**(두 오라클 44 · 분열 0) · ⚠️ 값 자체는 PRE 도 틀렸지만 **자기모순은 새것**이고, 리뷰가 **넷 크기까지 오염**됨을 보였다(`logic [((B>>>2)+8'd0)-1:0] bus` 가 22비트 vs 44) · ⭐ 옳은 규칙이 **한 파일 옆에 이미 있다** — `const_fn_width.rs:427` 의 `const_i64_is_unsigned_at(ctx_w, ctx_signed) && matches!(op, B::Shr|B::AShr)` · ⚠️ 착수 선행조건 = 알려진 **선언폭 provenance 벽**([[param-values-not-canonical-at-their-claimed-width]]) · ⓑ **`case` 영역** — 스크루티니와 라벨의 집합 부호는 `stmt_flow.rs:~605` 가 **계산은 한다**(`collective_signed`)지만, unsigned 일 때 이미 lower 된 스크루티니를 **바깥 `$unsigned` 로 감싼다**. `$unsigned` 의 인자는 self-determined 라 안으로 안 내려간다 ⇒ `case (b>>>2)` 에 unsigned 라벨이 있으면 vita 는 `eq236`, 두 오라클 `eq44` · ⭐ **분리 확인**: `case ((b>>>2)+8'd0)` 은 🆕 A 로 **이미 고쳐졌고**, 부호 있는 라벨만 있으면 원래 정확 ⇒ 결함은 **스크루티니↔라벨 경계 하나** · ⚠️⚠️ **2026-08-29 지어서·재서·되돌렸다**(§4.5.373/371/372 선례): 고침 자체는 찾았다 — 감싸는 대신 **부호를 나르는 문맥 lowering**(`lower_size_ctx_entry(scrutinee, w, /*ext=*/false)` · §11.8.1 size cast 가 이미 쓰는 그것 · 스크루티니 제 폭을 넘기면 **폭엔 no-op**)으로 재-lower 하면 `case`/`casez`/`casex` 6칸이 두 오라클과 일치하고 **`/`·`%` 도 같이 고쳐진다**(`case (b/c)` vita 1 → 3 = ivl · `b%c` 2 → 1) · ⚠️⚠️ **되돌린 이유 = 한 축에서 BLOCKING 넷, 전부 내 수정이 만든 것**: ⓐ 래퍼를 **폴백으로 안 남기고 교체**해 해결 못 하는 철자(signed `localparam`·계층 참조)에서 집합 규칙이 통째로 사라짐(`case (LP) -1: ; 4'hF: ;` 이 `4'hF` → `-1`) ⓑ **signed 함수호출 라벨이 unsigned 로 투표** — `expr_self_signed` 의 catch-all 은 절단 캐스트엔 보수적이고 여기선 **정반대**(주장이 답을 바꾼다) ⓒ 고치려 만든 `expr_sign_is_known` 의 `SysFunc => true` 가 `expr_self_signed` 의 **9-id 화이트리스트와 드리프트**(⚠️ **내가 그 함수 docstring 에 경고로 적어 둔 바로 그 드리프트**이고 **두 렌즈가 독립적으로** 잡았다) ⓓ 그것까지 고치니 **`$bits` 라벨이 2→1** — `$bits` 는 SV 상 signed `int` 인데 vita 는 **unsigned 상수로 접는다** · ⭐⭐ **뿌리 = 집합 부호 투표가 `expr_self_signed` 에 기대는데 그 *"unsigned"* 는 호출·비화이트리스트 시스템함수·거기서 접힌 상수에 대해 사실이 아니라 기본값**이다. 그 투표를 하중 부담으로 만드는 것이 **선행조건**이고 충족돼 있지 않다 ⇒ **선행조건 = 부호 provenance 를 "규칙으로 안다 / 기본값이다"로 구분하는 것**(그리고 `$bits`·`$clog2` 류 상수 폴드가 signed `int` 를 나르는 것) · ⚠️ 그 밖에 잰 것: 재-lower 가 진단을 **1 → 3** 으로 늘린다(이미 실패하는 설계에서만 · 일반 억제 기구는 실패 모드가 loud→silent 라 더 나쁘다) · 스크루티니의 §12.5 **공통 최대폭**은 별개 미구현이고 **부호 참여자 없이도** 나타난다(`case (b+c)` 8비트 vs 32비트 라벨) |
> | ~~**2**~~ | ✅ **RESOLVED §4.5.358 — 다만 큐에 적힌 셋이 아니었다**(2026-08-22) | ⭐ 재-census 결과 §4.5.349 가 남겼다던 **셋은 이미 전부 두 오라클과 일치**했다(이후 슬라이스가 닫았다). 대신 33칸 census 가 **새 칸 하나**를 냈고 그게 60칸 → 12칸 클래스로 자랐다: **real 타깃이 폭을 빌려줘 unsigned 피연산자가 64비트에서 평가**된다(`byte unsigned b=8; real r; r=-b;` 두 오라클 **248** / vita **1.84467e+19**) · 상세=ARCHIVE §4.5.358 |
> | ~~**2b**~~ | ✅ **CLOSED §4.5.361 — 분열이 부호가 아니라 "컨테이너 절단"이었다**(2026-08-22) | ⭐⭐ 판정을 **부호 축에서 물으면 영영 안 풀린다**: iverilog 는 `$itor(64'h1_0000_0008)` 을 **unsigned 든 signed `longint` 든 똑같이 8** 로 읽는다 ⇒ 부호 해석이 아니라 **32비트 컨테이너로 절단**이다. 채택하면 vita 가 지금 맞는 **2³¹ 이상 전 구간**이 correct-support → silent-wrong 으로 **내려간다** ⇒ 사다리 규칙상 금지 ⇒ **고칠 것 없음** · ⚠️ 곁가지로 **iverilog 결함 ⑥** 확정(오라클 주의 목록에 추가) · ⚠️ 구조적 봉인도 같이 기록: `real'(x)` 가 **같은 `SysFuncId::Itor` 로 lowering** 되고 그건 동결 sim-ir 타입이라 이 축을 건드리면 **format_version 29→30** 이다 · anti-절단 핀 = `oracle_split_rulings.rs::itor_keeps_the_value_instead_of_truncating_to_thirty_two_bits` |
> | ~~**3**~~ | ✅ **RESOLVED §4.5.359 — 음수 bound 의 남은 선언 스코프**(2026-08-22) | 큐엔 **셋**(포트·서브프로그램 지역·클래스 속성)인데 census 는 **여섯**이었다(+함수 반환 타입 · 함수 형식인자 · **static task 지역**) · ⭐ 고침은 여섯 복사가 아니라 **한 함수**(`record_declared_bounds_for` = 모듈 스코프에 인라인돼 있던 20줄) · 78칸(13 스코프 × 6 range) **전부 iverilog 일치** · 상세=ARCHIVE §4.5.359 |
> | 3b | 🆕 **클래스 속성만 남았다 — 구조가 다르다**(§4.5.359 · PRE==POST) | 클래스 필드는 **net 이 아니다**(`ClassField` → heap slot)인데 정규화 맵은 **NetId 로 키잉**된다 ⇒ 기록할 자리가 없고, 폭만 켜면 §4.5.350 리뷰가 잡은 하강(넓은 저장 + 정규화 안 된 select)이 된다 · **선행조건 = 필드 키 정규화 맵**(net 키가 아니라) · ⚠️ 오라클도 하나뿐이다 — iverilog 는 이 모양에서 `ivl_type_packed_msb >= 0` assertion 으로 죽고 verilator 만 답한다(`w=4`) |
> | 4 | 🆕 **`$itor` 가 real 인자의 IEEE-754 비트를 정수로 읽는다**(§4.5.361 곁가지 · silent-wrong) | `a = $itor(3.9);` → iverilog **4** / vita **4.61596e+18**(= 3.9 의 double 비트패턴) · ⭐ **같은 값의 `real'(3.9)` 은 vita 도 3.9 로 맞다** ⇒ 캐스트 경로는 맞고 `$itor` arm 만 인자를 integral 로 가정한다 · ⚠️ 오라클 하나(verilator 는 이 모양에서 ICE) — 다만 §20.5 가 `$itor` 를 **integral→real** 로 정의하니 real 인자는 애초에 도메인 밖이다 ⇒ **loud 도 정답 후보**(사다리상 silent-wrong 보다 위) · ⚠️ 착수 시 §2-2b 의 봉인 확인 필수: `real'(x)` 가 같은 `SysFuncId::Itor` 로 내려가므로 arm 을 나누면 **두 경로가 갈린다** |
> | 5 | 🆕 **`string'(<integral>)` 캐스트가 파서에서 막힌다**(§4.5.361 곁가지 · loud) | `s = string'(24'h610062);` → iverilog `len=2`·`s=="ab"` / vita **E2002 parse-reject**(`expected expression, found keyword 'string'`) · ⭐ 이 항목은 §2 가 아니라 **§3(loud→correct-support)** 성격이다 — 조용히 틀리지 않고 정직하게 거부한다 · ⚠️ 원래 보고는 *"NUL 스트리핑 차이"* 였는데 **그 repro 는 vita 에서 성립조차 안 한다**(파싱 실패) ⇒ NUL 축은 다른 repro 로 재측정해야 열린다 |
> | 6 | 🆕 **static function 이 모듈 net 에 쓴 값이 조용히 사라진다**(§4.5.362 곁수확 · **2-오라클** silent-wrong) | `function logic [3:0] f(); seq = seq + 7; f = 4'h3; endfunction` 에서 `a = f();` → 반환값 `a=3` 은 맞는데 **`seq` 가 0 그대로**(iverilog·verilator 둘 다 **7**) · ⭐ `automatic` 철자는 같은 본문을 **정직하게 거부한다**(E3009 = *"frame-call subset 밖"*) ⇒ 사다리상 **static 철자만 한 칸 아래**에 있다 — 같은 규칙의 두 철자가 갈린 §4.5.359 와 같은 모양 · ⚠️ 발견 경로가 기록할 만하다: 이 슬라이스의 **재배치가 관찰 가능한지** 묻는 soundness 프로브(부작용 있는 피연산자)를 짜다가 나왔다 — PRE·POST 동일이라 이 슬라이스 것이 아니다 |
> | 7 | 🔄 **REWRITTEN (§4.5.376)** — **a parent `initial` READING a child net at t0 sees X** (**2-oracle** · reachable today · ⚠️ **zero corpus demand**) | ⚠️⚠️ **The original wording of this row was measured wrong and it caused a revert.** It said `u1.s = 8'hAA` in a parent vs `initial s = 8'hEE` in the child gives vita `ee` and **"iverilog and verilator both `aa`"**. Re-measured: iverilog `aa`, **verilator `ee`** — verilator sides with VITA. Confirmed not a dropped write (with the child's competitor removed, verilator honours the parent's write). IEEE 1800 §4.7 makes `initial` order explicitly nondeterministic, so **write-vs-write across an instance boundary is an ORACLE SPLIT**, not a defect, and §3 ④ was reverted for it needlessly. **What survives as a real two-oracle silent-wrong is the READ direction**: a parent `initial` that reads a child net the child's `initial` writes gets **X** in vita and the value in both oracles — 10 cells (depth 1/2/3, two siblings, generate scope, read-into-local), no disagreement. ⭐ Narrow: a decl initializer (`init_ranks` sorts `RANK_MOD_INSTANCE` 1 before `RANK_MOD_OWN` 2), an `always @*` read, a fork-arm read, and a delayed child `initial` are **all already correct** — only a direct `initial`→`initial` read at t0 is wrong. ⚠️ **Demand is unproven**: no design in the ten-workload corpus does it, and the four testbenches that motivated §3 ④ all WRITE downward rather than read. ⇒ **Do not rank this by the ladder alone.** Fix shape if taken: `push_process` is a single funnel and `rank_path` is live there, so the rank is `rank_path + [own_slot, pid]`; but `sim_ir::Process` is FROZEN, `tie` must stay a **dense int in [0, nproc)** because `compose_child_tie` packs `(parent_tie+1) << 16`, so the sidecar is a PERMUTATION not a key; it must be threaded through **both** backends (`sched/scan_arm.rs` seeding and `native/run.rs`), and adding a `StagedExtraSidecars` field **bumps format_version to 30** (v26 did exactly this for `init_procs` — the earlier briefing's guess that 29 would hold was wrong). |
> | 8 | 🆕 **A clocking INPUT is writable through `$readmem*`** (§4.5.375 · hand-IEEE §14.3 · **no oracle** — iverilog 13 cannot parse clocking blocks) | `$readmemh("f.hex", c.cb.mem)` writes the clocking hold net at exit 0, while the direct `c.cb.s = 8'hAA` is correctly `E3009`. §14.3: a clocking input is read-only. ⚠️ First recorded as merely LATENT — a clocking input of an unpacked array gets a **scalar** hold net, so the §3 ④ exemption arm cannot reach it — but that is the wrong reason: the **reachable** half runs through the ORDINARY resolution, which has no `clocking_hold_nets` check (both hierarchical WRITE lanes do: `hier_defer/write.rs`). ⇒ the guard belongs on the shared read resolution, covering both halves. |
> | ~~9~~ | ✅ **RESOLVED §4.5.384 — a constant-context select of an ENUM LABEL** (2026-08-25 · 59칸 **FIXED 39 · REGRESSION 0 · MOVED 0**) | ⭐ 라벨의 폭도 **선언된 사실**이다(enum 의 base 타입) — 없던 건 `param_range` 항목뿐이고, `enum_base_range` 를 라벨을 바인딩하는 **세 자리**(모듈 본문·패키지·함수/태스크 본문)가 `param_meta` 옆에 기록한다 · ⭐⭐ §4.5.373 의 두 번째 선행조건(*그 폭에서 값이 canonical*)은 **다른 이유로 쓰인 게이트가 이미 세우고 있었다** — `enum_label_range.rs`(§6.19 위반 = loud E2002) · ⚠️⚠️ **비-0 LSB base 는 오라클이 갈린다**(`enum logic [39:8]` 에서 `EA[15:8]` = iverilog **171** / verilator **52**)고 ascending 은 iverilog 가 선언 자체를 거부 ⇒ **둘 다 declines**(PRE 그대로) · 상세=ARCHIVE §4.5.384 |
> | 10 | 🆕 **A >64-bit package parameter's DECLARED range reaches one of four consumer × spelling quadrants** (§4.5.383 · both adversarial lenses, independently · **2-oracle** · ⚠️ **pre-existing — PRE was wrong on all eight cells too**, so no ladder move) | `parameter [143:16] K = 128'h…;` then `pk::K[31:24]` is **ee** (both oracles ee) but the bare-imported `K[31:24]` is **cc**, and `$bits` of a net sized by either spelling is **204** where both oracles give 238. ⭐ The runtime `pk::` quadrant works because `packed.rs` asks `param_sel_range` directly and the new `pkg_const_range` answers; the other three do not, for TWO separate reasons. (a) The bare spelling: a wide import binds `wide_param_bits`, which `param_sel_range`'s `walk_scopes_key` does not look in — and widening that walk is **not** the two-line change it looks like, because it would put a range on a key whose value lives in a second map with its own ~10 binders, reintroducing exactly the staleness `bind_param_value` was built to make unrepresentable. (b) The width consumer: the value is folded by the WIDE bit domain, which by construction *"indexes positionally from 0 and carries no direction"* (`narrow_param_bits`) — so the prerequisite is that domain learning declared ranges, which is a capability, not a binding. ⇒ **the two halves are different machinery; do not take them as one item.** |
> | 11 | 🆕 **A module-scope ENUM LABEL is invisible to a body `localparam`** (§4.5.384 census · **2-oracle** · loud, not silent) | `typedef enum logic [31:0] { EA = 32'hAB34 } e_t; localparam int Q = EA;` is `E3009` *"undefined name `EA`"* where both oracles fold 52. ⭐ **It has nothing to do with selects** — the select-free spelling fails identically — and everything to do with PHASE ORDER: body parameters bind at (3b) `instance.rs` and the module's enum labels at (3c), so a `localparam` cannot see a label declared above it in the text. ⭐ The PACKAGE and wildcard-imported spellings of the same text fold, because a package's labels are folded before any module body binds — which is also the shape of the fix: labels are const declarations and belong in the same decl-order walk the body params already use. ⚠️ That walk is what makes it non-trivial: (3b) folds in DECL ORDER so `localparam C = A*B+1` works, and moving labels into it means deciding what a label whose value references a later parameter does. ⚠️ **And one spelling of it blames the wrong thing**: `localparam logic [15:0] R = {EA[3:0]{4'hA}};` says *"the replication `{n{…}}` has no constant-fold arm"*, but that arm exists — the parameter twin `{P[3:0]{4'hA}}` folds, and so does `{pk::EA[3:0]{4'hA}}`. The honest message is the one the `$clog2` spelling gives: *"undefined name `EA`"*. |
> | 12 | 🆕 **§6.19's enum-label range check is FAIL-OPEN once the base bound is not a literal** (§4.5.384 · **both lenses, independently** · 2-oracle · vita exits 0 where BOTH oracles reject) | `module top #(parameter W = 32); typedef enum logic [W-1:0] { EA = 32'hAB34 } e_t;` with `-G W=8` (or a header default of 8, or an instance override) is accepted at exit 0, and the bare name then reads the raw `0xAB34` while a select of it reads the truncated 8-bit view — **two answers to one label in one run** (`$bits` 43828 vs 52). iverilog `-Ptop.W=8` and verilator `-GW=8` both REJECT. ⭐ The gate is in the PARSER (`hdl-parser/src/typedefs.rs`) and needs a bare-literal bound; it also gives up on **every later label in the enum** after the first one it cannot fold itself, while elaborate folds with `const_eval_in_scope`, which is strictly stronger. ⚠️ Pre-existing and PRE == POST — recorded because §4.5.384's first draft rested its soundness on this gate. What actually makes that slice safe is that **its consumers narrow to the recorded width**; the doc now says so. Fix shape: the check belongs at ELABORATE, where the base range and the label value are both folded. |
> | 13 | 🆕 **A negative label in an UNSIGNED enum base keeps its sign** (§4.5.384 Lens A · **2-oracle** · silent-wrong) | `typedef enum logic [7:0] { EA = -8'sd2 } e_t;` then `$display("%0d", EA)` is **-2** in vita and **254** in iverilog and verilator (`%h` is `fe` in all three, and every select of it agrees — `EA[3:0]`=14, `EA[7:1]`=127). ⭐ The base is unsigned, zero-LSB — exactly the shape `enum_base_range` records — so the WIDTH provenance is right and only the SIGN is wrong: `param_meta` marks the label signed through its `\|\| v < 0` clause, which was added (§4.5.154) to keep a negative label usable on an illegal unsigned base and now outlives its reason. ⚠️ Pre-existing, PRE == POST, and the constant-domain consumers are still loud, so nothing acts on it yet — but it is the counter-example to *"a label's value is canonical at its declared width"*, and closing row 12 would make this reachable by a consumer that trusts the pair. |
> | 14 | 🆕 **The constant domain sign-extends across a MIXED-SIGN operand** (§4.5.384 Lens A NIT-3 · **2-oracle** · silent-wrong · NOT enum-specific) | `pk::EA ^ 64'h0` with `enum logic signed [7:0] { EA = -8'sd2 }` folds to `fffffffffffffffe` where both oracles give `00000000000000fe`: §11.8.2 makes the whole expression UNSIGNED as soon as one operand is, and the signed operand is then zero-extended, not sign-extended. ⭐ `|`, `&`, `+` and `*` share it, and the plain package-PARAMETER twin is wrong the same way ⇒ **the axis is the wide fold's sign rule, not enum labels** — they only made it visible. Pre-existing, PRE == POST. |
> | 15 | 🆕 **A CONSTANT select whose index is out of range is loud, where §11.5.1 says `x`** (round-34 R2 · **iverilog + IEEE**, verilator is not an oracle here · loud, not silent) | `localparam logic C = B[9];` over an 8-bit `B` is `E3009`; iverilog prints `x`, which is what §11.5.1 states. verilator prints `0` with a `SELRANGE` warning — a **2-state artifact**, so it does not get a vote on an x question. ⭐ The DIAGNOSTIC half was fixed in round 34 (it now names the index and its range and says what §11.5.1 makes the value); the VALUE half is blocked one level down. ⚠️ **The blocker is not the select.** The ≤64-bit parameter store is an `i64` with no unknown plane, which is why `localparam logic X = 'x;` and `localparam logic [7:0] X = 8'b1010_010x;` are BOTH loud today with no select anywhere. The >64-bit store DOES have one. ⇒ the prerequisite is an unknown plane in the narrow parameter store, which is the same prerequisite as the long-standing `'x`-valued-parameter row below — **one item, not two**. |
> | 16 | 🆕 **A parameter override that is an EXPRESSION with a context-determined top cannot be wider than 64 bits** (round-34 R5 residue · **2-oracle** · loud) | `leaf #(.K(128'h1 << 100))` on a `parameter logic [127:0] K` is `E3009`; both oracles apply it (`00000010000000000000000000000000`). ⭐ Every other wide-override spelling now lands — literal, positional, `-G`, `65'h…`, a replication, a cast, a named wide `localparam`, and a forward through an intermediate module — because `override_bits` folds the override at its own width. It declines a context-determined top ON PURPOSE: a shift folded at the operand's self width loses the bits the context would have kept (the rule `wide_top_is_self_determined` states). ⇒ the prerequisite is folding an override **at the target's declared width**, which the parent does not know — the same "the child's width is not visible in the parent" shape the `fill` channel exists to solve, so the fix is probably a fourth deferred channel rather than a wider fold. |
> | 17 | 🆕 **The oracles split on how wide an UNSIGNED narrow override is before it extends** (round-34 R5 census · **oracle split — do not chase**) | `leaf #(.K(32'd0 - 32'd1))` on `parameter logic [127:0] K`: iverilog `ffff…ffff` (sign-extends an unsigned 32-bit expression), verilator `0000…0000ffffffff` (zero-extends from 32), vita `0000000000000000ffffffff_ffffffff` (zero-extends from **64**, the i64 lane's width). ⚠️ vita matches NEITHER, and its width is the one thing all three could have agreed on — but with the two oracles split on the extension RULE there is no answer to adopt, and §6.20.2 does not settle it. Recorded so that a future slice does not "fix" it toward whichever tool it measured first. The neighbouring cells are not split: `64'hFFFF_FFFF_FFFF_FFFF + 64'd0` zero-extends in all three, and `-(64'sd1)` sign-extends in all three. |
> | 18 | 🆕 **A `defparam` override carries no signedness, so it cannot extend past the i64 lane** (round-34 R5 residue · loud-by-omission, silent for a NEGATIVE value) | `defparam u.K = 32'h7;` on a `parameter logic [127:0] K` is correct, but a NEGATIVE one still stops its sign at bit 63. ⭐ The other channels record `ResolvedOverride::signed` from the override EXPRESSION; the `defparam` collector folds to an i64 before the record exists, so the field is `None` and the extension declines rather than guessing — which keeps that channel exactly where it was. ⚠️ Fail-closed, but a residue: the fix is to compute the flag in the collector, where the expression still exists. |
> | 19 | 🆕 **A 2-D / 3-D / packed element as a continuous-assign LHS costs ~10× on BOTH backends** (round-34 R30-2 census · performance · mechanism NOT located) | Marginal cost per continuous-assign evaluation against 50.0 ns for a 1-D unpacked element: 2-D `arr[0:15][0:3]` **546.7 ns** native / 675.8 vm, 3-D 829.2 / 967.5, packed `logic [63:0][31:0]` 410.8 / 441.7. ⭐ The native/vm ratio stays 0.81–0.93 across all three, so this is SHARED PLUMBING, not a backend axis — which also means the R30-2 fix (a `Const` leaf's sign) cannot touch it. Needs its own census; the report's and §4.5.382's diagnoses were both refuted on the 1-D axis, so do not carry either forward here. |
> | 20 | ✅ **2026-08-28 해결(정확성으로 재등급)** — ⭐ census 가 이 행을 **성능 항목에서 silent-wrong 으로 올렸다**: 인라인 경로에서 `u = $random; g = u ^ u;` 가 **두 번 뽑아** vita 3533466533 / iverilog 0, exit 0. `case` scrutinee·`&&` 와 **같은 결함**(DAG 를 트리로) · 고침 = repeatable 하지 않은 RHS 로 정의한 로컬을 2회 이상 읽으면 **frame 으로 라우팅** · ⚠️ **성능 이득은 코퍼스에서 0**(인라인 함수 27개 중 25개가 body local 없음) · ⚠️ 적대 리뷰 BLOCKING 셋(intra-assignment delay 로 **correct→loud** · 치환 후 판정 누락 · `Cast` arm 부재) 전부 수정 · ⚠️ **미해결 잔여 둘**(부작용 콜리 · 범위 밖 원소 읽기)은 `body_reads_only_locals` 게이트 뒤 — 라우팅하면 `always_comb` 감도 목록에서 읽기가 사라진다 ⇒ 그 게이트를 넓히는 것이 선행조건 | 옛 내용 ↓ · **The inline function fold is exponential in a re-read local** (round-34 R4 census · performance · correctness-adjacent) | `elab_s` stays flat at 0.35 ms while `sim_s` spreads 0.16 s → 14.36 s ⇒ the arena SHARES a substituted subtree as a DAG and the evaluator re-walks it as a TREE. Inlining costs (references per statement)^(chained statements) where the frame path is linear: 6 chained statements reading the local ONCE is 0.19 s inlined / 0.24 s framed; reading it THREE times is **14.39 s / 0.35 s**, digest-identical, identical under all three backends. ⭐ So `able` is ANTI-correlated with speed on exactly the body shape a cryptographic combinational function has, and the CHANGELOG's filed improvement — *"widen the inliner to control-flow bodies"* — was the wrong direction (now corrected there). The real item is **per-activation memoisation of a shared sub-expression** (a refcount pass over the frozen arena, or a memo keyed on ExprId inside the expression compiler). |
> | 21 | 🆕 **A `**` (and any context-determined top) cannot be folded at a width WIDER than its operands** (round-34 R3 residue · **2-oracle** · loud, and the message got LESS specific) | `localparam bit [127:0] C = 3 ** 41;` is loud where both oracles print the exact `…1fa2a1cf67b5fb863`; so are `localparam longint D = 3 ** 40;` and `localparam integer L = 4'sd3 ** (64'd0 - 64'd8);`. ⭐ §11.6.1 makes `**` take its width from the CONTEXT, so the BASE must be evaluated at 128/64/32 bits — and the wide fold folds at the operand's own width and extends afterwards, which is a different answer. Same rule that keeps a size cast (`65'(64'd… + 64'd1)`) loud. ⚠️ **The diagnostic REGRESSED on these cells**: PRE said *"the `**` operation has no constant-fold arm"* (misleading, but specific) and it now falls to the unqualified *"value is not a foldable constant expression"*, because `unfoldable_reason` steps over a sub-expression the wide domain answers and the wide domain DOES answer this one — just not at the declaration's width. The honest message needs the declared width, which that helper does not have; the caller (`param_value_unfoldable`) does. |
>
> ✅ **u64 패턴 지수는 §4.5.348 로 RESOLVED · 폭-미상 wrapping 지수는 재센서스에서 소멸**
> (2026-08-20 · 상세=ARCHIVE): 후자는 **§4.5.345 가 `const_decl_wsign` 의 multi-packed 폭을 채우면서
> 이미 닫혔다** — 그 항목이 *"해소는 거부가 아니라 폭 모델 완성"* 이라고 적어 둔 그대로이고, 정작 그것을
> 이룬 슬라이스는 몰랐다(재센서스가 아니었으면 이미 고친 것을 다시 고쳤을 것이다). 전자의 뿌리는
> **지수를 나르는 i64 가 컨테이너이지 값의 타입이 아니라는 것** — `64'd0 - 64'd8` 은 부호 없는 뺄셈이라
> 18446744073709551608 인데 컨테이너의 부호 비트를 읽어 −8 로 보고 IEEE 음수-지수 표를 적용해 **조용히
> 0** 을 냈다(두 오라클·vita 런타임 전부 926288481). 지수의 부호가 값과 함께 다니게 했고, 크기가 도메인
> 밖인 경우는 **정직한 loud**. 곁: 밑수 0/±1 은 지수의 **패리티**만 쓰므로 도메인 밖 지수에서도 답한다.
> ⚠️⚠️ **모듈러 fold 는 지어서 되돌렸다** — mod 2^64 는 문맥이 64비트 이하일 때만 옳고(두 오라클로 6칸
> 확인), 모듈 스코프 fold 에는 문맥 폭이 없어서 `localparam [127:0] P = 3 ** 41` 이 **이미 잘린** 64비트
> 값을 zero-extend 한다 ⇒ loud→silent-wrong(96비트에선 silent-wrong→*다른* silent-wrong). **적대 2렌즈가
> 독립적으로 같은 BLOCKING** 을 냈다. 72칸 3-way **회귀 0 · wrong→LOUD 5**.
>
> ✅ **top-level 자기결정 위치는 §4.5.347 로 RESOLVED**(2026-08-20 · 상세=ARCHIVE): 착수 전
> **3-오라클 census 18칸**(iverilog·verilator·vita)이 스코프를 정했다 — **두 오라클이 합치하는 9칸만**
> 고치고 갈리는 축(untyped localparam 16 vs 0 · repeat 2 vs 18)은 손대지 않았다. 비교/등가/논리
> 연산자의 결과는 1비트이고 피연산자는 **서로에 대해** sizing 되므로 노드 전체가 자기결정 —
> `const_eval_in_scope` 이 그 자리를 무제한으로 접고 있었다. 곁으로 시프트 카운트(같은 부분식이 비교
> 아래에서만 맞던 자기 불일치)와 와일드카드 비교의 LHS, 그리고 `const_self_width` 의 **Replicate arm
> 부재**(replication 피연산자가 폭을 미상으로 만들어 규칙이 통째로 무력화)까지 닫았다. 193칸 3-way
> **회귀 0 · FIXED 15**. 분할은 `binop_result_is_context_determined` 라는 **와일드카드 없는 exhaustive
> match** 한 자리에 적었다(새 BinOp 는 컴파일을 깨뜨린다).
>
> ✅ **`const_eval_cast` 의 Size/Named arm 은 §4.5.346 으로 RESOLVED**(2026-08-20 · 상세=ARCHIVE): 피연산자를
> **먼저 `max(self, N)` 으로 sizing** 하고 부호를 피연산자에서 물려받는다 — §4.5.345 가 상수함수 본문 arm 에 깐
> **같은 라우팅**이고, 이제 `const_size_cast` 한 함수를 두 arm 이 공유한다. 절단이 근사가 아니게 되니 옛 arm 의
> *"부호 두 해석이 일치할 때만"* 자기제약이 통째로 필요 없어졌다(그 제약이 `8'(255)`·`4'(9)`·`8'(P)`·`64'(-1)`·
> `1'(3)` 같은 **평범한 좁힘 캐스트 20종**을 거절하고 있었다). 147칸 3-way **회귀 0 · FIXED 4 · LOUD→CORR 23**.
>
> ✅ **body-local init 조용한 0 · 자기참조 초기화 크래시 · 폭-0 타깃(옛 1번과 2번 앞머리)은 §4.5.345 로 RESOLVED**
> (2026-08-20 · 상세=ARCHIVE): 한 뿌리 = *해석기가 모르는 것을 값으로 만들었다*. 미fold 초기화는 **미바인딩**(읽을 때만
> loud · 죽은 초기화는 정답 유지) · 선언 이름은 동명 파라미터로 폴백 금지 · `body_decls` 는 본문 깊이에서(크래시→loud) ·
> multi-packed 폭은 **차원의 곱**. 곁: concat·replication·SIZE 캐스트를 carry-free wide folder 로 접어(모듈 스코프와
> **같은 헬퍼**) 초기화 loud 클래스를 correct 로 올렸다. 61칸 3-way **회귀 0 · FIXED 19 · LOUD→CORR 4 · wrong→LOUD 8**.
> ⚠️ **기록돼 있던 폭-0 메커니즘은 오진이었다** — *"`w.max(tw)` 가 `tw=0` 을 실폭처럼 쓴다"* 는 진단대로 고치면
> `bit [1:0][3:0] tt = 8'd100*8'd100` 이 16→**10000** 인 **correct→silent-wrong** 회귀가 난다(PRE-3-way 가 잡았다).
> `max(self, 0)` = 자기 폭은 옳은 degrade 였고, 결함은 *계산 가능한 곱-폭을 declined* 한 것이었다.
>
> ✅ **replication count·part-select 폭 lane(옛 이 표의 1번)은 §4.5.344 로 RESOLVED**(2026-08-19 · 상세=ARCHIVE):
> 실체는 `**` 미fold 가 아니라 한 결함의 두 철자(`const_bound_u32` 의 전면 decline + lowered-tree 얕은 fold 의
> 폭-무시)였고, 자기결정 tier + `lower_index_expr` 합의-보존 치환 한 자리로 index·bound·offset·width·write
> 퍼널이 함께 정합. 곁: §11.4.12.1 0-repl 합법성 loud 신설(verilator 동판정 · iverilog 는 중첩 위치에 관대) ·
> fill-literal 잠복 결함(32-bit all-ones) 해소 · 옛 decline-핀 6개가 자기 주석의 정답으로 강화. **잔여** =
> 폭-미상 leaf(const-array 원소) 위의 wrap bound 는 의도적 decline(verilator 오라클 44 확보 · 마커 핀) —
> 해소는 `const_self_width` 의 const-array-elem arm.
>
> ✅ **옛 1~3(한 뿌리 — $clog2 인자·캐스트 SIZE 식·real 변환 경계)은 §4.5.343 으로 RESOLVED**(2026-08-19 ·
> 상세=ARCHIVE): 셋 다 §4.5.339 의 자기결정 걷기 하나로 라우팅 — `const_clog2_selfdet`(세 arm 한 철자 ·
> §20.8.1 unsigned = 자기 폭 비트 패턴, verilator+런타임 핀) · `cast_size_bits`(lowering 의 두 번째 철자
> 제거 · 런타임 `((4'd9+4'd8))'(2)` 9→1) · `const_eval_real_in_scope` 상단 integral 게이트(측정된 클래스는
> `**` 만이 아니라 **모든 real 연산자의 정수 피연산자** + 삼항 cond §11.4.11 + `param_real_value`).
> 뮤테이션 10/10 · PRE-3-way 회귀 0 · 5,608 green. **잔여**: ⓐ wrap 하는 SIZE 의 **const-도메인** 셀은
> decline(loud E3009 · iverilog 는 1) — `const_eval_cast` 의 절단 fold 는 무제한 operand fold 위에서
> unsound(반례 실측 `4'((4'd8+4'd8)/4'd3)` SV 0 vs 절단 5) ⇒ 선행조건 = 아래 캐스트 뭉치와 같은 AST
> self-폭 패스 ⓑ real-반환 const fn 본문 폭(`function real f(); f = 4'd15+4'd1;` → 16.0 · self-det 0.0 이
> 정답 · PRE==POST 소형) ⓒ untyped localparam `L = 4'd15+4'd1` 은 **오라클 분열**(iverilog 16 = vita /
> verilator 0 · §6.20.2 해석차)이라 아래 divergence 목록에만 기록.
>
> 새 1~3 은 독립. 4~5 는 §4.5.343 이 곁으로 측정해 등재한 것(각각 전용 슬라이스).
> ⛔ **캐스트 뭉치(아래 🔴 넷)는 선행조건이 서기 전까지 착수 금지** — 전부 *"트리 전역 AST self-폭 패스가 먼저다"* 라고 자기 문구에 적혀 있다.
>
> **오라클 있는 것부터 위로.** 아래 🔴 중 A1~A7(오라클 ✓)이 §1 우선순위 ①에 해당하고, 무오라클/soundness 발굴분은 그 아래.

> **⚠️ 옛 §1 NEXT 에서 이관한 4건 (2026-08-18)** — 이 넷은 **§2 본문에 불릿이 없고 옛 §1 목록에만**
> 있었다(즉 §1 은 포인터가 아니라 **내용을 들고 있었다**). 이관 시 무손실 검증이 잡았고, 원문 그대로 옮긴다:
>
> - **폭 인식 상수 접기** — 자기결정 위치 셋(옛 「다음 착수 순서」 1~3)은 **§4.5.343 로 해소**; 남은 것은 top-level const 문맥 전반(generate-if cond·untyped localparam·range bound 등 — **위 표 1번**과 divergence 목록에 실측 기록)이고 그 해소가 곧 캐스트 뭉치의 선행조건(AST self-폭 패스)이다. ⚠️ §4.5.346 이 **캐스트 안쪽에서는 그 패스가 이미 선다**는 것을 증명했다(`const_self_width`+`const_signed_env` 로 충분 · `8'((4'd15+4'd1) > 4'd0)` 이 iverilog 0). ⭐ **인터프리터 coerce 가 가장 도달성 높은 진입점**이라고 기록돼 있었다.
> - ~~**package-scope `real`**(오라클 ✓)~~ — ✅ **RESOLVED §4.5.377**(2026-08-24). ⭐ **§3 ⑨ 와 같은 뿌리였다**: `package.rs` 의 파라미터 fold 가 `const_eval_in_scope`(정수 전용) 하나만 물어 real/string 을 아예 라우팅하지 않았다. 값은 넘어가는데 **도메인이 안 넘어가서** `pk::PR / 2` 가 정수 도메인에서 나눠 **1.0**(두 오라클 1.5, exit 0). 정수 twin 이 `pkg_consts` 에 남으므로 `parameter real R = 4;` 의 폭-문맥 사용은 그대로.
> - ~~**구조적 지연**(오라클 ✓)~~ — ✅ **RESOLVED §4.5.364**(2026-08-22 · 70칸 3-오라클 **FIXED 51 · REGRESSION 0**). 잔여는 아래 🆕 넷.
> - ~~**`real` → `input int` formal**(오라클 ✓)~~ — ✅ **RESOLVED §4.5.365**(2026-08-22 · 184칸 3-오라클 **FIXED 54 · REGRESSION 0** · 1,170칸 스윕 **1170/1170** iverilog 일치). 잔여는 아래 🆕 넷.
> - 🆕 **cont-assign 만 구동하는 wire 가 t=0 에 가짜 이벤트를 낸다**(§4.5.352 differential 렌즈 곁 발굴 ·
>   **iverilog 1오라클만 · verilator 미조회 ⇒ 착수 전 3-오라클 census 필수**). `wire b; assign #5 b = a;`
>   에서 vita 는 b 가 **z 로 시작**해 t=0 settle 이 x 를 쓰고 그 z→x 가 `changed` 라 `always @(b)` 를
>   깨운다; iverilog 는 b 를 처음부터 x 로 보고 t=0 이벤트가 **없다**. 딜레이 없는 `assign d = c ^ 1'b0;`
>   에서도 같다 ⇒ 딜레이 경로 특유가 아니라 **초기값 도메인**(z 시작 vs x 시작)의 문제.
>   PRE==POST-A==POST 로 **§4.5.352 가 만든 것이 아님이 확인**됐다(pre-existing). 여파 = 가짜 프로세스
>   기동이므로 사다리상 silent-wrong 후보.
>
> **🆕 §4.5.365 곁가지 — `int'($random*1.0)` 의 draw 횟수**(둘 다 틀렸고 값이 **바뀌었다**): `lower_real_to_int_cast` 의 ≤32비트 가지가 정확 반올림으로 바뀌며 피연산자 명명 횟수가 2→4 로 늘었고, 그 함수의 **다른 호출자**(`lower_prim_cast` = `keyword'(e)`)에는 **`expr_is_repeatable` 게이트가 없다** ⇒ `int'($random*1.0)` 이 캐스트당 2회 → 4회 draw(iverilog 는 1회). 어느 쪽도 맞은 적이 없지만 **silent↔silent 를 조용히 맞바꾸지 마라** 규칙상 기록한다. 봉쇄책은 이미 이름이 있다 — 바인드가 쓰는 그 게이트를 `lower_prim_cast` 에도 주는 것(단, 거기선 decline 이 real 을 정수 문맥에 그대로 두므로 **다른 처리**가 필요하다).
>
> **🆕🆕 §4.5.388 — a hierarchical reference into a generate block, and the green test that
>   hid it.** External report: *"vita can't do hierarchical references into generate blocks
>   (VITA-E3010)"*. ⭐ **The census refuted its scope twice before confirming a defect under
>   it.** `for`-generate references already worked in every form measured (`u.g[0].x` — net,
>   localparam, instance-inside, read and write); so did the INDEXED spelling of a conditional
>   block; so did the bare spelling from inside the SAME module (`gblk.x`). Exactly one axis
>   was broken — **the bare label one dot further out** — and the existing green test is why
>   it stayed invisible: `hier_ref.rs::named_generate_block_read` pins `gblk.x` and has been
>   green since the initial commit. 63-cell census, **FIXED 19 · REGRESSION 0 · STILL-GAP 0**.
>
>   **ROOT 1** (`hier.rs::hier_resolve`) — vita stores a singleton generate scope as
>   `label[0]`, so the bare spelling must be mapped onto it. Arm (b) did that and said, in as
>   many words, *"Map only the leading segment."* For `gblk.x` the block IS the leading
>   segment; for `u.gblk.x` the leading segment is the INSTANCE, arm (a) commits to it, and the
>   remainder was then looked up verbatim (`u.gblk.x`) against a net stored at `u.gblk[0].x`.
>   Fix = `hier_key_within`, walking the remainder segment by segment.
>
>   ⭐⭐ **ROOT 2, which the report never mentioned and no spelling could reach**
>   (`hdl-parser::generate::parse_gen_case_item`) — both arms called `parse_gen_branch().1`,
>   taking the items and **discarding `.0`, the label**, where the `if` and `for` arms bind it.
>   A named generate-case block minted **no scope at all** and its members landed in the
>   enclosing one. Measured in the VCD: the `case` spelling emits scopes `tb u` where the `if`
>   spelling of the same design emits `tb u g[0]` — so `u.g.x` AND `u.g[0].x` were both E3010,
>   the only generate kind unreachable by either spelling. Worse than unreachable on a name
>   collision: a parent with its own `x` got **E3009 "redeclared"** on a design iverilog and
>   verilator both run. Fixed by RE-WRAPPING the labelled body as a `GenItem::Block` — the
>   existing arm already knows how to scope it, so there is one spelling of `label[0]`, and
>   `GenCaseItem` needs no new field (which would have flipped the `hdl-ast` SchemaHash).
>
>   ⚠️⚠️ **MY OWN FIRST DRAFT TRADED LOUD FOR SILENT-WRONG, and the census caught it.** §27.4
>   makes a generate-FOR's blocks an ARRAY whose name is illegal unindexed at ANY trip count.
>   Storage cannot recover that — a one-trip loop and a conditional block both leave exactly
>   `g[0]` — so a fallback keyed on "does `[1]` exist" answered `u.g.x` on a two-iteration loop
>   with element 0's net at exit 0 (iverilog REJECTS it). Adding the `[1]` test fixed that cell
>   and left a subtler one: the ONE-trip loop still resolved, so `u.g.x` worked at one
>   iteration and went loud at two — the "which spelling you wrote decides whether it resolves"
>   footgun, which would start failing the day a parameter moved from 1 to 2. ⇒ `gen_loop_labels`
>   records the SYNTACTIC fact at the single site that mints a loop scope. Elaborate-side only,
>   never serialized ⇒ **format_version 29 unchanged**.
>
>   ⚠️ **Recorded, not changed**: vita accepts `u.g[0].x` on a CONDITIONAL scope, which both
>   oracles reject (§27.6 — a conditional block is not an array). That leniency is
>   **pre-existing** (PRE accepted it for `if`/bare), the value is correct, and this slice only
>   made `case` consistent with its siblings. Loud-ifying it would descend the ladder for a
>   spelling that has worked for years.
>
>   Gates: examples 4/4 byte-identical PRE vs POST · corpus **8/10, exit 0, every pinned digest
>   matched** · 12 new pinned tests (`gen_scope_hier_ref.rs`), including the for-generate guard
>   at 1/2/3 trips and three negatives.

> **🆕🆕 §4.5.387 (round-36 C) — ⭐ THE BIGGEST MEASURED LEVER IN THE SIMULATOR, and nobody
>   had measured it.** A continuous assign whose RHS contains ANY `Expr::Call` or ANY
>   `Expr::SysFunc` is re-evaluated **6.00× per input change** instead of 1.00×, measured
>   exactly (300,001 evals for 50,000 clocked iterations vs 50,001 for the same design with a
>   plain expression RHS; the same call in an `always @*` is 1.00×). Root =
>   `levelize::expr_is_pure_of_nets` (crates/sim-engine/src/levelize.rs:329), whose
>   `E::SysFunc{..} | E::Call{..} | E::ArrayItem{..} => false` arm sets `dirty_ok=false`, drops
>   the assign into `ca_always`, and makes `settle_cont_assigns` visit it on every fixpoint
>   pass of every settle, ignoring the `ca_dirty` worklist.
>
>   Measured cost of the SysFunc half alone, same design, only the RHS spelling varying:
>   `$unsigned(src) ^ …` **1.89×**, `… ^ 128'($bits(src))` — a compile-time constant —
>   **4.90×**. The doc comment justifies the blanket reject with *"`$random`/`$time` do not
>   depend on their inputs alone"*, which is true of those and false of the large pure
>   majority of the 79 `SysFuncId` variants (~22 are clearly impure).
>
>   ⭐⭐ **And vita's own inliner trips this.** `resize_inline_assign` seals its result with a
>   `$signed`/`$unsigned` (inline_fn.rs:631,655), which IS an `Expr::SysFunc` — so an inlined
>   function measured **1.98× SLOWER** (0.158 s vs 0.080 s, 253,126 vs 50,001 evals) than the
>   identical expression written by hand. A *perfect* control-flow inliner would still leave
>   the assign at ~5 evals/iter. That is an independent reason the round-35 ruling against
>   widening the inliner holds.
>
>   FIX ORDER, with measured prizes:
>   1. Refine the SysFunc arm to a per-`SysFuncId` ALLOW-list, `_`-free exhaustive so a new
>      variant cannot default to the pure side. Prize 1.89×–4.90×, plus 1.98× on every function
>      the inliner touches. ~79 variants to audit — a real bounded job, not obviously safe for
>      all of them.
>   2. Complete the dep set for `Expr::Call`: `expr_nets`' Call arm (levelize.rs:161) walks only
>      the ARGS, never the callee body, so the reject is SOUND today. Walking the body makes
>      certification possible. Prize on the reporter's exact repro: **5.95×**, byte-identical.
>   3. Only then the body cost (2.33× ceiling), where frame setup is just 7% of the per-call
>      cost — so neither half should be sold as "frame-call overhead".
>
>   ⚠️⚠️ **Certification changes the DIAGNOSTIC stream, and that must be adjudicated first.**
>   Measured on one out-of-range-read design with identical output: a pure RHS gives
>   `errors=5`, the same RHS wrapped in a no-op `$unsigned` gives `errors=9`. The error COUNT
>   today tracks an internal scheduling classification rather than the design. Fixing the arm
>   moves 9→5 (toward consistency) but WILL move golden error counts on any design with a
>   call/SysFunc-bearing cont-assign that reads out of range; run.rs:621 records picorv32
>   moving 6→9 for the opposite change.
>
>   ⚠️ Do NOT add a `pure` flag to `FuncDef`/`SimIr` — both are SchemaHash-frozen and it would
>   force format_version 29→30. Everything above is computable out-of-band (`levelize.rs` is
>   not serialized; a static-lifetime bit rides a `SimOpts`-style sidecar), so no bump is
>   needed.
>
> **🆕 §4.5.386 (round-35 R1) — the same row, now with the fan-out MEASURED and the
>   nesting half closed.** `coerce_two_state` names its operand once per TARGET BIT (it
>   builds a `Concat` of `CaseEq(Select(e, i), 1'b1)`) and the engine walks that DAG as a
>   tree, so the multiplier is exactly the cast's declared width: `byte'` 8, `int'` 32,
>   `longint'` 64, and `int'(int'(x))` **1024** — against iverilog's 1. ⭐ The
>   discriminator is 2-state-ness, not width: `integer'` and `int'` are both 32-bit
>   signed and differ by 27× in wall clock. `lower_prim_cast` now carries the SAME
>   `expr_may_be_unknown` guard its sibling `inline_fn.rs` formal-binding site always
>   had, which collapses the nesting (1024 → 32) and buys 8.6×–542× on the reporting
>   design with byte-identical output (64 x/z cells PRE == POST == live iverilog).
>   ⚠️ **The count is still wrong and this row stays open**: a single `int'(f())` names
>   `f` 32 times because a `Call` is conservatively unknown, and a WIDENING cast over a
>   call fans out to the wider width (`longint'(int'(g(1)))` = 64). Closing it needs the
>   `expr_is_repeatable` gate this row already names — a different predicate from the one
>   that shipped, which asks whether the coercion is NEEDED, not whether the operand may
>   be evaluated twice. Both are required for the count; only the second one is.
>
> **🆕 §4.5.387 (round-36 ITEM A) — the same row again: the coercion was applied at the
>   TARGET's width, not the operand's.** The external report called a frame call inside a
>   continuous `assign` its biggest single cost (13.98 s of 72.94 s). Its own control
>   ladder refuted that: replacing `int'(nb)` by the hand-written `{28'd0, nb}` in the
>   otherwise identical file took the repro from 69.62 s to 2.76 s, so **25× of a 633×
>   gap was one cast over a 4-bit operand**, and dropping the 128-bit part-select moved
>   nothing (68.33 s). ⭐ Root: `lower_prim_cast` RESIZED first and coerced the resized
>   value, so a widening cast paid `tw` `CaseEq` terms for `w` bits of operand — 32 terms
>   for a 4-bit `nb`. The extension bits are provably no-ops (unsigned: a literal 0, and
>   `0 === 1'b1` is 0; signed: `CaseEq` is a per-bit function, so mapping the sign bit
>   then replicating it equals replicating it then mapping each copy), so the coercion
>   now runs at the OPERAND's width and the extension is applied afterwards, with the
>   sign fill coerced SEPARATELY (deriving it from the coerced value would name the whole
>   `w`-term coercion a second time). Measured, release, interleaved, warm-up discarded:
>   the reporter's repro **69.6 s → 6.7 s (10.4×)**, stdout byte-identical; the 4-bit
>   ping count 32 → 4, `longint'(g(1))` 64 → 32; examples 000..003 stdout+VCD
>   byte-identical. The same reorder landed at the sibling `inline_fn.rs` formal bind
>   (21 binds reach the coercion across the whole `cli` suite, 5 of them widening).
>   ⚠️ **The count is still wrong and this row stays open.** After the fix the repro sits
>   at 6.7 s against the no-cast control's 2.76 s — the residue is the 4 surviving terms
>   plus the frame call itself, and the frame call is now the LARGER half, so the
>   reporter's original axis becomes the next one to measure. ⚠️ Two things measured and
>   NOT shipped: a **per-bit skip** inside `coerce_two_state` (emit the bit directly where
>   `expr_may_be_unknown` proves that bit known) fires **0 times** on 41 cast cells and on
>   the repro, because the predicate's `Select` arm forwards to the BASE without
>   projecting a `Concat`/`Replicate` by bit offset — inside a coercion we only build
>   because the base may be unknown, so every bit-select answers "unknown"; and the third
>   caller (`inline_fn.rs:396`, the R2 return coercion) is **genuinely different** — it has
>   no resize in front of it, so its target width is doing the ZERO-EXTENSION as well as
>   the coercion and narrowing it would change the value, not just the cost.
>   ⚠️⚠️ **The reorder's own silent-wrong, caught by measuring its premise while the whole
>   suite was green over it**: the equivalence rests on `w` being the operand's ACTUAL
>   width, and `ir_bits_of` answers `None` for a deferred hierarchical reference (also a
>   `string` net, the string-producing system functions, the element-typed
>   `pop`/array-reduction family), whereupon the caller FABRICATES 32. Both orders are
>   built on that guess but degrade differently — coerce-after takes the low `tw` bits of
>   a concat of unknown real width, coerce-first FREEZES the guess into the low half:
>   `longint'(u1.w40)` with `logic [39:0] w40` is `0000001234567800` in iverilog 13 and
>   in PRE, and the unguarded reorder printed `0000000034567800` at exit 0. ⇒ the reorder
>   is taken only where the width is a DECLARED fact
>   (`a_width_unknown_operand_keeps_the_resize_then_coerce_order`). Same shape as the
>   §4.5.371 lesson: computing a width is not vouching for its provenance.
>
> **🆕 §4.5.368 곁발굴**(§3 성격 · pre-existing): **`$realtobits`/`$bitstoreal` 이 64비트 아닌 인자를 조용히 받는다**(iverilog 는 *"requires a 64-bit argument"* 로 거부). vita 는 저64비트를 답한다 — 그 경로가 살아 있는 것이 §4.5.368 의 BLOCKING 이 도달 가능했던 이유다.
>
> **🆕 §4.5.367 곁발굴 둘**(전부 PRE==POST · pre-existing):
>
> - **queue·assoc 원소의 part-select 쓰기가 조용히 사라진다**(verilator 오라클 · iverilog 는 구문 거부). `logic [31:0] q[$]; q.push_back(32'hFFFFFFFF); q[0][15:8]=8'h0F;` → verilator `ffff0fff` / vita **`ffffffff`**(무변화). ⭐ **dyn(`q[]`) 철자는 맞는다**(`changes.rs` 의 DynArray arm) ⇒ 같은 규칙의 세 철자 중 하나만 구현돼 있다 = write-twin 갭(memory: widen-a-read-sweep-the-write-twin).
> - **폭 0 인덱스 part-select 를 조용히 받는다**: `parameter P = 0; t[i +: P] = …` 를 iverilog 는 *"Indexed part select width must be an integral constant greater than zero"* 로 **거부**하는데 vita 는 exit 0. §3(loud 승격) 성격.
>
> **🆕 §4.5.366 이 남긴 64비트 unsigned 잔여 셋**(전부 PRE==POST · 각각 다른 경로):
>
> - **module-scope `localparam` 의 `/`·`%`·`>>`·`>>>` 는 여전히 부호를 잃는다** ⚠️ **선언 폭이 있어도 그렇다**(적대 differential 실측: `localparam [63:0] P = 64'hFFFF…FFFF % 64'd10` 은 PRE==POST 로 18446744073709551615 · 오라클 5) — 그 자리는 **순서비교만** 폭 인식 walk 로 redirect 되고 `/`·`%`·시프트는 `ctx_w`/`ctx_signed` 를 읽는데 거기가 (64, unsigned) 가 아니다.(2-오라클 · `localparam longint unsigned L = 64'hFFFF…FFFF % 64'd10;` 오라클 5 / vita 18446744073709551615). ⭐ **비교는 고쳐졌는데 이것들은 아닌 이유**: `const_eval_in_scope` 의 Binary arm 은 **문맥-결정 아닌 연산자만** 폭 인식 walk 로 redirect 한다(`binop_result_is_context_determined`) — 문맥-결정 연산자는 자기가 **폭-무제한 signed** `const_binop` 으로 접는다. ⇒ 선행조건 = §2 머리말이 말하는 **AST self-폭 패스**: `const_eval_in_scope` 가 폭을 받아야 하고, 그러려면 무타입 `localparam L =` 의 폭(= RHS 자기 폭)을 `const_self_width` 가 **모든 모듈 스코프 노드에서** 답해야 한다. ⚠️ 상수함수 본문·선언 폭이 있는 대입은 폭 인식 walk 를 타므로 **거기선 이미 맞다**(§4.5.366 이 고쳤다).
> - **>64비트는 의도적 decline**(`w == 64` 만 unsigned 로 읽는다). i64 가 이미 절단했고 **두 방향이 서로 반대로 틀린다** — `(64'hFFFF…FFFF + 65'd1) > 64'hFFFF…FFFF` 는 signed 읽기가 맞고(오라클 1) `((65'd1-65'd2) > 65'd0)` 은 unsigned 읽기가 맞다(오라클 1). 어느 쪽도 지배하지 않으므로 **추측 금지** ⇒ pre-slice 유지. 핀 = `const_unsigned_at_sixty_four.rs::above_sixty_four_bits_keeps_the_pre_slice_answer`.
> - **`*` 의 64비트 unsigned 오버플로는 loud**(`64'h8000…0000 * 64'd2` 두 오라클 **0** / vita **거부**). `checked_mul` 이 declines 한다. ⚠️ 고치려면 **문맥 폭이 정확히 64일 때만** wrap 해야 한다 — §4.5.348 이 *"mod 2^64 는 문맥이 64비트 이하일 때만 옳다"* 로 지어서 되돌린 그 축이고, 이번엔 폭 인식 walk 안이라 문맥이 **있다**(그때는 모듈 스코프라 없었다) ⇒ 재검토 가능한 유일한 차이.
>
> **🆕 §4.5.365 가 남긴 formal-bind 잔여 넷**(전부 PRE==POST · 각각 다른 경로 ⇒ CLASS 분리 규칙으로 기록):
>
> - **바인딩 자리는 넷이 아니라 아홉이다 — 다섯이 남았다**(2-오라클 · 전부 `f(300.0)`→`input byte` 가 **300**, 오라클 44). 고친 셋(inline 함수 · frame 함수 · frame task)과 이미 맞던 하나(inline task = **formal-폭 지역 net 에 copy-in** = 참조 구현) 밖에: ⓐ **output formal 을 가진 frame 함수**(`emit_frame_func_out_call` = 별개 emitter) ⓑ **계층 task 호출**(`hier_defer/task_call.rs` — 인자가 `inline_task.rs` 에서 **formal 폭 없이** 미리 lowering 된다) ⓒ **계층 함수 호출** ⓓ **class 메서드/task** ⓔ **class 생성자**. ⚠️ ⓑⓒ 는 **구조가 다르다** — deferred-hier 구조체엔 `&ast::Expr` 가 없다.
> - **`expr_is_repeatable` decline 이 남기는 조용한 기본값**(2-오라클). 캐스트가 피연산자를 2~5회 명명하므로 **한 번만 평가 가능한** actual 은 좁히지 않는다: 사용자 `Call`(`f(rfn(3))`), real 배열/큐 원소(`f(ra[1])`), 화이트리스트 밖 SysFunc(`$sqrt`·`$itor`·`$bitstoreal`), `p::rf(...)` 스코프 호출. ⭐ `$random` 은 **decline 이 옳다**(적용하면 두 번 draw = 다른 silent-wrong · 실측으로 1회 draw 확인). ⚠️ 곁현상: 캐스트 산출물이 `$rtoi` 를 품어 **중첩 호출에서 깊이마다 번갈아** 맞는다(N=1 정답·N=2 오답…) — 그 self-blocking 이 노드 증가를 선형으로 묶어 준다.
> - **`time` 선언의 명시 `signed` 한정자가 버려진다**(2-오라클 · `input time signed k` 에서 `k/2` 가 오라클 −4, vita 9223372036854775804). `kind_signedness` 가 `time`→unsigned 로 하드코딩해 **formal net** 이 unsigned 로 만들어진다 ⇒ net 을 쓰는 **static task** 경로가 틀린다. §4.5.365 는 나머지 셋을 **좁게 decline** 해 PRE 를 보존했다(고치면 셋이 같이 틀려진다) — 뿌리를 고치면 넷이 함께 열린다.
> - **범위 밖 real 의 정수 클램프**(`real rv = 1.0e300; byte'(rv)` 두 오라클 **0** / vita **−1**). §4.5.365 가 호출 경로를 **vita 자신의 캐스트와 일치**시켰을 뿐 클램프 자체는 그대로다(`±inf`·NaN 포함 · PRE==POST).
>
> **🆕 §4.5.364 가 남긴 지연 잔여 넷**(전부 PRE==POST · 각각 다른 경로 ⇒ CLASS 분리 규칙으로 기록):
>
> - **런타임 변수 지연**(`assign #(dv) y = a;` · **2-오라클** · dv=5 면 두 오라클 지연 5, vita 0). 상수가 아니므로 fold 가 아니라 **엔진이 서스펜션 시점에 평가**해야 한다(절차적 `#delay` 는 이미 그렇게 한다). `#(D, dv)` 부분 fold 도 같은 뿌리 — rise 만 접히고 사이드카가 통째로 떨어져 **fall 이 rise 값**이 된다. 핀 = `structural_delay_scope_fold.rs::a_runtime_variable_delay_is_still_zero_delay_and_still_quiet`.
> - **사이즈드 음수 리터럴이 새 규칙에 도달하지 못한다** — `#(-4'd1)` 은 `const_delay_ticks` 가 먼저 답하고 그 안의 `const_eval_u32` 가 **32비트 `wrapping_neg`** 을 하므로 영영 발화 안 한다(두 오라클 **15**). ⭐ **파라미터 쌍둥이(`parameter [3:0] Q = -4'd1`)는 이제 15 로 맞다** ⇒ 같은 값의 두 철자가 갈린다 = 이 슬라이스 자신의 규칙이 틀렸다고 부르는 모양. 고침은 `const_delay_u64` 의 `_` arm 이 `Unary` 를 declines 하게 하는 것 **한 줄** 이지만 `#(-5000000000)` 같은 unsized 큰 음수의 폭 판정을 오라클로 먼저 재야 한다.
> - **zero-rise 사이드카 거래**(`#(ZERO_PARAM, F)` 의 fall 이 버려진다 · 리터럴 쌍둥이 `#(0,F)` 는 맞다). ⭐⭐ **뿌리는 fold 가 아니라 엔진**: `Some(0)` 은 CA 를 delayed 레인으로 보내고 vita 의 zero-tick write 는 **자기 타임스텝의 Postponed 리전 뒤에** 착지한다(`$strobe` 로 실측 — 두 오라클 1, vita 0). 그래서 rise 0 을 `Some(0)` 으로 만들면 fall 을 고치는 대신 rise 를 그 lag 로 내려보낸다(양쪽 다 2-오라클 합치) ⇒ **silent↔silent 맞바꿈 금지** 규칙으로 보류. **zero-tick lag 를 고치면 둘 다 열린다.** 핀 = `..::a_zero_rise_with_a_distinct_fall_is_a_recorded_residue`.
> - **TimeLit 이 식의 루트가 아니면 안 접힌다**(`#(5ns + 2ns)`·`#(2*5ns)`·`#(2.5ns)` · 두 오라클 8/11/4, vita 0). 새 레인은 **`TimeLit` 노드 자체**만 전역 tick 으로 스케일한다.
>
> **🆕 곁가지**(§4.5.364 적대 리뷰 · 이 슬라이스가 만든 것 아님): 지연 CA 의 **인덱스 안 함수 호출**(`assign #(D) y = arr[h(a)];`)이 `native_eval/compile.rs` 의 *"is_codegen_able must keep Expr::Call off the native/VM path"* 로 **패닉(exit 101)** — PRE 동일 · iverilog 는 돈다.
>
> ✅ **DEEP 쪽 1건(inner NET vs outer PARAM shadow)은 P1 로 해소됐다(2026-08-18 · ARCHIVE §4.5.342).**
> 예언됐던 선행조건(*order-independent name set*)은 필요 없었다 — 필요했던 것은 **선언 블록의 SPAN**
> (`hoist_block_local_nets` 가 이미 들고 있었다 · span 은 order-independent AST 사실이라 §4.5.218 재발
> 없음). census 가 *"셋을 함께 연다"* 도 정정했다: `repeat (LP)` 는 **이미 열려 있었고**
> (`const_bound_u32` 가 `const_eval_in_scope` 를 쓴다) 남은 것은 §4.5.276 후속 ①(`for` trip-count
> 식별자) 하나뿐이다. ⚠️ **형제 항목(package 변수 clobber)은 §2 에 불릿이 없었다** — 2026-08-22 재census 가
> 그 dangling 참조를 발견하고 아래 불릿을 복원했다(목록에서 떨어진 항목은 census 아홉 개가 전부 못 본다).

- **🔴 block-local 선언이 같은 이름의 IMPORT 된 package 변수를 clobber 한다**(pre-existing · **2-오라클 합치** ·
  2026-08-22 재census 가 복원). `package pk; integer pv = 5; endpackage` 를 `import pk::*` 한 모듈에서
  `begin : blk integer pv; pv = 99; end` 이후 `pk::pv` 가 vita **99** / iverilog·verilator **5**. block-local 을
  BARE NAME 으로 모듈 net 에 flatten 하는 v1 모델이 import alias 와 같은 칸에 앉는다(memory:
  block-local-flatten-model). 사다리상 silent-wrong 이고 오라클이 둘 다 답한다.

- **파라미터 셀렉트의 상수 폭 — RESOLVED §4.5.363**(2026-08-22 · 상세=ARCHIVE) · **package 스코프 = RESOLVED §4.5.383**
  (2026-08-25 · 세 철자 `pk::W[m:l]` / `import pk::*` 뒤 bare / `import pk::W` 뒤 bare 가 **한 갭**이었고,
  선행조건으로 적혀 있던 *"패키지 const 테이블이 선언 폭 provenance 를 나른다"* 가 `pkg_const_range` 로 답해졌다 ·
  상세=ARCHIVE §4.5.383).
  ⚠️⚠️ **이 목록의 나머지는 2026-08-25 재측정에서 전부 stale 이었다** — 뒤 슬라이스들이 조용히 닫았고, 목록만
  남아 있었다. PRE(`a8dcc75`) 실측 · 전부 iverilog 일치: 셀렉트들의 concat **68** · 파라미터에서 파생된
  파라미터의 셀렉트 **52** · 중첩 셀렉트 `W[15:0][7:0]` **52** · 헤더 파라미터가 다른 헤더 파라미터의 셀렉트로
  기본값을 받는 형태 **52** · `#(.N(W[7:0]))` override **52** · `defparam` **52** · >64비트 파라미터 셀렉트 **52**.
  이 축과 무관한 둘(struct 멤버 폭 = **파서** 갭 · 클래스 속성 = 셀렉트 없는 쌍둥이도 *"undefined name"*)도 그대로다.
  ⭐ 규칙: **잔여 목록은 다음 슬라이스의 진단을 정하므로, 인용하기 전에 PRE 를 돌려라**(§4.5.375 의 그 교훈).
  남은 것은 하나다:
  - **🔴 const 배열을 가리는 안쪽 스칼라 — GAP-G 의 shadow 검사가 첫 가지에만 없다**(pre-existing · §4.5.363
    최종 리뷰가 발굴 · **오라클 하나**: iverilog 는 unpacked array parameter 를 아예 거부하고 verilator 만 답한다).
    `localparam int ROT [0:3] = '{…}` 를 generate 안쪽 `localparam int ROT = 99;` 가 가리면 `logic [ROT[1]:0] v` 가
    vita `$bits=21`(PRE 동일) / verilator **2**. 근인 = `const_array_vals_of_base` 의 **첫 가지**가
    `walk_scopes_key` 히트에서 바로 반환해 **자기 둘째 가지가 하는 inner-wins 검사**(`local_decl_names` ·
    `lookup_scoped`)를 건너뛴다 ⇒ shadow 를 뚫고 바깥 배열을 읽는다. ⭐ **모듈 스코프 철자는 이미 정확**하고
    §4.5.363 이 개선까지 했다(`round4_report_gaps` 핀) — 남은 건 **중첩 스코프 철자 하나** = 이 저장소의 서명인
    *"한 규칙, 철자 하나 모자람"*. §4.5.363 은 자기 fold 가 그 비대칭을 **다른 silent-wrong 으로 바꾸지 않도록**
    arm 순서를 미러링해 두기만 했다(PRE 복원).
  - **loud 잔여**(오라클은 답한다 · 사다리 이동 없음): >64비트 파라미터 셀렉트(`wide_param_bits` 가 비트를 들고
    있는데 i64 `params` 엔 없다 · ⭐ 진단 문구는 §4.5.363 이 **사실대로** 고쳤다) · 모듈 헤더 파라미터가 다른
    헤더 파라미터의 셀렉트로 기본값을 받는 형태 · `#(.N(W[7:0]))` override · `defparam` · struct 멤버 폭(⭐
    **파서** 갭 — 셀렉트 없는 `logic [W-1:0]` 도 E2002 로 죽는다 = 이 축과 무관) · 클래스 속성(셀렉트 없는
    쌍둥이도 *"undefined name"* = 이 축과 무관).

- **🔴 오름차순 음수 bound 가 클래스 속성에서만 클램프된다**(§4.5.350 적대 리뷰 발굴 · 포트·서브프로그램 지역은
  **§4.5.359 로 RESOLVED** · PRE==POST). `class C; logic [-3:0] q;` 가 W3056 + exit 0 로 **틀린 값**(verilator 4비트 ·
  iverilog 는 이 모양에서 assertion 사망 = **오라클 하나**). 구조가 다르다 — 클래스 필드는 net 이 아니고 정규화 맵은
  NetId 키다(**위 표 3b**). ⚠️ 더 값싼 절반이 §2 에 없었다: **정규화 안 된 클래스 필드 셀렉트는 lsb≠0 인 평범한
  `logic [7:1] q` 에서도 오늘 깨져 있다**.

- **🟡 packed 음수 low bound 를 가진 dim 의 비트 선택은 좌표를 만들지 못한다**(§4.5.350 적대 2렌즈 수렴 ·
  PRE 도 같은 자리에서 패닉했다). `logic [-3:0][1:0] x; x[-3]` — 옳은 좌표는 `(lo+size-1) - idx` 의 **부호 있는**
  뺄셈인데 `dim_coord` 의 오름차순 arm 이 그것을 안 짓는다. §4.5.350 이 거짓 불변식(`debug_assert`)을 **두 빌드
  모두 loud** 로 바꿔 놨다(release 는 그전까지 `lo.max(0)` 로 **조용히 틀린 비트**를 읽었다). 전체 값과 `$bits` 는 정확.

- **파라미터와 넷을 같은 이름으로 선언하면 vita 만 받는다**(pre-existing · PRE==POST · **두 오라클 모두 거부** ·
  P1 슬라이스가 발굴). `localparam N = 7; logic [3:0] N;` 를 iverilog 는 *"'N' has already been declared
  in this scope"*, verilator 는 *"Duplicate declaration of signal"* 로 거부하는데 vita 는 받고 이름을
  **파라미터로** 해석한다(`r=7`). §3.1 과 같은 부류(**vita 가 지어낸 확장**)이므로 loud 화는 사다리
  하강이 아니다. ⚠️ 그 답이 shadow 규칙의 `!params` 절이 유일하게 관측되는 자리라, 지금은 **불변임을
  핀**해 뒀다(`block_local_shadows_param.rs`).

- **🔴 파라미터의 PART-SELECT 를 폭 바운드로 쓰면 조용히 1비트가 된다**(pre-existing · PRE==POST ·
  오라클 ✓ iverilog · §3.3 슬라이스가 발굴). `localparam logic [31:0] W = 32'hdeadbeef; logic [W[7:0]-1:0] v;`
  가 vita `$bits(v)=1` / iverilog **239**. 전체 파라미터(`logic [W-1:0]`)는 정상이므로 갈리는 것은
  **part-select 가 상수 바운드 도메인에 안 닿는 것**이다 — §4.5.344 가 연 lane 들과 이웃(
  part-select 폭 lane)과 같은 뿌리로 보인다. **silent** 이므로 §2 정본 우선순위 ①.

- 🟡 **`%h` 가 1비트 미지 EXPRESSION 결과를 `x` 로, iverilog 는 `X` 로 찍는다** (§4.5.326 차분에서 발견·PRE==POST·이 슬라이스와 무관) — `$display("%h", ^a)` 에서 `a` 에 x 가 있으면 vita `x` / iverilog `X`. **같은 값의 1비트 NET 은 양쪽 다 `x`**(`reg [0:0] a = 1'bx`) 이므로 iverilog 자신이 일관되지 않고, IEEE §21.2.1.3(*"모든 비트가 같은 미지값이면 그 미지값(소문자), 섞이면 대문자"*)은 vita 편이다. 값이 아니라 **렌더링**이며 `%b`·`%0d` 는 일치한다. 승격 조건 = 제2 오라클이나 LRM 재독으로 판정을 굳힐 때. 215설계 중 17칸.

- ~~**🔴 사이즈 캐스트 `N'(expr)` 의 문맥 규칙**~~ **RESOLVED**(§4.5.316·상세=ARCHIVE) — IEEE §11.8.1 의 `max(self, N)` + 부호 무조건 전파. **잔여 6건**(전부 적대 리뷰 실측):
  - ⓐ **너비 프로브가 진단을 두 번 낸다**(§4.5.316 이 만든 품질 회귀·값은 정확): 좁힘 판정을 위해 노드를 한 번 낮춰 `ir_bits_of` 만 읽는데, 그 lowering 이 **진단과 사이드테이블 등록을 전부 실행**한다 — `8'(a >> pk::nope)` 가 E3009 **×2**, `2'({u1.nope} % 4)` 가 E3010 ×2(지연-hier 리졸버에서), `MAX_ELAB_ERRORS=200` 캡도 두 배로 먹는다. 그리고 죽은 노드가 **아티팩트에 직렬화**돼 깊이 32 중첩에서 `.velab` 가 **5.7×**(5884 B vs 1035 B) 커진다. §4.5.317 이 real 거부를 얹으면서 **`%`·`>>`·`>>>` 는 캐스트당 2건**이 됐다(프로브의 기존 일반 real 진단 + 새 진단 · iverilog 는 1건 · `<< <<< ** / * + -` 와 leaf 는 1건 = 최대 +1). **정본 해소 = 낮추지 않고 `w` 를 답하는 AST self-폭 패스**(그러면 프로브 자체가 사라지고 이 중복도 같이 사라진다).
  - ⓑ `w = ir_bits_of(plain)` 은 **노드**의 폭이라 캐스트 피연산자 **전체**의 self 폭을 못 본다(더 넓은 형제가 올린다 — `2'((s8>>u3)*s16)` vita `11` / 두 오라클 `01`, 11칸).
  - ⓒ **폭을 모를 때는 어느 기본값도 옳지 않다 — 시도했고 되돌렸다**(2026-08-08). **증상은 concat 래퍼가 없어도 난다**(적대 리뷰 실측·오라클 ✓): `2'(u1.mem[0] % 4)`·`2'(u1.k[7:0] % 4)` 가 vita `xx` / iverilog `11`, `string s="A"` 에서 `2'(s % 4)` 가 `xx` / hand-IEEE `01`. 즉 *"게이트가 bare 계층 피연산자를 거절한다"* 는 **whole-net 읽기에만** 참이고, 원소·비트/부분선택·string 은 그대로 arm 에 들어온다. `ir_bits_of` 가 `None` 을 내는 곳(실측): 지연-hier 읽기(whole-net·원소·비트선택) · `string` 넷 · **string 을 내는 `SysFunc` 가족**(`$sformatf`·`s.substr()`·`s.toupper()`). ⚠️ **클래스 필드·dyn 배열 원소·로컬 호출은 `None` 이 아니다**(초판 기록이 틀렸다 — 두 검출기로 실측). 계층 **호출**은 양쪽 다 loud 라 도달 불가.
  이제 `unwrap_or(n)` 은 **넓힘 가지**를 골라 이 arm 이 고치려던 좁힘 결함을 재현한다. **그러나 `unwrap_or(u32::MAX)` 로 좁힘 가지를 기본값 삼으면 더 나빠진다**: ⓐ `w` 는 가지 선택뿐 아니라 좁힘 가지의 **fill 문맥**으로도 쓰여(`lower_ctx_or_plain(lhs, w)`) `4'('0 / {u1.k})` 가 `0000` → **`xxxx`**(`fill_literal_const` 이 `width=u32::MAX` 상수를 만들고 `alloc_width` 는 캡을 건다 = 비일관 상수 → W4025 → X) · `.velab` **22.4×** · RSS **10 MB → 1.1 GB** ⓑ 좁힘 가지는 노드를 **자기 폭**으로 남기는데 `lower_size_cast` 는 **자기 몫의 `unwrap_or(32)`** 로 판단하므로 `32'({u1.k[7:0]} / d4)` 가 **8비트**를 낸다(넓힘 가지는 항상 `n` 비트라 우연히 정합이었다 · replicate 없이도 재현). 실측 회귀 41칸.
  ⚠️ **`unwrap_or(32)` 는 fill 축에서 4/4 정답**(`u32::MAX` 는 0/4)이지만 **가지 축에서는 다르다**(참 폭이 32 초과면 다시 좁힌다) — 한 변수가 두 질문에 답할 수 없다는 것이 이 항목의 결론이고, 저장소에는 이미 같은 뜻의 `unwrap_or(32)` 관례가 네 자리에 있다.
  **⇒ fallback 선택 문제가 아니다** — `w` 를 **알 수 있게** 만드는 것(= 낮추지 않고 답하는 AST self-폭 패스, ⓐ와 같은 선행조건)만이 답이고, 그것이 서면 프로브·진단 중복·아티팩트 팽창이 함께 사라진다.
  ⚠️ 초판 기록의 *"피연산자 확장은 값을 보존하므로 몫이 안 바뀐다"* 는 **일반적으로 거짓**이다(`b=-4'sd8, c=-4'sd1` 에서 `8'(b/c)`=8 · `4'(b/c)`=−8, 두 오라클도 그렇다) — 도달 가능 집합이 전부 `ext == false` 라서 **공허하게** 참이었을 뿐이다. 같은 이유로 초판이 `>>` 를 "잔여" 라 적은 것도 틀렸다: `2'({u1.k} >> '1)` 는 main `01` / 되돌린 구현 `11` = iverilog 라 그 절반은 **수정**이었다.
  - ~~ⓓ **넓힘 가지의 fill 이 깨진다**~~ **RESOLVED**(§4.5.318 — `lower_size_leaf` 가 fill 을 `n` 에서 짓는다).
  - ~~ⓔ **`**` 의 지수가 밑수의 부호를 따른다**~~ **RESOLVED**(§4.5.319·상세=ARCHIVE) — 근인은 이 기록이 지목한 `lower_expr_ctx` 가 아니라 **엔진**이었다(`eval_core.rs` Pow 팔의 `exp.signed = base.signed`). 지목된 자리도 별개의 live defect 여서 함께 고쳤고(`logic [7:0] r = a ** '1` → vita 0 / 오라클 2), `expr_size_ctx` 의 `Pow` 프로브-가지 이동도 그때 회귀 0·FIXED 72 로 착지했다. **잔여 = 아래 "지수 자기결정 CLASS" 항목.**
  - ⓕ **4-state 좁힘이 x 를 떨어뜨린다**(`a=8'bxxxx_0011` 에서 `2'(a+1)`·`2'(a*2)`·`2'(-a)`·`2'(a-1)` 이 전부 known / iverilog `xx` · `<<`·`&` 는 정상) — 저비트 폐쇄가 **2-state 논증**이라는 것의 대가.
  - ⚠️ **생존 뮤테이션 5**(리뷰 실측): `n >= w` → `n > w` 는 **죽지 않고 오히려 fill 칸 3개를 더 맞힌다**(`>=` 쪽에 근거가 없다) · `unwrap_or(32)`(ⓒ) · 넓힘/좁힘 가지의 **시프트 양을 문맥결정으로** 바꾸는 둘(킬러 설계는 리뷰가 제시했다) · `narrow_op` 와 `coerce_sign` 은 **서로 중복**이라 신규 테스트가 둘의 **논리합만** 핀한다.
- **🔴 사이즈 캐스트 주변의 pre-existing 셋(PRE==POST·같은 슬라이스가 실측)** — ⓐ **저비트 폐쇄는 2-state 논증이다**: `logic [7:0] a=8'bxxxx_0011` 에서 `2'(a+1)` 이 vita `00` / iverilog `xx`(캐스트 없는 `a+1` 은 vita 도 `xx` 로 정확 = 좁히기만의 문제). 비트연산과 `<<` 는 4-state 에서도 폐쇄(4,116칸 발산 0)라 절반만 틀린 논증이다. ~~ⓑ **`**` 는 음수 지수에서 폐쇄가 아니다**~~ **RESOLVED**(§4.5.319) — `Pow` 가 `Div|Mod|Shr|AShr` 폭-프로브 가지로 옮겨졌다(회귀 0 · FIXED 72). ⓒ **fill 리터럴이 `lower_size_ctx` 를 거치면 캐스트 문맥을 잃는다**(`8'(a+'1)` vita `…110` / 오라클 `…100` · `8'('1>>1)` vita `0` / 오라클 `01111111` · **비트연산 철자도 같다**: `4'('1 ^ a)` a=4'hD 에서 vita `1100` / iverilog `0010`, `9'(('1+one) % (sb**3))` vita `000000010` / iverilog `000000000` — 캐스트 없이는 둘 다 일치) — Size arm 주석이 *"fill 은 N 으로 자란다"* 라고 적은 바로 그 경로를 `is_size_ctx_operation` 게이트가 우회시킨다. ⚠️ **HEAD 에서 ⓒ가 재현되지 않는다**(§4.5.339 착수 재현: 72칸 매트릭스[9 연산자 × fill 2 × 위치 2 × 대상폭 2] **발산 0** · 인용된 네 철자 `8'(a+'1)`·`8'('1>>1)`·`4'('1 ^ a)`·`9'(('1+one) % (sb**3))` 전부 iverilog 일치) — 중간 슬라이스가 부수로 닫았다. ⓐ·ⓑ 는 유효.
- **🔴 크기 캐스트가 파라미터·패키지 파라미터·함수 호출 leaf 를 못 본다 — 그리고 고치려면 AST self-폭 패스가 먼저다**(pre-existing·PRE==POST·오라클 ✓·§4.5.318 이 지어서 재고 **되돌렸다**). `ast_ctx_signed` 가 leaf 하나라도 `None` 이면 캐스트가 문맥 하강을 **통째로** 건너뛰므로, 그 leaf 종류에서는 §4.5.212 의 carry 결함이 그대로 살아 있다 — 63칸 매트릭스에서 **35칸 발산**(`8'(P*Q)` = 13 / 오라클 45 · `16'(P<<4)` = 0 / 오라클 3840 · 넷과 `localparam int` 는 0/9). ⚠️ **해석기를 붙이는 것만으로는 안 된다**(전부 실측):
  - ⓐ 붙이면 그 트리가 `lower_size_ctx` 로 들어가는데, **안쪽 부분식의 self 폭이 `n` 을 넘으면 폭 모델이 틀린다**(§4.5.316 의 프로브도 `n` 에서 다시 잰다) → 6,720칸 중 **42칸 correct→silent-wrong**(`4'(13 + (PS>>2))` PRE −4 = 두 오라클 / POST 0 · net 쌍둥이는 PRE 도 틀림 = 맞바꾸기). **선행조건 = 낮추지 않고 트리 전역 self 폭을 답하는 AST 패스**(§2 의 ⓐ·ⓑ·real+fill 84칸과 같은 뿌리).
  - ⓑ **함수 호출 leaf 는 `extend_to` 가 두 번 참조한다**(`Select{base:e}` + `Concat[fill,e]`) → `40'(rnd(0)+0)` 이 함수를 **두 번** 부르고 두 `$random` draw 를 한 값에 **섞는다**(exit 0). §4.5.310 이 인덱스에서 겪은 그 부류 — **복제 가능성 술어로 게이트**하거나 값을 한 번 묶어야 한다.
  - ⓒ **함수 formal·함수/블록 local 은 결합 바인딩 집합에 없다** → lowering 을 흉내 내 결합 키를 먼저 잡으면 그것들을 지나쳐 **동명 모듈 파라미터의 부호**를 가져온다(52칸 중 11 회귀). 반대로 넷 우선(현재)이면 generate 스코프 localparam 이 바깥 넷에 가린다(`16'(X*S)` = 3 / iverilog 195, PRE==POST). **어느 고정 순서도 규칙이 아니다** — lowering 의 실제 결정 절차를 그대로 따라야 하고, 거기엔 `subst_lookup`/`out_subst_lookup` 같은 인라인 치환 경로가 더 있다.
- **🔴 크기 캐스트의 real 이 fill 과 만나면 아직 조용하다**(pre-existing·PRE==POST·오라클 ✓·§4.5.317 적대 2렌즈 실측). §4.5.317 이 **퍼널이 만드는 모든 피연산자**를 막았지만, 판별자 **셋이 동시에 성립**하면 퍼널에 애초에 안 들어간다 — 반대편이 **fill(`'0`/`'1`)** ∧ real 원천이 **평범한 real 넷이 아님**(`parameter real`·real 리터럴·real 함수 반환·`$signed(r)`·`$realtime`·`$sqrt(r)`) ∧ 연산자가 **real 비전파**(`& | ^ << >> >>> %`). 288칸 매트릭스 중 **84칸**(`4'(RP ^ '0)`·`4'($sqrt(r) & '1)`·`4'(b >>> (rt ^ '0))` 이 전부 exit 0·iverilog 는 전부 거부). 두 자물쇠가 같이 열려야 샌다: `ast_ctx_signed` 가 그 leaf 에서 `None` 이라 `lower_size_ctx` 를 **안 타고**, `expr_is_real` 의 `Binary` 팔에는 **비트/시프트/`%` 가 없다**. 하나만 고치면 다른 하나가 남으므로 **인스턴스가 아니라 CLASS** 이고, 선행조건은 위 ⓐ/ⓑ 와 같은 **트리 전역 AST self-폭/도메인 패스**다.
- **`$signed(real)`/`$unsigned(real)` 이 위치 의존적이다**(pre-existing·PRE==POST·§4.5.317 발굴). 캐스트 안 15자리는 거부하지만 7자리는 exit 0 — 평범한 산술(`$signed(r)*2` → 15)·`%0d`/`%0f` 포맷·int/real 대입. iverilog 는 `The argument to $signed must be a vector type` 로 **전부** 거부한다. 곁: **2인자 `$signed(r, u)` 를 조용히 받는다**(iverilog: *"takes exactly one(1) argument"*) — §4.5.317 은 그 형태에서 **어느 슬롯이든** real 이면 캐스트를 거부하도록 맞췄지만, 스펠링 자체의 거부는 별건이다.
- ~~**🔴 elaborate 상수접기가 `**` 지수를 자기결정하지 않는다**~~ **RESOLVED**(§4.5.339·상세=ARCHIVE) — 세 fold 사이트가 **한 헬퍼**(`const_pow_exponent_selfdet` → §4.5.186 폭 모델의 `eval_const_env_self`)를 공유한다. §4.5.319 가 닫은 다섯 철자에 이은 **여섯 번째이자 마지막**. 97칸 3-way **FIXED 10 · LOUD→CORR 2 · WRONG→LOUD 2 · 회귀 0**. **잔여 = 아래 「폭-미상 wrapping 지수」 한 줄뿐**(genvar 부호는 그 슬라이스가 함께 고쳤다).
- ~~**폭-미상 leaf 위의 WRAPPING 지수**~~ **소멸**(§4.5.348 재센서스 — §4.5.345 의 multi-packed 폭이 그 기록이 예측한 선행조건이었고, 그것으로 이미 닫혔다: `3 ** (m - 8'd250)` 이 두 오라클과 같은 59049).
- ~~**🔴 런타임 mixed-real Binary 가 정수 피연산자를 64비트로 넓힌다**~~ **RESOLVED**(§4.5.349 ·
  상세=ARCHIVE) — §11.8.1 의 변환 경계를 self-det 로. **잔여 셋**(전부 pre-existing · PRE==POST ·
  **2-오라클 합치** · §4.5.349 적대 리뷰 실측):
  - **`automatic`(framed) real 함수를 직접 피연산자로 쓰면 여전히 넓힌다** — `fa(1) + (-s)` 가 두
    오라클 **−7** / vita **9**. 곁: `{fa(1), 1'b0}` 이 **조용히 통과**(형제 형태 넷은 전부 loud). 공유
    규칙의 `Call` arm 이 그 형태에 **도달하지 않는다**(resolver 문제가 아니다 — static 함수는 인라인돼
    `Signal` arm 이 답하므로 정확하고, 임시변수 경유 `t = fa(1); t + (-s)` 도 정확).
  - **package/class 함수도 같은 구멍** — `p::one() + (-s)` 와 `c.getr() + (-s)` 가 두 오라클 −7 / vita 9.
  - **나머지 변환 경계는 아직 문맥-결정** — `real r; r = (-s);` 가 두 오라클 −8.0 / vita **8.0**,
    `r = (s+s)` 가 0.0 / **−16.0**. 즉 §4.5.349 는 **Binary(산술·비교)와 Ternary** 를 닫았고
    **단순 대입**은 남았다.
- **real-반환 const fn 의 본문 — §2 가 아니라 §3 이다**(2026-08-22 재측정). 로드맵이 적어 둔 *"16.0 을 조용히 접는다"* 는 **재현되지 않는다**: HEAD 는 `localparam real R = f();` 에 `E3009 … not a foldable constant expression` 을 낸다(iverilog 는 0.000000 = self-det 4비트 0). 즉 **honest-loud** 이므로 사다리상 §2 가 아니라 **§0-T2 real const-fold 잔여**의 한 철자다.
- ~~**u64 패턴 지수를 i64 도메인이 음수로 읽는다**~~ **RESOLVED**(§4.5.348 — 지수의 부호가 값과 함께 다닌다 · 크기가 도메인 밖이면 정직한 loud).
- ~~**인터프리터의 폭-0 타깃 · fold 안 되는 body-local 초기화의 조용한 0 · body-decl 초기화 크래시**~~ **RESOLVED**(§4.5.345·상세=ARCHIVE). **잔여 넷**(전부 §4.5.345 가 적대 리뷰로 실측):
  - **decl-init 호출 사슬이 깊이 캡 64 에 걸린다**(correct→loud · 두 렌즈가 독립 발견). 서로 다른 상수함수 70개가 각자 `int t = f<다음>();` 로 잇는 사슬이 iverilog 71 / PRE 71 / POST loud. **완화가 강하다**: 같은 설계를 *문장* 철자(`f<k> = f<k+1>() + 1;`)로 쓰면 **PRE 도 이미 loud** 였다(64 는 합의된 상수) — 즉 §4.5.345 는 decl 자리를 문장 자리에 맞췄을 뿐이고, 그 과금이 곧 자기참조 크래시를 없앤 것이다. 무손실 대안(리뷰 제안·미구현) = **이미 실행 중인 함수로 재진입할 때만** 한 레벨 과금(callee 이름은 `eval_const_call` 이 들고 있다) — 비순환 사슬은 안 막힌다.
  - **4-state 지역변수의 무초기화 기본값이 0 이다**(pre-existing·PRE==POST). `integer x; g = x + 1;` 이 vita **1** / iverilog **x**. i64 해석기가 unknown 을 못 나른다 — 2-state(`int x;`)는 정답.
  - **packed 차원 곱이 u32 를 넘으면 패닉**(pre-existing·PRE==POST·진단 없음). `bit [65535:0][65535:0] tt;` 가 `attempt to multiply with overflow` 로 abort. §4.5.345 의 `checked_mul` 자리가 아니라 그 위(넷 할당)다.
  - **선언 폭 모델이 셋인데 하나만 packed 를 본다**(maintainability). `const_decl_wsign`(곱 · §4.5.345) · `const_bound.rs::decl_is_wide`(첫 차원만) · `ast_kind_range_width`. 지금은 **건전**(차원을 무시하면 폭을 과소평가 → 과잉 decline, 단조) 하나 명시되지 않은 의존이다 — 폭을 *줄일* 수 있는 차원 규칙이 생기면 조용히 깨진다.
- **🔴 i64 상수 도메인이 오버플로에서 거절한다 — 언어는 문맥 폭에서 wrap 한다**(pre-existing·CLASS·
  **2-오라클 합치**·§4.5.348 실측). `+`·`*`·`**` 셋 다: `3037000500 * 3037000500` 이 두 오라클 145474192
  인데 vita loud · `64'h7FFF… + 64'd1` 이 0 인데 loud · `3 ** 40` 이 689956897 인데 loud. ⚠️⚠️ **문맥
  폭 없이 모듈러로 접지 마라** — §4.5.348 이 `**` 에 대해 지어 봤고 **loud→silent-wrong** 이 나왔다
  (mod 2^64 는 ≤64비트 문맥에서만 옳은데 모듈 스코프 fold 는 폭을 모르고, `localparam [127:0]` 타깃이
  이미 잘린 값을 zero-extend 한다). ⇒ 선행조건 = **폭-인식 모듈 스코프 fold**(§4.5.347 이 캐스트·비교
  안쪽에서는 그 패스가 선다는 것을 증명했다). vita **런타임은 전부 정확**하므로 오라클은 자기 안에 있다.
- **untyped `localparam` 의 거대 `**` 는 iverilog 가 행(hang) 한다**(§4.5.348 실측 · 오라클 부재 기록).
  `localparam L = 3 ** (64'd0 - 64'd8);` 에서 iverilog 가 임의정밀도로 3^(2^64−8) 을 계산하려 무한히 돈다
  (10분 100% CPU 확인 후 종료). 그 철자는 **verilator 단독**으로만 판정할 수 있다.
- **⚡ 상수 도메인의 비교/논리/삼항조건 fold 가 §4.5.347 이후 ~3배 느리다**(값은 전부 정답 · 리뷰 실측).
  `const_int_selfdet` 이 평가 전에 `const_self_width` 와 `const_signed_env` 로 트리를 두 번 더 걷고,
  비교 arm 이 lhs·rhs 를 폭·부호로 각각 다시 걷는다 = **피연산자당 6 walk vs 옛 2**. 실측(병리적):
  1,500개 localparam × 60항 체인 0.35 → 1.00 s · 이중 generate-for(같은 체인이 bound) 3.14 → 11.73 s
  (3.7×) · **컨트롤(같은 크기에 `+`) 1.00×** 라 비용은 정확히 리다이렉트다. ⚠️ **현실 설계에서는
  측정되지 않는다** — picorv32 elaborate 0.030 → 0.030 s · 평범한 3,000회 generate-for 0.116 → 0.162 s.
  후보 처방 둘: ⓐ 폭과 부호를 **한 번의 walk 로 융합**(6→4 walk) ⓑ generate-for bound 의 **genvar-free
  부분식 메모**(반복 배수를 없앤다 — 병리 케이스의 진짜 승수). 값이 아니라 시간이므로 차분에는 안 보인다.
- **`==?`/`!=?` 좌편향 체인은 여전히 2^depth**(pre-existing · §4.5.346 이 비-와일드카드 쪽만 닫았다 ·
  §4.5.347 이 그 상수를 2.6× 키웠다). 깊이 22 에서 30 s → 79 s. 값은 정확(iverilog 일치).
- **폭이 정확히 64 인 비교는 여전히 조용히 틀린다**(pre-existing · PRE==POST · **2-오라클 합치**).
  `((64'd1 - 64'd2) > 64'd0)` 이 두 오라클 1 / vita 0 — `masking = ctx_w > 0 && ctx_w < 64` 의 off-by-one
  이고, 63비트 쌍은 §4.5.347 이 고쳤다. 위 「64비트 상수」 항목과 같은 도메인 경계.
- **🔴 64비트 상수의 부호 없는 값이 i64 도메인에서 음수로 읽힌다**(pre-existing·PRE==POST·**2-오라클 합치**·§4.5.345 라운드 3 발굴). `localparam L = (64'hFFFFFFFF00000000 > 0) ? 111 : 222;` 가 vita **222** / iverilog·verilator **111**. 같은 결함이 `parameter [63:0] BIG = 64'hFFFF…; (BIG > 0)` 철자에도 있다. 뿌리 = `const_eval_i64_lit` 의 64비트 재해석 arm(*"magnitude misuse 는 range 검사가 loud 로 잡는다"* 고 적혀 있으나 **비교 위치엔 그 검사가 없다** — §2 의 u64 지수 항목과 **같은 주석·같은 구멍**). ⚠️ §4.5.345 는 이 클래스를 **새 문법(concat/replication)으로 넓히지 않으려고** 64비트 배치를 의도적으로 decline 한다(핀 `a_placement_that_does_not_fit_the_i64_domain_declines`) — 이 항목이 닫히면 그 decline 도 함께 열 수 있다. 처방 후보 = i64 대신 `(bits, width, signed)` 를 나르거나, 부호 없는 64비트를 만드는 자리에서 loud.
- **placement/캐스트 fold 의 잔여**(§4.5.345 가 연 lane 의 경계 · 전부 honest-loud). carry 연산이 든 concat(`{4'd2,(4'd1+4'd1)}` iverilog 34) · concat 안의 x/z · **prim/signing 캐스트**(`int'(7)` iverilog 7 — 그 피연산자는 자기결정이라 SIZE 의 `max(self,N)` 규칙과 다르다) · 지역변수에서 온 replication count(iverilog 도 거부). carry-free folder 를 넓히는 것은 *두 번째 산술 철자*라 금지 — 늘리려면 해석기 자신의 폭-인식 걷기로 라우팅해야 한다(위 표 1번과 같은 처방).
- **self-referential 반환 range 는 여전히 스택 오버플로**(pre-existing·오라클 없음 — iverilog 도 내부 abort·§4.5.339 발굴). `function [f():0] f();` — `const_fn_ret_wsign` 경로가 call 깊이를 안 끈다. 처방은 §4.5.339 가 default 에 쓴 **같은 한 줄**(`depth + 1`).
- **`coverpoint_domain` 의 Pow arm 이 정본을 미러한다는 주장과 어긋난다**(pre-existing·minor·§4.5.339 발굴). 그 함수는 `max(lw,rw)`/`ls && rs` 로 접는데 정본(`ir_bits_of`·`sim-ir::selfwidth`)은 **LHS 폭·base 부호**다. 영향 = 커버리지 auto-bin 개수뿐.
- **기본 백엔드의 Mul 체인이 밑수를 n 번 재-lower 해 진단을 n 배로 낸다**(pre-existing·PRE==POST·§4.5.319 라운드 3 발굴). `native_eval` 이 `a ** n`(작은 상수 n)을 Mul 체인으로 펼칠 때 `lhs` 를 n 번 낮추므로, 밑수에 범위 밖 배열 읽기가 있으면 `always @(posedge clk) r <= m[idx] ** 16;` 이 interp/native 에서 **E4002 2건**, bytecode 에서 **8건 + "further suppressed"**(8-cap 소진). **값은 안 틀린다**(체인 leaf 집합이 순수 — `Expr::Call` 은 명시 거부) — 갈리는 것은 진단 수뿐이고, 그것이 8-cap 을 먹어 뒤쪽 진단을 지운다. 같은 클래스가 `native/dirty.rs` 에 이미 기록돼 있다.
- ~~**🔴 좁히는 사이즈 캐스트의 결과가 더 넓은 대입 문맥에서 절단되지 않는다**~~ **RESOLVED**(§4.5.320·상세=ARCHIVE) — 캐스트가 `$signed`/`$unsigned` 로 **자기결정 경계를 실체화**한다. 그 슬라이스가 남긴 잔여는 아래 다섯 줄(전부 "elaborate 가 피연산자의 모양을 틀리게 안다" 한 CLASS).
- **🔴 prim cast 가 타깃 폭을 문맥결정 피연산자에 내리지 않는다**(pre-existing·PRE==POST·오라클 ✓·§4.5.320 발굴). `a=8'hFF` 에서 `int'(a*a)` 가 vita `00000001` / iverilog `0000fe01`, `shortint'(a*a)` 가 `00000001` / `fffffe01`. size cast 는 §4.5.212 이후 `lower_size_ctx_entry` 로 내리는데 `lower_prim_cast` 는 `lower_ctx_or_plain`(fill 만)이다. **그대로 배선하면 회귀한다** — prim cast 는 real 피연산자를 합법으로 받는데(`int'(r)` = 반올림) `refuse_real_size_operand` 가 그것을 loud 로 만든다. 선행조건 = 아래 real 항목과 같은 **AST 도메인 패스**.
- **🔴 `ir_bits_of` 가 클래스 필드의 폭을 핸들넷에서 읽는다**(pre-existing·§4.5.320 발굴). `try_class_field_read` 가 `Signal{net: 32비트 핸들넷, word: field-id}` 를 만들고 진짜 폭은 `class_field_widths` 사이드카에만 둔다 — `ir_bits_of` 는 사이드카를 안 보므로 **틀린 `Some(32)`** 를 낸다(`16'(c.sb)` = `xxxd`, hand-IEEE `fffd`). §4.5.320 은 캐스트에서 **정본 폭과 대조해 봉인을 거절**하는 것으로 막았을 뿐 `ir_bits_of` 자체는 그대로다 — 소비자가 많아 **CLASS**, 정본은 `canonical_self_width`. 같은 뿌리의 두 번째 증상(라운드 4 실측·PRE 동일): 캐스트 폭이 지어진 32 와 **같아지면** `Ordering::Equal` 이 resize 를 통째로 건너뛰어 캐스트가 사라진다 — `32'(c.s8)` = `fd`(8비트·hand-IEEE `fffffffd`) · `32'(c.s8 + ua[0])` = `fa`(캐리 소실·`000001fa`).
- ~~**🔴 `resize_inline_assign` 은 봉인 안 된 쌍둥이다**~~ **RESOLVED**(§4.5.321·상세=ARCHIVE) — 스탬프가 무조건이 되어 선언 반환 폭이 봉인한다. **잔여 넷**(전부 pre-existing·PRE==POST·적대 2렌즈 실측):
  - **🔴 인라인 경로가 선언 폭을 본문 안으로 안 내린다 — 그리고 프레임 경로는 내린다**(오라클 ✓). `function [31:0] fh(input [7:0] x); fh = fld * x;`(fld = 8'hFF) 가 static 에서 `00000001`, **`automatic` 에서 `0000fe01` = iverilog**. 같은 본문이 lifetime 철자 하나로 갈린다. 근인 = `lower_ctx_or_plain(rhs, ctx_w)` 는 **fill 만** 크기를 주고 문맥결정 연산에는 §4.5.212 를 적용하지 않는다 — prim cast 항목과 **같은 클래스**(폭이 피연산자에 안 내려간다).
  - ~~**🔴 인라인 인자 바인드가 formal 의 선언 타입을 적용하지 않는다**~~ **RESOLVED**(§4.5.325·상세=ARCHIVE) — 폭·부호·2-state 를 한 게이트로 함께 적용한다(1,920칸 FIXED 636·REG 0). **잔여 = 아래 다섯 줄**(전부 같은 바인드가 *못 고친* 자리이고 각각 다른 퍼널이다).
  - **🔴 프레임 인자 바인드가 §11.6.1 확장 부호를 안 쓴다**(오라클 ✓·§4.5.325 실측). 좁은 signed actual 이 더 넓은 **unsigned** formal 로 갈 때 0확장한다 — `function automatic [31:0] ff(input [15:0] x); ff = x;` 에 `8'shf7` 이 `000000f7` / iverilog `0000fff7`. **세 퍼널이 공유**한다(프레임 함수·프레임 태스크·클래스 메서드)라 CLASS 이고 별도 슬라이스. ⭐ 판별: 평범한 넷 대입 `u = 8'shf7` 과 **모듈 포트 연결**은 이미 정답이므로 문맥 하강이 아니라 **바인드**가 자리다. 1,920칸 중 24칸.
  - **🔴 `expr_is_repeatable` 이 배열 원소를 거절해 `f(mem[i])` 가 바인드를 못 받는다**(오라클 ✓). 상수 인덱스여도 `Signal{word:Some}` 이면 거절(중복 언급이 E4002 를 두 번 낸다 — §4.5.312)이라 signed/2-state formal 에 배열 원소를 넘기면 pre-slice 바인드가 남는다: `gs(arr[2])` 가 `00…f7` / iverilog `ff…f7`. 필요한 것은 반복 가능성이 아니라 **부작용 없는 중복**(진단 1회 보장). `f(mem[addr])` 는 일상 관용구라 실blast 는 "불순 actual" 보다 훨씬 넓다.
  - **🔴 계층 참조·클래스필드 actual 은 선언 폭 자체를 못 받는다**(PRE==POST). placeholder/지어진 32 라 `trusted_self_width` 가 `None` → 바인드 전체가 사퇴하고, 호출 결과가 선언 폭이 아니라 **actual 폭**으로 나온다(`function [63:0] gs(input [7:0] x); gs={x,x};` 에 `hi.hv` → 8비트 · 본문이 `x*x` 면 8비트 산술). generate 스코프 이름도 같다. 위 "지어진 폭" 항목과 **같은 퍼널의 다른 철자**.
  - **🔴 `cast_operand_is_real` 의 AST 절반이 bare 단일 세그먼트만 본다**(오라클 ✓·§4.5.325). `pa(f(0))` 는 4(정답)인데 **같은 callee** 를 `pa(p::f(0))` 로 쓰면 f64 페이로드가 2-state formal 로 들어간다(`c.cm()` 도). 한 설계가 한 callee 를 두 가지로 답한다 = §4.5.310 의 "철자로 인식" 재발. 넓히면 호출부 8곳을 건드리므로 별도 슬라이스.
  - **⚠️ 4-state actual → 2-state formal 의 강제가 런타임 O(선언폭)**(§4.5.325 실측 · 3백엔드 동일 · 폭에 선형: `byte` 12.8× `shortint` 23.8× `int` 46.4×). `coerce_two_state` 가 비트마다 피연산자를 언급하고 엔진이 DAG 를 트리로 걷는다. **알려진 값이면 안 짓는** 술어가 절반(2-state actual)을 지웠고, 나머지 절반은 `function int f(input logic [31:0] x)` 라는 흔한 철자다. 진짜 수정 = x/z→0 **IR 프리미티브 하나**(format bump) 또는 엔진 memoize. 같은 비용이 `expr_cast.rs` 캐스트 경로에 pre-existing. ⚠️ 완화 둘(per-query 메모·노드 예산)을 지어서 **둘 다 개선 0** 으로 반증했다 — 비용은 깊은 워크 하나가 아니라 **바인드 개수**에 있고(인라인 fold 가 호출마다 별도 서브트리를 만들어 아레나가 진짜로 2^n 개 노드), 영속 캐시는 in-place 패치(placeholder 는 패치 전에 "알려진 1비트 Const" 로 읽힌다)와 충돌한다.
  - **🔴 인라인 body-local 의 2-state 선언은 x/z 를 안 떨어뜨린다**(오라클 ✓·PRE==POST). 함수 본문 안 `bit [7:0] b; b = x;` 가 `x7` / iverilog `07`. `fold_straight_line` 이 폭·부호는 적용하고 2-state 단계만 없다 — 바인드와 **같은 IEEE 규칙의 두 번째 철자**라 그 둘은 함께 한 자리로 합쳐야 한다.
  - ~~**🔴 상수 인덱스 carve-out 이 인덱스를 무부호로 읽는다**~~ **RESOLVED**(§4.5.324·상세=ARCHIVE) — signed 상수를 인덱스 도메인으로 부호확장한다. **위 인라인 바인드 항목의 선행조건이 이것으로 섰다.** 잔여 두 줄은 아래.
- **🔴 queue·dynamic 배열의 인덱스에는 봉인이 아예 없다 — 상수도 넷도**(pre-existing·오라클 ✓·§4.5.324 적대 리뷰 발굴). 256엔트리 `int q[$]` 에서 `q[-8'sd1]` 과 `q[s8]`(s8 = −1)이 **진단 없이** 원소 255 를 읽는다(iverilog 는 기본값 `0`). `int d[]` 도 같다. **쓰기 쪽은 loud**(W4020)라 read/write 비대칭이고, `dynarr.rs` 가 인덱스를 직접 낮춰 `seal_index_unsigned` 를 아예 안 부른다 — 고정 배열의 carve-out 보다 **넓은** 클래스다. ⚠️ verilator 는 2의 거듭제곱 크기에서 마스킹하므로 이 칸의 오라클이 아니다.
- **🔴 함수 호출 인덱스는 어느 봉인에도 못 온다**(pre-existing·오라클 ✓·§4.5.324 발굴). `arr[fneg(0)]`(`fneg` 가 `-8'sd1` 반환)이 **조용히** 원소 255 를 읽는다(iverilog `xx`) — 런타임 봉인이 `Call` 을 **반복 가능성**에서 거절하기 때문(부호 fill 이 두 번 부른다). §4.5.320/321 의 같은 뿌리.
- **🔴 `$bits` 를 상수 인덱스 식에 쓰면 `x` 를 읽는다**(pre-existing·오라클 ✓·§4.5.324 발굴). `m[$bits(m[0]) - 4'sd9]` 가 vita `xx` / iverilog `10` — 같은 값의 리터럴(`m[8 - 4'sd9]`)과 `$clog2` 철자는 정상이라 **`$bits` 한정**이다.
- **🔴 평범한 벡터의 비트/부분선택은 음수 상수 인덱스를 무부호로 읽는다**(pre-existing·오라클 ✓). `logic [15:0] pv = 16'hFFFF` 에서 `pv[-2'sd1]` 이 `01`·`pv[3'sd7 -: 2]` 가 `3` / iverilog `0X`·`x`. 별도 퍼널(`sealed_signed_index`/`norm_sub_k`)이라 §4.5.324 가 안 닿는다.
  - **🔴 인라인 바인드의 폭 결정이 `ir_bits_of` 의 지어진 폭을 믿는다**(pre-existing·§4.5.323 실측). 클래스 필드는 핸들넷의 32 를 답하므로 절단/확장 판정이 뒤집힌다 — `function [63:0] i16(input [15:0] x); i16 = x;` 에 `i16(c.bu)`(8비트 필드)가 `xxc3`. 창은 `필드폭 < formal폭 < 32`. 정본은 `canonical_self_width`.
  - **🔴 `real` rhs 는 §10.7 을 통째로 건너뛴다** — `resize_inline_assign` 의 `expr_is_real` early-return 이라 선언 폭이 절단을 못 한다(`f = r + x*x` 가 `013b` / iverilog `3b`). 봉인이 못 막는 유일한 문.
  - **🔴 `!trusted_w` 카브아웃 아래는 아직 샌다**(설계상 트레이드). 클래스 필드 rhs 가 지어진 32 == 선언 폭이면 같은-폭 팔이 맨 노드를 돌려준다 — `function [31:0] fh; fh = c.big + 1'b1;`(40비트 필드)가 대상 폭에 따라 값이 달라진다(`00000000` vs `0000010000000000`). 무조건 봉인하면 이 칸은 낫지만 위 첫 항목의 판별 칸이 깨진다 = **지어진 폭을 믿을 수 없다는 것의 대가**.
  - 곁: **actual 이 formal 보다 넓으면 바인드에서 절단하지 않는다**(의도적·`inline_fn.rs`) — `function [7:0] f(input [3:0] x); f = x;` 에 `f(8'hFF)` 가 `ff` / iverilog `0f` · **함수 반환을 replication count 로** 쓰면 `{f(8'h02){1'b1}}` 가 0 / iverilog `f`.
- **🔴 캐스트/인라인의 확장 부호가 미러에서 온다 — 클래스 필드는 순수한데도 못 받는다**(pre-existing·PRE==POST·§4.5.321 적대 리뷰 발굴). §4.5.320 은 넓히기 arm 의 정본 부호 채택을 **불순한 피연산자(프레임 `Expr::Call`)** 때문에 보류했는데, **signed 클래스 필드**도 그 arm 에 도달하고 그것은 **순수·반복 가능**하다 — `function signed [63:0] fw; fw = c.sf;`(`sf = 8'hAB`)가 `00…ab` / hand-IEEE `ff…ab`. 즉 정본 부호 채택은 **반복 가능성 술어로 게이트한 진짜 수정**이지 무가치가 아니다(전용 슬라이스).
- **🔴 캐스트의 문맥폭이 안쪽 자기결정 노드에서 멈춘다**(pre-existing·PRE==POST·두 오라클 ✓·§4.5.320 발굴). `64'(-16'(u16))` 가 `…fffb` / 두 오라클 `…fffb` 가 아니라 vita `000000000000fffb`(오라클 `fffffffffffffffb`), `8'(s4 * 4'(s8))` 가 `…f9`(−7) / `…09`(+9), `16'(s8 + 4'(u8))` 가 `000c` / `010c`. 무중첩 대조군 `64'(-u16)` 은 정상이라 트리거는 **중첩 캐스트·`$signed`/`$unsigned` 노드**다. 10,368칸 중 143칸, 전부 signed 좌변 + 안쪽 캐스트가 더 좁을 때.
- **넓히는 캐스트가 불순한 피연산자의 부호 수정을 못 받는다**(§4.5.320 이 의도적으로 남긴 값). `extend_to` 의 부호 fill 이 피연산자를 두 번 부르므로 `16'(f())`·`int'(f())` 는 PRE 의 무부호 답을 유지한다(오라클 `fffd`/`fffffffd`). 닫으려면 **피연산자를 한 번만 부르는 4-state 보존 확장** 또는 callee 순수성 술어가 필요하다. 곁: `int'`/`byte'`/`shortint'`/`longint'` 는 `coerce_two_state` 때문에 피연산자를 **32/8/16/64회** 평가한다(PRE 동일).
- **캐스트가 원소의 부호를 청구하지 못하는 나머지 철자들**(전부 pre-existing·PRE==POST·§4.5.320 라운드 4 실측). `unpacked_elem_signed` 는 base 가 **단일 세그먼트 ident** 인 경우만 청구하므로 아래가 남는다 — 전부 `40'(x[0]*1)` 형태에서 vita `00000000fd` / iverilog `fffffffffd`: 다차원 `g[i][j]` · **패키지 한정 `pk::pm[0]`** · 프레임 로컬 배열 · dyn 배열/queue 원소(`net_is_static_array` 가 의도적으로 제외) · 인터페이스 배열 원소(같은 인터페이스의 **스칼라** `ii.v` 는 정상). ⚠️ **패키지 철자가 가장 급하다** — `arrays.rs` 는 `pkg::arr[i]` 를 원소로 라우팅하는 전용 arm 을 이미 갖고 있으므로 **분류기가 자기 lowering 리졸버와 어긋나 있고**, 그 결과 한 설계 안에서 같은 배열의 두 철자가 다른 값을 낸다(`pm[0]` 은 정답 · `pk::pm[0]` 은 오답). PRE 는 둘 다 틀렸으므로 회귀는 아니지만 분기 자체가 신호다. 곁: **계층 배열 원소의 캐스트는 부호를 잃는다**(⚠️ 2026-08-22 재측정으로 **정정** — 옛 문구의 `x` 주입은 더 이상 재현되지 않는다): `u1.sarr[0]` 무캐스트 읽기는 `fffffffffffffff9` 로 정확한데 `16'(u1.sarr[0])` 이 vita `000000000000fff9` / iverilog `fffffffffffffff9` — 남은 것은 위 부호 축 하나다.
- **🔴 fill override 가 타깃 폭이 아니라 32비트로 접히는 자리 셋**(pre-existing·PRE==POST·오라클 ✓·§4.5.314 적대 2렌즈 발굴). §4.5.314 는 `'0`/`'1` 을 **선언 폭에서 다시 접도록** 퍼널을 세웠고 그 폭이 존재하는 곳은 전부 고쳤으나, `param_decl_width` 가 `None` 을 내는 세 형태는 여전히 부모 쪽 32비트 fold 로 떨어진다 — ⓐ **>64비트**(`parameter [127:0] K` + `'1` → 하위 64비트만 `0000…ffffffffffffffff`, iverilog 는 128비트 전부 1). 이것은 "wide 파라미터의 OVERRIDE 는 loud" 라는 아래 §3 불변식의 **구멍**이기도 하다(같은 선언에 명시 리터럴 `128'hFF…` 를 주면 loud 인데 `'1` 은 조용히 통과) ⓑ **`time`**(64비트 모델이 아니라 32비트 — `#(.T('1))` 이 4294967295, iverilog 18446744073709551615) ⓓ **`real`**(`#(.R('1))` 와 `-G R='1` 이 둘 다 `4294967295.0` 를 설치한다 — iverilog `1.0` · 두 채널은 서로 일치) ⓒ **untyped**(IEEE §12.2.2 는 파라미터가 override 의 폭·부호를 **받는다**고 한다 — `parameter K = 3` + `#(.K(64'hDEADBEEF))` 가 vita −559038737 / iverilog 3735928559, 그리고 `'1` 은 iverilog 가 1비트로 봐 `1`). 셋은 한 뿌리(**타깃 타입의 폭을 정하는 규칙**)이므로 한 슬라이스로 다뤄야 하고, 세 채널(`#()`·`defparam`·`-G`)이 지금은 **서로 일치**한다(§4.5.314 가 맞춘 것) — 고칠 때 그 일치를 깨지 마라.
- **파라미터 선언 fold 가 네 벌**(pre-existing·PRE==POST·오라클 ✓·§4.5.314 적대 2렌즈 실측). 정본은 `params.rs::bind_one_param` 이고 나머지 셋이 각자 빠뜨린 것이 다르다 — `instance.rs` 의 모듈 본문 fold(override 처리 없음) · `generate.rs` · `package.rs`. 측정된 불일치: generate/package 는 `const_eval_in_scope` 로 접어 **fill 기본값을 선언 폭으로 안 접고**(`parameter [63:0] Q = '1` → `00000000ffffffff`), `package.rs` 는 **`param_range` 를 기록하지 않으며**(패키지 `parameter [15:8] P` 의 부분선택이 `x`) **`string`/`real` 을 라우팅하지 않는다**(자기 기본값만으로 E3009). §4.5.314 가 인터페이스 복사본(넷째)을 정본으로 흡수했으므로 남은 셋도 같은 방식이 가능하다 — **인스턴스가 아니라 CLASS 이므로 발견한 자리에서 고치지 말 것**(§4.5.311 교훈).
- **`--obs-dir` 의 run.json 이 `-G` 를 안 싣는다**(pre-existing·§4.5.314 발굴). `plusargs` 와 `source.blake3` 는 있는데 파라미터 override 는 없어서, `-G W=9` 와 `-G W=100` 으로 돌린 두 런의 run.json 이 타임스탬프 말고는 **동일**하다 — 효과가 **다른 설계**인 유일한 플래그에 대해 G2 관찰 rail 이 눈멀어 있다. §4.5.314 는 같은 논거로 `-v` echo 에 행을 넣었고 rail 에는 적용하지 않았다(§6 OBS-1 잔여와 같이 다룰 것).

**문서화된 divergence (수정 비대상·핀됨):**

- **untyped localparam 의 정수 init 폭 — 오라클이 갈린다**(§4.5.343 실측). `localparam L = 4'd15 + 4'd1` 이 vita **16** = iverilog 16 / verilator **0**(§6.20.2 를 self-det 로 읽음). 두 오라클이 갈리고 vita 는 iverilog 편이므로 기록만.

- **iverilog 는 64비트 unsigned `%` 에서 자기모순**(§4.5.366 적대 differential 실측): `64'hFFFFFFFFFFFFFFFF % 64'd10` = **5** 인데 **같은 값**을 다르게 적은 `(64'd0 - 64'd1) % 64'd10` = **1**(같은 설계 안에서). ⇒ 그 축의 오라클로 `(0-1)` 철자를 쓰지 마라.
- 크로스스코프 t0 decl-init race(양쪽 §6.8 합법·self-consistent) · 런타임 구성 `-0.0` 표시 · iverilog 자인 결함들(expression-force "evaluated once" 등).
- **`$stime` 의 부호 — 두 오라클이 갈리고 규격이 정했다**(§4.5.320). `16'($stime)` 이 t=0x8000 에서 vita/verilator 5.050 `00008000`, iverilog 13.0 `ffff8000`. IEEE 1364-2005 §17.7.2 = "returns an **unsigned** integer that is a 32-bit time". vita 는 캐스트 **밖에서도** 이미 무부호였고(`q = $stime` at t=2^31 → `0000000080000000`, PRE 동일) iverilog 만 signed `integer` 를 돌려준다 — PRE 가 캐스트 자리에서만 iverilog 와 맞았던 것은 자기모순이었다.
- **`#(.S("str"))` 가 적용되기 전에 W3056 을 한 번 낸다**(pre-existing·값은 정답): 부모 쪽 숫자 fold 가 먼저 실패해 "override 는 상수가 아니다; 기본값 유지" 를 찍고, 그 다음 string 채널이 정상 적용한다. 경고가 사실과 반대라 거슬리지만 값은 iverilog 와 일치한다.

## 3. Loud→supported 후보 (현재 전부 loud=안전 · additive)

> **⑫ 상수 함수의 누산기가 i64 보다 넓다** (2-오라클 · **verilog-axi 를 막는 마지막 것** ·
> §4.5.379 가 *"비싼 절반"* 이라 부르며 § ② 뒤로 재배치했고, ② 가 §4.5.382 에서 섰으므로 **이제 이것이
> 줄의 앞**이다). `axi_crossbar_addr.v:144` 의 `M_BASE_ADDR_INT = calcBaseAddrs(…)` — 본문이 루프로
> 128비트를 쌓는 상수 함수. `eval_const_call` 의 env 는 **i64 맵**이라 폭에서 끊긴다. 선행조건 =
> 상수함수 인터프리터의 env 를 `WideBits` 로 올리는 것(값 표현 하나가 아니라 `ConstWidths` 짝까지).
> §4.5.382 가 지은 `fold_self_bits` 의 산술/비교 arm 이 그 계산 자체는 이미 할 수 있다.

> **⑬ 런타임 진단에 `file:line` 이 없다** — **RESOLVED (V33-8, 2026-08-26)** for the two codes the
> report named. W4029/E4002 (array word index) and W4023 (`$readmem*`/`$writemem*`/`$fread`) now
> anchor at `file:line:col [in instance]`, one-shot and staged alike, and the reporter's shape
> (one table read from three places) yields three distinct lines. The prerequisite this line asked
> for turned out to be ALREADY BUILT: `stmt_locs` (the #10 severity sidecar, `.velab` v29) is a
> StmtId → `SourceLoc` map outside the golden IR, so the slice only widened WHICH statements earn
> an entry (array-indexing / `$readmem*` / call-bearing) and taught the engine which StmtId is
> executing (`SimState::cur_stmt`, published by all three process-body executors through the
> `Kernel::k_set_cur_stmt` seam).
>
> Residues, each with its reason:
> - **호출된 서브루틴 본문 안의 접근은 CALL 문에 귀속된다**(subscript 줄이 아니라). tier-3 arena 는
>   접근을 RECORD 만 하고 caller 의 문장 경계에서 drain 하므로 callee 의 StmtId 를 알 방법이 없다
>   (두 번째 `cur_stmt` 원본 또는 `Rc<Cell>` 공유가 필요). 측정: publish 를 넣으면 `--backend interp`
>   가 `d.sv:6`, `--backend native` 가 무위치 ⇒ **한 설계가 플래그 하나 차이로 두 줄**. 백엔드 일치를
>   택했고 테스트로 핀했다(`a_read_inside_a_subroutine_is_named_by_its_call_statement`).
> - **terminator 조건**(`if (mem[i])`)의 접근은 위치가 없다. 블록의 마지막 문장 뒤에 평가되므로
>   `cur_stmt` 를 NO_STMT 로 지운다 — 확신에 찬 틀린 줄보다 무위치가 낫다.
> - **cont-assign settle · t0 arm · delayed-CA apply** 의 drain 도 같은 이유로 무위치.
> - 그 밖의 엔진 진단(W4022 closed-fd, W4028 plusargs, delta-limit 등)은 아직 `location: None`.
>   같은 `cur_stmt` + `stmt_diag_meta` 로 한 줄씩 열 수 있다(배관은 이미 있다).
>
> 비용(실측): `.velab` +5.3%(keccak_f_arr) — 모든 문장이 배열 인덱스이자 호출인 합성 최악에서 +50%.
> 런타임 회귀 없음(keccak_f_arr N=1000 인터리브 A/B: PRE 8.362 s · POST 8.330 s · 분산 ~0.6%).
> ⚠️ 곁수확: `SourceMap::resolve` 의 line/col 이 **파일 처음부터의 선형 스캔**이라 문장당 해석이
> `velab` 를 0.01 s → 0.10 s 로 만들었다 ⇒ 파일별 line-start 인덱스를 메모이즈해 원상 복구
> (모든 기존 진단 경로도 같이 빨라진다 · `line_col_index_matches_the_linear_walk` 로 두 철자 동치 핀).

> ~~**⑭ 프로세스별 평가 횟수/시간 관측 수단**~~ ✅ **RESOLVED (the observability half) — R7/§4.6**
> (2026-08-26). (aes_top R14 · OBS · §6 과 인접). 리포터의 top(엔진 5 + 코어 1)이 **≈20 cycle/s** 라
> 전량 스윕을 vita 로 못 돌리고, *"행(hang)"* 과 *"느림"* 을 구별할 수단이 없어 타임아웃 상수를
> plusarg 로 빼야 했다. 요구는 성능 개선 **또는** `--obs-dir` 에 프로세스별 평가 횟수/시간.
>
> **Shipped**: `--obs-procs` (deterministic per-body evaluation COUNTS) and
> `--obs-procs-time` (adds cumulative wall clock), both surfacing as `run.json`'s
> `processes` object — one row per process AND per continuous assign, sorted
> most-evaluated first, each naming its construct `kind`, its instance `scope`
> and its `file:line:col`. SPEC = [19-ai-agent-observability §4.6](preview/19-ai-agent-observability.md),
> user docs = `docs/manual/004_cli-reference.md`. ⭐ The IDENTITY is the part that
> made it useful: an index answers nothing, and a module instantiated 40 times
> needs `scope` to tell its copies apart (measured on a 4-way generate loop —
> `tb.gb[0]`…`tb.gb[3]`, same line, separate rows). ⚠️ Synthesized bodies are
> labelled apart (`var_init`/`sva`/`covergroup`/`clocking`/`port`/`net_init`)
> because saying `always` at a line with no `always` keyword is the same
> misdirection the feature exists to prevent — including the parser's synthetic
> `initial` wrapper around a module-level `assert property`. ⚠️ Timing is a
> SECOND flag on purpose: counts are deterministic (byte-diffable `run.json`),
> wall clock is not, and for a one-bit continuous assign the two `Instant::now()`
> can exceed the work they measure. Cost when NOT asked for: **inside the noise**
> on `bench/keccak` (−0.34% / +0.12% over two rounds), **+1.3%** on a
> deliberately seam-dominated synthetic — see §4.6's table.
>
> ⭐ **Round-35 R3 corrected one of these labels' locations.** `port` rows shipped
> with `("", 0, 0)`, on a doc comment claiming a port hookup "has no source span
> of its own that would help a reader — the useful half is the INSTANCE". The
> reporter measured **1,267 `port` rows = 51% of all evals** on their design, and
> `scope` cannot resolve them because one instance there carries **39**
> connections: the biggest category in the profile was the only one nobody could
> act on. A `port` row now reports the PORT CONNECTION in the parent's
> instantiation (`.p(expr)` from its `.`, so `col` separates connections written
> on one line; the expression itself for a positional list). `("", 0, 0)` is now
> reserved for what genuinely has no source text — a `.*` wildcard synthesizes
> one connection per unnamed port — and an unpacked-array port's per-element rows
> honestly share their one connection's position. Reporting only (`ProcIdent` is
> an elaborate-side sidecar, not a `sim-ir` type): examples 4/4 stdout + VCD +
> `.vu` + `.velab` **byte-identical PRE vs POST**, format_version stays 29.
>
> **NOT shipped — the headline ask stays open**: the reporter wants ~440 cycle/s
> and measures 20.4, which is a **21x scheduler/executor question**, not an
> observability one. That axis is Phase D (machine-code codegen) + the arena
> prerequisite, tracked in §5.1. What ⑭ delivers is the reporter's own stated
> fallback: enough per-body attribution to cut the cost on their side.
>
> ⭐ **Round-36 R2 added the half INSIDE a process row — §4.9 `builtins`.** The
> reporter came back with the granularity limit: their single largest row is one
> `initial` at `tb_aes_top:729` worth **60% of the run**, because it calls a
> vector-driver stack and every nested cost is summed into the caller — so the
> profile says THAT the testbench is expensive and not WHICH LINE. They asked for
> (1) a call tree to task granularity, or failing that (2) per-builtin cumulative
> time. **(2) shipped**: `run.json` gains a `builtins` object beside `processes`
> under the same two flags — one row per system task / system function / method
> form (`$fgets`, `$sscanf`, `.push_back()`, `.size()`, `$sformatf`, …), keyed by
> the NAME the author typed (a builtin has no declaration site, so there is no
> `file:line:col` twin to report). ⭐ Two fields answer *"may I add these up?"* in
> the file rather than in a doc: `attribution:"self"` (a row EXCLUDES builtins
> nested inside it — `$display("%0d", q.size())` is a real nesting, measured:
> `$fdisplay` 0.0424 s + `$sformatf` 0.0292 s under a 0.0767 s process row, where
> an inclusive convention would have summed to 0.100 s > the parent) and
> `included_in_processes:true` (that time is ALREADY inside the process row —
> the useful arithmetic is subtraction, not addition). ⚠️ **Four seams, one
> object**: `builtins::dispatch_with` (every system task, all four backends),
> `exec::apply_effect` (statement-effect system functions), `EvalCtx::
> eval_sysfunc_ctx` (the pure half — string/queue/array/math), and the `&self`
> frame executor's own three arms, which do NOT go through `dispatch` — a hook
> only in the shared funnel would have under-reported every print inside a subset
> function body. Accumulators are interior-mutable (`RefCell`/`Cell`, the
> `RngCells` pattern) precisely because three of the four seams hold only
> `&self`. Cost when NOT asked for: **+0.6…0.9%** on a constructed worst case
> (3M pure `$clog2`/`$countones` evaluations, two independent interleaved rounds)
> against a builtin-free control that moved **−1.9%** — i.e. under this method's
> own noise here. format_version stays **29** (no frozen type touched; the name
> tables are free functions in `sim-ir`), examples 4/4 stdout byte-identical.
>
> ⚠️ **(1) is NOT shipped, and the reason is structural.** vita lowers a
> subroutine call two ways: a frame body with a runtime call seam, and an INLINE
> splice into the caller at elaborate time (`inline_task.rs`/`inline_fn.rs`;
> round-35 R4 measured 14.39 s inline vs 0.35 s frame, so both are live). A
> profile built on the call seams would report **0 calls** for every inlined
> subroutine, and a task showing 0 reads as *"free"* — the one answer a profile
> must never give about the thing the user is hunting. Prerequisite, recorded:
> an elaborate-time record of which call sites were inlined and into which
> caller, so a seam-less task can be reported as *inlined into its caller* rather
> than as absent. The identity half already exists (`Sidecars::func_names` is
> parallel to `SimIr.funcs`; only a declaration `file:line:col` twin is missing).


> ### 🆕🆕 **워크로드 코퍼스가 연 줄** (§4.5.369 · **①은 §4.5.370 으로 RESOLVED** · 2026-08-23)
>
> 허가적 라이선스 서드파티 RTL **여덟**을 오라클로 고정해 훑은 결과다(`crates/corpus-runner`,
> 상세 = [study/03](study/03-workload-corpus.md)). ⚠️ 이 다섯은 **우리 프로브가 아니라 남의
> 코드**가 찾았고, 앞의 둘은 §2 큐에 **한 줄**로 있던 축이 실제로는 설계 셋을 막고 있었다는 뜻이다.
> 우선순위는 위에서 아래로 — 위의 둘이 코퍼스 8 중 3 을 막는다.
>
> ~~**① 문자열 상수 도메인이 리터럴 전용이다**~~ ✅ **RESOLVED — §4.5.370**(2026-08-23). ⭐⭐ 참조
> 구현이 이미 트리 안에 있었다: `const_fn.rs::const_str_in_scope` 가 StrLit·Paren·Ident·PkgScoped 를
> 이미 풀고 있었는데 **소비자가 하나뿐**이었다(문자열 **동등비교** 전용 — 아무도 **값**을 안 물었다).
> 고침 = 그 도메인에 `Ternary`(§11.4.11 · 조건은 정수 도메인과 **같은 철자** `const_int_selfdet` ·
> **양 arm 이 다 문자열이어야** 한다 = fail-closed)와 `Concat`(§11.4.12) 두 arm 을 더하고, 파라미터
> **값** 소비자 일곱을 약한 쌍둥이(`param_str_literal`)에서 그리로 라우팅. `systask.rs` 의 진단
> 가드는 *"리터럴인가"* 라는 **다른 질문**이라 그대로 뒀다. 21칸 3-오라클 **FIXED 17 · ok→wrong 0**,
> 11설계 PRE/POST **바이트 동일**. ⚠️⚠️ 첫 구현이 **loud→silent-wrong 을 만들었다**: 이 도메인의
> 값은 **따옴표를 포함한 raw** 라 `{"RE","D"}` 가 `RE""D`(정수로는 1092756034)가 됐다 ⇒ 내용만
> 이어 붙이고 **다시 따옴표를 씌운다**. 잔여 = **package 스코프**(아래 ⑨).

> ~~**② 정수 상수 도메인의 replication COUNT 가 리터럴 전용이다**~~ ✅ **RESOLVED — §4.5.382**
> (2026-08-25 · 2-오라클 · format 29 불변 · 상세=ARCHIVE).
>
> ⭐⭐ **§4.5.371 이 지어서·재서·되돌리며 기록한 BLOCKING 셋이, 전부 같은 한 가지로 답해졌다** —
> count 를 **별도 evaluator** 가 아니라 **주변 fold 가 이미 쓰는 그 name resolver** 로 접는 것.
> ⓐ *"폭 쌍둥이를 같은 순간에 넓혀야 한다"* — 넓힐 쌍둥이가 없다. 폭 소비자(`const_placement_wide`)와
> 값 소비자가 **같은 `fold_self_bits`** 를 부르므로 한 번의 수정이 둘 다 움직인다.
> ⓑ *"상수함수 지역변수를 잡거나, 지나쳐 모듈 파라미터를 잡는다"* — resolver 에 `is_count` 플래그를
> 하나 달아 **count 위치에서는 env 에 이름이 있으면 그 자리에서 declines** 한다(지나치지도, 잡지도
> 않는다). 실측으로 확인: `int n = 2; {n{4'hA}}` 는 vita E3009 · iverilog *"a reference to a net or
> variable (`n') is not allowed in a constant expression"*.
> ⓒ *"`const_eval_in_scope` 가 깊이를 0으로 재시작해 스택 오버플로"* — 이 walk 는 evaluator 를 아예
> 부르지 않는다. 받은 AST 위를 재귀할 뿐이다.
>
> ⚠️ **verilog-axi 는 아직 안 돈다** — 하지만 막는 자리가 **두 번 이동했다**: `S_THREADS`(count) →
> `$clog2(M_ISSUE[n*32 +: 32]+1)`(per-port 벡터 슬라이스 · 같은 슬라이스에서 해소) →
> 지금은 **`calcBaseAddrs(…)`**, i64 보다 넓은 누산기를 가진 **상수 함수**(= §4.5.379 가 *"비싼 절반"*
> 이라고 재배치한 그 항목). 코퍼스 핀은 새 거절 문구로 갱신했다.

> **③ 잔여 = 조건부·반복 평가 위치, 그리고 `$feof` 가 살아남는 문장** (§4.5.374 가 나머지를
> 열었다). 직접-rhs 제한 자체는 사라졌다 — `hoist/special.rs` 가 호출을 temp 로 끌어내
> **NBA rhs · `if` 조건 · `case` scrutinee · `repeat` count · 시스템/유저 태스크 인자 · 식 중첩 ·
> lvalue 인덱스** 를 전부 연다(darkriscv 전체 SoC 가 이걸로 돌고 다이제스트가 오라클과 일치).
> 아직 loud 인 것과 그 선행조건:
>
> ⓐ **`&&`/`||` 우항 · `?:` arm** — §11.4.7/§11.4.11 상 건너뛸 수 있는 평가라 무조건 hoist 하면
> 없어야 할 파일 읽기가 일어난다. `hoist/general.rs` 는 **guard block** 으로 처리하는데,
> 가드된 fd 읽기는 이 슬라이스가 잰 것보다 큰 주장이다 ⇒ 선행조건 = `guarded_hoist` 를 이 계열에
> 적용하고 그 아래에서 fd 상태 순서를 증명하는 것.
> ⓑ **`while`/`for` 조건** — 반복마다 재평가돼야 하는데 한 번 hoist 는 한 번만 읽는다.
> 선행조건 = 루프 본문 안으로 들어가는 재작성(`lower_shortcircuit_cond` 가 조건 operand 를 자기
> 블록의 전체 식으로 만드는 것과 같은 모양).
> ⓒ ⭐⭐ **`$feof` 가 살아남는 문장 전부** — `$feof(fd)` 는 인자에 대해서만 pure 하고 **파일
> 위치를 읽는다**. hoist 는 변이를 그 앞으로 옮기므로 `x = $feof(fd)*10 + $fgetc(fd)` 가 EOF
> 근처에서 갈린다(vita 9 / iverilog −1 · **파일 중간에서는 일치**해서 프로브가 못 잡는다).
> 지금은 좌우를 안 가리고 통째로 거절한다 — *"소스에서 이 `$feof` 가 그 호출보다 먼저
> 평가되는가"* 를 답하려면 `general.rs` 의 `order_walk` 급 순서 판정기가 필요하고, arm 마다
> 순서 규약이 달라(`assign_seq` 는 rhs→인덱스, `Case` 는 scrutinee→라벨, 태스크는 인자열)
> 방어 가능한 단일 술어가 없다 ⇒ **선행조건 = 순서 판정기**.
> ⓓ **fail-closed 로 유예된 것** — 별칭을 이름으로 못 붙이는 읽기(self-계층 `m.a` · `p::v` ·
> `Shape::NoHoist` 자식)가 있고 그 문장의 호출이 ref 를 쓰면 거절한다. 선행조건 = 루트 대신
> **net 동일성**으로 겹침을 판정하는 것.
>
> ⚠️ `$sformatf`(문자열 temp + `sformatf_expr_ok` 의 측정된 degenerate-eval 함정) ·
> 시드 `$random`/`$dist_*`(비균일 형제의 **선재 vita-iverilog 발산** — 새 위치로 옮기면 틀린 값이
> 늘어난다) · `$cast`(temp 타입이 목적지를 따른다)는 이 계열에 **의도적으로 안 들어갔다**.

> **④ A hierarchical WHOLE unpacked array as a `$readmem*`/`$writemem*` argument** —
> ✅ **RESOLVED (§4.5.376)** (2-oracle · serv, picorv32, ibex · format 29 unchanged).
>
> ⚠️ **This line was wrong twice, in opposite directions, and both errors are worth keeping.**
>
> **The first wrong version** claimed `dut.ram.mem[i] = …` was `E3009`. Every ELEMENT shape
> works and has since June — a census of eleven (element read/write, variable index,
> part-select, bit-select, multi-dim, non-blocking, inside a task) found no failure. Three
> slices closed it: **N3.1** (`95cc674`), its **multi-dim follow-on** (`7d2f9b4`), and
> **HIER-REST track 9**. What was actually refused is the WHOLE array.
>
> ⚠️⚠️ **The second wrong version was the PREREQUISITE that reverted it.** §4.5.375 built
> this, matched forty-odd shapes, then reverted on the claim that *"vita runs a PARENT's
> `initial` before its child's while **both oracles** run the child's first"*, which would
> make a RAM that loads its own memory overwrite the testbench's load. §4.5.376 re-measured
> the exact design that claim cites:
>
> | | `$readmemh` child-vs-parent competition |
> |---|---|
> | iverilog | `aa bb cc dd` (child's `initial` first) |
> | **verilator** | **`01 02 03 04`** (parent's first) |
> | vita | `01 02 03 04` — **== verilator** |
>
> Verified not to be a dropped write: with the child's competing load removed, verilator
> honours the parent's hierarchical `$readmemh` (`aa bb cc dd`), as does a plain
> `u1.s = 8'hAA`. So the write-vs-write case is an **ORACLE SPLIT** — IEEE 1800 §4.7 makes
> `initial` execution order explicitly nondeterministic and the two oracles use that freedom
> in opposite directions — not a two-oracle silent-wrong. Same ruling as §4.5.372's
> cont-assign order, where verilator also sided with vita.
>
> ⭐ **The demand it feared is absent from every testbench that motivated the feature.**
> serv passes `.memfile("src/sw/blinky.hex")` and never sets `+firmware=`, so
> `servant_sim.v:20` never fires; picorv32's `wb_ram` is instantiated without `.memfile`;
> picorv32's `axi4_memory` has no load of its own. Not one has a child that loads the same
> array. ⚠️⚠️ And the serv digest the revert cited (`…523a` vs `…543a`) **cannot have come
> from this construct**: serv does not elaborate with or without it — PRE and POST both stop
> at the same three §3 ⑦ `generate-if condition is not a constant` errors, byte-identical.
>
> **The implementation is the one §4.5.375 described**, and it is small: `expr_array_view`
> already resolves dotted paths, a cross-instance name simply has no net yet (the child's
> nets are created in pass 8, after the parent body lowers in pass 7), so it defers — and the
> deferred placeholder is ALREADY `Signal { net: POISON_NET, word: None }`, the exact shape
> the local path builds by hand. Only the read guard stood in the way, asking *"does this
> have a plain readable value?"*, which is the right question for `x = dut.mem;` and the
> wrong one for a task that wants the array rather than a value. The exemption is
> consumer-scoped — the `$readmem*`/`$writemem*` MEMORY POSITION only (§21.4's arg 1), keyed
> on the eid the call registers — so `x = dut.mem;` stays loud, and events and dynamic
> handles stay loud in every position because they have no array to hand over either.
> `$readmem*` also denies a const array-parameter target at RESOLVE time (the local arm's
> twin, restricted to the family that WRITES, so `$writememh(f, dut.P)` still passes).
>
> **Measured**: 18 cells — 8 positive shapes (depth 1/2, `$readmemb`, start/end addresses,
> `$writememh` round-trip, filename from a `reg [1023:0]`, parenthesised, generate-scoped) all
> matching iverilog; 3 edge shapes (interface member unchanged, time-ordered overwrite,
> UPWARD reference `tb.mem` from a child); 7 negatives holding loud or unchanged.
> `$writememh` output file **byte-identical** to iverilog's.
>
> ⚠️ **What did NOT ship, and is a separate row**: a parent `initial` READING a child net at
> t0 gives X in vita where both oracles give the value (10 cells, no oracle disagreement).
> That is a real §2 silent-wrong — but it is **pre-existing, independent of `$readmem`, and
> exercised by zero of the ten corpus designs**. Recorded as **§2 row 7**; it does not gate
> this construct, and the write-vs-write cell above is a split rather than a defect.
>
> ⚠️ Two more, measured while there, both pre-existing and neither this item's business:
> `$readmemh` into a `wire` array is accepted at local/hierarchical parity (iverilog refuses,
> **verilator accepts** ⇒ oracle split); and — ⭐ **corrected on re-review** — a clocking
> INPUT is writable through `$readmem*` **today, on the shared path**:
> `$readmemh("f.hex", c.cb.mem)` writes the hold net at exit 0, while `c.cb.s = 8'hAA` is
> correctly `E3009`. §14.3 says a clocking input is read-only (hand-IEEE — iverilog 13 cannot
> parse clocking blocks at all). The first reading of this was that the hole was merely
> LATENT, on the grounds that a clocking input of an unpacked array gets a SCALAR hold net;
> that is why the exemption arm cannot reach it, but it is **not** why the hole is unreached —
> the reachable half runs through the ordinary resolution. A guard on the exemption alone
> would close the unreachable half and leave the live one open, so it belongs on the shared
> path, as its own item (**§2 row 8**).

> **⑤ struct 타입 localparam 의 named assignment pattern** (**오라클 없음 → hand-IEEE**). ibex 의
> `localparam exc_cause_t E = '{irq_ext: 1'b1, lower_cause: 5'd3};` 를 vita 는 E2002 로 거절하는데,
> **iverilog 13 도 못 읽는다**(같은 줄에서 syntax error, positional 로 바꾸면 `net_scope.cc:449`
> assertion 으로 **abort**). 그래서 ibex 는 코퍼스에서 빠졌다(계약 ② = 오라클 없는 워크로드는
> 안 받는다). ⚠️ 다만 [[no-oracle-not-a-defer-reason]] — *"오라클이 없다"* 는 미루는 이유가 아니라
> **LRM 에서 hand-IEEE 로 지으라**는 뜻이다. 현대 SV 코어(ibex·OpenTitan 계열)의 입구다.
>
> **⑥ (사용성) auto-top 이 미인스턴스화 루트를 전부 잡는다** — ✅ **RESOLVED (§4.5.378)**,
> 다만 **큐가 지목한 그 원인 때문이 아니다**.
>
> ⚠️ **census 가 진단을 반박했다.** 줄은 *"미인스턴스화 루트를 전부 잡는 것"* 을 결함으로 봤는데,
> 그건 **IEEE 1364·iverilog 가 하는 그대로**다 — 실측: 독립 모듈 둘이면 iverilog 도 **둘 다**
> `initial` 을 돌리고(`VAL=A|VAL=B`), 공유 서브모듈의 오류도 **루트마다 한 번씩** 낸다
> (`in libtop.i` **그리고** `in tb.i`). vita 는 거기에 더해 `W-ELAB-AUTOTOP-AMBIGUOUS` 로 루트
> 이름까지 알려 준다(iverilog 는 아무 말도 없다) ⇒ **루트 선택은 손대지 않았다.**
>
> ⭐ **진짜 결함은 다른 것이었다**: serv 의 경고 21개 중 **19개가 *"output port left unconnected"***
> 인데, 그건 **루트라면 반드시 성립하는 조건**이다(top 의 포트는 정의상 어디에도 안 붙는다).
> ⚠️ **auto-top 과 무관**하다 — 포트를 가진 top 을 `--top` 으로 **하나만 핀해도** 똑같이 경고한다.
> serv 에서 `--top tb` 가 조용했던 건 그 테스트벤치가 **마침 포트가 없어서**였다. 두 오라클 다
> 침묵한다(iverilog 는 `-Wall` 로도).
> ⇒ 고침 = **루트에서만** 그 경고를 끈다(`wire_ports(…, is_root)`). ⚠️ **포트 바인딩은 대용물이
> 될 수 없다** — 자식을 `dut u();` 로 써도 같은 빈 바인딩으로 들어오는데 거기선 그 경고가 **진짜
> 정보**다 ⇒ 판별자는 호출자가 아는 `parent_inst.is_none()`.
> **측정**: serv auto-top **경고 21 → 2**(남은 둘 = auto-top 모호성 + timescale, 둘 다 실행 가능한
> 정보) · 오류는 6 그대로(루트 2 × 실제 오류 3 = iverilog 와 같은 중복) · `--top tb` 는 3/1 불변.

> ~~**⑦ 상수 도메인에 리덕션 연산자(`&`·`~&`·`|`·`~|`·`^`·`~^`)가 없다**~~ ✅ **RESOLVED —
> §4.5.382**(2026-08-25 · 2-오라클 · format 29 불변 · **serv PROMOTED · 코퍼스 7/10 → 8/10** ·
> 상세=ARCHIVE).
>
> ⭐⭐ **§4.5.373 이 기록한 두-단계 선행조건이 둘 다 `param_range` 하나로 답해졌다.**
> ⓐ *"폭이 declared provenance 여야 한다"* — `param_decl_range` 는 **선언된 range / 타입 /
> sized 리터럴에서만** 기록하고, 값-추론은 아예 들어오지 않는다(`param_meta` 와 다른 점이 이것뿐이고
> 그 하나가 전부다). ⓑ *"그 폭에서 값이 canonical 이어야 한다"* — 선언 range 가 있으면 바인딩에서
> `coerce_param_value_with` 가 이미 그 폭으로 자른다. **§4.5.373 의 반례 그대로 실측**:
> `parameter A = 4'h1; localparam logic [3:0] W = A<<4;` 는 vita 0 · iverilog 0.
>
> ⚠️ 무타입 파라미터(`localparam W = A<<4;`)는 여전히 **loud** 다 — 폭이 값에서 추론되므로 위 두
> 조건 중 어느 것도 성립하지 않는다. 그것이 §4.5.373 이 잰 바로 그 셀이고, 회귀 테스트로 **거절**
> 을 핀했다(`wide_const_domain::an_inferred_width_never_supplies_a_reduction`).
>
> ⚠️ *"같은 벽을 세 문으로 쳤다"* 던 셋(§4.5.371 의 select 바운드 · concat 폭 · 리덕션)은 **셋 다**
> 이 슬라이스에서 열렸다 — 벽이 하나였다는 §4.5.373 의 판단이 맞았고, 그 하나가 폭의 출처였다.

> **⑧ 함수 본문의 `$finish`/`$stop`** (2-오라클 · **verilog-ethernet 을 막는 것** ·
> ⚠️⚠️ **§4.5.372 가 지어서·재서·되돌렸다 — 선행조건이 기록됐으니 다음 시도는 여기서 시작하라**).
> 진단은 *"함수 본문이 **system task call** 을 쓴다"* 고 말하는데 10칸 census 는 훨씬 좁다 —
> `$display`·`$write`·`$error`·`$warning`·`$fatal`·`$fflush` 와 시스템 **함수** 전부가 이미 돌고
> **`$finish`·`$stop` 딱 둘**만 거절된다. `lfsr_mask` 의 `$finish` 는 **실행되지 않는 방어 가지**다.
>
> ⭐ 넣으면 verilog-ethernet 이 **elaborate 를 통과한다**(실행은 프레임 레짐 탓에 매우 느리다).
> 그런데 넣으려면 **본문이 중간에 멈춰야** 하고, 거기서 막힌다:
> ⓐ 프레임 본문이 중간에 멈추면 **반환값이 정의되지 않는다** — 그리고 vita·iverilog·verilator 가
> **셋 다 다른 답**을 낸다(iverilog 는 대입 자체를 안 하고, verilator 는 본문을 끝까지 돌리고,
> vita 는 중단하면서 부분값/x 를 **커밋한다**). `r = f(7)` 의 lvalue 에 무엇을 쓸지가 미정이므로
> 어느 것을 골라도 loud 를 silent 와 맞바꾼다 ·
> ⓑ 쓰기-다음-검사 순서가 `Op::ends_statement` 에 박혀 있고, 그 순서는 `mem[f(i)] = 1` 의 E4002 를
> 지키려고 **의도적으로** 그렇게 돼 있다(`backend.rs:505-524`) ⇒ `$finish` 레인을 그 레인과
> **분리**해야 한다(`call_end.is_some()` 이 판별자가 된다) ·
> ⓒ 라우팅이 셋이다 — `elaborate/frames_classify.rs` · `sim-engine/native/frames.rs` ·
> `state/frame_eval.rs` — 하나만 가르치면 native 가 vm 으로 폴백하며 *"실행기가 드롭한다"* 는
> **거짓 진단**을 낸다 · ⓓ `Step::Fatal` 소비부 넷이 전부 `Error` 를 가정하므로 이유를 따로 실어야
> 한다(`call_end` + `latched_end()`, 지어서 측정했고 되돌린 패치에 있다).
>
> ✅ **분리해서 실은 절반(§4.5.372)**: `$fatal` 이 `$display`/`$write` 의 **인자에서** 걸리면 그
> 출력이 나가면 안 된다(§20.10) — 경계가 출력 뒤라 vita 가 iverilog 보다 한 줄 더 찍던
> **pre-existing silent-wrong**. 잔여 = `$fdisplay`/`$fwrite`·`$strobe`/`$monitor` 는 같은 검사가
> 없어 여전히 한 문장 늦다(⚠️ 그 값이 곧 ⓐ 의 질문이라 지금 고치면 silent↔silent 맞바꿈이 된다).

> **⑨ 파라미터 선언 fold 의 네 번째 복사본(`package.rs`)이 string/real 을 라우팅 안 한다** —
> ✅ **RESOLVED (§4.5.377)** (2-오라클 · format 29 불변).
>
> ⚠️ **census 가 큐 줄을 또 넓혔다.** 줄은 *삼항*(`parameter SI = (S=="AUTO") ? "RED" : S;`)을
> 갭으로 지목했지만, 실제로는 **패키지 안의 string/real 이 전부** loud 였다 — `parameter S = "RED";`
> 라는 **바레 리터럴**까지. fold 가 `const_eval_in_scope` **하나만** 묻는데 그건 정수 전용이라
> 리터럴이 **착지할 도메인이 없었다**. 19칸 census: string 9 · real 4 가 loud, 정수·모듈스코프
> 컨트롤은 불변, 그리고 **§2 의 package-real silent-wrong 이 같은 뿌리**(값은 넘어가고 도메인이
> 안 넘어가 `pk::PR / 2` 가 1.0) ⇒ **§2·§3 두 줄이 한 수정으로 닫혔다**.
>
> ⭐ **참조 구현은 `generate.rs`**, 그리고 그게 옳은 이유는 형태를 정하는 성질을 공유하기 때문이다:
> generate 스코프 상수도 패키지 상수도 **override 채널이 없다** ⇒ 선언 기본값이 언제나 바인딩이고
> 폭을 선언에서 가져와도 된다. arm 순서는 real → string → 정수이고 앞 둘은 **return 이 아니라
> fall-through**(§4.5.364 의 그 순서). ⚠️ 큐가 권한 *"`bind_one_param` 이 흡수"* 는 하지 **않았다** —
> `bind_one_param` 은 override 채널을 다루는 쪽이라 성질이 다르다. 대신 **generate 와 package 가
> 같은 케이스**라는 것이 이 슬라이스의 판정이다.
>
> ⚠️⚠️ **내 수정이 만든 회귀 둘, 내 soundness 렌즈가 잡았다** — 둘 다 *"모든 패키지 파라미터는
> `pkg_consts` 에 있다"* 는 **불변식을 내가 깨뜨린** 것이다: ⓐ 이름공간 중복 검사가 i64 `consts` 를
> 물어서 `parameter S = "RED"; int S;` 가 **exit 0 으로 통과**(두 오라클 거절 · 정수 twin 은 loud
> 유지) ⇒ 검사가 묻는 것이 **NAME SPACE** 이므로 인자를 이름 집합으로 **재타입**했다 ⓑ
> `nonconst_bound_reason` 의 `pkg::` arm 이 *"알 수 없는 `pkg::name` 은 기존의 조용한 unfoldable
> 동작을 유지한다"* 고 **자기 주석에 예고한 구멍**으로 `logic [P::S-1:0] v;` 가 **조용히 1비트**(두
> 오라클 5391684) ⇒ **모듈 스코프 쌍둥이가 loud** 이므로 branch parity 로 복구(값 도메인에 escalation
> 을 넣지 않는 자리 = §4.5.370 의 그 함정 회피).
>
> ⭐ **곁수확**: `const_str.rs` 의 `PkgScoped` arm 은 `str_param_raw` 를 **`"P::S"` 라는 철자**로
> 찾고 있었는데 **그 키를 쓰는 생산자가 없다**(fold 는 `$pkg$P.S`, 모듈 스코프는 `module.name`)
> ⇒ 지원되는 것처럼 읽히는 **도달 불가 코드**였다(§4.5.370 의 *"소비자가 하나뿐인 참조 구현"* 과 같은 모양).
> 고치니 `generate if (P::S == "AUTO")` 가 산다.
>
> ⚠️ **열지 않은 한 형태 = 와일드카드 import**(`import pk::*;` 뒤의 바레 이름). **loud 유지**(fold 는
> 성공하고 import 바인딩만 없다). `apply_import_consts` 가 §26.8 wildcard-origin·모호성 부기와 함께
> `params`(i64)로 재바인딩하는데, string/real 사이드맵에 같은 대우를 주는 것은 **라우팅이 아니라 배관**
> 이고 호출부가 둘이라 별도 항목이다. 핀 = `string_const_domain.rs` · `real_params.rs`.
> ⚠️ 또 하나 = **real 이 generate-if 조건**(`generate if (P::R > 1.0)`)은 loud 유지 —
> `const_real.rs` 에 `PkgScoped` arm 이 아예 없다(조용하지 않다).

> 🆕 **지연 멀티드라이버의 wire 해석**(§4.5.364 곁수확 · **2-오라클** · E3001). `assign #(D) bus = en ? d : 1'bz;`
> 를 둘 이상 겹쳐 쓰는 tri-state 버스 관용구가 **exit 1** 로 거절된다 — `check_whole_net_multidriver` 가
> *"드라이버 중 하나라도 delayed 면 4-state wire 해석 대상이 아니다"* 로 엔진 자격(`md_nets`)을 그대로 비추기
> 때문이다. ⚠️ **이 행은 §4.5.364 가 만든 것이 아니라 드러낸 것이다**: 그전엔 파라미터 지연이 조용히 **버려져**
> 같은 설계가 비-지연 멀티드라이버로 통과하고 있었다(1 ns 격자 실측: `t=11 bus=1` / iverilog `bus=z`) ⇒
> silent-wrong → honest-loud = **사다리 상승**. ⚠️ 적대 렌즈가 *"PRE 는 맞았다"* 로 BLOCKING 을 냈다가
> **자기 프로브의 10 ns 격자가 버려진 2 ns 지연을 못 본 것**임을 확인하고 철회했다 — 세 가지 구성으로
> 반례를 시도했으나 지연 드라이버는 `[0,D)` 동안 x 를 쥐고 PRE 엔 그 구간이 아예 없어 **구조적으로 불가능**.
> 승격 조건 = 엔진의 `md_nets` 가 delayed 드라이버를 해석하는 것.

> **§4.5.350 follow-on 3건**(전부 적대 2렌즈 실측 · **verilator 오라클 ✓** · iverilog 는 셋 다 거부해 부분 오라클):
> ① **dyn/queue/assoc 원소의 음수 packed bound** — `logic [-3:0] q[$]` 는 W3056 클램프 유지(그 원소 net 은
> `elaborate_netvar_decl_inner` 의 early-`continue` 경로에서 만들어져 선언 사이드맵에 안 닿는다 · verilator
> `q[0][-3]`=1). ② **음수 bound net 의 PART select** — 읽기가 과잉 loud 다: 인덱스형 `q[-3 +: 2]`·`q[-1 -: 2]` 는
> **이미 정확**하고(오라클 일치) 막힌 것은 `[msb:lsb]` 형태뿐인데 그 바운드 fold 가 unsigned 라서다 —
> §4.5.350 이 만든 `const_bound_signed` 가 그 도구다. 쓰기는 비대칭(`x[-3:-2] = …` 은 조용히 **정확**하고
> `x[-1:0] = …` 은 *"out of order"* 라는 **사실과 다른 방향 진단**으로 loud). ③ **packed 음수 dim 의 비트 선택**
> = §2 의 `dim_coord` 항목과 같은 뿌리.

> **✅ 외부 리포트 2건 — round-32(hash_top) · aes_top (2026-08-25 · §4.5.380).**
> 둘 다 `6c4be81` 기준이라 **HEAD(4 슬라이스 뒤)에서 전 항목 재현부터** 했다([[external-report-fresh-probe-triage]]).
> **해결 4 · 기록 6.**
>
> | 항목 | 판정 |
> |---|---|
> | **N32-1** `$bits(<식>)` 을 packed 선언 바운드로 → **조용히 1비트** | ✅ **FIXED** — silent-wrong. 값 fold(`bits_of_selfdet`: 리터럴·concat·replication, **leaf 는 전부 `bits_of_view`** = 이미 도는 `$bits(<이름>)` 과 같은 resolver) + **못 접는 형태는 loud**(`nonconst_bound_reason` 의 SysCall arm — `$bits(<미선언 이름>)` 조차 1비트 net 을 조용히 만들고 있었다). 포트 바운드라 **모듈 경계를 넘어** 8비트 actual 이 잘렸다 |
> | **N32-3** 거절 진단이 §4.5.374 가 **지운 규칙**을 인용 + 캐럿이 문장 머리 | ✅ **FIXED** — ⚠️ **같은 죽은 규칙을 인용하는 자리가 둘**이었다(`$fgetc` 계열 `(v9)` · `$value$plusargs` `(v7)`), 그리고 **이유가 서로 다르다**(평가 **횟수** vs ref **쓰기 순서**) ⇒ 문장도 둘. 캐럿을 **호출**로 |
> | **aes §4** `a[7:0][3:0]` 이 W2004 를 안 낸다 | ✅ **FIXED** — ⭐ iverilog 의 문구가 규칙을 정확히 준다(*"All but the final index in a chain of indices must be a single value, not a range"*)라 **파싱 시점에 판정 가능**. ⚠️ *"두 번째 select 면 경고"* 는 **틀린다** — `mem[i][3:0]`(인덱스 후 select)은 어디서나 합법이고 도처에 있다 ⇒ 판별자는 **직전 select 가 range 였는가** |
> | **aes §5** 진단 둘이 한 사건에 **모순** | ✅ **FIXED** — `128'hdead…` 는 상수인데 *"is not a constant; default kept"* 였고 실제로는 companion 검사가 **error** 를 낸다 ⇒ **wide 리터럴**만 새 문구로 분리. ⚠️ 진짜로 default 가 유지되는 경우(`#(.W(sig))`)는 **옛 문구가 참**이고 핀이 있다 |
> | **aes §1** 상수 도메인이 **값 2⁶³ 로 두 레인**, 혼합 거부 | 📌 **기록** — 리포트의 축 규명이 정확하다(폭이 아니라 값). 이 중 **reduction 부재는 §3 ⑦**(§4.5.373 이 지어서 되돌림), **wide `+`/비교·select 폭 상한**은 §2 의 *"파라미터 값이 주장 폭에서 canonical 하지 않다"* 벽과 같은 축 ⇒ **한 슬라이스 아님** |
> | **aes §2** 인라이너 판별자가 `automatic` 만이 아니다 | 📌 **기록 + CHANGELOG 정정** — 자체 실측 확인: plain 3/5 · `automatic`·`for`·`if`·`case` **전부 1/5** · `p::f()` 2/5. CHANGELOG 표가 *"one keyword"* 라 했는데 **거짓**이었다(그 표 자체가 08-18 정정본이다) ⇒ **8행 표로 재작성**. 인라이너 확장은 §3 |
> | **aes §3** `always_comb` 구동 변수의 선언 초기화자 무경고 | 📌 **기록** — ⚠️ **iverilog 도 통과시킨다**(실측) ⇒ 2-오라클 결함이 아니라 **lint 기회**(xrun·verilator 는 error). W2004 급 경고 한 줄이 요청 |
> | **R30-1** 빠진 패키지 → E2002 7줄, 패키지 이름 0줄 | 📌 **기록** — tf-port 자리의 `IDENT::IDENT` 는 스코프 타입일 수밖에 없으므로 파서가 타입으로 받고 elaborate 가 *"unknown package"* 를 말하면 7줄이 1줄이 된다. 파서 작업 |
> | ~~**R30-2** unpacked-array element LHS~~ | ✅ **RESOLVED 2026-08-26 — and the diagnosis was refuted twice.** The axis is a SIGNED CONSTANT operand in the RHS, not the LHS storage class: over 34 cells native was ahead on every 1-D array-element-LHS cell and 3.9× ahead on the report's own headline shape. See §5. |
> | **N32-2** replication count 의 **typed 철자**는 width twin 선행조건이 없다 | 📌 **기록(§3 ②)** — 좁힘 관찰: 폭이 선언에 있으므로 ⓐ 는 불필요, ⓑⓒ 는 남는다 |

> **✅ 외부 리포트 aes_top 2판 — 16항목 중 15 해결 (2026-08-07 · §4.5.313).** unpacked
> 배열 포트 · 선택적 import 의 패키지-스코프 해석 · `pkg::f()` 제약 완화 · 콤마 import ·
> 포트 연결의 사용자 함수 · `RPS'()` 폴딩 · string 파라미터 · 64비트 초과 파라미터 ·
> `-G`/`--param` 오버라이드 · E4002/W4029 분리 · `$sscanf` scanset · 선언 전 사용 loud ·
> W1018 부분 timescale. **남은 1건 = §3.1 DPI-C(영구 비목표)**.
>
> 그 과정에서 **리포트에 없던 silent-wrong 7건**과 **적대 2렌즈의 결함 17건**을 닫았고,
> 남은 잔여는 아래 두 줄이다:
>
> - **unpacked 배열 포트의 방향 불일치는 loud**(`[0:3]` ↔ `[3:0]`). IEEE 1800 §7.6 은
>   원소를 **위치로** 짝지으므로 flat-index 연결이 순서를 뒤집는다(vita 4 / iverilog 1 로
>   실측). 두 번째 대응 규칙을 짓는 대신 거절했다 — 구현하려면 위치↔인덱스 매핑을
>   `wire_array_port` 와 배열 대입 양쪽이 **한 철자**로 써야 한다.
> - **wide(>64비트) 파라미터의 OVERRIDE 는 loud**. 선언은 `wide_param_bits` 로 값을
>   지키지만 override 채널은 i64/string 뿐이라 `#(.K(128'h…))` 는 거절된다(값이 조용히
>   기본값으로 가지 않도록 loud 로 만든 결과). 캐리려면 `ResolvedOverride` 에 wide 슬롯이
>   필요하고, 자식의 선언 폭을 모르는 부모 스코프에서 접어야 하므로 `fill` 과 같은
>   "원문을 넘겨 자식 폭에서 재폴딩" 형태가 된다.

**⭐ 외부 리포트 aes_top 3판(2026-08-18 · §4.5.341)이 남긴 것 — 전부 "하류가 거부할 것을 미리 거부한다" 부류:**

> 3판은 §3.2(문자열 escape) · §3.4(거짓 W3056) · §3.8(코드 키) · §3.9(`--help`) · §3.10(위치) ·
> §3.11의 보고 절반을 §4.5.341이 닫았다. 아래는 **다른 클래스**라 분리한 잔여다.

- ✅✅ **3판 클리어 라운드 완료 (2026-08-18 ~ 08-19 · 순서 1~10 + P1 · 11 슬라이스).** 상세
  전문(클리어 표 + 항목별 기록)은 **[ROADMAP_ARCHIVE §4.5.342](ROADMAP_ARCHIVE.md)** 로 이관.
  결과 한 줄: §3.1(a)(b) `W2004` · §3.3 wide fold · §3.5-①③ `repeat` 가족 · §3.6-①② default
  clocking/disable iff · §3.7 INPUT string formal · P1 inner-NET shadow · **#9 staged `file:line`
  (format v28)** · **#10 런타임 severity `file:line`+인스턴스 (format v29)** — §3.11 만 **지어서
  재고 되돌렸다**(아래 잔여). 게이트 5,533 → **5,596 green** · 커밋 `b472b67`…`b868ff3`.

- **라운드가 남긴 잔여 (전부 loud 또는 경고 없음 = 안전 · 각자 독립 슬라이스):**
  - **§3.1(c) `always_comb` 구동 변수의 선언 초기화자** — verilator `MULTIDRIVEN` error · xrun
    `*E,MULAXX` · iverilog 는 실행(2:1). elaborate 층이고 감도 리스트 합성이 그 집합을 이미 안다.
    곁가지: `a[7:0][3:0]`(slice-of-slice)은 이름에서 시작해 **아직 경고하지 않는데** iverilog 는
    거부한다 — 별개 판별자 필요(doc-15 에 기록).
  - **§3.3 잔여** — wide `localparam` 의 **part-select fold**(`{A[127:64], 64'h0}` · iverilog 는
    접는다 · 인덱스 fold 가 필요한 별개 arm) · **wide 파라미터 OVERRIDE 는 여전히 loud**(2판 잔여 ·
    `ResolvedOverride` 채널).
  - **§3.7 잔여** — static task 의 `string` **output/inout** formal(INPUT 은 닫혔다 · 전용 loud
    메시지가 "INPUT 은 지원된다" 까지 말한다 · `automatic` 이 우회).
  - **§3.11 `function automatic` 인라인 — 선행조건이 바뀐 채 열려 있다.** *"비-재귀 automatic 은
    인라인과 의미상 동일"* 은 **측정으로 반증**(전 스위트 15 실패 · `$random` 이 두 번 — 인라인
    확장이 피연산자를 **두 번째로 이름 부른다**). 길 둘: ⓐ 인라인이 피연산자를 **한 번만** 이름
    부르게(또는 callee-purity 술어 — 그러면 인라인 경로의 핀된 결함 다수가 함께 사라진다) ·
    ⭐ⓑ **codegen 쪽을 연다**(`is_codegen_able` 의 `Terminator::Call` 거부 = §5 T1/T2) — 프레임
    경로를 유지한 채 리포터의 지표(`user_call_in_expr` 87%)가 오르므로 **사다리를 안 건드린다**.
    상세 = ARCHIVE §4.5.342 의 8번 항목.
  - **#9 잔여** — `velab -L`(worklib 병합) 경로는 여전히 위치가 없다: 병합이 여러 CU 를 한 unit 으로
    합치는데 각 CU 의 스팬이 자기 확장 버퍼(0부터)를 인덱싱해 **좌표공간이 겹친다**. 틀린 CU 의 맵으로
    풀면 틀린 file:line(없는 것보다 나쁘다) ⇒ `None` 유지 · 닫으려면 병합 시 스팬 오프셋 재작성(AST
    전체 walk). 실측: worklib 테스트 중 elaborate 진단 텍스트를 단언하는 것 0.
  - **#10 잔여** — sid 가 없는 런타임 진단군(RunRange/W4020/W4022/W4028 등)은 접근 **문장**을 아는
    키가 엔진에 없다. 넷 선언 지점을 표준 위치 슬롯에 찍으면 접근 지점으로 오독된다(없는 것보다
    나쁘다) ⇒ 위치 없음 유지 · §4.5.341 이 이미 배열 **이름**을 붙여 남은 값은 작다.

**§4.5.314 이 남긴 1건 (오라클 ✓ iverilog):**

- **`defparam` 이 INTERFACE 인스턴스에 안 닿는다** — `interface ifc; parameter D = 8; … endinterface` + `ifc a(); defparam a.D = 255;` 가 `W3056 … matched no instance` 를 내고 **기본값을 유지**한다(iverilog `d=ff`, vita `d=8`). PRE·POST 동일한 pre-existing 이고, 경고가 있으므로 silent 는 아니다. 원인 = `defparams` 소비가 `elaborate_instance` 에만 있고 `iface_inst.rs` 는 자기 `overrides` 만 본다 — 인터페이스 바인딩 루프가 §4.5.314 에서 정본 바인더를 쓰게 됐으므로 `defparams.remove(path)` 를 같은 자리에서 병합하면 된다.

**슬라이스 #8(§5.1-ae)이 남긴 1건 (오라클 ✓ iverilog):**

- **`$writemem*` 의 타깃이 서브루틴의 whole unpacked-array 로컬이면 E3009** — `task automatic t; reg [7:0] loc[0:1]; … $writememh("x.txt", loc);` 가 *"a whole unpacked-array formal has no value here"* 로 거부된다(iverilog 는 실행하고 파일을 쓴다). 두 백엔드 동일한 pre-existing honest-loud. ⚠️ **여는 슬라이스는 seam 도 함께 지어야 한다** — #8 의 `read_task_net` 은 이 거부를 **도달 불가 논거**로 삼아 아레나를 맨손으로 읽고(프레임·힙 넷은 그 store 가 소유하지 않으며 `assert_owns` 는 `debug_assert!` 다), 그 논거의 핀이 `writemem_targets_the_seam_cannot_own_are_refused_before_the_backend` 다.

**외부 round-28 이 남긴 4건 (§4.5.284 · 전부 실사용 ASIC 트리에서 실측된 사이트 · 오라클 ✓ iverilog):**

- **양 끝이 음수인 ASCENDING 팩트 범위가 폭 1 로 클램프**(`reg [-33:-2]` → `$bits` vita 1 / iverilog 32 · **loud**: `W3056`). 하강 쌍둥이 `[-2:-33]` 와 혼합 `[3:-2]` 는 정상이라 갭은 그 한 조합뿐. §4.5.308 differential 이 잔차 308행의 원인으로 실측했고, 그 행들은 폭이 틀려서지 정규화 때문이 아니다(`array_geom.rs` 의 `allow_neg_lsb` opt-in 경로).
- **`$value$plusargs` 의 `%0d` 등 폭-지정 스펙은 과잉거부**(§4.5.304 differential 기록): iverilog 는 `%0d` 를 받는다(값 5 기록), vita 는 E3009 loud. 스펙 파서가 폭 수식자를 벗기면 끝 — 단 `exec::plusargs::effect` 의 conv 문자 추출과 **한 철자**여야 한다(거부가 풀리면 `'0'` 이 %s 로 읽히는 함정이 정본 술어에 기록돼 있다).
- **static 함수의 로컬을 정의-대입 前에 읽으면 E3010**(§4.5.311 그라운딩 발굴·오라클 ✓ iverilog 는 X 로 실행): `function integer f(input integer x); integer s; begin s = s + x; f = s; end` 이 `undeclared net/variable top.s` 로 거부된다. 뿌리는 블록-로컬 flatten 모델(definite-assignment 를 요구)이고, **본문에 제어 흐름이 하나라도 있으면 프레임이 되어 정상 동작한다** — 즉 갭은 "직선 본문 + 읽고-쓰는 static 로컬" 조합뿐. (③층 S3a 가 이 비대칭에 걸려 static 슬랩 경로를 "도달 불가" 로 오판했다.)
- **프레임-로컬 배열의 범위 밖 읽기에 E4002 가 없다**(§4.5.311 differential 발굴·값은 iverilog 와 일치): 모듈 배열의 같은 접근은 E4002+exit 1 인데 프레임 로컬은 조용히 X 를 내고 exit 0. 프레임 로컬 배열은 **packed 슬롯**이라 array-word 개념이 없어서 진단이 구조적으로 안 붙는다 — elaborate 가 슬롯의 원래 기하를 남겨야 한다. vita 내부 비일관(모듈 vs 프레임)이 판정 근거다.
- **겹침 CA 의 E3001 일부는 iverilog 가 해상하는 형태**(§4.5.303 경계 실측): 같은-범위 part-select 쌍(`assign z8[3:0]=…` ×2)·delayed+plain 겹침을 iverilog 는 비트 단위 wire 해상으로 받는다(`zzzz0xx1`). vita 는 E3001 loud — 정직하나 미지원. 비트 단위 드라이버 맵(§2 ⓑ 와 같은 인프라)이 선결.
- **`specify … endspecify` 블록 수용** — `specparam` 은 §4.5.284 에서 모듈 레벨 상수로 받게 됐지만 **블록 자체는 아직 E2002** 다. 리포터의 우회는 이제 "블록을 지우되 `specparam` 은 키워드째 살린다"로 단순해졌으나, 벤더 모델(EFUSE·표준셀)이 `specify` 를 갖고 오는 것은 흔하므로 스크립트 없이 받는 것이 목표. **범위** = 블록 안의 `specparam` 을 모듈 아이템으로 hoist + 경로지연/타이밍체크는 버린다. ⚠️ **선행 결정 하나**: 버린다는 사실을 말할 창구가 없다 — 파서에 경고 채널이 없고(`hdl_parser::parse` 는 에러 채널만), 조용히 버리면 `$setup` 등이 **loud→silent 하강**이다. `ModuleItem` 에 마커 variant 를 두고 elaborate 가 `W3056` 을 내는 것이 현재 후보(`.vu` 해시 re-pin, format 불변).
- **이벤트 컨트롤의 계층 참조 실지원** — `always @(`TOP.a_uVDC.RTRIM_I)`(파운드리 ADC 모델). **읽기는 이미 동작**하므로 sensitivity 등록만의 문제다. §4.5.284 는 진단만 정확하게 했다(EVENT CONTROL 이라고 말하고 동작하는 우회를 제시). **범위** = `deferred_hier` 와 같은 형태의 지연 해소가 필요한데, 패치 대상이 `ir::Expr::Signal` 의 net 슬롯이 아니라 **`Process.sensitivity.edges[i].net`** 이고 그 프로세스는 아직 push 되지 않았다 → (proc_idx, edge_idx) 를 예약해 두고 instance 확정 후 패치하는 새 lane.
- **cross-process `disable`** — 파운드리 EFUSE 모델의 방어적 관용구. **리포터가 값싼 경로를 지목했다**: 대상 블록에 지연문이 없어 `disable` 실행 시점엔 이미 완료 상태이므로 **실질 no-op** 이다. 즉 *"현재 suspend 상태가 아닌 대상에 대한 `disable` 은 아무 일도 하지 않는다"* 만 구현해도 이 라이브러리가 통과한다 — 전체 cross-process 의미(suspend 된 프로세스 강제 종료)를 다 짓지 않아도 된다. ⚠️ **경계 주의**: "아무 일도 하지 않는다"가 **suspend 중인 대상까지 조용히 무시**하면 silent-wrong 이다. 대상이 활성이면 여전히 loud 여야 한다.
- **`E3010`/`E3009` 의 file:line 이 일관되지 않다** — 붙는 자리도 있고(`d_trunc.v:3:20`) 계층 경로만 나오는 자리도 있다. 다른 코드(`E2002` 등)는 `file:line:col` 이 정확히 붙어 대비된다. 리포터는 **모듈을 하나씩 elaborate 하는 이분탐색**으로 사이트를 찾아야 했다. §4.5.249 의 `diag::SpanResolver` 가 이미 있으므로 **앵커를 안 넘기는 호출 지점 전수**가 범위.


**~~frame-body validator over-scan~~ RESOLVED §4.5.172** (pre-existing false-REJECT · §4.5.171 적대 agent 발굴 · V5 무관): `classify_frame_body`의 linear `block_base..func_blocks.len()` 스캔이 **POST-PASS**(`resolve_frame_task_rejects`·전 func lower 후 subset task 검증)서 `func_blocks.len()`=**전체** 끝이라 **뒤에 정의된 func 블록까지 over-scan**→그들의 (합법적으로) out-of-frame인 output-formal write를 자기 것으로 오판→subset task를 E3009("assignment to a net outside the function")로 false-reject(iverilog는 accept). **fix = reachable-block CFG walk**: entry(`self.funcs[fid].entry`)서 자기 CFG 엣지(`Goto`/`Branch`/`Call`→`ret_bb`[callee entry 아님]/`Delay`·`Wait`→`resume`)만 순회→다른 func·dead 블록 미방문. **correct-or-loud**: 미방문 블록은 타 func이거나 dead(실행 안 됨)이므로 verdict는 false-reject만 DROP(silent-wrong 불가). `frame_task_pending` 튜플서 dead `base` 필드 제거. 적대 differential 全 MATCH(nested call·if/case/for·suspendable-mixed·5-task interleave·still-loud=모듈 net write). 신규 `frame_subset_overscan.rs`×4. 상세=ARCHIVE §4.5.172.

**round-20 이 남긴 2-세그먼트 호출의 정밀도 손실(오라클 ✓ · 의도된 사다리 교환):**

- **output-formal 호출 왼쪽의 `c.m()`·`pkg::f()` 가 loud** — `order_walk` 의 opacity 는 "이 호출이 mutated root 를 읽을 수 있나"를 물어야 하는데, 2-세그먼트 호출은 `callee_body_cannot_touch`(단일 세그먼트 전용)로 답할 수 없다. §4.5.276 은 **내장 컨테이너/문자열 메서드만** 적극 식별해 통과시키고 나머지는 loud 로 잘랐다. 그래서 **자기 멤버만 읽는 클래스 메서드**(`class C; int k; function int get(); return k;` → PRE `q=11` 정답) · **자식 인스턴스 함수**(`u.size()`) · **모듈 self-path 함수**(`t.size()`) · **covergroup 메서드**(`ci.get_coverage()`)가 정직한 loud 가 됐다. **`pkg::f()` 는 복구했다** — `inline_pkg_function` 이 self-contained·straight-line 본문만 받으므로 의무가 상류에서 이미 이행돼 있었다(§4.5.276 6라운드). **이것은 교환이다**: PRE 는 2-세그먼트 callee 본문이 root 를 읽으면 **무조건 조용히 틀렸고**(계층 self-path `t.o` 로 `q=12`, 정답 11 — iverilog 확인), 그 본문을 여기서 벳할 방법이 없었다. correct-support = 클래스 메서드/패키지 함수 본문을 해소하는 리졸버(§4.5.275 후속의 `c.m(x) + f(x)` 와 같은 전제) → 그 슬라이스에 묶는다.

**round-20 에서 실측한 dyn-array-formal 위치 갭 — 두 hoister 의 위치 집합 통일(오라클 ✓ iverilog 9/10):**

- **dyn-formal 호출이 못 가는 자리 = `&&`/`||` 우변 · 다른 호출의 인자 · select/lvalue 인덱스 · `case` scrutinee · `repeat` 카운트 · cast/replicate 피연산자.** 17-위치 행렬 실측(§4.5.276): 지원 7(직접 rhs·concat·비교·systask 인자·`?:` arm·NBA rhs·`return`) / loud 10(그중 **9건은 iverilog PASS = false-loud** · 나머지 1건 `case` scrutinee 는 iverilog 가 `string` case 를 거부해 무오라클 → hand-IEEE · **wrong 0**). 근인은 §3.1 과 **같은 뿌리**다 — dyn 용 좁은 hoister(`has_unhoistable_dyn_formal_call`/`hoist_dyn_formal_calls`)와 §4.5.275 의 범용 hoister(`shape()` 기반)가 **서로 다른 위치 집합**을 갖는다. correct-support = 범용 hoister 가 dyn-formal 호출도 흡수(leaf 에서 copy-out 대신 `__t = f(arr)` 를 `lower_stmt` 로 방출; dyn callee 는 input formal 만 가지므로 순수해서 eval-order 수리가 불필요). **착수 시 함정(§4.5.278 로 절반 완화)**: `hoist_stmt_general` 의 stand-down 은 이제 `in_frame_body()` 가 아니라 `frame_fn_lowering` 이다 — 프레임 **태스크** 본문은 hoist 하므로 그 절반의 함정은 사라졌다. 남은 것은 프레임 **함수** 본문뿐이고, 거기서는 `dyn-formal hoist 가 지원되는 것이 정상`(Family C·r17)이라 stand-down 을 statement 단위가 아니라 **call-kind 단위**로 쪼개야 한다. 그래서 별도 슬라이스로 뒀다.

**round-19 에서 loud 로 올린 silent-wrong 2건 — correct-support 후속(오라클 ✓):**

- ~~**frame body 안의 파일 읽기**(R19-X2)~~ **RESOLVED(§4.5.277·상세=ARCHIVE)** — 답은 `&self` 실행기에 파일 상태를 interior-mutable 로 여는 것이 아니라, **그 서브루틴을 `&mut` 실행기로 라우팅**하는 것이었다(분류기가 rhs 의 문장 수준 효과를 보게). **잔여** = 직접-rhs 가 아닌 자리의 파일 읽기 — `return $fgets(line, fd);` 는 정직한 loud(E3009, "…only as the direct rhs of a blocking assignment"). 라우팅 술어와 로워링이 **같은 형태**(직접 rhs)에 키잉되어 있기 때문이고, 넓히려면 둘을 함께 넓혀야 한다.
- **package 함수의 default 인자 스코프**(R19-X1 잔여). §4.5.274 의 `default_binding_matches_decl_scope` 는 `tf_decl_scope`(= func_table 을 채운 인스턴스 프리픽스)와 비교하는데, package 함수는 **import 한 모듈**의 프리픽스로 기록된다 — IEEE 는 package 스코프에서 평가한다. 가드는 개선이지 완결이 아니다. correct-support = 심볼별 선언 스코프 기록.

**string/heap:**

- ~~frame string LOCAL 대입(E3018→heap slot)~~ **RESOLVED**(§4.5.167·상세=ARCHIVE) — 잔여 loud**: static(non-`automatic`) task inline string local(`hoist_inline_task_locals`:18119)=Wire→E3018(inline 경로=frame-slot 아님·순진 String化시 str_bytes twin 미적용→silent 회귀 위험·별개 inline-string-storage 슬라이스). substr-actual `s[i]` · string part-select `s[i:j]`.
- dyn string 요소 method(`s[i].len()`) · whole-element read as value(`x=arr[i]`) · record array-of-record.
- queue/assoc의 string·real 요소 · string queue · block-local queue decl.

**함수/패키지/formal:**

- ~~control-flow pkg fn~~ · ~~pkg `function string`~~ = **둘 다 이미 동작**(2026-07-25 재그라운딩: `9` / `hi`). 잔여 = pkg TASK statement call뿐. §4.5.111.
- array formal 재전달(nested/recursion) · non-zero-LSB 원소 · 2-D/non-zero-base/signed/task array formal. §4.5.110 slice 밖.
- 음수-LSB 멤버 sub-select 정식 지원(§4.5.114). md-packed nested part-select WRITE `x[j][m:l]`/`arr[i][j][m:l]`=**§4.5.145 지원**(descending zero-lsb leaf·const packed idx); fail-closed 잔여(전부 loud·honest)=ascending/non-zero-lsb leaf·**genvar-index**(`x[g][m:l]`=const-fold 안 됨·over-reject)·const-OOB packed idx=silent no-op(read path 공유·값 무손상).
- method/ctor NAME-default class-scope 해석(§4.5.90) · G4 string-return frame call(§4.5.129).
- **round-14 리포트 잔여 (전부 loud·오라클=iverilog ✓·§4.5.167서 스코프)**:
  - ~~V3/V4 (task body 内 timing/suspend·최우선)~~ **RESOLVED**(§4.5.168/§4.5.169·상세=ARCHIVE) — 잔여 loud(§2 defer 후보·전부 correct-or-loud)**: 용·`reserve_frame_local_decl` 4-site 통일·엔진 frame_local=net-range라 자동 격리·적대 differential common 全 MATCH·automatic-array 격리/subroutine port는 vita>iverilog) · `wait(<frame-local>)`(level-wait re-eval window 미복원)·`repeat(<non-const>) @`(hidden counter SHARED net·동시활성화 오염)·NBA-to-frame-local(illegal)·`fork`/`disable fork`/`wait fork`-in-task(fork-family 미지원). 각 근본해결=per-activation repeat counter·in-frame fork machinery(별개 슬라이스). **§4.5.169 잔여 loud(safe gap)**: frame-local array의 multi-dim·non-zero-based·non-simple-element·whole-copy(`b=a`)·`foreach`·NBA-elem·`'{…}` init.
  - ~~V2A (task input dyn-array formal)~~ **RESOLVED**(§4.5.170·상세=ARCHIVE) — 잔여 loud(V5 의존)**: **automatic/recursive task**의 dyn-array formal은 frame formal이 scalar slot이라 handle-in-slot(V5) 필요→loud-defer. write-to-formal/sign-mismatch/non-bare/queue actual=correct-or-loud.
  - ~~V5 (frame dyn-array local `new[]`)~~ **RESOLVED**(§4.5.171·상세=ARCHIVE) — 잔여 loud(follow-on)**: (a) **FUNCTION** dyn-array local(`&self` 동기 `run_frame_call`/`run_task`가 `new[]`=`&mut` heap 실행 불가·구조적) (b) **recursion/concurrent**(per-activation heap stash 필요=진짜 격리) (c) multi-dim/packed/non-bit-vector element.
  - **잔여 loud(follow-on)**: (a) FUNCTION dyn-array local (b) recursion/concurrent(per-activation heap stash) (c) multi-dim/packed/non-bit-vector element.

**~~inline(static-task) 경로 `foreach`-on-dyn-formal~~ RESOLVED §4.5.174** (§4.5.170 gap·§4.5.173 발굴): static(non-`automatic`) task/function의 `input b[]` formal에 `foreach(b[i])`가 E3009("first/next/last/prev are only supported as the DIRECT rhs...")였음. **원인**: 파서가 `foreach(b[i])`를 `__st=b.first/next(__i)`로 uniform desugar→elaborator가 array KIND로 재작성하는데 dyn/queue dense-walk dispatch가 `dyn_handle`(=`lookup_net_scoped`)로 resolve→`dyn_subst` formal alias(read-only input dyn-array formal은 real net 아님) 못 봄→`b.first(__i)`가 generic method 경로로 falling through→E3009. **fix**: `foreach`는 배열 READ이므로 dispatch(dispatch gate + resolution 2 site)를 `dyn_handle_read`(alias 우선 consult)로 교체→formal이 module dyn-array와 **동일한 dense walk**로 라우팅. `dyn_subst`는 inline dyn-formal body 밖에서 empty→**모든 다른 array(fixed/module dyn/queue/assoc) byte-identical**. 적대 differential 全 MATCH(index+element·signed·two-foreach·module dyn/fixed/queue regression). **잔여 loud(별개 follow-on)**: FUNCTION+`foreach`(control flow라 R2-inline 불가→framed→framed function dyn formal 미지원)는 여전히 loud=function-frame dyn-formal 슬라이스 소관.
- ~~round-16 리포트(executor-bound)~~ **RESOLVED**(§4.5.194·상세=ARCHIVE) — **잔여 loud(follow-on·correct-or-loud)**: ① dyn local/formal을 쓰는 **recursion**(per-activation heap stash 필요) ② **string/real/class-handle** element의 dyn formal(input 포함 제외) ③ unwritten output formal = IEEE §13.5.2 empty copy-out(iverilog는 by-ref로 caller 값 유지=非준수).

**~~frame-body dyn-formal FUNCTION call~~ RESOLVED §4.5.195 (round-17 C)**: `s=sum(b)` in a TASK body(리포트 케이스)가 E3009였음. §4.5.194 RefCell dyn_heap이 `&self` executor의 snapshot-marker heap-copy를 가능케 함→elaborate gate `in_frame_body` 제거·`classify_frame_body`가 전용 `dyn_formal_marker_stmts` set의 marker만 허용(§7.10 whole-copy는 frame body서 loud 유지)·`run_frame_call`/`run_task`에 marker arm(`frame_dyn_copy_out`). TASK-body direct/buried·function-local-arg·module-process 全 iverilog MATCH·recursion=F4004 loud. **잔여 loud(correct-or-loud follow-on)**: FUNCTION이 자기 dyn-formal을 다른 함수로 **재전달**(`fsum(c)` where c=이 함수의 formal)—framed function formal이 heap-resident라 `dyn_array_actual_net`이 re-forward source로 못 resolve(기존 "array formal 재전달" 갭). `frame_body_dyn_formal_call.rs`×7.

**round-17 D-가족 (리포트 §5 minor 잔여 — §4.5.196서 tractable 4항목 RESOLVED):**

- ~~계층 태스크호출 `u1.tk(x)` (TASK·L)~~ **RESOLVED**(§4.5.196/§4.5.197/§4.5.198·상세=ARCHIVE) — 잔여 loud(correct-or-loud)**: output/inout/array/string formal[cross-boundary copy-out·별개]·STATIC(non-automatic) task hier-call[non-framed→force-frame이 local caller 회귀 위험=large·**정공법=frame⊂inline 동등화**·§4.5.198 step 1 착수]·~~module ARRAY-element write in framed task~~[**§4.5.198 RESOLVED**·`word.is_none()` 게이트 제거→suspendable `&mut` array write]·nested-in-frame-body hier enable[`task_calls_func` target transitivity]. 신규 `hier_task_call.rs`×14·`frame_task_module_write.rs`×6·기존 `frame_subset_overscan.rs` 1 flip.

- ~~frame↔inline parity · 배열 formal shape · hier-task 계열~~ **RESOLVED**(§4.5.198~208·상세=ARCHIVE) — frame ⊇ inline 완성, 배열 formal은 input/output/inout × 1-D/multi-dim × zero-based-any-direction + non-zero-base-ascending × automatic/static/function 전부 커버. **잔여 loud**: ① non-zero-base **descending** 배열 formal ② **hier-task OUTPUT/INOUT 배열 formal**(deferred copy-out·hard) ③ frame-formal 배열을 nested hier로 forward ④ generate-block 内 hier task call(pre-scan이 top-level proc block만 봄) ⑤ hier-task의 string formal.
- **잔여 follow-on (전부 loud·correct-or-loud)**: **function이 자기 dyn-formal 재전달**(`return sum(c)`·framed callee·risky: mutual-recursion soundness hole→frame-route 시 신중 guard 필요·`nested_in_frame_body_loud` 유지) · **block-local automatic lifetime**(loop-body `automatic int j=k*10` 등·per-activation storage 필요=deep block-local-flatten 인프라·iverilog도 "Overriding default variable lifetime" 거부) · **chained-method** `q.min()[0]`(vague·iverilog syntax-error·no-oracle). **이미 동작(vita>iverilog)**: output-fn-in-`while`(D4)·task OUTPUT array(D5b·§4.5.193).

**소형 큐:**

- **태스크/함수 지역 `localparam` 이 파서에서 거부된다**(pre-existing · PRE==POST · 오라클 ✓ iverilog · §3.5-① 슬라이스가 발굴): `task automatic t; localparam int K = 3; …` 가 `E2002 expected statement, found keyword 'localparam'`. IEEE 1800 §6.20 은 서브루틴 본문의 상수 선언을 허용한다. 우회는 모듈 스코프로 올리는 것이고, 그 우회가 가능하므로 소형.


- **`$typename` 의 enum / packed struct 렌더**(§4.5.239 발굴·무오라클): base 타입으로 나온다(`logic[1:0]`·`logic[3:0]`; IEEE §20.6.1 은 `enum{...}`·`struct packed{...}`). 타입 이름 렌더링 한정이라 값 영향 없음 — 현행은 `typename_pins.rs` 가 핀.

- **`$value$plusargs` expression 문맥 = 소형 후보 아님(§4.5.240 정정)**: 관용적 배치 **둘 다 이미 동작**한다 — `ok = $value$plusargs(…)` 와 **`if ($value$plusargs(…))`**(iverilog 일치, §4.5.240 서 핀). 남은 loud 는 `$display("%0d", $value$plusargs(…))` 류뿐이고, 이는 **side-effect sysfunc 패밀리 전체의 설계**다(seeded `$random`·`$fopen`·`$sformatf`·fd-advancing 파일읽기 — 전부 single-eval 보장을 위해 statement-form 으로 lower). 임의 expression 위치는 desugar 할 statement 가 없어 loud 가 정답이며, 넓히려면 **패밀리 전체의 desugar 확장**이 선행(소형 아님).
- **파일 위치 함수군 + `$sscanf` = honest-loud**(§4.5.238 실측): `$ftell`/`$sscanf` 등은 E3009 "unsupported system function in expression", `$fseek` 는 **W3056 warn+skip**. iverilog 는 전부 동작(`A=6 B=0 C=6 D=0`, `$sscanf`→`2 12 34`). **silent-wrong 아님** — ② loud→supported 후보. **스캔 소진**(§4.5.239): `$typename` 은 iverilog 미구현인데 vita 정확 → 핀. `%u`/`%z` 는 양쪽 다 문서화된 선택(vita 무출력·iverilog raw 바이트), `%l` 은 cosmetic.

- **`%p` — CLOSED for every aggregate vita can declare (V34-5, 2026-08-26)**; what is left is a DECLARATION gap, not a render gap. Shipped, verilator-5.050-matched: fixed-size unpacked array (incl. multi-dim), dynamic array, queue, associative array (integer and string keys), packed struct, plain integral, `%0p`, and a `string` variable (`"hi"`, the old residual). ⚠️ **iverilog 13 does not implement `%p` at all** — it warns `unknown format`, prints `<%p>` literally, and refuses an aggregate argument (`does not support argument type (vpiMemory)`) — so verilator is the ONLY oracle here and every format above is its measured output. RESIDUAL: ⓐ an UNPACKED STRUCT and a fixed array of `string` are E3010 at the DECLARATION (`u_t u;` / `string sa[2];` are not declarable), so `%p` has no net to render — re-filed under those features, not under `%p`; ⓑ a ONE-ELEMENT unpacked array (`int a[0:0]`) is **deliberately loud**: `sim_ir::NetVar` carries only `array_len`, which is 1 for a scalar too, and elaborate's `unpacked_array_nets` never reaches the engine, so admitting it would print the element without its braces at exit 0 (prerequisite = array-ness in the IR or a new sidecar + format bump); ⓒ **two recorded divergences from verilator, both pinned with the reason** — a NEGATIVE integer assoc key (vita iterates in IEEE §7.9.4 SIGNED key order, verilator sorts the rendered hex; and the declared key width is cast away before the IR so `-1` renders 64-bit) and an unpacked array of `real` (verilator prints only element 0 while rendering a QUEUE of the same shape correctly — self-contradicting, so vita applies verilator's own recursive rule). x/z digits have no oracle (verilator is 2-state and cannot compile the assignment) and render exactly as `%0h` does.

- **string byte select on a PARENTHESIZED base가 조용히 0**(§4.5.220 재감사 발굴·pre-existing·scalar/fixed/dyn 全 형태): `(p)[0]`=0 vs `p[0]`=119. **paren-select 일반 갭이 아님** — `logic[7:0] v; (v)[0]`·`(v)[3:0]`은 bare 형태와 동일하게 정확. string 전용이며 근인은 `string_index_read`의 base gate가 `Ident|BitSelect`만 matches!하고 `Paren`을 unwrap 안 함(반면 `expr_is_string_ast`에는 `Paren` arm이 있음)→byte-select 경로에 도달 못 하고 width-0 handle의 packed bit-select로 낙하. **§4.5.220이 고친 것과 동일 실패 클래스**(gate가 자기 술어를 under-approximate)·한 gate 옆. iverilog는 구문 자체 거부(오라클 無).
- **scalar `real`의 part-select write가 조용한 no-op**(§4.5.220 재감사 발굴·pre-existing): `real x; x[3:0] = 4'hF`→값 불변·무진단. iverilog는 거부("can not select part of real"). §4.5.220이 dyn `real` ELEMENT(`r[0][3:0]`)를 loud화했으므로 **이제 scalar 쪽이 뒤처진 비대칭**(방향이 string 케이스와 반대·회귀 아님).
- **array reduction method가 var-init initializer에서 loud**(§4.5.219 재감사 발굴·pre-existing): `string s = $sformatf("%0d", arr.sum());` 및 배열 원소 형태 모두 E3009 "unsupported hierarchical function call arr.sum" — t0 pre-sweep 경로의 갭이며 scalar/array 동일(=게이트 문제 아님). `q.size()`·`.len()`·`.substr()`·`.name()`은 동작.
- **string-array 잔여 → §0 승격 큐 T1로 이관(2026-07-25)**: FIXED string array decl-init(`string s[2]='{"a","b"}`·module/block 양쪽 loud·iverilog ✓·§4.5.183 기록 항목) · fixed array **runtime index**/`foreach`(dyn 배열은 이미 동작→fixed만 element-net 표현 때문에 const-index 전용) · `string q[$]`(queue of string·iverilog ✓) · multi-dim `string s[2][2]`(iverilog ✓) · hierarchical `u.s[0]` · **frame-local**(task/function body) string array(static task=E3018·function/automatic=E3009). dyn element의 byte select `d[0][0]`(no-oracle)도 잔여.

- ~~gen/iface string decl-init~~ **RESOLVED**(§4.5.228). ~~generate-case 스코프 이름 `gcase[0].x`~~ **RESOLVED**(§4.5.388 — 뿌리는 파서가 `parse_gen_branch().1` 로 **라벨을 버린 것**이라 스코프가 아예 안 생겼다). 잔여 = 인터페이스 queue 원소의 **계층 read**(`u.q[0]`).
- SYS-READ hier-element dest · hier-write sentinel panic→loud · generate-내 `import` · package 자기-func init(㉽). explicit `import p::t`(TYPE)=**§4.5.148 지원**.
- `$fmonitor`/`$fstrobe`(파일 strobe/monitor) — 현재 W3056 skip=**파일출력 silent drop**(non-silent·warned). 지원=**format bump 필요**(`SysTaskId` 변종 ① or 직렬화 사이드카 ②·staged 파리티): `FmtCapture`에 `fd:Option<u32>` 추가(engine-local)+strobe drain을 `file_write` 라우팅·전용 슬라이스. STDIN read(결정성 설계 필요).
- compound-const `==?` fold=**§4.5.146 지원**(sized 패턴)·잔여 fail-closed loud=unsized x/z 패턴(`'hx` self-width truncation)·negative-signed LHS·non-literal RHS. param override 비상수(W3056→error) · longint MIN fold(package) · loud-message 품질 2건(`[bit]` 캐스케이드·typedef-키 메시지).
- `case (x) inside {…}`(§12.5.4 wildcard case)=vita E2002 parse-reject(loud)·③ 후보(no-oracle: iverilog 13.0 `case inside`/`inside` op/array reduction method 全 거부→hand-IEEE `==?`+내부차분). `inside` operator는 지원(== 시맨틱·§11.4.13). based-literal 내 whitespace(`64'sh FFFF`)=vita lexer reject(loud) vs iverilog 허용(minor·§4.5.147 발굴).
- ~~enum label 범위검증 부재~~ **RESOLVED**(§4.5.165·상세=ARCHIVE) — 잔여(fail-open·§2 invalid-program 한정)**: sized/based-literal label(`{A=8'hFF}`)·param-width base(`[N-1:0]`)의 out-of-range는 `const_lit` 미fold(decimal-only)라 skip=silent-truncate 잔존(iverilog reject하나 유효 프로그램 무영향·fail-open>over-reject·const_lit 확장 or elaborate-time 검사=별개 슬라이스).
- **sized-literal enum label → enum-method loud**(§4.5.164 발굴·pre-existing): `enum bit[3:0] {A=8'hFF}`처럼 label이 non-foldable(sized-literal/식)이면 `enum_defs` 미등록 → `.first`/`.next`/`.name` 全 enum-method가 E3010/E3009 loud(honest·유효값엔 무영향). `const_lit`이 unsized-decimal만 fold. diagnostic 품질 minor(`.name` 메시지="hierarchical function call deferred"·오도). fix=enum label const-fold 확장 or label-site 진단 개선. 부수: function-port receiver `.name`/`.first`=E3010(`var_enum`이 tf-port 미bind·enum-method-family 공통)·chained `x.name().len()`·`arr.min()[0]`=iverilog도 거부(no-oracle).
- **partial-timescale 정책 진단**(`--timescale-policy`·`W-PARSE-TIMESCALE-PARTIAL`/`E-PP-TIMESCALE-PARTIAL`): 일부 모듈만 `` `timescale `` 선언 시 현재 무진단 1ns/1ns 디폴트(전무 케이스만 W1017). doc-08 §15 설계는 문서화됨·`rt.default_used` 신호 이미 존재 — 배선만 필요. §4.5.151 발굴.

**외부 round-20 리포트(2026-07-27) = 12 가족 중 10 RESOLVED**(§4.5.229 part-select 바운드 · §4.5.234 enum formal `.name()` · §4.5.248 8 가족 · §4.5.249 §6 진단 위치). **잔여** = ① §4.11 의 미격리 79 건(리포터의 file:line 격리 대기 — 다만 **round-16 의 §3.4 가 그 가족의 지배적 형태였다**: 두 중첩 레벨 재사용은 §4.5.268 에서 correct-support) ~~② 같은-이름 가족의 마지막 2형~~ **부분 RESOLVED**(§4.5.268) — 한 블록이 다른 블록을 **감싸는** 쌍은 shadowing 이라 여전히 loud(별개 규칙), 모듈 넷과 이름이 겹치는 블록 로컬도 그대로. **두 중첩 레벨의 형제 트리**만 열렸다 ③ `$sformatf` 를 **ternary arm / 단락 우변 / `$monitor`·`$strobe` 인자 / 태스크 인자**에 두는 형태. ③의 뿌리는 하나로 좁혀졌다 — `eval` 의 `SysFuncId::Sformatf` arm 이 포맷 문자열을 무시한다(§4.5.252). `format_args_str`/`render_template` 을 `&SimState` 가 아니라 리더 제네릭으로 올려 `EvalCtx` 에서 쓸 수 있게 하면 ③이 통째로 닫히고 statement-level hoist 를 은퇴시킬 수 있다 — 리포트 자신의 16-블록 충실 복제본도 재현 못 했고 이 체크아웃에서는 볼 수 없다. §4.5.249 의 file:line 이 리포터가 좁힐 수단이다.

**외부 round-16 리포트(2026-07-29·`6b6b8ef` 기준) = 뿌리 7 + 진단 품질 4, 전부 RESOLVED**(§4.5.266/267/268). 리포트가 센 진단 85 건 중 오진이 62 건이었고, 근인별로는 §3.1 definite-assignment 의 제어-흐름 격자(49) · §3.4 두 단계의 스코프 중첩 불일치(17+1) · §3.5 declarator 단위가 아닌 선언 단위 거부(9) · §3.2 문장 위치 호출(4) · §3.3 고정 `automatic` 배열(2) · §3.6 SoA record queue 의 discarding pop(2) · §3.7 `return f(dyn)`(1) 이다.

**잔여 = 없다.** 다만 이 라운드가 **의도적으로 loud 로 남긴** 것 3가지는 갭이 아니라 규칙이다: ① 원소별로 채운 고정 배열에서 **커버리지를 증명할 수 없는** 형태(계산 인덱스·조건부 쓰기·불완전 집합) ② 한 프레임 본문 표현식 안에서 **같은 dyn-formal 함수를 2회** 부르거나 **자기 재귀**로 부르는 형태(마커 슬롯이 하나) ③ 한 블록이 다른 블록을 **감싸면서** 같은 이름을 재선언하는 shadowing(§3.4 가 연 것은 형제 트리다). 셋 다 진단이 그 이유를 직접 말한다.

**§3.8(리포트의 "미분류 1건")도 §3.4 와 같은 뿌리였다** — 구조체 로컬이 멤버별 넷으로 분해되면서 `automatic` 플래그를 잃어 STATIC coalesce 분기의 다른 문구를 달고 나왔을 뿐이고, 두 레벨 형태에서 스코핑을 잃은 것이 정확히 그 멤버들이었다(6b6b8ef 에서 그 문구 그대로 재현·현재 동작). 같은 문구를 다는 이웃 형태 하나는 **loud 로 남아야 한다** — 초기화자를 가진 static 블록 로컬 둘이 한 이름이면 평탄화된 넷 하나에 pre-arm 초기화가 둘 걸려 뒤가 앞을 덮으므로, 받아들이면 앞 블록이 뒤 블록의 값을 읽는다(iverilog 7/9 → 9/9). 둘 다 핀.

**리포트가 오라클로 쓴 iverilog 13.0 의 결함 2건**(vita 가 IEEE 정답): ① 루프 본문 블록이 **로컬을 선언하면** `break` 가 `continue` 처럼 동작한다(`for (i<4) begin int L; if (i==2) break; … end` → iverilog 가 i=3 을 계속 돈다; 로컬이 없으면 iverilog 도 정확히 멈춘다 — vita 는 PRE/POST 동일하므로 이 라운드와 무관하다). ② `case` item 안의 `continue` 에서 `vthread.cc` assertion 으로 abort 한다(abort 전 출력은 vita 와 일치). 둘 다 회귀 테스트 주석에 기록.

**외부 리포트 잔여 (§6-2 → ARCHIVE · 전부 no-oracle 또는 docs):**

- EXT2-A2c: packed multi-dim param `localparam logic[1:0][7:0] PK=…`(외부 0회 사용·hand-IEEE+내부차분).
- EXT2-NAP: named assignment pattern `'{k:v}`(외부 0회).
- EXT2-DOC: 문서 stale(CLI-ref·lang-ref·system-tasks·explain — 외부 2회 보고).

**진단 품질 잔여 (외부 round-29 · 2026-08-18 — 넷 중 셋 해소, 아래가 잔여):**

- **런타임 진단에 위치가 없다 — 시각만 있다.** R29-4 의 절반은 닫혔다(`[at time N]` — `sim_time` 은
  **모든 런타임 emitter 가 이미 찍고 있었고 렌더러가 버리고 있었다**). 잔여 = **인스턴스 경로**와
  **file:line**. 리포터 설계는 한 번 실행에 `W4029` 8건 + `W4007` 1건이 뜨는데 트리의 `unique case`
  사이트가 **11 파일 22 곳**이라 시각만으로는 좁혀지지 않는다. ⚠️ **round-28 의 "E3009 file:line 비일관"
  과 다른 항목이다** — 그쪽은 elaborate 진단의 앵커 누락이고 이건 **런타임 진단에 앵커 개념이 없는 것**
  (엔진은 span 이 아니라 IR 위에서 돈다). 재사용 지점 = §4.5.249 `diag::SpanResolver` + StmtId→span
  사이드카. doc-15 의 W4007 예시(`… at tb.dut.u_fifo time=620ns`)가 목표 형태다.
- ~~**`unique`/`priority` 위반이 RTL `$warning` 과 코드를 공유한다**(R29-3)~~ **RESOLVED
  (2026-08-18)** — `W-RUN-UNIQUE-VIOLATION`(**W4031**) 신설 · `SeverityKind::UniqueViolation`
  (기존 `SeverityTable` 트레일러에 **마지막으로** 추가 ⇒ v26 값은 그대로 디코드 · **format_version
  27**) · 파서 desugar 가 `$warning` 대신 `$__vita_unique_violation` 을 낸다. 양방향 실측:
  `-Wno-` 가 각각 하나만 끄고, `-Werror=W-RUN-USER-WARNING` 이 **`$warning` 없는 설계를 더 이상
  안 깨뜨린다**(errors=2 → 0). ⭐ 곁가지로 **`$__vita_` 를 예약 네임스페이스로** 만들었다 —
  desugar 가 elaborate 에게 말하는 채널이 **이름뿐**이라(마커 필드는 frozen AST 형상을 바꾼다)
  소스가 그 이름을 쓸 수 있으면 **위반하지도 않은 violation 을 소스가 파일할 수 있다**. 규칙이지
  목록이 아니라서 다음 desugar 도 자동으로 덮인다.
- ⚠️ **`cli` 의 lib 테스트 타깃은 제품 형태에서 컴파일되지 않는다**(pre-existing · 2026-08-18 발견 ·
  `7de9364` 에서도 동일). `cargo test -p cli --no-default-features --lib` 이 **E0004** 로 죽는다 —
  lib **테스트** 타깃이 dev-dep 을 링크하면서 **sim-engine 의 `oracle` 만 되살아나고** cli 자신의
  feature 는 꺼진 채라, `Backend` 는 세 variant 인데 `backend_name` 의 `#[cfg(feature="oracle")]`
  arm 둘이 잘려 나간다(§5.1-as feature-unification 함정의 **비대칭 형태**). ⚠️ **CI 는 못 본다** —
  `build-no-oracle` 축은 `-p sim-engine` 만 테스트한다(그래서 그 축은 초록이고 **135 green** 이 맞다).
  ⚠️ **그러므로 `-p cli` 를 그 명령에 더하지 마라** — 오늘은 실패가 곧 이 항목이지 회귀가 아니다.
  수리 = cli 의 dev-dep 이 sim-engine 을 `default-features = false` 로 잡거나, 두 crate 의 `oracle`
  을 하나로 묶는 것.
- **`error_at` 진단은 앵커와 `found` 가 다른 토큰을 가리킬 수 있다.** `error_at` 은 **더 이른 노드**에
  보고하는데 `found` 는 커서 토큰이라(`g[w].u.q` → 앵커는 `w`, 메시지는 `found '.'`) 둘이 갈린다.
  R29-1 이 이 둘을 **별개 필드로 분리**했으므로(자기 토큰만 인용 = 절대 틀리지 않는다) 오늘은
  correct-but-confusing 이다. 사이트 10곳.

**deep 잔여(저우선):**

- t0 race 그라운딩(계단식 CA 체인) · `@(*)` decl-init wake · runtime `==?` pattern.
- inline body NON-fill context-width · modport 방향 강제 · force part-select · assoc 배열-key/clocking 배열-output word0.
- ~~음수 range bound~~ **RESOLVED**(§4.5.228) — net·multi-packed inner·unpacked·VCD 범위. 정규화 저장(`[w-1:0]`) + 선언 바운드 사이드맵, 선택 정규화와 폭을 **함께** 켜는 opt-in. `[W-1:0]`-with-W==0 param underflow 는 graceful width-1 유지(test `v3_12`). **잔여**: PART select(§2) · 포트/formal(의도적 비대칭).
- **VCD 잔여 fidelity**(§4.5.138 range fix 후·전부 **cosmetic·decode 동일**·§4.5.139서 VALUE 검증 완료: x/z·real·wide·readmem·format 全 decoded waveform iverilog 일치). 남은 encoding 차이: ① value 미압축=vita full-width(`bxxxxxxxx`) vs iverilog leading-redundant strip(`bx`·`b0`)—decode 동일·큰 golden churn ② t=0 초기덤프 구조=vita `$dumpvars`에 pre-assign X + `#0` change vs iverilog settled값—final 동일 ③ var-type=logic 절차구동시 vita `wire` vs iverilog `reg`(연속구동=both wire·usage 의존이라 non-trivial)·`int`=`reg` vs `integer` ④ real size `64` vs `1` ⑤ `parameter` 미덤프. + 근본: elaborate packed-md NetVar.lsb stale(lib.rs 8435/7862·VCD helper서 flat fallback 우회).
- **real const-fold 전면 미지원**(§4.5.141 발굴): `localparam/parameter real` = `2.0+3.0`·`*`·`/`·`-`·`**` 全 E3009 "not foldable"(iverilog=folds). `localparam=$clog2(real-lit)`도 동근(const_eval_in_scope=i64-only·real arg→None loud·§4.5.143 런타임은 해결). 런타임 real 산술은 정상(§4.5.141서 `**`도 지원)·const 경로만 uniformly loud. const_eval_in_scope에 real f64 arithmetic 추가 필요(broad·non-silent).
- **X-bearing integral→real 변환 divergence**(§4.5.141 발굴): vita=whole X값→`0.0`(`real_arg`=`to_i128_signed().unwrap_or(0)`) vs iverilog=per-bit X→0(예 `4'bxx01`→1). `$itor`/`$sqrt`/`$pow`/real-`**` 공통·pre-existing. non-silent 아니지만 divergent(impl-defined X→real).
- **width>128 정수→real 변환**=여전히 `0.0`(§4.5.151서 `to_i128_signed`를 128-bit lane까지 확장 — 65..=128 signed/unsigned는 수정 완료·>128만 잔여). 초희귀(129-bit+ 값의 real 대입)·word-grid f64 근사 필요.
- **x/z-fill const param LHS→0**(§4.5.146 발굴): `localparam logic [W] P = 'x`=const_eval가 `fill_to_i64`/`fill_literal_const`로 0 bind(x 소실)→**全 const 연산자 상속**(`P==0`·`P+1`·`P ==? pat` 4-state 결과 divergent). upstream param binding 근원·contrived(all-x const 선언)·broad. §4.5.146 `==?` fold는 sized 패턴만이라 무영향(a=int).

## 4. SVA / 검증 honest-loud 잔여

- empty-match `##0`/unbounded `##[m:$]` 융합(§16.9.2.1 불연속·오라클 부재).
- N2c full sequence local var(중첩 attempt 각자 데이터=L급; 단일-capture ✅).
- later-antecedent read · outer-`|=>` prop-ref skew 고급형(2-cycle·중첩·cross-clock).
- SVA-QUAD collapse default-flip(`VITA_SVA_COLLAPSE` opt-in 상태 — full-VCD 골든 audit 선행).
- N4 clocking 잔여: non-`#1step` skew·INOUT·multi-event-list clock·non-net bind·hier input drive·cross-hier `@(inst.cb)`. **(2026-08-18 · round-29 R29-2 로 절반 해소)** — 블록 전역 `default input SKEW [output SKEW];`/`default output SKEW;`(IEEE §14.3)는 **파싱·적용된다**(스킵 없이 각 아이템에 스탬프 ⇒ 판정은 여전히 elaborate 한 술자리). 잔여 = **skew 값 자체**. ⭐ **그중 `output #0` 은 다른 종류의 잔여다** — vita 의 무-skew output 모델이 *"엣지에 Active 리전에서 `source = holding`"* = **자기 나름의 `#0` 근사**이므로, 같은 구성이 **암시 철자는 돌고 명시 철자는 loud** 다. 승격하려면 IEEE 의 Re-NBA 리전 드라이브와의 차이를 오라클로 확인해야 하는데 **iverilog 는 `clocking` 을 파싱 못 하고 verilator 는 Observed 리전 샘플**이라 앵커가 없다 ⇒ 별도 슬라이스(hand-IEEE §14.11/§14.16). `input #0`/`#N`/`##N` 은 **진짜로 다른 샘플링 리전**이라 그대로 loud.
- class: down-cast `Derived'(base)`($cast 런타임 타입가드 선행) · real→longint cast · base-shadow 명시 접근 `Base'(d).v` · cast-as-receiver `(B'(d)).foo()`.

## 5. perf / 하드닝 — ⛔ **성능 축은 수확 체감 도달**(Phase D 종료 2026-08-17) · 열린 것 = 하드닝 1건 + §5.1 나머지

✅ **RESOLVED 2026-08-26 (round-34) — and the axis was neither the one §4.5.382 recorded
nor the one the report proposed.** Details = ARCHIVE. One line: **the only family where
`native` loses to `vm` is a SIGNED CONSTANT operand in the RHS.** `wprog`'s entry gate
declined a `Const` leaf on sign alone, which took the whole continuous assign off the
compiled op-stream (64 assigns × 200k cycles: signed native **118.1 ns/eval** vs vm 73.2
= **1.61×**; unsigned native 41.1 vs vm 72.8 = 0.56×). Relaxed for a `Const` leaf only ⇒
signed 1.706 s → **0.726 s (−57%)**, identical to its unsigned twin. A 96-cell battery
(every admitted operator × a signed constant in each operand position) is
native == vm == iverilog.

⚠️⚠️ **Two earlier conclusions were refuted here.**

- **§4.5.382's diagnosis** (*"the axis is the computed RHS array index"*, 0.92× / 2.05× /
  1.03×) and **the report's** (*"the axis is the unpacked-array-element LHS"*, 1.66×) are
  **both wrong**. Over a 34-cell sweep, EVERY 1-D array-element-LHS cell has native
  ahead (0.246 … 0.837), and native is **3.9× faster** on the report's own headline
  shape. A computed index adds only +11% on native (83.3 → 92.5 ns); the +91% is vm's.
- ⭐⭐ **Dropping the sign half of the gate ENTIRELY was already built, measured and
  reverted on 2026-08-20** — `wprog.rs`'s own module header records it as *"SOUND · slow
  lane −19.0% · and **1.00×**"*. That measurement was right and its conclusion was scoped
  to picorv32 and keccak. `localparam int` is what SV RTL writes and a generate genvar is
  the same cell, so a hash or cipher round was falling off this path entirely.
  ⇒ **the §4.5.369 lesson again**: our own benchmarks find what we already suspect.

**⚠️ Remaining performance residue — a 2-D / 3-D / packed element LHS is a ~10× cliff on
BOTH backends** (round-34 census · mechanism **NOT located**). Against 50.0 ns for 1-D:
2-D **546.7** (native) / 675.8 (vm), 3-D 829.2 / 967.5, packed `logic [63:0][31:0]`
410.8 / 441.7 ns. The native/vm ratio stays 0.81–0.93, so this is shared plumbing rather
than a backend axis. Needs its own census before a fix shape — do not guess.

**⚠️ And the inline fold is exponential** (round-34 R4 census). `elab_s` stays flat at
0.35 ms while `sim_s` spreads 0.16 s → 14.36 s ⇒ the arena SHARES the substituted subtree
as a DAG and the evaluator re-walks it as a TREE. Inlining is (references per
statement)^(chained statements) where the frame path is linear: 6 statements reading the
local ONCE is 0.19 s inlined / 0.24 s framed, and reading it THREE times is
**14.39 s / 0.35 s** — digest-identical, and the same under all three backends.
⇒ ⭐ **`able` is a coverage number, not a speed proxy.** Widening the inliner to
control-flow bodies is the WRONG DIRECTION; the real item is **per-activation
memoisation of a shared sub-expression**.

**⚠️ 하드닝 1건 — 프로세스 수준 메모리 가드가 없다 (슬라이스 #8 실측 · 오너 승인 2026-08-15).** 폭주한
`vita` 하나가 **33 GB × 2** 를 잡아 32 GB 머신을 jetsam → WindowServer 크래시 → **userspace watchdog
커널 패닉**까지 몰고 갔다(2026-08-14). 기존 가드(`max_deltas`·`max_body_steps`·`time_limit`)는 **델타도
문장도 진행하지 않는 루프**(시스템태스크 내부)를 구조적으로 못 본다. 이번 슬라이스는 **그 루프를
카운트 기반으로** 바꿔 실측된 형태를 닫았고(`$writemem*`), **일반 가드는 남는다**. 설계 제약 둘을 먼저
정해야 한다: ⓐ **할당 카운팅 전역 allocator** 는 매 할당에 원자연산 둘을 더한다 — 성능 축이 몇 주에 걸쳐
지운 바로 그 비용이라 기본 ON 은 회귀다 · ⓑ **RSS 샘플링 워치독 스레드**(초당 1회)는 핫패스 비용이 0
이지만 macOS 에서 `mach_task_basic_info` = **unsafe FFI** 라 unsafe 정책의 지정 모듈 확대가 선결이다
(Linux 는 `/proc/self/statm` 로 safe). ⇒ ⓑ + 기본 상한(물리 RAM 의 1/4 등) + `--max-mem` 이 현재 후보.

### 5.0 ★★ ③층 — 성능 축 **수확 체감 도달**(그 판정의 근거표 · 2026-08-10)

| 단계 | 상태 |
|---|---|
| T0 · S0 · S1(전부) | ✅ |
| **S2** 폭별 특수화 | ✅ **닫힘**(§4.5.329, 프로파일 근거) |
| **S3a** 호출 흡수 | ✅ |
| **S3** 바디 코드젠 | ✅ 슬라이스 1~2 · ⛔ **cranelift 절은 census 가 반증**(§4.5.334) |
| **S4** 스케줄 소거 | ⛔ **중단 판정 발동 — 이득 <1.3×**(§4.5.335) · 스케줄러 유지 |
| **S5** NBA 전용화 | ⛔ 같은 이유로 보류 — 표적이 `k_schedule_nba_scalar` **3.8%** 뿐 |
| **S6** 3-OS 결정성 | 코드젠이 없으므로 **불필요**(현 native 는 arch-무관 Rust) |

**picorv32 native/vm = 1.73×**(0.504 vs 0.876) · 고정 비용 19 ms(3.7%) → **상한 26.8×**
(⚠️ doc-21 §6.3 의 *"~85 ms · 14% · 상한 7×"* 는 낡았다 — 이 실측으로 정정).

> **⭐⭐ 왜 여기서 멈추는가 — 프로파일이 세 단계를 연달아 닫았다.**
> S3 의 cranelift 논거는 *"leaf load 와 산술을 인라인한다"* 인데 실행 census 는 **산술 1.5% ·
> `Load` 39.3%(이미 직접 접근) · 인라인 불가한 30.8% 가 하필 이 저장소가 한 철자로 통합한 4-state
> 규칙 함수들**이었다(§4.5.334). S4 의 표적(`propagate` 2.1 + `wake` 1.4 + `pass` 구성 ~2)은
> **≈6% = 1.06×** 로 자기 중단 판정(<1.3×) 아래다(§4.5.335). S5 도 3.8% 다.
> **1.73× 와 26.8× 사이는 vita 가 일부러 안 하는 것**(2-state 좁히기 · levelize)과 **없는 축**
> (멀티코어)이 메운다 — VCS·Xcelium·verilator 가 하는 일이 그것이다. 셋 다 **정확성 계약을 거래**
> 하거나 **아키텍처를 바꾸는** 일이라 오너 판정 사안이다.

### 5.1 ✅ **완료** — 실행기 둘로(interp = 오라클 · native = 제품) · 오너 지시 2026-08-10

> ⭐ **Phase A~D 가 모두 끝났다.** 다음에 무엇을 할지는 [§5.2 재개 지점](#★★★★-52-재개-지점--세션이-끊겼다면-여기부터-2026-08-17) 이 정본이고,
> **실행 기록 전문(슬라이스 59건)은 [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md)** 로 이관했다(무삭제·§번호 보존).
> 아래 §5.1 나머지는 **아직 열린** 보류 항목이다.

**목표 형태**: `--backend interp`(테스트 전용 오라클) + `--backend native`(제품). **②층 VM 은퇴.** ✅ 달성.

**왜**: ⓑ(코드젠)가 언젠가 필요해지면 비싼 것은 cranelift 가 아니라 **4-state 규칙이 두 철자가 되는
것**이다. 실행기가 둘이면 그 긴장이 *"두 번째 철자를 만드는 일"* 에서 *"오라클이 지키는 코드
생성기를 만드는 일"* 로 바뀐다.

### 5.1 나머지 (전부 보류 판정 — 트리거 시만)

- **CI 를 `cargo nextest` 로 옮긴다**(2026-08-06 실측·오너 지시로 별도 슬라이스): `cargo test --workspace` 는 450 타깃을 **순차** 실행해 실행 단계만 **724 s** 인데(per-target 시간 합은 62 s — 나머지는 프로세스 기동), `cargo nextest run --workspace` 는 같은 5183 테스트를 **30 s** 에 돌린다(24×). 로컬은 이미 nextest 로 통일했고 CI 4잡(ci.yml)만 남았다. ⚠️ 두 실행기는 **빌드 트리를 공유하지 않으므로** 번갈아 쓰면 전환마다 ≈470 s 재빌드를 문다 — CI 도 옮겨야 로컬/CI 가 한 실행기가 된다. ⚠️ 버전은 **0.9.100** 을 쓴다(0.9.143 은 rustc 1.91 요구·저장소 핀은 1.85). 슬라이스 내용 = ci.yml 4잡 교체 + nextest 설치 단계 + `--locked` 유지 + 3-OS 에서 결과 동일 확인. ⚠️ **선결 조건 — temp 디렉터리 이름이 nextest 아래에서 충돌한다**(2026-08-07 실측): 통합 테스트 **368개 파일**이 `temp_dir()/vita_<tag>_<pid>_<프로세스별 카운터>` 를 쓰는데, nextest 는 **테스트마다 프로세스를 새로 띄우므로** 카운터가 매번 0 부터 시작하고 **PID 재사용**이 이전 실행의 디렉터리와 겹친다 — `cli::obs::compile_error_writes_no_obs` 가 그렇게 한 번 빨갛게 났고(격리 실행·연속 두 번의 전체 실행은 초록) `cargo test`(한 프로세스)에서는 구조적으로 안 난다. 이름에 프로세스 고유값을 더하거나 생성 전에 기존 디렉터리를 지워야 CI 를 옮길 수 있다.
- **"새 Rust 를 지원한다" 는 정책이 검증되지 않는다**(2026-08-06 실측): 툴체인 정책은 *"MSRV 에 상한을 두지 않는다 — 새 Rust 가 나오면 지원하는 쪽으로 따라간다"* 인데, `rust-toolchain.toml`·`Cargo.toml rust-version`·ci.yml **4잡 전부 1.85.0 고정**이고 `stable`/`beta` 잡이 **0개**다. 즉 바닥은 강제되지만 **천장은 아무도 안 밟는다** — vita 가 1.91/최신에서 빌드되는지 확인하는 곳이 없다. 1.85 자체는 의도된 값이다(§4.5.150 에서 fst-writer 0.3.x 의 edition 2024 때문에 1.82→1.85·vita 크레이트는 edition 2021 유지·저장소 안에 1.85 초과를 요구하는 것 없음·PUBLIC 이라 MSRV 상향은 사용자 약속을 좁힌다). 필요한 것은 상향이 아니라 **비차단 `stable` 잡 1개**.

- **`native::run::executor_rows` 가 모든 `simulate` 호출에서 돈다**(§4.5.298 리뷰): 백엔드와 무관하게
  전 프로세스의 전 문장을 훑는다. 요청 백엔드로 가드하면 run.json 의 `native` 판정이 요청에 따라
  달라져 "census 는 실행기와 무관" 계약이 깨지므로 **무조건이 맞다** — 비용이 문제가 되면 캐시.


- **u32::MAX 를 넘는 `#delay` 는 여전히 CLAMP(§4.5.292 잔여)**: 랩(7× 조기 발화)은 고쳤지만
  표현 불가 값은 조용히 `u32::MAX` 로 잘린다. 표현하려면 IR 필드(u32)를 바꿔야 하고(동결 타입 →
  format bump), 알리려면 **새 W-code** 가 필요하다. 지금은 4.29e9 틱에 실제로 도달하는 런에서만
  틀리다(전에는 그런 delay 전부가 틀렸다). 이득 = correct-or-loud 완성.
- **③층 판정에 "오늘의 커널이 돌릴 수 있는가" 층이 없다**(§4.5.292): run.json 은 `eligible`(범위)
  과 `buildable`(오늘의 저장소)만 싣는데, `$sformatf`·`$display`·transport-delay NBA·재arm 은
  **적격이고 빌드되지만 커널이 아직 없다**. `kpred::rhs_routes_to_worker` 가 그 질문의 절반을
  이미 답하지만 게이트에 안 물려 있다 — 4b 가 dispatch 를 배선할 때 **세 번째 층**으로 실어야
  그 전까지 "적격"이 능력으로 오독되지 않는다.
- **③층 quiescence 가 커널의 `delayed_nba` 를 안 본다**(§4.5.295·**S1d-4c-2 필수**): 엔진의 `next`
  는 `Scheduler` 의 `wheel`/`delayed_ca`/`delayed_nba` 최소값인데 네이티브 런에서 그 맵은 비어 있다
  (모든 `k_schedule_nba*` 가 커널 큐로 간다) → **트랜스포트가 유일한 대기 작업이면 quiescent 로
  보고되고 업데이트가 사라진다**. `k_schedule_nba_at` 이 loud 패닉에서 조용한 enqueue 로 내려온
  상태라 이건 **사다리 하강의 유일한 잔여**다.
- **S1d-4d 바이트 동일 게이트가 만날 pre-existing 오라클 차이 6건**(§4.5.295 differential 실측·전부
  PRE==POST): ① `$finish` 와 같은 틱의 pending NBA/트랜스포트를 iverilog 는 적용+덤프하는데 vita 는
  안 한다(**파형만**·stdout 은 일치) ② VCD intra-tick 입도 — vita 는 NBA store 마다, iverilog 는
  정착값만(`native/dirty.rs` 의 의도된 설계) ③ t=0 initial 블록 순서(소스 순서 의존·LRM 미정의)로
  `@(negedge clk)` 가 iverilog t=2 · vita t=0 ④ **t0 arm 순서** — vita 는 `initial` 이 t=0 에 쓴
  x→0/x→1 전이로 static edge 를 발화시키는데 iverilog 는 안 한다(리셋 펄스를 t≥1 로 옮기면 일치) ⑤
  **`always @(*)` 의 읽기 집합이 비면** vita 는 t0 에 한 번 돌고 iverilog 는 안 돈다 ⑥ **`.velab` 은
  별개 `vcmp` 실행 간 바이트 재현이 안 된다**(RULEV-MTIME `(mtime,size)` 스탬프 8바이트 — 의도된
  설계지만 **naive staged 아티팩트 바이트 게이트는 flaky** 해진다 · work lib 을 고정하면 재현됨).
  **게이트를 짜기 전에 여섯 개를 어느 쪽으로 고정할지 정해야 한다.**
- **`$monitor`/`$strobe` 의 ③층 렌더 경로**(§4.5.294 거부): dispatch 는 등록만 하고 렌더는
  `sched/run_loop.rs::flush_postponed` 에서 일어나는데 그 경로는 리더를 안 받는다. 배선하면 거부를 풀
  수 있다(S1d-4c 와 한 슬라이스 — 그 경로가 곧 리전/포스트포운드 큐다).
- **`NetArena` 의 `fd_eof` X-poison 구멍**(§4.5.293): 아레나의 defaulted `NetReader` 메서드 중
  `fd_eof` 만 "heap/class/frame 없음" 논증이 안 덮는다 — 지금은 **`$feof` 과잉표시가 가리고 있을 뿐**이라
  그 과잉표시를 고치면 `$display("%0d", $feof(fd))` 가 적격이 되고 아레나는 X 를, 엔진은 실제 플래그를
  낸다. 아래 `$feof` 항목과 **한 슬라이스로 묶어야 한다**.
- **`$feof` 가 정본 stmt-effect 술어에서 과잉표시**(§4.5.291 적대 리뷰 실측): `k_feof` 는
  `read_state[fd].eof` **순수 읽기**이고 elaborate 주석도 그렇게 적는데 `sysfunc_is_stmt_effect` 가
  `true` 로 표시한다 → `e = $feof(fd);` 는 ③층 게이트가 거부하고 `while (!$feof(fd))` 는 통과한다.
  술어를 한 소비자에서만 고치면 **철자가 둘**이 되므로(두 백엔드가 갈릴 수 있다) 정본을 고쳐야 하고,
  그것은 **tier-2 의 컴파일 게이트도 함께 넓히는** 변경이라 자체 byte-identity 논증이 필요한 별도
  슬라이스다. 이득 = ③층 과잉거부 1형 해소 + tier-2 가 그 바디를 컴파일 대상으로.
- **`NetSlot.prev` 는 워크스페이스 전체에서 읽는 곳이 0** (§4.5.290 적대 리뷰가 필드명을 바꿔 빌드해 증명 — 선언·생성자·`propagate_changes` pass (c) 쓰기 세 곳만 걸린다). 즉 pass (c) 의 `clone_from` 2회/변경넷/델타가 **핫 루프의 순수 죽은 일**이다. 제거는 바이트 동일이 자명하나(아무도 안 읽음) 그 자명함 자체를 검증해야 하므로 **별도 슬라이스**로 둔다 — 값은 perf 축, 위험은 "정말 아무도 안 읽는가" 하나.

- **✅ COMB-DEPTH 해결 (2026-08-01) — dirty-settle. 깊이 24 에서 14.1× · 출력 바이트 동일 · 골든 재판정 0건.** `settle_cont_assigns` 가 매 델타 전체 cont-assign 을 전수 재평가하던 것을 **의존성이 움직인 것만** 재평가하도록 바꿨다(`propagate_changes` 를 305→15.5 ms 로 만든 dirty-list 와 같은 형태). 인스턴스 체인, 총 사이클 고정:

  | depth | before | after | 배속 |
  |---|---|---|---|
  | 1 | 7.8 ms | 5.7 ms | 1.4× |
  | 6 | 71.2 ms | 13.3 ms | **5.4×** |
  | 12 | 229.6 ms | 26.1 ms | **8.8×** |
  | 24 | 814.4 ms | 57.9 ms | **14.1×** |

  **2차 항이 사라졌다** — 24× 깊이에 10.2× 로, cont-assign 없는 순수 체인(5.3×)과 같은 선형 형상. 검증 = 5016 tests green · examples 4종 **stdout+VCD 바이트 동일**(PRE `df3d8df` vs POST) · depth-6 체인 iverilog 차분 일치(`acc=00001f68` 3자 동일) · clippy/fmt clean · `SimIr` 무변경.

  **건전성 논거**: 의존성이 안 움직인 assign 은 같은 값을 다시 계산하고 write 퍼널이 same-value write 를 변경 없이 버리므로, 건너뛴 방문은 **관측상 no-op** 였다. 그래서 모든 위험이 **의존성 집합의 완전성**에 있고, 그것을 `levelize::ca_deps` 가 positive allow-list 로 인증한다 — delayed · multi-driver 멤버 · 비순수 RHS(`SysFunc`/`Call`/`ArrayItem`) · **heap handle 의존**(dyn/queue/assoc/class: 핸들 net 이 안 바뀌어도 내용이 바뀔 수 있어 `note_change` 가 보고하지 않는다)은 전부 **무조건 재평가** 목록으로. 방문 순서는 오름차순 = **선언 순서**(기존 fixpoint 와 동일, 골든 다수가 의존).

  ⭐ **구현 중 발굴**: `Expr::Signal { word }` 이 상수 인덱스가 아니라 **ExprId** 였다(`eval_core` 가 평가한다). `expr_nets` 가 그걸 안 걷고 있었고, 랭크에선 무해했지만 **dirty-settle 에선 `assign y = mem[idx]` 가 `idx` 변경에 재평가되지 않는 silent-wrong** 이 됐을 자리다 — 배선 전에 잡아 수정.
- **🔴 COMB-DEPTH 중간 판정 (2026-08-01, 위에서 해결됨) — 원인이 §4.5.278 이 지목한 자리가 아니었다. levelize 는 불필요하고, 오너가 승인한 정확성 대가도 불필요하다.** 오너가 재진입 트리거 ①②를 둘 다 승인해 D(levelize)에 착수 → **랭크(`sim_engine::comb_ranks`)를 만들고 랭크 순 Active 드레인 + 랭크 사이 settle 을 구현해 측정했더니 깊이 1~24 전 구간에서 1.00×** → **폐기·되돌림**(아무것도 안 하는 knob 을 남기지 않는다 = §4.5.278 `Value::resize` 판례). 총 사이클 고정·깊이만 스윕(`perf_depth_cost_shape`):

  | depth | 단일 모듈(cont-assign 없음) | 인스턴스 체인(포트 cont-assign) |
  |---|---|---|
  | 1 | 3.3 ms | 7.8 ms |
  | 6 | 8.4 ms | 71.2 ms |
  | 12 | 15.3 ms | 229.6 ms |
  | 24 | 31.7 ms | **814.4 ms** |

  **순수 `always_comb` 체인은 깊이에 선형**(24단이 1단의 9.6× = 깊이 페널티 0)이고 wake 사슬이 델타당 프로세스 1개라 **애초에 정렬할 배치가 없다.** 2차 비용은 **cont-assign 을 거칠 때만** 나타난다(24× 깊이에 104×). 뿌리 = `settle_cont_assigns` 가 **매 델타마다 전체 cont-assign 을 fixpoint 까지 전수 재평가**하고, 깊이 D 체인은 전파에 D 델타를 쓰며 O(D) 개의 assign 을 들고 있다 → O(D²). **프로세스 드레인 순서로는 손댈 수 없다.** 레버 = **dirty 기반 settle**(RHS 넷이 바뀐 assign 만 재평가) — `propagate_changes` 를 305→15.5 ms 로 만든 dirty-list 와 같은 형태. **결정적으로 이건 프로세스 실행 순서를 안 바꾸므로 골든 재판정이 전혀 필요 없다** — 오너가 승인한 대가를 치르지 않고 얻는다. 잔존물 = `comb_ranks`(조합 깊이 보고용, 정확·저비용). 다음 수 = **dirty-settle**. 상세 = `crates/sim-engine/src/levelize.rs` 모듈 독 · 구 판정 아래.
- **COMB-DEPTH(외부 round-23 §3.2) — 구 판정(2026-07-31): vita 결함 아님, 재진입 트리거만 등재.** 총 작업량을 고정한 깊이 스윕에서 **iverilog 가 같은(더 가파른) 스케일링**을 보였다(UNROLL 1→12 에서 iverilog 4.15× vs vita 3.55×, 절대 wall 은 vita 가 0.80–0.99×). 깊이 비용은 인터프리티드 이벤트구동 델타사이클의 성질이고, 리포터의 비교 대상(Xcelium, <10분 vs vita ~13시간 추정)은 **컴파일-타임에 levelize 하는 컴파일드 시뮬레이터**다. 원인은 특정됨: UNROLL=6 에서 사이클당 프로세스 활성이 `7 6 5 4 3 2 1…` 삼각형(≈D²/2)이고, 단 사이 전파가 cont-assign settle 즉 **델타 경계**를 거치므로 **배치 정렬은 지렛대가 아니다**(오름/내림 정렬 4.86/4.85 s 동일 — 실측으로 기각). 승격하려면 **rank 순 active 배출 + rank 사이 settle**(levelize)이 필요하고, 이득 상한은 ≈D/2(UNROLL=6 에서 ~3×)인데 **프로세스 실행 순서를 바꾸므로 다중 프로세스 `$display` 순서가 이동**한다(IEEE §4 는 region 내 순서를 implementation-defined 로 두지만, vita 의 골든은 iverilog 순서에 핀되어 있다). 재진입 트리거 = ①깊은 조합 cone 실수요 + ②순서 이동을 감수한다는 오너 판정. 상수항은 별도 축이다: trivial flop 200k 사이클에서 vita 1.03M cyc/s vs iverilog 1.96M cyc/s(**1.91×**) — `Value::resize`/`mask_top` 원워드 fast-path 는 **측정 이득 0** 으로 기각(§4.5.278). 상세=ARCHIVE §4.5.278 · 가속 분석 = [preview/18](preview/18-acceleration-analysis.md).
- SVA-QUAD default-flip = §4 항목과 동일(full-VCD audit 선행).
- FMT-CACHE part b(render_template pre-segment) · GEN-3X-STR part a(unroll plan 캐시=byte-identity 위험>이득) — 저ROI 보류.
- QUEUE-MID-ON: 스펙 내재 O(n)(iverilog 동일) — 영구 비권장·monitor-only.
- 백로그 원문·완료 32건 = ARCHIVE §5.

---

## ★★★★ 5.2 **재개 지점 — 세션이 끊겼다면 여기부터** (2026-08-19)

> **Phase A · B · C · D 가 전부 끝났고, 3판 클리어 라운드(1~10+P1)도 끝났으며, §2 정확성 큐의
> 자기결정 위치 셋(옛 「다음 착수 순서」 1~3)도 끝났다**(2026-08-19 · ARCHIVE §4.5.342/§4.5.343 —
> #9/#10 이 staged·런타임 진단에 `file:line` 을 줬고 format 은 **29** 다).
> 아래는 *"다음에 무엇을 해야 하나"* 의 **유일한 정본**이고,
> 이 절만 읽으면 재개할 수 있게 적었다. 종류별 우선순위 원칙은 [§1](#1-착수-우선순위--원칙만-여기--현재-큐는-52-재개-지점)
> 이고 **현재 큐는 여기 하나뿐이다** — §1 에 두 번째 큐를 만들지 마라(2026-08-18 에 그렇게 썩은 적이 있다).
> 실행 기록(Phase A~D · 슬라이스 59건)은 [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md).

### 지금 상태 (한 화면)

| | |
|---|---|
| 기본 백엔드 | **`native`**(③층) · 코퍼스 **100.00%** 실행 · 발산 0 |
| 제품 형태 | `--no-default-features` = **실행기 하나** · 게이트 거부는 **치명** |
| 성능 | 벤치 **10/10 에서 native < vm** · picorv32 native/vm **0.60** (⚠️ round-29 가 지적한 **레짐 갭**을 메워 8→10 · 아래 §round-29 §5) |
| 코드젠 | **기본 OFF · 기각됨**(§5.1-be) — 빌드·배선·측정·정확성은 전부 갖춰 둔 상태 |
| 게이트 | **6,006 tests green** · 워크로드 코퍼스 **8/10** · no-oracle 축 green · clippy 0 · fmt 0 · format_version **29** · MsgCode **68** (2026-08-25 · ARCHIVE §4.5.383) |

### 다음 후보 — 우선순위 순

| 순위 | 트랙 | 왜 여기 | 착수 조건 / 첫 걸음 |
|---|---|---|---|
| **1** | **정확성 큐 — §2 silent-wrong 잔여** | 이 저장소의 **최상위 원칙**이 정확성이고, 성능 축은 수확 체감에 도달했다 | ⚠️ **§2 를 위에서부터 읽지 마라 — 그 절은 주제별 묶음이지 착수 순서가 아니다**(맨 위 뭉치는 *AST self-폭 패스*라는 큰 선행조건에 막혀 있다). **착수 순서는 §2 머리말의 「다음 착수 순서」** 를 따른다 · 착수 전 오라클로 재현. ~~**다음 = ⓓ**~~ **RESOLVED §4.5.383** · 다음 후보 = §2 표 **9**(enum 라벨 셀렉트) / **6** / **4** · ⚠️⚠️ **ⓔ 는 §4.5.376 census 가 강등**했다(*"두 오라클 다 자식 먼저"* 가 거짓 — verilator 가 **vita 편** · §3 ④ 를 막고 있지도 않았다 · 남은 결함은 읽기 방향뿐이고 **코퍼스 수요 0** ⇒ 브리핑은 아래) · ~~ⓐ§4.5.364~~ ~~ⓑ§4.5.365~~ ~~ⓒ§4.5.366~~ RESOLVED(잔여는 §2) |
| **2** | **§3 loud → correct-support 승격** — ⭐⭐ 착수 순서를 **워크로드 코퍼스가 정한다**(§3 머리 블록) | 오늘 loud 인 것이 **실물 IP 를 막고 있다는 것이 측정됐다**. ~~①~~ **RESOLVED(§4.5.370)** — 문자열 상수 도메인이 열려 serv·verilog-ethernet 이 **더 깊은 갭으로 전진**했다 | **②** 는 §4.5.371 이 **되돌렸다**(메커니즘은 §3 ② 에 기록 · 선행조건 = 깊이를 이어받는 상수 평가 진입점). ~~⑧~~ 도 §4.5.372 가 **되돌렸다**(선행조건 = *멈춘 프레임 본문의 반환값* · 상세 §3 ⑧). ~~⑦~~ 도 §4.5.373 이 **되돌렸다**(②와 **같은 벽** — 상세 §3 ⑦). ~~③~~ **RESOLVED(§4.5.374)** — darkriscv 전체 SoC 가 처음 돌았다. ~~④~~ **RESOLVED(§4.5.376)** — §4.5.375 가 되돌렸던 것을 **census 가 그 revert 를 반박**하고 재랜딩했다(*"두 오라클 다 자식 먼저"* 가 거짓 · verilator 는 vita 와 같은 `01 02 03 04` · 네 테스트벤치 중 경쟁 로드를 가진 것이 **하나도 없다** · serv 는 ④ 유무와 무관하게 §3 ⑦ 로 거절). ~~⑨~~ **RESOLVED(§4.5.377)** — census 가 갭을 *"삼항"* 에서 **패키지 안 string/real 전부**로 넓혔고 **§2 의 package-real silent-wrong 과 같은 뿌리**여서 두 줄이 함께 닫혔다. ~~⑥~~ **RESOLVED(§4.5.378)** — census 가 원인을 바꿨다(루트 선택은 **iverilog 와 동일** = 결함 아님 · 진짜 결함은 *"top 의 포트가 unconnected"* 경고로 **auto-top 과 무관**). ⚠️⚠️ **⑪ 은 착수 불가로 판정**(§4.5.379 census) — verilog-axi 의 **54 에러가 전부 ② 한 뿌리의 연쇄**다(`parameter S_THREADS = {S_COUNT{32'd2}}` = 파라미터 count replication) ⇒ ⑪ 의 네 칸은 **② 가 서기 전엔 보이지도 재현되지도 않는다** · ⭐ 그리고 census 가 ⑪ 의 이름도 반박했다: **폭이 아니라 값**이다 — `[127:0]` 선언에 값이 `0x1122` 면 **이미 돈다**, 막히는 건 **값이 64비트를 넘을 때**(65비트 리터럴 · part-select 로 쌓는 128비트 누산기) · ⚠️ **싼 절반(넓은 리터럴 반환)은 수요 0**, verilog-axi 가 쓰는 건 **비싼 절반**(루프 누산기 = `calcBaseAddrs`) = §4.5.373 이 되돌린 그 형태 ⇒ **② 뒤로 재배치** · 참고로 넓은 도메인 자체는 이미 있다(`fold_self_bits`/`WideBits`/`BitPacked`, 단 **carry-free**) ⇒ ⭐ **다음 표적 = §0 T2 의 `real` const-fold** · ⚠️ **⑧⑪ 은 §2 급 잔여를 남겼다**(§3 ⑧ 의 `$fdisplay`/`$strobe` 한 문장 lag) · **⑩ 은 §2 급**(조용히 자른다) · ⚠️ *"오라클이 없다"* 는 미루는 이유가 **아니다**(⑤ ibex) |
| **2b** | **§0 T2 잔여** — ~~sized-literal enum label~~ **RESOLVED(§4.5.379)**(⚠️ census 가 이름을 반박: 막힌 건 sized-literal 이 아니라 **상수 이름 라벨**이었다) | §3 과 같은 사다리 방향인데 **오라클이 이미 답한다**라 더 싸다 | 남은 하나 = `real` const-fold(= §4.5.229 가 남긴 `int'(<real param>)` 바운드의 **선행**) · ⚠️ 그 잔여는 §0 항목 8 이 *"의도적 loud"* 로 적어 둔 것이고 **i64 twin 은 시도 후 철회**(5건 silent-wrong)이므로 착수 전 census 필수 |
| **3** | **§6 G2 OBS 잔여** | 최종목표 G2 축이고 정확성과 **직교**라 병렬 가능 | SPEC = [preview/19](preview/19-ai-agent-observability.md) · 남은 항목은 §6 표 |
| **4** | ⭐ **성능 — 표적은 frame 레짐이다**(2026-08-23 · §4.5.367 S0 실측으로 재규정 · **§4.5.369 워크로드 코퍼스로 재가격**) — ⚠️⚠️ **코퍼스가 그림을 양방향으로 바꿨다**: 남이 쓴 RTL 다섯에서 vita 는 iverilog 대비 **기하평균 1.61×** 로 앞선다(sha256 2.89 · biriscv 1.88 · aes 1.74 · picorv32 1.44 · darkriscv **0.78**) — *"iverilog 와 동률"* 은 **우리가 쓴 keccak 두 설계**가 만든 그림이었고, 그 둘은 우리에게 유리한 벤치가 아니라 **가장 어려운 벤치**였다(빼면 1.30 → **1.60**) · ⭐ 표적은 여전히 frame 레짐이다: keccak_f_arr 의 **65.0%** 가 `run_frame_call` 안(콜 귀속)이고 **0.57× 로 코퍼스 최악**이다 · ⚠️⚠️ **2026-08-28 재측정으로 이 행의 수가 거의 전부 갈렸다**(세 변경): 호출을 담은 프로세스 본문이 컴파일 백엔드에서 배제되지 않고 · `case` 가 scrutinee 를 한 번만 평가하고 · ⭐ **등폭 whole-net 읽기가 72바이트 `Value` 를 세 번 옮기던 것을 제자리에서 답한다**(`resize_keep_sign` — 앞의 둘은 호출 모양이라 keccak_f_arr 가 안 움직였는데 이건 **모든 읽기**에 있어 둘 다 움직였다) ⇒ keccak_f **8.11 → 4.45 s**(−45%) · keccak_f_arr **17.24 → 13.64 s** · 다이제스트 불변 · 코퍼스 대 iverilog(**같은 날 네 번째 변경 = 4b 의 스케줄러 스크래치 재사용까지 포함**): keccak **1.10× → 2.18×** · biriscv 1.88 → **2.27** · aes 1.74 → **2.22** · picorv32 1.44 → **1.54** · sha256 2.89 → **3.11** · ⭐ **darkriscv 0.78 → 1.01(동률 도달)** · **serv 0.80 → 0.92** · keccak-arr 0.53 → **0.68** ⇒ 기하평균 **1.55×**(도는 여덟) · **1.68×**(서드파티 여섯) · **1.89×**(serv 뺀 다섯 = 옛 1.60 과 같은 집합) · ⭐⭐ **darkriscv·serv 는 아레나 가설과 무관한 이유로 지고 있었다** — 기본 백엔드가 델타마다 `Vec` 두 개를 새로 할당했고 interp 는 수년째 재사용한다(§4b) · 이 행이 *"지는 이유가 아레나 가설과 다를 수 있다"* 며 darkriscv 를 다음 계측 대상으로 지목한 것이 정확했다 · ⚠️⚠️ **아레나 항목 자체가 재가격됐다**: `frame_bodies` 가 sha256·picorv32·darkriscv 에서 **0**(함수 38·33·9개가 전부 인라인)이라 아레나는 그 셋에 **아무것도 안 해준다** — 수요는 aes 18 · biriscv 7 · keccak 3 이 전부다 · ⭐⭐ **그런데 5a·5b 는 그 셋도 움직였다**(2026-08-28 · 다섯 번째 변경): 세 프레임 진입점이 **호출마다** 콜리의 로컬 윈도를 IR 에서 다시 지었고(`Vec` 할당 + `locals_len`개 `Value` 생성 + pop 에서 free) 그건 **불변 IR 의 순함수**다 ⇒ 함수당 템플릿 1회 + 캡 있는 free-list · ⭐ 그리고 **로컬이 전부 static 인 함수**(= 평범한 비-`automatic` 베릴로그 함수)는 그 윈도를 짓고 `entry().or_insert()` 에 넘겨 **첫 호출 빼고 전부 버렸다** · 5b = 정적 slab 이 dense `FuncId` 키의 `BTreeMap` 이었다 ⇒ **인덱스 `Vec`**(결정성은 구성상 보장 · 아무도 iterate 안 함) ⇒ keccak **−7.7%** · aes **−6.8%** · keccak-arr −3.5% · sha256 **−3.3%** · picorv32 −2.2% · biriscv −1.4% · serv −0.5% · darkriscv +0.6%(잡음) · ⚠️ **sha256·picorv32 는 `frame_bodies`=0 인데 움직였다** — 그쪽에서 움직인 건 **버려지던 윈도 arm 과 slab 인덱스** 쪽이다 · ⭐⭐ **그래서 6–10주 추정의 자리가 좁아졌다**: 아레나 아래로 잡아 둔 세 슬라이스 중 **둘이 윈도의 표현을 전혀 안 건드리고** 코퍼스 전역 이득을 냈다 — 그 추정이 실제로 사는 곳은 **5c**(평탄 워드 버퍼로 `wprog` 가 프레임 본문을 컴파일)뿐이고, **이 둘 이전에 적힌 수가 아니라 새 프로파일로 다시 가격해야 한다** · ⚠️⚠️ **그 재가격을 했고 결과가 두 방향으로 반대였다**(2026-08-28 · `/usr/bin/sample` · leaf 귀속 · idle 스레드 제외): ⭐ **상한은 내려간 게 아니라 올라갔다** — 프레임 호출 안에서 제네릭 evaluator/`Value` 가 먹는 몫이 **aes 68.0%(상한 3.13×) · keccak_f_arr 60.4%(2.52×) · keccak_f 39.3%(1.65×)** 이고, 프레임 호출 자체의 비중은 aes **88.8%** · arr 82.5% · keccak_f 44.8% ⇒ ⭐⭐ **2.33× 는 keccak_f_arr 한 행에서 나온 수인데 실제 최대는 aes 다**(우리가 안 쓴 서드파티 · 이미 2.22× 로 이기고 있는 행) · ⚠️⚠️ **그런데 다음 표적은 그게 아니다** — `frame_bodies`=0 인 darkriscv(동률)를 처음 재니 **제네릭 evaluator 가 45%**(eval_ctx 16.8 + eval_binary_ctx 6.7 + read_net 5.6 + log_eq 5.2 + clone 3.6 + from_packed 2.8 + mask_top 2.7 + resize 2.4)인데 컴파일된 `WProg::run` 은 **6.8%** 뿐이고, **body 는 13/16 이 able 이다** ⇒ 갭은 body 승인이 아니라 **body 안 표현식의 wprog decline** · 임시 계측(실행 가중 · 커밋 안 함)으로 census: **compile 요청 568k 중 declined 325k(57%)** 이고 **한 노드 종류가 78%** — `Ternary` w=32 가 **252,975 declined / 17,487 ok** · 그 다음이 `BitXor` w=92(width>64 · 20k) · `Eq` w=1 **19,999/0** · `LogOr` w=1 16,645/2,504 · `LogAnd` w=1 **8,746/0** · `Ne` w=1 **2,500/0** · ⭐⭐ **그리고 작업 목록이 작다**: wcache 때문에 `compile` 은 (eid,w,signed) 당 한 번만 도는데 Ternary 의 **실제 거절은 29건**(else-subtree 17 · cond-subtree 6 · then-subtree 4 · **LoadIdx 2**)이고 그 29개가 253k 번 요청된다 ⇒ **Ternary 자체가 아니라 가지가 거절한다** · w=1 비교가 이 설계에서 **한 번도 컴파일 안 된다**(31k 요청 · 0 성공)는 것은 `lw.signed != rw.signed` 게이트이고, wprog 헤더가 이미 *"round-34 리포트가 가져온 모양에서 같은 게이트가 2.9×"* 라고 적어 둔 그 가족이다 · ⚠️ 헤더의 옛 census 는 **picorv32·keccak 두 설계**였다 — 이 프로젝트가 §4.5.369 에서 잡은 바로 그 함정이고, darkriscv·serv 는 그때 없었다 · ⇒ **다음 성능 슬라이스는 5c(아레나)가 아니라 `wprog` 표현식 승인**이다: 29개짜리 목록 · 아레나가 아무것도 안 해주는 셋(sha256·picorv32·darkriscv)을 덮는다 · 6–10주가 아니다 · ✅ **2026-08-29 해결**: 첫 실패 노드로 census 를 좁히니 **84% 가 한 능력**이었다 — `wprog` 가 **좁은 값을 넓은 문맥으로 확장하지 못한다**(비교 피연산자 폭 불일치 273k/325k · 그 아래 `Signal`/`LogNot`/`Select` 의 `sw.width != w` ~26k) ⇒ 좁은 노드를 승인하되 **어떻게는 LRM sizing 규칙이 정한다**: self-determined(리프·select·concat·모든 1비트 결과)는 **제 폭에서 계산 후 변환**, context-determined(`~`·비트연산·`+`/`-`·시프트·`?:`)는 **문맥 폭에서 계산** · 부호확장은 `value::resize_word`(= `Value::resize` 가 쓰는 **그 함수**)를 불러 **두 번째 철자를 안 만든다** · 절단은 여전히 거절 · ⚠️⚠️ **`sw.width < w` 는 *"좁게 접고 넓혀라"* 가 아니다** — 첫 버전이 전부에 적용해 `logic [7:0] s = v[8:11] + 4'd1` 이 **16 대신 0**(15+1 을 4비트에서 접음), 핀 테스트가 잡았다 ⇒ 분류는 연산자 enum 위 **`_`-free match** · **darkriscv −6.2% · serv −2.7% · picorv32 −1.8%**(aes·keccak 은 frame 안이라 flat) · 다이제스트 전부 불변 · **darkriscv 동률 → 1.08× 앞섬** · 배터리 7,960 → **8,225** 승인 + **넓히기 스윕 45,180개** 전부 제네릭과 값 일치 · 리뷰 differential **CLEAN**(~10만 케이스 · 비교는 64×64 전수) · ⭐ 그 fuzz 가 **pre-existing silent-wrong** 하나를 2-연산자로 줄여 냈다 ⇒ **§2 🆕 A** · 남은 상한은 여전히 **호출 레짐 자체**다 — 같은 알고리즘의 flat 철자가 **0.59 s vs 4.07 s = 6.9×**(8.9× 에서 내려옴 · `bench/keccak/RUN.md` · ⚠️ 양끝이 같이 줄어서 각 변경이 시사하는 것보다 **느리게** 닫힌다) · ⚠️ **다만 지는 둘의 공통점은 아직 안 쟀다** — `keccak_f_arr` 는 호출마다 25원소 배열을 짓고 `darkriscv`(0.78×)는 그런 게 없다. **다음 성능 슬라이스의 첫 계측은 darkriscv 여야 한다**(우리가 안 쓴 설계이고, 지는 이유가 아레나 가설과 다를 수 있다) · ⚠️ **arena 가 선행조건임이 가격됐다**: `wprog::compile` 은 **모듈 프로세스 body 에만** 호출되고(`frame_decline=0`), `WProg::run` 은 `arena.buf[slot]` 을 읽는데 **frame local 엔 슬롯이 없다** — 2026-08-28 에 그 이유가 한 줄로 확정됐다: **프레임 윈도는 `Vec<Value>`**(`state/mod.rs:585`)라 슬롯 읽기마다 `w[slot].clone()` 이 72바이트를 복제하고(`frame_eval.rs:281`), `wprog` 의 `Load { vi }` 가 요구하는 **평탄한 u64 쌍이 아니다** ⇒ `arena.frame` 이 프레임 넷을 통째로 declines(`wprog.rs:461`) · POST 프로파일(keccak_f · 5,949 샘플): 제네릭 walk `eval_ctx`+`eval_binary_ctx`+`arith`+`case_eq` **25.1%** · leaf 마다 `Value` 를 짓는 `read_net`+`from_packed` **7.9%** · `Value::clone` **5.0%** vs 컴파일된 `WProg::run` **2.0%** ⇒ 프레임 본문이 컴파일되면 대체되는 몫이 **~38%** · ⭐ `Value::resize` 는 이 프로파일에서 **사라졌다**(직전 1위) (6–10주 · 상한 2.33× — ⚠️ 그 2.33 은 **keccak_f_arr 하나**에서 나온 수다) · bounded 조각 둘이 이미 수확: §4.5.367 part-select 쓰기 **−15.6%**, §4.5.368 no-op `mask_top` 제거 **keccak_f_arr −11.1% · keccak_f −13.2%** · ⚠️ **VCS/Xcelium 은 이 프로젝트가 한 번도 측정한 적이 없다** — 목표를 유지하려면 라이선스 환경에서 코퍼스 single-core 실측을 확보하는 것이 열린 항목이다 |
| **4b** | ✅ **2026-08-28 해결 — 남은 것은 없다.** `native::run::propagate` 가 **델타마다** `Vec` 둘을 새로 할당(picorv32 5.5M · serv 7.0M · sha256 15M 회 · sha256 전체 `Vec` 성장의 **74.9%**)하고 `settle_cont_assigns` 가 `ca_dirty` 용량을 버렸다(`note_change` push 한 줄이 serv 의 **6.2%**) ⇒ 커널 스크래치로 take/restore · **serv −14.6% · sha256 −9.8% · picorv32 −5.0% · darkriscv −4.9%** · ⚠️ 옛 문구가 **파일도 메커니즘도 틀렸다**: `sched/propagate.rs` 는 **interp** 이고 이미 고쳐져 있으며, ⓐ `k_schedule_nba_scalar` 의 `chunks[0].clone()` 은 **할당을 안 한다**(`LvalChunk` = 32바이트 `Copy` · 어느 설계에서도 0.36%) ⇒ ⓐ 기각 · ⓒ 는 별개 항목이 아니라 ⓑ+새 항목으로 **해소** · 잔여 후보(follow-on): `fire_waiters` 의 `Vec<bool>` · `settle_cont_assigns` 의 md-group `vals` (interp 는 둘 다 풀링한다) | 옛 내용 ↓ · **옛 성능 후보**(이미 지어 둔 빠른 경로를 안 부르는 자리 + 델타당 도는 O(설계) 스캔)(⚠️ **2026-08-21 재규정 · 2회 수확**) | ⭐ **§4.5.351(−3.9%) → §4.5.352(−10.5%)** 로 두 번 연속 답이 나왔다. 표적은 코드젠도 스케줄러 재작성도 아니라 **"증명해 놓고 버린 자리"**(플랫 store 3리전 = 블로킹·NBA·settle, **셋 다 완료**)와 **"델타당 함수 안의 O(설계 크기) 스캔"**(§4.5.352 ⓐ = 278.6M 회 헛돌기) 이다 | ⚠️ 옛 문구의 "스케줄러 29%" 는 §4.5.352 이후 더 줄었다 — `settle_cont_assigns` self **8.83% → 1.62%** 실측(분모 맞춘 6 s 창). **다음 후보(POST 프로파일 실측)**: ⓐ `k_schedule_nba_scalar` **4.3%**(NBA 스케줄마다 `chunks[0].clone()`) ⓑ `propagate` **2.5%**(델타마다 `Vec` 셋 — 옛 D6 표적, **크기 미측정**) ⓒ 할당자 잔여. ⛔ `drain_range_diags` 1.8% 는 **이미 재고 기각**(§5.1-bb — 비용은 문장당 호출 자체) · ⚠️ 착수 전 **나눗셈 필수**(rules: 나눗셈은 게이트가 아니라 탐색) |
| **5** | ⛔ **D2-b(저장소 2-state)** | **거부됨** — 트랩이 사다리 하강이다 | 재개하려면 **정확성 거래 없는 방법**을 먼저 찾아야 한다 |
| — | ⛔ **cycle-based 모드** | **거부됨 2026-08-20 · doc-20 M4** — picorv32 비율 **10.32** vs 게이트 1.84(미달 5.6배). 조합 블록이 사이클당 **0.097 회**만 평가되므로 이벤트 구동이 이미 조합 작업의 90.3% 를 건너뛴다 ⇒ cycle-mode 는 **10 배 더 일하고 0.84 배 아낀다** | 재진입 = 블록당 평가/사이클 **≥1** 인 코퍼스가 실수요로 나타날 때 |
| **6** | ⛔ **코드젠 재착수** | **기각됨** — 경계가 ~38%, 천장이 11% | 재개 조건 **하나**: leaf 로드와 2-state 산술을 **생성 코드 안에 인라인**(호출 0)하고 **의미를 두 번 적지 않을** 방법 |

> ~~⚠️ **`bench/sha256` 는 빈 디렉터리다**(2026-08-21 §4.5.352 리뷰 실측).~~ **해소 2026-08-23 (§4.5.369)** —
> secworks/sha256 이 핀된 SHA 로 들어왔고 벤치는 이제 **워크로드 코퍼스 10개**다(`corpus-runner list`).
> 성능 회귀 스윕에 *"keccak·sha256·picorv32"* 라고 적은 옛 문장들은 그때는 실제로 **둘**이었다.
> `crates/sim-engine/tests/perf_baseline.rs` 의 `SHA256_INLINE`/`SHA256_FUNCS` 는 별개로 남아 있고,
> 그 하네스의 **10 형태 중 9 는 연속대입이 0개** 라는 제약도 그대로다.

### ★★ §2 다음 하나 — 착수 브리핑 (**ⓐ = §4.5.364 · ⓑ = §4.5.365 · ⓒ = §4.5.366 으로 완료**)

~~**ⓓ package 스코프 파라미터 셀렉트**~~ — ✅ **RESOLVED §4.5.383**(2026-08-25 · 세 철자가 한 갭 · grids A/B/C **FIXED 39 · REGRESSION 0** · format 29 불변). ⭐ 선행조건으로 적혀 있던 *"패키지 const 테이블이 선언 폭 provenance 를 나른다"* 는 정확했고, 답은 **새 맵 하나**(`pkg_const_range`, 모듈 twin 과 **같은 `param_decl_range_opt`**)였다 — §4.5.382 의 교훈이 여기서는 *찾으면 있다* 가 아니라 *없으면 짓되 같은 생산자로 지어라* 로 적용됐다. ⚠️⚠️ 큐 문구의 절반이 틀렸다: *"런타임 레인은 세 툴 다 이미 맞다"* 는 **zero-LSB 선언에서만** 참이고, `parameter [39:8] B` 는 `pk::B[15:8]` 을 **171** 로 찍었다(두 오라클 52 · exit 0) — 큐가 모르던 silent-wrong. 잔여 = §2 의 새 9번(enum 라벨 셀렉트 · **모듈 스코프도 같다**).

**다음 착수**는 §2 「다음 착수 순서」 표에서 고른다 — 후보 = **9**(enum 라벨 · 2-오라클 · 모듈+패키지 동시) · **6**(static function 이 모듈 net 에 쓴 값이 사라진다 · 2-오라클) · **4**(`$itor` 가 real 인자의 비트를 읽는다). 또는 §3 ⑫(verilog-axi 상수함수 wide 누산기 · 코퍼스가 지목).

**⚠️⚠️ ⓔ was DEMOTED by its own census (§4.5.376).** It was ranked first this morning on two
claims, and the census refuted both.

*Claim 1 — "both oracles run the child's `initial` first."* Re-measured on the exact design
the queue cites: iverilog `aa bb cc dd`, **verilator `01 02 03 04`** — vita's answer.
Confirmed not a dropped write (remove the child's competitor and verilator honours the
parent's hierarchical `$readmemh`). IEEE 1800 §4.7 makes `initial` order explicitly
nondeterministic, so the write-vs-write class is an **oracle split**, and §4.5.372's
cont-assign ruling already treats "verilator sides with vita" as exoneration.

*Claim 2 — "it is the sole blocker of §3 ④."* Not one of ④'s four motivating testbenches has
a competing child load (serv never sets `+firmware=`; picorv32's `wb_ram` gets no `.memfile`;
`axi4_memory` has no load of its own), and serv does not elaborate with OR without ④ — PRE
and POST both stop at the same three §3 ⑦ errors. ⇒ **④ was never blocked, and §4.5.376
re-landed it.**

**What survives of ⓔ** is the READ direction — a parent `initial` reading a child net at t0
gets X where both oracles give the value (10 cells). Real, two-oracle, and **exercised by zero
of the ten corpus workloads**, against a blast radius of every multi-module design, both
backends, and a `format_version` bump to 30. Full statement and fix shape = **§2 row 7**.

⭐⭐ **The lesson is about the queue, not the feature**: a revert writes its reason into the
queue line, the briefing, AND a test docstring at once, so a wrong reason acquires three
corroborating copies within one slice. Re-measure a revert's rationale before building on it —
see ENGINEERING_RULES, *"A revert's reason is a measurement, not a finding."*

~~**ⓐ 구조적 지연의 값 fold 가 리터럴 전용**~~ — ✅ **RESOLVED §4.5.364**(2026-08-22 · 70칸 3-오라클 **FIXED 51 · REGRESSION 0** · 5,785 green · format 29 불변). 큐엔 *"파라미터"* 한 줄이었고 census 는 **레인 셋**(정수 자기결정-unsigned · real · TimeLit)이었다. 잔여 넷 = §2 「🆕 §4.5.364 가 남긴 지연 잔여 넷」 · 곁수확 §3 행 하나. 상세=ARCHIVE §4.5.364.

~~**ⓑ subprogram formal-bind**~~ — ✅ **RESOLVED §4.5.365**(2026-08-22 · 184칸 **FIXED 54 · REGRESSION 0** · 5,796 green · format 29 불변). ⚠️ 착수 census 가 큐 문구의 절반을 반박했다(`%0d` 반올림·등가·automatic 은 이미 정확) 그리고 자리는 **셋이 아니라 아홉 중 셋**이었다. 잔여 넷 = §2 「🆕 §4.5.365 가 남긴 formal-bind 잔여 넷」. 상세=ARCHIVE §4.5.365.

~~**ⓒ 정확히 64비트 문맥의 비교**~~ — ✅ **RESOLVED §4.5.366**(2026-08-23 · 120칸 4-오라클 전이 **ok→wrong 0 · wrong→ok 14 · 오라클 분열 0** · 5,806 green · format 29 불변). ⭐ 규칙은 하나 — *"이 i64 는 마스킹이 정규화할 수 없는 unsigned 값을 나른다"* — 이고 폭 인식 walk 의 **소비자 넷**(순서비교 · `/`·`%` · 두 시프트 · leaf 재해석)이 그것을 묻는다. ⚠️ 착수 census 가 클래스를 *"비교"* 에서 넷으로 넓혔고, 적대 렌즈가 **`>>>` 도 부호 민감**(§11.4.10)임을 잡았다 — 비교 리다이렉트가 그 결함을 **드러내서** 14칸이 correct→silent-wrong 이 될 뻔했다. 잔여 셋 = §2 「🆕 §4.5.366 이 남긴 64비트 unsigned 잔여 셋」. 상세=ARCHIVE §4.5.366.

~~**ⓓ package 스코프 파라미터 셀렉트**~~ — ✅ **RESOLVED §4.5.383**. ⭐ 되돌렸던 시도(*"`package.rs` 의 param 기록 자리에 `param_range` 삽입을 더한다"*)가 왜 안 됐는지도 그 기록이 정확히 적어 뒀다 — **키가 안 닿는다**(패키지 const 는 `pkg_consts`/`pkg_const_meta` 별도 테이블이고 `param_sel_range` 의 스코프 걷기는 `params`/`symbols` 만 본다). 그래서 답은 module 쪽 맵에 밀어 넣는 것이 아니라 **패키지 쪽에 twin 을 두고 `param_sel_range` 가 그것을 묻게 하는 것**이었고, import 는 값 옆에 range 를 **함께** 바인딩한다.

### 스케줄러 축 프로파일 (2026-08-18 · 측정 트리거 실행 결과)

**방법**: release + 심볼(`CARGO_PROFILE_RELEASE_{STRIP=none,DEBUG=1}`) · picorv32 를 `-G CYCLES=1000000`
으로 **12.6 s** 워크로드로 확대 · `/usr/bin/sample vita 10 -wait` · self time · 작업 샘플 6,929
(대기 스레드 제외). ⚠️ **첫 실행은 버려라** — 콜드 캐시가 native 를 1.04 s 로 보이게 했다(안정값 0.51).
재현된 비율: native **0.51** / vm 0.88 / interp 1.39 ⇒ **native/vm 0.58**(기록된 0.60 과 일치).

| 층 | self | 비고 |
|---|---:|---|
| 표현식 평가 | **41.5%** | Phase D 가 작업한 축(`exec_vm::run` 11.8 · `WProg::run` 10.5) |
| **스케줄러** | **29.0%** | **미최적화** — `settle_cont_assigns` 9.2 · `dispatch_body` 7.8 · `k_schedule_nba_scalar` 3.9 · `simulate` 3.7 · `propagate` 2.3 · `wake` 1.6 |
| 쓰기 퍼널 | 17.1% | `write_lvalue_inner` 6.8 · `write_chunk_word` 5.4 · `write_routed` 3.2 |
| 할당자 | 9.3% | **호출자가 스케줄러 쪽이다**: `propagate` 3.3 · `eval_ctx` 2.9 · `simulate` 0.7 · `note_change` 0.7 · `wake` 0.6 |
| 진단 | 1.3% | `drain_range_diags` |

⭐⭐ **코드젠 재착수의 측정 트리거는 발화하지 않았다.** 실제 설계에서도 op 디스패치가 앉은 두 함수
(`exec_vm::run` + `WProg::run`)의 **self 합이 22.3%** 이고 그중 제거 가능한 것은 디스패치 부분뿐이라
§5.1-be 가 잰 **천장 8.9~11.3%** 와 같은 자리다 — **거래는 여전히 11% 벌고 25~38% 내는 것**이다.

⭐⭐ **대신 프로파일이 더 큰 표적을 줬다.** 스케줄러 29% + 그 축의 할당 5.8% ≈ **35%** 가 미최적화이고,
첫 자리는 **D6 과 정확히 같은 모양**이다 — `native/run.rs::propagate` 가 **호출(=델타)마다**
`Vec::new()` 를 셋(`changed`·`woken`·`clocked`) 만든다. ⚠️ 다만 **크기는 아직 안 쟀다**: 이 3.3% 는
할당·해제 자체의 비용이지, 재사용으로 얼마가 회수되는지는 A/B 로 재야 한다(§5.2 착수 의례 3번).

### ⚠️ 외부 round-29 §5 — "Keccak 지배 워크로드에서 native 가 vm 보다 느리다" · **재현 실패, 그리고 벤치 갭은 진짜였다** (2026-08-18)

리포터가 자기 AES/hash 트리에서 잰 값: **SHA-2 지배 워크로드는 native 가 1.09~1.76× 빠른데
Keccak 지배(`+CAVP_KMAC_RUN`)는 native 가 ~5% 느리다**(4쌍 전부 vm 이 빨랐다). 진단은
*"native 가 못 건드리는 비용이 body 실행기 밖에 있고 거기선 tier-3 오버헤드만 남는다"*.

⭐ **지적 자체는 옳았다 — 벤치 8 형태가 전부 절차 바디 지배였다.** `always @(posedge clk)` 안에
산술을 넣은 여덟 샘플은 **한 레짐의 여덟 표본**이지 여덟 레짐이 아니다. 그래서 **두 형태를 더했다**
(`perf_baseline.rs` · 8 → **10**):

| 새 형태 | 무엇 | interp | vm | native | native/vm |
|---|---|---:|---:|---:|---|
| `cont-assign-heavy` | Keccak 스타일 조합 라운드(theta/chi)를 **연속대입**으로 · 바디는 NBA 다섯 | 45.6 | 45.1 | 34.1 | **0.76** |
| `heap-heavy` | string/queue 처닝(레코드 파싱 TB 의 레짐) | 54.9 | 53.3 | 45.6 | **0.86** |

⚠️⚠️ **그런데 부호가 반대다 — 두 형태 다 native 가 이긴다.** 그리고 **왜 이기는지가 리포트의 진단을
반증한다**: 두 형태 모두 **vm/interp ≈ 0.97** = 바이트코드 VM 이 사실상 아무 기여도 안 하는데
(컴파일할 바디가 없다) **native 는 여전히 15~24% 빠르다** ⇒ tier-3 의 이득이 바디 컴파일에서만
오는 게 아니라 **넷/settle 경로(아레나)에서도** 온다. *"바디 밖이면 오버헤드만 남는다"* 는 참이 아니다.

⚠️ **이 저장소의 실물 Keccak 도 같은 답을 준다**(`bench/keccak` · Keccak-f[1600] · `+N=400`):
`keccak_f` **1.97/2.31 = 0.85** · `keccak_f_arr` **4.48/4.81 = 0.93** · `keccak_f_flat`
**0.152/0.242 = 0.63**. 셋 다 native 가 빠르다.

⇒ **리포터의 숫자는 그들의 설계에 대한 측정으로 남기고, 그 메커니즘은 확인되지 않은 것으로 기록한다.**
가를 수 있는 것 = 그들의 설계, 또는 더 가까운 프록시. ⭐ **다음에 볼 자리**: 그 워크로드는 199 초
동안 **0.87 MB 의 CAVP 레코드**를 처리한다 — Keccak 산술이 아니라 **레코드 파싱**(`$sscanf`·파일·
string·queue)이 지배일 수 있다. `heap-heavy` 가 그 레짐의 첫 근사인데 **합성이라 작다**(설계당 큐
하나·문자열 하나). 실물 규모의 파일 구동 TB 형태가 다음 프록시다.

⭐ 부수 소득: **`native/vm < 1.00` 이 8/8 → 10/10** 이 됐고, 이제 그 문장은 **두 레짐**에 대한
진술이다.

### 착수 전 의례 (어느 트랙이든)

1. **census/프로파일을 다시 돌린다** — 앞 슬라이스가 게이트를 움직이면 다음 표적의 숫자도 움직인다(§5.1-p 가 정한 규칙이고 Phase D 에서 **네 번** 값을 냈다).
2. **[ENGINEERING_RULES](ENGINEERING_RULES.md) 를 읽는다** — Phase D 가 규칙 **여덟 개**를 추가했다.
3. 성능 슬라이스면 **A/B 를 두 번** 재고 **대조군**(그 코드를 안 부르는 형태)이 얼마나 움직이는지로 노이즈를 캘리브레이션한다 — ⚠️ 이 기계는 load average 가 5~7 일 때가 있고 그때 ±5% 가 뜬다.

---

## ★★★ 정본 실행 순서 Phase A~D — ✅✅ **전부 완료 (2026-08-12 확정 → 2026-08-17 종료)**

> **실행 기록 전문(3,074 줄 · 슬라이스 59건)은 [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md)
> 로 이관했다**(2026-08-18 · 무삭제 · **§번호 보존**). 코드 주석·커밋의 `§5.1-<x>` 참조는 거기서
> `#### 5.1-` 로 검색한다. **§4.5.x 슬라이스 상세**는 여전히 [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md) 다.

| Phase | 결과 |
|---|---|
| **A** V1 커버리지 | 코퍼스 **6,470/6,470 = 100.00%** · 거부 0 · 발산 0 |
| **B** 빌드 분리 | 기본 백엔드 **native** · `W4030` 교체 경고 · **`oracle` feature** 로 제품 표면이 실행기 하나(**삭제 0**) · 제품 빌드 거부는 치명 `F4004` · CI 축 `build-no-oracle` |
| **C** interp 강등 | interp = **테스트 도구**(제품 표면 아님) · **성능 최적화 영구 제외**가 규칙 |
| **D** 기계어 코드젠 | 벤치 **10/10 에서 native < vm**(착수 때 셋에서 졌고 최악 2.52×) · ⛔ **코드젠은 지어서·배선해서·재서 기각** |

**여기서 살아 있는 판정 둘** — 상세·재개 조건은 [§5.2 재개 지점](#★★★★-52-재개-지점--세션이-끊겼다면-여기부터-2026-08-17) 의 5·6번 행이 정본이다:

- ⛔ **D2-b(저장소 수준 2-state)** — 거부. 트랩이 **사다리 하강**이다. 재개하려면 *정확성 거래 없는 방법*이 먼저다.
- ⛔ **cranelift 재착수** — 기각. 경계가 런의 **~38%** 인데 천장이 **8.9~11.3%** 다(§5.1-be). ⭐ 재개 조건은 **하나**: leaf 로드와 2-state 산술을 **생성 코드 안에 인라인**(호출 0)하면서 **의미를 두 번 적지 않을** 방법. 전제조건은 Phase D 가 이미 만들어 뒀다(leaf = 컴파일 시점 인덱스의 워드 둘 · 산술 = 평범한 정수 연산).

⚠️⚠️ **이 순서를 다시 읽을 사람에게** — Phase D 착수 시 인용하던 *"cont-assign 체인 깊이 24 에서 104×"* 는
**틀린 인용이었다**(§5.1-av 가 실측으로 잡았다: 그 숫자는 `✅ COMB-DEPTH 해결(2026-08-01)` 의 **before**
열이고, dirty-settle 이 이미 배송돼 세 백엔드 모두 선형이다 = **D3 는 착수 전에 이미 끝나 있었다**).
⚠️ **cranelift 를 먼저 얹지 마라** — §4.5.334 census 가 반증한 것이 정확히 그것이다.

## 6. G2 — AI-Agent 친화 OBS 트랙 (SPEC=[preview/19](preview/19-ai-agent-observability.md))

> 완료: OBS-0 스펙 · OBS-1a run.json+results.jsonl(§4.5.73) · OBS-1b coverage.json(§4.5.99) · OBS-2 v1 trace.jsonl(§4.5.100) · OBS-3 stage.jsonl(§4.5.101) · OBS-S0 설계구조 export `--hier-tree`/`--inst-paths`(2026-07-13). teeth=3-way 내부 차분(JSONL≡VCD≡`$display`)+결정성 골든. 틀린 로그=silent-wrong과 동급. 값 인코딩 실태(§4.5.151 문서 정합)=trace `old`/`new`=full-width 4-state binary·stage `vals[]`=`%0d` decimal — doc-19 §3 pin 4에 기록.

| 단계 | 산출물 | 공수 |
|---|---|---|
| OBS-2 잔여 | sva.jsonl(R-L6·SVA property명+support-cone v0) · per-element array probe · class/event probe(no-oracle) | M |
| OBS-1 잔여 | staged/vrun obs(.velab source-identity) · compile-fail manifest · `--seed` | S-M |
| R-L4 | 로그 채널 분리 | M |
| OBS-4 | `vrun --control stdio` JSON-RPC(peek/poke/step/run_until)+poke 저널 replay | L |
| OBS-5 | snapshot/restore/rewind(엔진 상태 postcard 직렬화) | L-XL |
| OBS-6 | X-origin·region-annotated events·정적 backward cone | L+ |

- 비목표: FSDB/UCDB·SQLite 내장·waveform GUI·UVM 연동. VCD는 사람용 유지.

> **✅ E-SPIKE (2026-08-01) — 프로세스 융합이 컴파일드 전환의 enabler 임이 실측 확정.** C-GAIN 곡선이 "VM 수익 = 활성당 작업량의 함수"(1문장 0.99× · 64문장 0.75×)임을 보였으므로, 남은 질문은 **무엇이 활성당 작업량을 올리는가**였다. 답 = **프로세스 융합**. 동일 로직·동일 깊이·동일 사이클을 세 형태로 재비교(`perf_fusion_spike`):
>
> | depth | form | interp | vm | vm/interp | VM 배속 |
> |---|---|---|---|---|---|
> | 24 | instances(포트 cont-assign) | 59.7 ms | 46.8 | 0.78× | 1.28× |
> | 24 | separate(모듈 내 `always_comb` 24개) | 31.8 ms | 20.4 | 0.64× | 1.56× |
> | 24 | **fused(`always_comb` 1개·24문장)** | **21.0 ms** | **9.4** | **0.45×** | **2.22×** |
> | 48 | instances | 143.4 ms | 120.4 | 0.84× | 1.19× |
> | 48 | separate | 73.2 ms | 49.0 | 0.67× | 1.49× |
> | 48 | **fused** | **39.8 ms** | **16.5** | **0.41×** | **2.44×** |
>
> **융합은 두 축을 동시에 민다** — ①인터프리터 자체가 143.4→39.8 ms(**3.6×**, 프로세스 수·델타 수·활성당 오버헤드 감소) ②그 위에서 VM 수익이 1.19×→**2.44×**. 합산 **8.7×**(instances+interp 143.4 → fused+VM 16.5). 곡선은 depth 48 에서도 **포화 전**(0.45→0.41)이라 JIT 여지가 그 위에 남는다. ⇒ **F 기각 판정(잔여 1.26–1.50×)은 융합 전 바디 크기에서 잰 값이므로 ② 이후 재측정 대상.** 등가성: `fused == separate` 가 전 깊이 일치(`instances` 는 stage 모듈이 매 단 `+1`, 나머지는 `+i+1` 이라 설계상 다른 함수 — 값 차이는 의도된 것).
>
> **✅ E-OPP (2026-08-01) — 융합 기회 실측. 실 RTL 형태에는 있고, 인접-프로세스 융합만으로는 못 잡는다.** 기계를 짓기 전에 기회를 재는 규율(§C/§F 를 살린 것과 동일):
>
> | 설계 | 프로세스 | 인접 융합 | 사슬 | **복사-가로지르기** |
> |---|---|---|---|---|
> | inst-chain d=24 (실 RTL 형태) | 26 | **0** | 0 | **23** |
> | separate d=24 (모듈 내 체인) | 26 | 23 | 24 | 23 |
> | sha256 · expr-heavy · examples 4종 | 2–5 | 0 | 0 | 0 |
>
> **인접 융합은 실물에서 0 이다** — 실 RTL 의 단 사이는 `always_comb r` → `assign y = r` → 포트 → 다음 단이라 **중간에 복사 cont-assign 사슬이 낀다**(포트 연결 하나가 복사 1개가 아니라 **사슬**로 내려간다 — 복사 1개만 가로지르는 1차 구현은 여전히 0 이었고, 사슬 끝까지 따라가서야 23 이 나왔다). ⇒ **② 의 유효 형태 = 복사-사슬을 가로지르는 융합**, 인접 융합이 아니다. examples 가 0 인 것은 제약이 아니라 그 설계들이 2–5 프로세스라 **조합 사슬 자체가 없어서**이고, 그런 설계엔 깊이 비용도 없다 — **기회는 비용이 있는 곳에 정확히 있다.**
>
> **② 잔여 공수(복사-사슬 융합)**: 바디 병합(블록 재번호·`Goto` 타겟 오프셋) · 융합된 복사 assign 을 settle 에서 제외 · 활성/wake 부기(융합된 producer 가 독립적으로 깨지 않게) · 출력 바이트 동일 게이트. **엔진 init out-of-band** 라 `SimIr`/format_version 무영향.
>
> **다음 = ② 선택적 프로세스 융합(복사-사슬 형태).** 안전 조건(순서 이동 회피) = P·Q 둘 다 Comb · P 가 net n 을 blocking 쓰기 · **Q 가 n 의 유일한 level 독자** · n 의 blocking writer 가 P 뿐 · P 가 n 외에 쓰지 않음. 이 조건이면 P·Q 사이 델타에 끼어들 관찰자가 없어 융합이 순서를 안 옮긴다. 변환 위치 = **엔진 init(out-of-band)** — `SimIr` 무변경이라 골든/format_version 무영향. G2 제약: `--probe`/계층 VCD/`%m`/계층 참조가 지목하는 net 은 보존(융합해도 중간 net 쓰기는 write 퍼널을 그대로 타므로 VCD 기록은 유지된다).

> **🔴 ② 융합 REVERTED (2026-08-01) — 만들고, 재고, 게이트까지 걸고, 그래도 틀렸다.** 아래 ②/③/④ 기록은 유효하나 **융합 구현은 되돌렸다.** 이유:
>
> | stimulus | iverilog | vita nofuse | vita **fuse** |
> |---|---|---|---|
> | `#1 clk=1; #1 clk=0` (코퍼스 형태) | `00000288` | `00000288` ✓ | `00000288` ✓ |
> | **`clk=~clk; #1`** (init 과 같은 활성) | `xxxxxxxx` | `xxxxxxxx` ✓ | **`0000017c`** ✗ |
>
> **뿌리**: unfused 는 깊이 D 체인이 **D 델타에 걸쳐** 전파되므로, 같은 배치에서 깨어나 체인 **출력**을 읽는 프로세스는 *부분 전파된* 값을 샘플링한다. 융합하면 한 활성에 완주해 그 독자가 *완전 전파된* 값을 본다 → **exit 0·진단 없음·값이 다름 = silent-wrong**.
>
> 구현한 안전 조건은 체인의 **중간 net**(아무도 안 읽음)을 지켰지만 **출력이 언제 fresh 해지는가**를 지키지 않았다. 그런데 그 출력의 독자가 바로 flop — 조합 cone 의 존재 이유다. "출력에 동시 독자 없음"을 요구하면 안전 집합이 빈다 ⇒ **intra-delta 순서를 iverilog 에 핀한 시뮬레이터에서 융합은 의미보존이 아니다.**
>
> ⭐ **게이트가 왜 못 잡았나**: 코퍼스 전 설계가 `#1 clk=1` 형태(초기화와 첫 엣지가 다른 타임스텝)였다. **게이트는 그 안에 든 형태만큼만 강하다.** 반례를 `a_comb_chain_output_is_sampled_mid_propagation` 으로 영구 핀(양 백엔드) — 누가 융합을 다시 지으면 즉시 붉게 뜬다. 코퍼스 체인 템플릿은 유지(독립적 커버리지 개선).
>
> **오너가 승인한 ②(순서 이동·골든 재판정)로도 이건 못 산다** — `$display` 순서가 아니라 **오라클 대비 값 발산**이고, IEEE 는 intra-delta 순서를 implementation-defined 로 두므로 둘 다 합법이지만 vita 의 차분 방법론 전체가 iverilog 일치에 서 있다. 사다리 규칙상 silent-wrong 생성기는 기본 off 여도 실을 수 없다.
>
> **남는 것**: E 축은 닫힌다. 융합 없이는 ③의 f 상승(33→71%)도 없으므로 ④(F 기각)는 **더 강하게** 유지된다.

> **(기록) ②/③/④ 측정 (2026-08-01) — 융합 랜딩 · F 최종 기각 · 다음 레버는 넷 개수다.**
>
> **② 프로세스 융합 랜딩**(`SimOpts.fuse`, 기본 off). 인접 융합(②a)은 실물에서 0 이라 **포트-연결 복사 사슬을 가로지르는 형태**(②b)까지 구현. 전 지점 **출력 바이트 동일**:
>
> | shape | d | interp off→on | VM off→on |
> |---|---|---|---|
> | in-module | 48 | 73.1 → 42.2 (1.73×) | 49.4 → 19.8 (**2.50×**) |
> | instances | 48 | 145.5 → 73.0 (1.99×) | 121.2 → 51.8 (**2.34×**) |
>
> 게이트 = `fused_equals_unfused_over_corpus`(72 설계 × off/on × 양 백엔드: stdout·VCD·summary) + 공허성 teeth + **`fused_instance_chain_matches_iverilog`**(외부 오라클 핀 `acc=0000340c`, iverilog 13 실측). 디버그로 잡은 것 2건: prelude 멤버의 `rearm` 이 사이클마다 Level waiter 를 누수시켜 **2차 슬로다운**(2.1→378.2 ms) · 등가 게이트가 **공허하게 통과**(코퍼스에 융합 대상 0 → 체인 템플릿 추가).
>
> **③ Amdahl 천장 재측정** — 융합이 f 를 두 배 이상 올린다:
>
> | shape | fuse | total | f | 상한 |
> |---|---|---|---|---|
> | in-module 48 | off → **on** | 73.8 → **23.9 ms** | 33.3% → **71.2%** | 1.50× → **3.47×** |
> | instances 48 | off → **on** | 126.2 → **52.8 ms** | 15.2% → **33.1%** | 1.18× → **1.49×** |
>
> **④ F(JIT/네이티브) 최종 기각.** 잔여 여유 = 상한 ÷ VM 실배속:
>
> | shape | VM 배속 | 상한 | **JIT 잔여** |
> |---|---|---|---|
> | in-module 48 | 2.13× | 3.47× | 1.63× |
> | **instances 48 (실 RTL 형태)** | 1.41× | 1.49× | **1.06×** |
>
> **실 RTL 형태에는 사실상 여유가 없다.** 융합 후에도 `total 52.8 / vm 17.5 / fallback 1.2` → **엔진이 64.6%** 다(넷 부기·propagate·NBA). 바디-측 백엔드는 그걸 못 건드린다. ⇒ BACKEND ④(cranelift JIT)는 **닫는다**.
>
> **다음 레버가 특정됐다 = 넷 개수 축소.** 융합은 복사 *assign* 을 없앴지만 중간 **net 자체**는 남는다(48단 × 단당 여러 넷). VCS 계열 flatten 의 나머지 절반이 정확히 이것이다. 단 이건 `--probe`/계층 VCD/`%m`/계층 참조가 지목하는 대상을 지우므로 **G2 목표와 정면 충돌** — 성능 트레이드오프가 아니라 목표 판정 사항이다.

## 7. 조건부 / 장기 (재진입 트리거 충족 시에만 승격 · 정확성과 직교)

| id | 항목 | 트리거 |
|---|---|---|
> **🔴 정정 (2026-08-01, 같은 날) — 아래 "B 가 진짜 레버" 판정은 내 오독이었다. 실제 `try_compile` 로 재니 native-eval 은 이미 97% 를 컴파일한다.**
>
> 아래 인구조사는 `classify_binop` 의 7개 op 를 "지원 집합"으로 읽었는데, 그건 한 경로일 뿐이고 `native_eval::compile` 의 lowering 은 **비교·등가·case-등가·논리이항을 별도 arm 으로 이미 처리한다**(`compile.rs:336`, `:389`). 신규 `sim_engine::native_eval_coverage` 로 **실제 `try_compile` 을 호출해** 재측정:
>
> **picorv32+tb: codegen-able 바디의 assign RHS 691/709 = 97% 컴파일됨.**
>
> ⇒ **빠진 lane 은 병목이 아니다. B 는 다시 저ROI 다.**
>
> **그러면 왜 VM 이 1.04× 인가** — 정정된 답: 컴파일이 되는데도 안 빨라진다는 것은 **비용이 디스패치가 아니라 인터프리터와 VM 이 공유하는 프리미티브**에 있다는 뜻이다. 프로파일이 그걸 지목했다: `Value::resize`/`mask_top`/`from_packed` **30%** + net read **9%**. 그리고 doc-18 의 2026-06-07 항이 **이미 같은 것을 적어뒀다** — *"진짜 지배 비용은 bit-serial 처리 · 인터프리터·VM 공유 경로"*.
>
> ⇒ **F(JIT)도 같은 공유 경로를 상속하므로 상한을 못 가져간다 → F 는 닫힌 채로 유지되고, 이제 근거가 확실하다.**
> ⇒ **실물 RTL 의 레버는 백엔드가 아니라 공유 value/net 프리미티브다.**
>
> ⭐ **교훈(이번 세션 4번째 같은 형태)**: 계측이 자기 형태만큼만 본다. 이번엔 **코드를 읽어서 "지원 집합"을 추론**한 게 오독이었다 — 지원 여부는 **그 함수를 실제로 호출해서** 재야 한다.
>
> <details><summary>(오독이었던 원래 인구조사 — 기록 보존)</summary>
>
> **🔴 2026-08-01 — B 축(native-eval 잔여 lane)의 "저ROI" 판정은 벤치마크가 만든 착시였다. 실물 설계의 진짜 레버다.**
>
> `--backend vm` 으로 PicoRV32+TB 를 돌리고 샘플링하니 **인터프리터의 `eval_ctx` 가 self-time 56%** 였다. VM 이 native-eval 로 못 먹는 식을 전부 트리워크로 넘기고 있다는 뜻. 연산자 인구조사(`perf_real_design_operator_census`):
>
> | op | 개수 | native-eval |
> |---|---|---|
> | Add · Sub | 633 | ✅ |
> | **LogAnd** | **237** | ❌ BAIL |
> | **Eq** | **146** | ❌ BAIL |
> | **CaseEq** | **120** | ❌ BAIL |
> | **LogOr** | **70** | ❌ BAIL |
> | Ne·Shl·Shr·AShr·Lt·Gt·Ge | 31 | ❌ BAIL |
>
> **컴파일 가능 662/1266 = 52%**, 그리고 bail 은 **식 단위**라 트리 어딘가의 `Eq` 하나가 식 전체를 넘긴다 ⇒ 실효 커버리지는 52% 보다 훨씬 낮다. **미지원 604 중 573(95%)이 상위 4개(`&&`·`==`·`===`·`||`)** 다.
>
> ⭐ **왜 "저ROI" 로 판정됐었나** — 이 저장소의 벤치마크가 **native-eval 이 이미 지원하는 연산으로 쓰여 있었다**(`EXPR_HEAVY` = `+` 사슬 · `STRUCT_HEAVY` = select/concat/replicate). **벤치마크는 자기가 재도록 만들어진 것을 쟀다.** 실물 RTL 은 명령 디코드(`insn[6:0] == 7'b0110011`)와 조건 논리가 지배한다.
>
> ⇒ **F(JIT)의 "잔여 2.37×" 는 JIT 기회가 아니라 native-eval 의 빠진 lane 이다.** opcode 4개로 될 일에 JIT 을 사는 구조. **B 를 최우선으로 올리고 F 는 그 뒤에 재평가한다.**
>
> </details>

| BACKEND | ① ~~2-state 별도 모드~~ **기각 (2026-08-01 M1 실측: `unk` 평면 = 비용의 7%, 기준 30%)** — [preview/20](preview/20-cycle-mode-feasibility.md) · cycle-based 는 따라올 이유 없음(융합/levelize 실물 기각) ② PDES BSP 병렬(Amdahl 상한 T4≈2.5x) ③ native-eval 잔여 lane(signed>64·>128bit·sysfunc·real) ④ in-process JIT(cranelift-jit) 스파이크 | ① 대형 RTL 실수요 ② 지속 W≥64+grain≥200ns ③ 저ROI 상시 defer ④ **미평가** — P0a 후보에 없었다(§아래) |

> **🔴 바디-측 백엔드 축 최종 정산 (2026-07-31 C-GAIN 실측) — 축 자체를 닫는다.** G0 이 "C(allow-list 확장)의 **상한**은 2.84–4.24× 로 F 보다 크다"고 해서 **실현치**를 쟀다. C 가 흡수할 바디는 `#delay` 스티뮬러스 = eval-light 이므로, 활성당 작업량 스윕(`perf_work_per_body_crossover`, 엣지 100k 고정)으로 C 를 짓지 않고 답이 나온다: **1–2 문장/활성에서 vm/interp = 0.99×**(무승부 — 활성당 고정비인 레지스터 리스·프롤로그·디스패치가 상각 안 됨), 의미 있는 이득은 **8문장 이상**부터. 스티뮬러스 바디는 1–3 문장이다 ⇒ **C 실현치 = +0.2–0.3%**(sha256 총 37.7→37.6 ms · clock-bound 32.7→32.6 ms) vs 상한 2.84–4.24×. 게다가 resume-PC 상태기계(L)라는 새 silent-wrong 표면을 지불한다. **C = 기각.** ⇒ **A 출하 · B defer · C 기각 · F 기각**으로 바디-측 축은 소진. 남은 레버는 **엔진 축**(24–35% · 이미 두 라운드 수확)과 **스케줄 축**(D levelize · E flatten)뿐. 상세 = [preview/18](preview/18-acceleration-analysis.md) §C-GAIN.
>
> **BACKEND ④ 판정 (2026-07-31 G0 실측) — 스파이크 권고 철회.** `run_body` 스크래치 타이머로 **적격 바디의 wall-clock 비중 `f`** 를 재니(측정 후 되돌림), **클럭드 RTL 은 `f` = 31–55%** 였다(sha256-round 55.1% · clock-bound 31.1%). 어떤 바디-측 백엔드든 상한이 `1/(1−f)` = **1.45–2.23×** 인데 **VM 이 이미 1.15–1.49× 를 회수**했으므로 **F 의 잔여 여유는 1.26–1.50×**. cranelift 의존성 ~30개 + unsafe 표면 + 다세션 공수의 대가로는 안 맞는다 → **④ 는 defer**(재진입 트리거 = `f > 0.85` 인 실수요 워크로드, 예: 바디 안에 루프가 통째로 든 behavioral 연산 — 실측 벤치 3종이 그 형상이라 `f≈100%`). **C(allow-list 확장)가 F 보다 큰 레버**다(부적격 바디를 흡수하면 상한 2.84–4.24×) — 단 그 바디들은 eval-light 라 실현치는 별개로 재야 한다. 남는 **24–35% 는 엔진 작업**(settle·propagate·NBA·VCD)이라 C 도 F 도 못 건드린다. **결론: 78× 급은 바디 실행 속도가 아니라 스케줄 변경(D levelize·E flatten)에서만 나온다.** 상세 표 = [preview/18](preview/18-acceleration-analysis.md) §G0.
>
> **BACKEND ④ 배경 메모 (2026-07-31, 위 판정 이전).** P0a(2026-06-06)가 네이티브 방출을 기각한 근거는 전부 **소스 방출**에 특유하다(런타임 rustc/cc + libloading · host LLVM 재증명). **in-process JIT 은 후보에 오른 적이 없다** — 아키텍처 문서 6곳이 "후속 JIT 백엔드"를 예고만 하고 결정 기록엔 빠졌다. 실측: `cargo info cranelift-jit` → **0.121.2, `rust-version: 1.85.0`**(vita MSRV 정확 일치) · Apache-2.0 WITH LLVM-exception ⊂ vita `MIT OR Apache-2.0` · 순수 Rust(doc-03 §외부 의존성 정책 통과) · 의존성 build.rs 는 이미 허용(규칙은 **vita 자체 크레이트**에만). 결정성 핀 10개 중 F 가 실제로 건드리는 건 **#6 float 표면 동결**(회피법이 P3 계약에 이미 문서화 — JIT 이 그 함수를 직접 호출)과 **#9 unsafe**(`forbid(unsafe_code)` 없음 · prod 에 SAFETY 주석 unsafe 1건) **둘뿐**이다. `SchemaHash`/`format_version`/3-OS 바이트동일/`--locked` 는 무영향(백엔드는 `SimOpts` out-of-band). 최대 자산 = **P5 게이트가 이미 있다** — 세 번째 백엔드가 teeth 를 공짜로 상속. 남는 리스크: P9 allow-list 가 여전히 천장(JIT 도 같은 것을 상속하므로 적중률은 안 는다) · 이득 상한 A+F ≈ 6-15×(78× 는 levelize+flatten 없이 불가). **권고 = 전체 커밋이 아니라 lane 하나 스파이크(M · 2-3세션)로 P5 통과 여부를 사서 판정.**
| VHDL | VHDL 프론트엔드(9-value std_logic 매핑·별도 파서·GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`(포트 strength) | 파형 툴 수요 (FST=**§4.5.149·150 지원** — `$dumpfile("x.fst")`/`-o x.fst`; known-edge=소형 타임테이블 fst-writer [issue #4] loud 거부, preview/07 참조) |
| MVP-CUT | string concat-nonassign · wildcard assoc `[*]` · package internal-import/scoped-call 잔여 · cross-frame disable | 개별 수요 시 |

## 8. 비계획 (영구 비목표 · gap 아님)

- **DEFPARAM**(IEEE deprecated·`#(.param())`로 충분) · **IMPLICIT-NET**(정책=E3010 명시 에러) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM 생태계·unique/priority 다중-match 검사).

## 9. 완료 이력 포인터

- 완료 슬라이스 상세 로그(§4.5.x) = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존).
- **Phase A~D 실행 기록(§5.1-x · ③층 native 백엔드 · 슬라이스 59건) = [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md)** — 2026-08-18 이관(무삭제·§번호 보존). 코드 주석·커밋의 `ROADMAP §5.1-<x>` 는 거기서 찾는다.
- 구 §0~§7 원문 = [ROADMAP_ARCHIVE_2026-07-16.md](ROADMAP_ARCHIVE_2026-07-16.md)(§번호 보존).
- 탄 단위 내러티브·방법론 교훈 = [DEVLOG.md](DEVLOG.md)·ARCHIVE §3.
- 외부 호환성 리포트 1·2차 전말(A1~C1·EXT2 체인) = ARCHIVE §6·§6-2 — **잔여는 위 §3 "외부 리포트 잔여" 3건뿐**.
