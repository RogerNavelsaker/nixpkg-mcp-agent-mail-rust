# Controller-Composition Safety Proof — bd-db300.7.8.4

> Regime overlap rules, kill-switch precedence, timescale separation,
> forbidden feedback loops, and operator safety semantics for the
> performance-program decision plane.

---

## 0. Controllers In Scope

| ID | Controller | Owner bead | Signal domain | Action domain | Timescale |
|----|-----------|-----------|---------------|---------------|-----------|
| C1 | **Admission / Tail Guardrail** | bd-db300.5.4.3 (E4.3) | queue depth, wake-to-run, p95/p99 latency, retry rate, fallback rate | admit / defer / backpressure / shrink_helper_budget / force_safe_mode | **Fast**: 1-10ms decision window |
| C2 | **Adaptive WAL Policy** | D1 track | WAL frame count, sync latency, flush coalesce ratio, checkpoint pressure | per-commit-sync / batched-sync / deferred-sync / force-checkpoint | **Slow**: 100ms-1s observation window |
| C3 | **Placement / Spillover** | E6 track | LLC miss rate, remote-ownership fraction, cross-CCD wakeup rate, helper-lane utilization | pin-local / spill-to-remote / expand-helper-budget / shrink-helper-budget | **Slow**: 500ms-5s observation window |
| C4 | **SSI Abort Policy (DRO)** | bd-3t52f | abort rate, conflict topology, skew estimate, expected loss | standard-SSI / relaxed-SI / aggressive-retry / force-serializable | **Fast**: per-txn decision, 10ms EWMA |
| C5 | **HTM Guard** | bd-77l3t | abort rate (EWMA), CPU probe | available / disabled / user_disabled | **Slow**: 5s cooldown, only in flat combiner |

---

## 1. Regime Axis Definitions

Per G5.5 (regime atlas), the canonical axes are:

| Axis | Values | How measured |
|------|--------|-------------|
| **Concurrency** | c1, c2, c4, c8, c16+ | active writer count |
| **Contention geometry** | disjoint, moderate-overlap, hot-page | conflict-rate / abort-rate |
| **Read/write mix** | write-heavy, mixed, read-heavy | write-fraction over 1s window |
| **Transaction shape** | point (1-2 pages), medium (3-20), scan (20+) | avg pages-per-txn |

A **regime cell** is a 4-tuple. Controllers declare which cells they are **active** in.

---

## 2. Pairwise Composition Analysis

### 2.1 C1 × C2: Admission ↔ WAL Policy

**Shared signals:** flush latency (C1 reads it as tail evidence; C2 controls it).

**Conflict risk: HIGH.**
If C1 observes high p99 and applies backpressure while C2 simultaneously
switches from per-commit-sync to batched-sync (reducing flush frequency),
C1's backpressure starves the batch — fewer commits arrive, so the batch
never fills, sync latency stays high, C1 keeps applying backpressure.
**This is a positive feedback loop → starvation.**

**Timescale separation:** C1 is fast (1-10ms), C2 is slow (100ms-1s). ✓ Separated.
But C1's `shrink_helper_budget` action can change flush throughput within C2's
observation window, causing C2 to observe a transient it didn't cause.

**Composition rules:**
1. **C2 MUST NOT change sync policy while C1 is in `force_safe_mode`.** Safe-mode
   implies the system is already degraded; WAL policy changes add unpredictable
   latency transients. C1's kill-switch takes precedence.
2. **C1 MUST NOT use flush-latency as a direct signal when C2 is mid-transition.**
   Implementation: C2 publishes a `wal_policy_transition_epoch` counter. C1
   discounts flush-latency samples whose epoch differs from the previous sample.
3. **Forbidden combination:** C1=`backpressure` + C2=`batched-sync`. If C2 selects
   batched-sync, C1 must treat it as an implicit capacity expansion and raise its
   admit threshold proportionally to the expected batch size.

**Kill-switch precedence:** C1 > C2. Admission is the outer safety boundary.

### 2.2 C1 × C3: Admission ↔ Placement

**Shared signals:** wake-to-run latency (C1 reads it; C3 controls it via placement).

**Conflict risk: MEDIUM.**
If C3 spills work to a remote NUMA node (increasing wake-to-run), C1 may
interpret the higher latency as overload and apply backpressure. This is
a false positive — the system has MORE capacity, not less.

