# vitamin — 잔여 작업 트래커 (Remaining Work)

> **"goal까지 남은 것"의 상위 스냅샷.** 재계획 시점마다 통째로 갱신한다(과거 판본은 git 이력이 보존).
>
> - **기준(2026-08-24)**: format_version **29** · **5,904 tests green**(+ 제품 형태 lib green · **`VITA_JIT=1` 로도 전 스위트 green**) · CI **3-OS + `build-no-oracle`** green · MsgCode **68** · **MSRV 1.85** · **기본 백엔드 = `native`**.
> - **최신 완료**: **§4.5.374**(**직접-rhs 전용 시스템함수를 식 어디에나 — 참조 구현이 또 트리 안에 있었다** · 코퍼스가 지목한 §3 ③ · **darkriscv 전체 SoC(3,115줄·9모듈)가 처음 돌고 다이제스트가 라이브 iverilog 일치**(`b4a7bb6d411fea85`) · census 42칸 **PRE 41 loud → POST 31 correct · WRONG 0**, 유일 예외였던 `if ($value$plusargs(…))` 는 `lower_branch_cond` 가 **한 계열·한 위치**만 desugar 한 것이라 `!` 하나에 도로 E3009 · ⭐ `hoist/general.rs` 가 이미 같은 변환과 평가-순서 **단일 정본**(`shape()`)을 갖고 있어 그대로 소비 · ⚠️⚠️ **적대 2렌즈 BLOCKING 여섯 + 리뷰 전 내가 잡은 둘, 전부 내 게이트의 구멍** — write 인자 표가 **뒤집힘**(`$fgets`/`$fread` 는 arg 0) · `NoHoist` 자식 읽기가 안 보이는데 노드는 살아남음 · 별칭(`m.a`=`a` · `p::v`) · 미러링하며 조건 누락 · 새 sigil 이 파형에 샘 · temp 를 unsigned 로 지어 `$fgetc!=-1` 이 영원히 참 · 인덱스만 hoist 해 평가 순서 뒤집힘 · ⓕ ⭐⭐ **내가 이 슬라이스에서 쓴 docstring 의 전제가 거짓**(*"fd 상태는 어떤 식도 읽을 수 없다"* — `$feof` 가 읽는다 · **값 의존적**이라 파일 중간 프로브는 초록) ⇒ §4.5.373 의 *"정리로 좁혔으면 전제를 재라"* 가 **한 슬라이스 만에 재발** · differential 렌즈 112칸 스윕 **DIVERGE 0** · examples 4/4·코퍼스 10 PRE==POST) · 직전 = **§4.5.373**(**리덕션 연산자가 상수 도메인에 없다 — 지어서·재서·되돌렸다** · 코퍼스가 지목한 §3 ⑦ · 넣으면 **serv 가 돌고 다이제스트가 오라클과 일치**(코퍼스 7/10 → **8/10**)했지만 **BLOCKING 넷**에 되돌렸다 · ⭐ census 가 진단을 넓힌다(generate 만이 아니라 **여섯 연산자 · 두 철자 전부** 2-오라클 loud)고 좁힌다(**값 walk 하나만** 비어 있다 — 폭은 이미 1, 부호는 이미 unsigned) · ⚠️⚠️ 좁힘을 **정리**로 세웠다: *"창을 넓히면 피연산자 바깥 비트만 더해지니 «어떤 비트든 set 인가»는 불변"* ⇒ `|`/`~|` 만 이름 위에서 허용 — **그 정리의 전제가 거짓이다**: vita 는 파라미터를 **주장한 폭으로 감싸지 않고** 기록해 `parameter A=4'h1; localparam W=A<<4;` 가 **16/32비트**(두 오라클 **0/4비트**) ⇒ 여분 비트가 **안쪽**에 있어 `generate if (|W)` 가 가지를 뒤집는다 ⇒ **이름 위의 리덕션은 여섯 전부 신뢰 불가** · ⭐⭐ 선행조건이 **두 단계**(폭의 declared provenance + **값이 그 폭에서 canonical**)이고 리터럴만 허용하는 반쪽은 **수요 0**(SERV 는 이름을 쓴다) ⇒ 전부 되돌림 · ⚠️ **같은 벽을 세 문으로**(§4.5.371 select 바운드 · concat 폭 · 리덕션) ⇒ 큐 세 줄이 아니라 **벽 하나**로 기록 · **제품 코드 변경 0**) · 직전 = **§4.5.372**(**`$fatal` 이 자기 출력을 못 막았다 — `$finish` 승격은 지어서·재서·되돌렸다** · 코퍼스가 지목한 §3 ⑧ · ⭐ census 가 진단을 반박(*"system task call"* 이라는데 10칸 중 **`$finish`·`$stop` 둘만** 거절) · **실은 것** = §20.10 상 `$fatal` 은 종료하는데 래치를 **문장 경계**에서 처리해 `$display("VAL=%0d", f(7))` 의 출력이 나갔다(pre-existing silent-wrong · task/대입 위치는 원래 정확) · ⚠️ 술어는 **`call_fatal && !finished`** — 래치가 안 지워져 맨 셀로 물으면 `final` 출력을 삼킨다 · ⚠️⚠️ **되돌린 것** = `$finish`/`$stop` 승격(넣으면 verilog-ethernet 이 elaborate 통과) — **BLOCKING 여섯 중 넷이 내 수정의 산물**이고, 마지막이 결정적이었다: 본문을 중간에 멈추면 **반환값이 정의되지 않고** vita **x** / iverilog **55** / verilator **21** 로 **셋이 갈린다** ⇒ 선행조건 기록 후 되돌림) · 직전 = **§4.5.371**(**concat 의 폭을 값에서 추론했다 — count 넓힘은 지어서·재서·되돌렸다** · 코퍼스가 지목한 §3 ② 착수 · **실은 것 = 폭 축**(`{2{32'd2}}` 이 35→**64**, 미override `{2{8'h1}}` 이 32→**16** · 둘 다 pre-existing silent-wrong) · 그 arm 의 게이트 둘은 리뷰가 측정한 뒤 붙었다(§6.20.2 상 **최종 override 값**이 범위를 준다 / `declared_only` 는 **leaf 단위**로 증명해야 한다 — resolver 가 값-추론 폭을 읽으므로) · ⚠️⚠️ **되돌린 것 = count 넓힘**: verilog-axi 를 54→4 까지 내렸지만 BLOCKING 넷 중 셋이 내 수정이 만든 것이고(폭 쌍둥이 미동조 · 상수함수 지역변수 170 · 고쳤더니 shadow 를 지나쳐 43690 · 콜 깊이 재시작으로 **스택 오버플로**), count 축은 상수함수 **스코프**·**깊이**·**provenance** 를 동시에 만족해야 해 폭 축과 **다른 머신러리**다 ⇒ 메커니즘을 §3 ② 에 기록하고 되돌림) · 직전 = **§4.5.370**(**문자열 상수 도메인이 리터럴 전용이었다 — 참조 구현은 소비자가 하나뿐이었다** · 코퍼스가 지목한 §3 ① · ⭐⭐ `const_str_in_scope` 가 이미 이름·패키지까지 풀고 있었는데 **호출자가 하나**(문자열 동등비교)라 아무도 **값**을 안 물었다 · 큐엔 *"삼항"* 한 줄인데 census 는 **도메인 전체** · ⚠️⚠️ **BLOCKING 넷 중 셋이 내 수정이 만든 것**(따옴표 밀수 · **폭을 안 들고 다니는 side map** · `""` 의 NUL 한 바이트 · *"문자열인가"* 가드를 **값 도메인**에 물어 correct→loud, 되돌리니 override 를 삼켜 silent-wrong) · ⚠️⚠️ **재리뷰 BLOCKING**: 게이트 술어 `ty==Implicit` 의 틈으로 **범위 없는 logic/reg/bit**(폭 1비트) 가 빠졌다 — 파서가 `var_kind` 를 기록했다 **버려서** `parameter bit P` 와 `parameter P` 가 구별 불가능 ⇒ 파서 세 줄 · ⭐⭐ **곁수확** `localparam bit N=8'hFF` 가 255/8 → **1/1**(pre-existing silent-wrong) · 27칸 **FIXED 15 · ok→wrong 0** · 14설계 중 12 바이트 동일) · 직전 = **§4.5.369**(**워크로드 코퍼스 — 남이 쓴 RTL 로 값을 매기기** · 성능 판단이 오래 설계 **둘** 위에 서 있었다: picorv32 와 **우리가 재려고 쓴** keccak · 허가적 라이선스 서드파티 **여덟**을 핀된 SHA 로 가져와 오라클로 고정 · ⭐⭐ **첫 수확이 성능이 아니라 정확성**: 여덟 중 **셋이 거절**되고 셋이 **전부 같은 축**(상수 도메인 파라미터 폴딩 — 문자열 삼항·문자열 Ident·정수 replication)인데 **우리 프로브에서 나온 §2 큐엔 그 축이 한 줄**이었다 ⇒ 우리 프로브는 우리가 의심하는 것을, 남의 RTL 은 **우리가 의심하지 않는 것**을 찾는다 · 최소 재현이 축을 **삼항 하나**로 좁혔다(iverilog·verilator 둘 다 `RED`) · **성능은 양방향으로 뒤집혔다** — 서드파티 다섯 기하평균 **1.60×**(sha256 2.89·biriscv 1.88·aes 1.74·picorv32 1.44·darkriscv **0.78**), 일곱 전체 1.30 ⇒ ⭐ **우리가 쓴 keccak 둘이 평균을 끌어내리고 있었다** · ⚠️ 지는 둘의 공통점은 아직 안 쟀다 ⇒ 다음 성능 계측은 **darkriscv** 부터 · ⚠️⚠️ 적대 soundness 렌즈 **BLOCKING 넷, 넷 다 도구가 자기 목적을 배신하는 모양** — ⭐⭐ `Grade::Promoted` 가 **도달 불가능**(exit 코드를 `expect` 옆 자유 필드로 둬서 거절 행이 `1` 을 들고 있었다 ⇒ 갭이 닫히면 **"was loud, now crashes"** 빨간불 = 코퍼스가 존재하는 이유인 그 사건이 거짓 문구로 실패 보고) ⇒ **`Expect::Runs { exit }`** 로 표현 불가능하게 · 하네스가 전부 gitignore 라 **여덟 행이 다른 기계에서 복원 불가**하고 그 실패가 vita 회귀로 채점됨 ⇒ gitignore **allow-list** 전환 · 파이프 미드레인(verilog-axi 가 19,238 B) ⇒ **거절하는 설계를 행업으로 무고** · fmt 실패 · ⚠️ NIT 로 **내가 ENGINEERING_RULES 에 ★★★ 로 적은 "순차 측정은 불가능하다" 가 거짓**임이 반증됐다 · **제품 코드 변경 0** · 도구 = `crates/corpus-runner`(stub → 실물 · 의존성 0) · 상세 = [study/03](study/03-workload-corpus.md)) · 직전 = **§4.5.368**(canonical `Value` 를 재수립하지 말고 **주장하라** · `resize` 의 no-op 팔이 `mask_top` 을 무조건 부르며 이미 성립하는 불변식을 재수립 · 호출부 31곳 중 30이 생산자 ⇒ `debug_assert!(is_canonical())` · ⚠️⚠️ 적대 soundness 가 `$realtobits` 로 불변식을 반증 — ⭐⭐ *"5,812 테스트에서 발화 0"* 은 **커버리지 진술이지 증명이 아니다**) · 직전 = **§4.5.367**(frame part-select 쓰기의 per-bit 루프 → word-parallel · **−15.6%** · ⭐ S0 계측이 **arena 가 진짜 선행조건**임을 가격하고 리뷰의 메커니즘 주장을 반박) · 직전 = **§4.5.366**(상수 도메인이 **정확히 64비트**에서 unsigned 를 잃는다 · 비교를 고치자 `>>>` 의 잠복 결함이 드러남) · 직전 = **§4.5.365**(real actual 이 정수 formal 의 폭을 안 받는다 · **참조 구현이 이미 트리 안에** 있었다) · 직전 = **§4.5.364**(구조적 지연의 값 fold 가 리터럴 전용 · 리뷰가 **내 수정의 수정**을 세 번 잡았다).
> - (이전 세대 슬라이스의 상세 서사는 전부 ARCHIVE 로 이관 — 이 문서는 상위 스냅샷만 둔다.)
>
> - 잔여 상세 목록(정본) = [ROADMAP.md](ROADMAP.md) · 완료 상세 = [ROADMAP_ARCHIVE.md](ROADMAP_ARCHIVE.md)(§번호 보존) · 이력 = [DEVLOG.md](DEVLOG.md) · 실행 큐 = `LOOPROMPT.md` NEXT.
> - **최종 목표**: **G1** = icarus·verilator·xcelium·vcs급 *정확한* RTL 시뮬레이터(correct-or-loud) · **G2** = AI-Agent 친화 simulator(SPEC=[preview/19](preview/19-ai-agent-observability.md)).