**Timescale separation:** C1 fast (1-10ms), C3 slow (500ms-5s). ✓ Separated.

**Composition rules:**
1. **C1 MUST distinguish local-wake-to-run from remote-wake-to-run.** If C3 is
   actively spilling, C1's wake-to-run evidence must be partitioned by locality.
   Remote wake-to-run above baseline is expected, not pathological.
2. **C3 MUST NOT expand helper budget while C1 is in `shrink_helper_budget` state.**
   These are contradictory actions on the same resource. C1 precedence wins.
3. **Forbidden combination:** C1=`force_safe_mode` + C3=`spill-to-remote`. Safe-mode
   means retreat to conservative baseline; remote spillover adds latency variance.

**Kill-switch precedence:** C1 > C3.

### 2.3 C1 × C4: Admission ↔ SSI Abort Policy

**Shared signals:** abort rate, retry rate.

**Conflict risk: MEDIUM.**
C4 may relax isolation (SI instead of SSI) to reduce aborts. C1 sees abort-rate
drop and admits more work. But the relaxation hides write-skew anomalies that
accumulate silently. This is not a feedback loop but a **correctness erosion path**.

**Timescale separation:** Both fast (per-txn). ⚠ NOT separated.

**Composition rules:**
1. **C4 MUST NOT relax isolation based on C1's admission state.** The isolation
   level is a correctness property, not a throughput knob. C4 decisions are
   independent of load.
2. **C1 MUST treat abort-rate as an input, not a control target.** C1 cannot
   instruct C4 to reduce aborts — that would be using correctness as throughput.
3. **C1 MAY use abort-rate as evidence for `defer` or `backpressure`,** since high
   aborts under contention indicate the system is above optimal concurrency.
4. **Forbidden combination:** C4=`relaxed-SI` + C1 using abort-rate as admission
   signal. If C4 is in relaxed mode, C1's abort-rate evidence is unreliable
   (artificially low) and must be discounted.

**Kill-switch precedence:** C4 correctness > C1 throughput. If C4 detects write-skew
risk and forces serializable, C1 must accept the resulting abort-rate increase.

### 2.4 C2 × C3: WAL Policy ↔ Placement

**Shared signals:** I/O bandwidth (both compete for it).

**Conflict risk: LOW.**
C2 controls sync frequency; C3 controls where computation runs. They share
I/O bandwidth indirectly but their action spaces don't directly conflict.

**Timescale separation:** Both slow (100ms-5s). ⚠ Overlapping.

**Composition rules:**
1. **C2 and C3 MUST NOT both adjust simultaneously.** One must be the "fast slow"
   (100ms) and the other the "slow slow" (1s+). Assign: C2=100ms, C3=1s.
2. **C3 placement decisions MUST account for C2's current sync mode.** If C2 is
   in per-commit-sync, I/O is already the bottleneck; C3 should avoid remote
   placement that adds I/O latency.

**Kill-switch precedence:** C2 > C3. WAL durability overrides placement optimization.

### 2.5 C2 × C4: WAL Policy ↔ SSI Abort Policy

**Shared signals:** None directly.

**Conflict risk: LOW.**
C2 affects commit latency; C4 affects commit success rate. Independent axes.

**Composition rules:**
1. No special rules needed. These controllers operate on orthogonal concerns.

**Kill-switch precedence:** C4 (correctness) > C2 (performance).

### 2.6 C3 × C4: Placement ↔ SSI Abort Policy

**Shared signals:** None directly.

**Conflict risk: LOW.**
C3 affects where work runs; C4 affects whether work succeeds.

**Composition rules:**
1. No special rules needed.

**Kill-switch precedence:** C4 (correctness) > C3 (performance).

### 2.7 C5 (HTM) interactions

C5 is narrowly scoped to the flat combiner fast-path. Its only action is
to attempt or not-attempt HTM before the combiner lock.

**Conflict with C1:** None. C5 doesn't affect admission.
**Conflict with C2:** None. C5 doesn't affect WAL.
**Conflict with C3:** LOW. If C3 places combiner threads across NUMA nodes,
HTM abort rate may spike (cache-line conflicts). C5's dynamic disable
handles this automatically. No composition rule needed beyond C5's existing
EWMA monitor.
**Conflict with C4:** None. C5 operates below the transaction level.

C5 is **composition-safe by construction** — it has no authority over shared
resources and its failure mode (fall through to lock) is the current baseline.

---

## 3. Kill-Switch Precedence Order

```
  HIGHEST PRECEDENCE
  ┌─────────────────────────────────┐
  │ C4: SSI Abort Policy            │ ← correctness
  │    (write-skew prevention)      │
  ├─────────────────────────────────┤
  │ C1: Admission / Tail Guardrail  │ ← user-visible SLO
  │    (queue/latency/retry safety) │
  ├─────────────────────────────────┤
  │ C2: WAL Policy                  │ ← durability
  │    (sync mode/checkpoint)       │
  ├─────────────────────────────────┤
  │ C3: Placement / Spillover       │ ← throughput
  │    (NUMA/LLC/helper lanes)      │
  ├─────────────────────────────────┤
  │ C5: HTM Guard                   │ ← micro-optimization
  │    (combiner fast-path)         │
  └─────────────────────────────────┘
  LOWEST PRECEDENCE
```

**Semantics:** When a higher-precedence controller's kill-switch trips, all
lower-precedence controllers MUST:
1. Freeze their current state (no new transitions).
2. Revert to their conservative baseline within one observation window.
3. Remain frozen until the higher-precedence controller signals `disarmed`.

**Implementation:** Each controller publishes a `kill_switch_state: {disarmed, armed, tripped}`
in the shared policy snapshot (per G6.4). Before any transition, a controller
checks all higher-precedence kill-switches. If any is `tripped`, the transition
is rejected.

---

## 4. Timescale Separation Matrix

| Controller | Decision period | Observation window | Mandatory gap to next-faster |
|-----------|----------------|-------------------|------------------------------|
| C4 (SSI) | per-txn (~μs) | 10ms EWMA | — (fastest) |
| C1 (Admission) | 1-10ms | 10-50ms sliding | ≥10× faster than C2 |
| C5 (HTM) | per-apply (~ns) | 5s EWMA | — (self-contained) |
| C2 (WAL) | 100ms | 100ms-1s | ≥5× faster than C3 |
| C3 (Placement) | 1s | 1-5s | — (slowest) |

**Rule:** Two controllers that share a signal MUST have decision periods separated
by at least **5×**. If this cannot be guaranteed, the faster controller must
discount samples that overlap with the slower controller's transition window.

**Violation detector:** The shared policy snapshot includes each controller's
`last_transition_epoch`. At decision time, if a peer controller's
`last_transition_epoch` is within the current observation window, the sample
is marked `tainted` and excluded from the evidence vector.

---

## 5. Forbidden Feedback Loops

| # | Loop | Controllers | Mechanism | Why dangerous |
|---|------|------------|-----------|---------------|
| F1 | Starvation spiral | C1 + C2 | C1 backpressure → C2 batch underfill → latency stays high → C1 keeps backpressure | Queue drains to zero, throughput collapses |
| F2 | False-overload retreat | C1 + C3 | C3 spills remote → C1 sees high wake-to-run → C1 backpressure → C3 has less work → C3 retracts spillover → C1 admits → C3 spills again | Oscillation between expand and retreat |
| F3 | Correctness erosion | C1 + C4 | C4 relaxes to SI → aborts drop → C1 admits more → higher conflict → C4 sees write-skew → C4 re-tightens → aborts spike → C1 sheds → cycle | Violation of isolation guarantees during the relaxed window |
| F4 | Dual adjustment chaos | C2 + C3 | Both adjust at same timescale → C2 changes I/O pattern → C3 misattributes → C3 re-places → C2 re-observes | Neither converges; both chase each other's transients |

**Prevention rules:**

- **F1:** C2 MUST NOT enter batched-sync while C1.state ∈ {backpressure, force_safe_mode}.
- **F2:** C1 MUST partition wake-to-run by locality when C3.state = spill-to-remote.
- **F3:** C4 isolation level changes are NEVER influenced by C1 state. C4 is autonomous.
- **F4:** C2 and C3 have 10× timescale separation. C3 freezes during C2 transitions.

---

## 6. Activation-Regime Overlap

Each controller declares which regime cells it is allowed to be active in.
Overlap means two controllers may both be making decisions for the same workload.