## A. 현재 상태 한 줄 요약

> **★★★ 2026-08-17 — Phase A~D 가 전부 끝났다. 다음에 무엇을 할지는 [ROADMAP §5.2 재개 지점](ROADMAP.md) 이 정본이다.** 요약: **A** 커버리지 100.00% · **B** 제품 표면 native 하나 · **C** interp = 테스트 도구 · **D** 성능(벤치 **8/8 에서 native < vm** · 착수 때 최악 2.52×). ⭐⭐ **코드젠은 지어서·배선해서·재서 기각**(런의 ~38% 가 shim · 천장 8.9~11.3% · §5.1-be) ⇒ **성능 축은 수확 체감**이고 **다음 우선순위는 정확성 큐(§2) → loud 승격(§3) → OBS(§6)** 다. ⚠️ 성능을 다시 본다면 **미측정 축은 스케줄러**다(picorv32 비율이 안 움직인 이유는 아직 안 쟀다).
>
- **열린 silent-wrong 의 정본 목록·개수 = ROADMAP §2**(여기 개수를 복제하지 않는다 — 두 번 적으면 하나는 썩는다).
- 외부 리포트 1·2차(EXT2)·round 3~19 = **사실상 완결**(잔여 3건=A2c·NAP·DOC, 전부 no-oracle/docs).
- 나머지 잔여는 **honest-loud=안전**(ROADMAP §3~§5) + **G2 OBS 트랙**(ROADMAP §6).