| Regime cell | C1 | C2 | C3 | C4 | C5 | Overlap risk |
|------------|----|----|----|----|----|----|
| c1/disjoint/write-heavy/point | active | active | dormant | active | dormant | C1×C2×C4: 3-way. C4 independent. C1×C2 per §2.1 |
| c4/moderate/mixed/medium | active | active | active | active | active | ALL ACTIVE. Highest interference risk. |
| c8/hot-page/write-heavy/point | active | active | active | active | active | ALL ACTIVE. Same. |
| c1/disjoint/read-heavy/scan | dormant | dormant | dormant | dormant | dormant | No controllers active. Baseline. |
| c16+/disjoint/write-heavy/point | active | active | active | active | dormant | 4-way. C5 auto-disabled (high contention). |

**Rule:** In any cell where ≥3 controllers are active, the composition
must be validated via shadow-oracle (per G5.6) before becoming a default.
A divergence from the conservative oracle in any active-3+ cell triggers
the outer kill-switch (C1 force_safe_mode).

---

## 7. Unsafe Compositions — Rejection Table

These combinations MUST be statically rejected by the policy engine.
They cannot be enabled even by operator override.

| # | Composition | Why rejected |
|---|------------|-------------|
| R1 | C4=relaxed-SI + C1 using abort-rate | Abort-rate is unreliable under relaxed isolation; C1 would over-admit |
| R2 | C1=force_safe_mode + C3=spill-to-remote | Safe-mode means minimize variance; remote spillover adds variance |
| R3 | C1=backpressure + C2=batched-sync | Starvation spiral (F1) |
| R4 | C1=shrink_helper_budget + C3=expand-helper-budget | Contradictory actions on the same resource |
| R5 | C2 transition + C3 transition (within same window) | Dual-adjustment chaos (F4) |
| R6 | Any controller transition while higher-precedence kill-switch = tripped | Frozen-state invariant violation |

**Enforcement:** The shared policy snapshot includes a `validate_composition()`
function that checks these rules before any controller commits a transition.
Rejected transitions are logged with `rejected_composition_reason` and the
transition is rolled back to the previous state.

---

## 8. Replay and Interference Scenarios

Each scenario is designed to expose a specific composition failure.

| Scenario | Controllers exercised | What it tests | Pass condition |
|----------|----------------------|---------------|----------------|
| S1: Starvation probe | C1 + C2 | F1 starvation spiral | Throughput > 0 within 500ms of backpressure onset |
| S2: False-overload probe | C1 + C3 | F2 oscillation | ≤2 C3 state transitions per C1 observation window |
| S3: Correctness erosion | C1 + C4 | F3 isolation violation | Zero write-skew anomalies under any C1 state |
| S4: Dual-adjustment | C2 + C3 | F4 convergence failure | Both controllers converge within 5 observation windows |
| S5: Kill-switch cascade | C4→C1→C2→C3 | Precedence chain | All lower controllers frozen within 1 window of C4 trip |
| S6: All-active regime | C1+C2+C3+C4+C5 | c4/moderate/mixed/medium | Shadow-oracle divergence < 1% on 1000-txn workload |
| S7: Regime transition | All | Workload shifts c1→c8 mid-run | No controller oscillation during 1s transition window |
| S8: HTM abort storm | C5 + C3 | C3 remote placement triggers HTM aborts | C5 dynamic-disable within 100ms, no throughput regression vs baseline |

**Artifact:** Each scenario produces a structured log line with fields:
`trace_id, scenario_id, controller_a, controller_b, shared_signal,
activation_regime_overlap, timescale_fast_ms, timescale_slow_ms,
fallback_precedence, kill_switch_precedence, oscillation_metric,
fairness_score, shadow_divergence, rejected_composition_reason`.

---

## 9. Operator Safety Semantics

### 9.1 Default behavior (zero-config)

All controllers start in their **conservative baseline**:
- C1: admit all (no guardrails active until load > threshold)
- C2: per-commit-sync (SQLite-compatible default)
- C3: OS-default placement (no pinning)
- C4: standard SSI (full serializable)
- C5: UNAVAILABLE (Phase 1; no HTM)

No controller activates its adaptive mode without explicit evidence that
the regime benefits from it. The regime atlas (G5.5) defines the activation
frontier: which cells have enough evidence for which controller to activate.