## B. 다음 착수 후보 (⚠️ **정본은 [ROADMAP §5.2 재개 지점](ROADMAP.md)** — 아래는 그 요약이다)

> ⚠️⚠️ **2026-08-18 정정**: 이 표는 예전에 *"정본 순서·상세 = ROADMAP §1"* 이라고 적혀 있었는데
> **§A 는 §5.2 를 정본이라 적고 있었다** — 포인터가 둘이고 §1 쪽이 썩어 있었다(그 NEXT 0번은
> 2026-08-03 에 완료된 `③층 S1d-4a` 였다). **§1 은 이제 시간 불변 원칙만 갖고, 현재 큐는 §5.2 하나다.**

| # | 항목 | 근거/오라클 |
|---|---|---|
| **1** | **§2 오라클-有 silent-wrong** — ⚠️ **§2 를 위에서부터 읽지 마라**(맨 위 뭉치는 *AST self-폭 패스* 선행조건에 막혀 있다). 착수표 = §2 머리말의 「다음 착수 순서」. **2026-08-22 재census 실측 상위 넷**(~~ⓐ = **§4.5.364**~~ · ~~ⓑ = **§4.5.365**~~ · ~~ⓒ 64비트 unsigned = **§4.5.366**~~ 로 RESOLVED · 각각 잔여는 §2): ⓓ **package 스코프 파라미터 셀렉트**(§4.5.363 잔여) | iverilog + verilator 라이브 차분 |
| **2** | **§3 loud→supported** — ⭐⭐ 착수 순서는 **워크로드 코퍼스**가 정한다(§4.5.369). ①(문자열 삼항 §4.5.370 ✅) 이후 **②·⑦·⑧ 은 셋 다 같은 벽에서 멈췄다** — 지어서·재서·되돌렸고 메커니즘을 큐에 적어 뒀다(②·⑦ = **파라미터 폭의 provenance**, 그리고 ⑦ 이 그 아래 한 겹을 더 팠다: **값이 기록된 폭에서 canonical 하지 않다** / ⑧ = **중간에 멈춘 본문의 반환값**이 세 도구에서 갈린다). ③(§4.5.374 ✅ — 잔여는 조건부·반복 평가 위치와 `$feof`, 셋 다 선행조건을 §3 에 기록) ⇒ **다음 표적 = ④**(계층 unpacked 원소 · serv · 2-오라클) → ⑨(`package.rs` 네 번째 복사본) → ⑥(auto-top). 그다음이 3판 라운드 잔여(§3.1(c)·§3.3·§3.7·§3.11) | ③④ 2-오라클 · ⑤ hand-IEEE |
| **2a** | ⭐ **워크로드 코퍼스 확장** — 지금 **10 중 3 이 거절**이고, 도는 일곱은 전부 cpu 아니면 crypto 다(stream·fabric 이 각각 하나뿐인데 **둘 다 거절 상태**). §3 ①② 가 닫히면 그 편향이 사라진다. **ibex** 가 열리면 코퍼스 첫 SystemVerilog 워크로드 | `corpus-runner list` · [study/03](study/03-workload-corpus.md) |
| **2b** | **§0 승격 큐 T2 잔여 2건** — `real` const-fold(= `int'(<real param>)` 바운드의 선행) · sized-literal enum label | iverilog ✓ 2/2 |
| 3 | ~~**§2 DEEP** — inner NET vs outer PARAM shadow~~ ✅ **P1 로 해소**(2026-08-18 · ARCHIVE §4.5.342 — 답은 name-set 이 아니라 선언 블록의 SPAN · `repeat (LP)` 는 이미 열려 있었다) — 남은 형제 = §4.5.276 후속 ①(`for` trip-count 식별자) · package 변수 clobber | iverilog ✓ |
| 4 | **§6 OBS** — OBS-2 sva.jsonl(R-L6) 또는 OBS-1 잔여(staged obs·`--seed`) | 3-way 내부 차분 |
| 5 | **성능 — 스케줄러 축**(사다리 아래) — 측정 완료: 스케줄러 **29.0%** self + 그 축의 할당 5.8% ≈ **35%** 미최적화 · 첫 표적은 `propagate` 의 델타마다 `Vec` 셋 | 프로파일 실측 2026-08-18 |
| 6 | DEEP-defer 재개(%c/%s UTF-8 pipeline·derived-localparam self-width·`$unit` typedef ②) | 전용 인프라 슬라이스 |
| **0** | **✅✅✅ Phase A~D 전부 완료 (2026-08-17)** — **A** 커버리지 **100.00%**(거부 0) · **B** 제품 표면이 **native 하나**(`oracle` feature · 삭제 0) · **C** interp = 테스트 도구(성능 최적화 **영구 제외**) · **D** 벤치 **10/10 에서 native < vm** + **코드젠 기각**. **실행 기록 = [ROADMAP_ARCHIVE_PHASE_A-D.md](ROADMAP_ARCHIVE_PHASE_A-D.md)** · 해설 = [study/02](study/02-v1-native-coverage.md) | **5,596 green** · clippy 3 구성 0 |

> **순서 주의**: 원칙은 `① 오라클 있는 CRITICAL silent-wrong > ② loud→supported > ③ 전제조건 충족 honest-loud > ④ OBS`([ROADMAP §1](ROADMAP.md)).
> **성능은 이 사다리 위에 올라오지 않는다**(2026-08-17 이후 · Phase D 종료로 옛 "T 단계 최우선" 오너 지시는 소멸).

## C. 잔여 분류 (요약 — 상세=ROADMAP 해당 §)

| 분류 | 항목 수 | 내용 | 정본 |
|---|---:|---|---|
| correct-support 승격 큐 | 6 | **T1 전부 완료** · T2 독립 4 · T3 전제조건 2 | ROADMAP §0 |
| 🔴 silent-wrong 잔여 | ~30 | **오라클-有 7**(part-select 바운드·**net 선언 초기화 fill 폭**·**오름차순 음수 bound 의 포트/서브프로그램/클래스 스코프**·package real·~~구조적 지연~~(§4.5.364 해소 · 잔여 4)·~~real→int formal~~(§4.5.365 해소 · 잔여 4)·block-local package clobber — ~~replication count~~ 는 §4.5.350 해소) + DEEP 5(UTF-8 pipeline·derived-param width·`$unit` typedef·enclosing-const·packed-WIDTH sibling) + 중형 ~20 + 무오라클 3 | ROADMAP §2 |
| honest-loud 잔여 | 35 + **round-28 4건** | string/heap·함수/formal·소형 큐·EXT2 3건·deep 저우선(VCD fidelity·X→real·x/z-fill param) + **§4.5.284 follow-on 4**(`specify` 블록·이벤트 컨트롤 계층참조 실지원·cross-process `disable` no-op·E3010/E3009 file:line 일관성 — 전부 실사용 ASIC 사이트, 오라클 ✓) | ROADMAP §3 |
| SVA/검증 잔여 | 6 | empty-match 융합·N2c full·prop-ref skew 고급형·QUAD default-flip·N4 clocking 잔여·class down-cast | ROADMAP §4 |
| perf/하드닝 | 5 + **T0~T4** | ⭐ **T0~T4 = 측정된 10.7× 청구**(doc-21 §7.3 · VM 커버리지 0% · 프레임 호출 650 ns vs iverilog 375 ns · 함수 지역 배열 원소 쓰기 514 ns vs 24 ns). 기존 5건은 전부 보류 판정(SVA-QUAD flip·FMT-CACHE b·GEN-3X-STR a·QUEUE-MID + **COMB-DEPTH**: 깊이 비용은 iverilog 도 같음이 실측 = vita 결함 아님. levelize 승격은 프로세스 실행 순서 이동을 감수해야 하고 이득 상한 ≈D/2) | ROADMAP §5 |
| G2 OBS | 6단계 | OBS-2 sva.jsonl(M) → OBS-1 잔여(S-M) → R-L4(M) → OBS-4 control API(L) → OBS-5 snapshot(L-XL) → OBS-6(L+) | ROADMAP §6 |

## D. 별도 관리 — 트리거 충족 시에만 승격 (정확성과 직교)

| id | 항목 | 트리거 |
|---|---|---|
| BACKEND | cycle-based 컴파일드 · PDES BSP(T4≈2.5x 상한) · native-eval 잔여 lane | 대형 RTL 실수요 · W≥64+grain≥200ns · 저ROI defer |
| VHDL | VHDL 프론트엔드(GHDL 오라클·E7xxx) | SV plateau + 값도메인 결정 + GHDL 셋업 |
| VCD-EXT | `$dumpports*`(포트 strength) | 파형 툴 수요 (FST=**§4.5.149·150 지원**; known-edge=소형 fst-writer issue #4) |
| MVP-CUT | string concat-nonassign·wildcard assoc `[*]`·cross-frame disable 등 | 개별 수요 시 |

## E. 비계획 — 영구 비목표 (gap 아님)

- **DEFPARAM**(deprecated) · **IMPLICIT-NET**(정책=E3010) · **OOS**(synthesis·waveform GUI·UPF/SDF/DPI-C·shortreal·trireg·UVM·unique/priority 다중-match).