### 9.2 Operator overrides

| PRAGMA | Controller | Effect | Precedence |
|--------|-----------|--------|-----------|
| `fsqlite_admission_mode = {auto, off, force_safe}` | C1 | Manual control | Overrides C1 adaptive; other controllers still respect C1 precedence |
| `fsqlite_wal_sync_policy = {auto, per_commit, batched}` | C2 | Manual WAL mode | Overrides C2 adaptive; checked against C1 kill-switch |
| `fsqlite_placement_mode = {auto, local_only, allow_remote}` | C3 | Manual placement | Overrides C3 adaptive |
| `fsqlite.serializable = {ON, OFF}` | C4 | Isolation level | C4 autonomous; C1 adjusts |
| `fsqlite_disable_htm = {ON, OFF}` | C5 | HTM guard | C5 autonomous |

**Rule:** Manual overrides bypass the adaptive logic but NOT the composition
rejection table. Setting C2=batched + C1=force_safe still triggers R3 rejection.
The operator sees a warning log and the rejected override is not applied.

### 9.3 Degraded-mode contract

When C1 enters `force_safe_mode`:
1. C2 reverts to per-commit-sync.
2. C3 reverts to local-only.
3. C5 reverts to disabled.
4. C4 remains autonomous (correctness is not degraded by safe-mode).
5. All transitions are logged with `control_mode=degraded`.
6. Exit from safe-mode requires C1's evidence vector to be below threshold
   for 3 consecutive observation windows (hysteresis).

---

## 10. Proof Summary

### Theorem 1: No forbidden feedback loop can persist

**Proof:** Each forbidden loop (F1-F4) has a static prevention rule that
blocks the necessary precondition. F1 is blocked because C2 cannot enter
batched-sync while C1 is in backpressure. F2 is blocked because C1
partitions wake-to-run by locality. F3 is blocked because C4 is autonomous
from C1. F4 is blocked by 10× timescale separation. ∎

### Theorem 2: Kill-switch cascade terminates in bounded time

**Proof:** There are 5 controllers in a strict total order. When the
highest active kill-switch trips, each lower controller freezes within
one observation window. The maximum cascade time is the sum of observation
windows: 10ms + 100ms + 1s + 5s ≈ 6.1s. No circular dependency exists
because the precedence order is acyclic. ∎

### Theorem 3: Composition validation is decidable and O(1)

**Proof:** The rejection table has 6 static rules. Each rule checks at most
2 controller states (current + proposed). The `validate_composition()`
function iterates the 6 rules and returns accept/reject. No dynamic
analysis or convergence check is required at decision time. ∎

### Theorem 4: Conservative baseline is always reachable

**Proof:** Every controller has a defined conservative baseline state.
The degraded-mode contract (§9.3) specifies the revert sequence.
Reverting is a single atomic state change per controller (no multi-step
protocol). The kill-switch cascade (Theorem 2) guarantees all controllers
reach their baseline within 6.1s of the highest kill-switch trip. ∎

### Theorem 5: Shadow-oracle divergence gates activation

**Proof:** Per §6, any regime cell with ≥3 active controllers requires
shadow-oracle validation before becoming a default. A divergence > 1%
triggers C1's kill-switch, which cascades to freeze all lower controllers
(Theorem 2) and revert to baselines (Theorem 4). The system cannot
persist in an unvalidated composition. ∎

---

## 11. Assumptions Ledger

| # | Assumption | If violated | Mitigation |
|---|-----------|------------|------------|
| A1 | Timescale separation holds at runtime | Controllers chase each other's transients | Tainted-sample detector (§4) discards overlapping observations |
| A2 | Kill-switch state is published atomically | Lower controllers read stale state | Use seqlock or atomic snapshot for policy struct |
| A3 | C4 is truly autonomous | C4 relaxation creates correctness risk | C4 never reads C1/C2/C3 state; enforced by API design |
| A4 | Observer overhead is negligible | Telemetry itself creates load | Budget telemetry at ≤1% of decision-period duration |
| A5 | Regime cell classification is stable within observation window | Controller acts on wrong regime | Require ≥3 consecutive consistent classifications before activation |
| A6 | Conservative baseline is correct | Baseline itself has bugs | Tested independently via existing conformance suite |
