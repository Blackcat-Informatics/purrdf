# Issue #1258: purrdf-sparql-eval: a governed evidence-answer entry point with a stop signal

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

`NativeSparqlEngine` has governed entry points for plain SPARQL evaluation: `query_governed_fallible_view` takes a `QueryGovernors` value with a polled stop signal, and a trip returns a typed `BudgetExhausted` outcome with the partial rows. The evidence-answer entry point, `query_evidence_answer_fallible_view` (`crates/sparql-eval/src/engine.rs`, about lines 5105-5237 at the 3.x pin), has no governed sibling and takes no `QueryGovernors` value, so an evaluation that goes through it cannot be cancelled by its caller.

## Request

A governed counterpart of `query_evidence_answer_fallible_view` that accepts a `QueryGovernors` value, including the stop signal, and returns the same typed budget-exhausted outcome as `query_governed_fallible_view` when a governor trips, with the partial evidence answer withheld (never returned as a complete answer).

## Why a consumer needs it

A consumer that serves queries to long-lived clients must be able to cancel an evaluation when its client goes away or when the host is shutting down. Plain evaluations can be cancelled through the governed path. An evaluation that asks for its evidence answer cannot, so it keeps running, and holding its memory, after its caller has gone.

## Acceptance

- A public governed evidence-answer entry point with the stop signal and the same governor set as `query_governed_fallible_view`.
- A test that a tripped stop signal ends the evaluation with the typed outcome and no answer.
- The existing ungoverned entry point is unchanged.


## Comments (3)

### paudley — 2026-11-12T03:19:42Z

## Reply on the governed evidence-answer issue

This request matches what we do. Our query service evaluates every query through the fallible-view door under a governor value whose only member is a stop signal, and the evidence path is the one place where the stop cannot be applied. Answers:

**Which forms.** A governed counterpart of the existing evidence entry point is enough; no new forms and no new visitor. Observation: the service we run exposes SELECT and ASK only and refuses CONSTRUCT and DESCRIBE at the API, and our use of the evidence door reads either a boolean or a solution sequence out of the evaluation inside the existing callback, so the callback shape you already have is what we use. We would not use a cursor-style entry for evidence answers. Inference: the callback needs the outcome and the answer status together, which the current entry point already gives.

**On a trip.** For a stop that means the caller went away or the host is shutting down, withholding the answer is fine. For a trip caused by resource pressure, our callers need the delivered prefix and an evidence status for that prefix, typed as incomplete, rather than nothing: "closed on trust, open on evidence". We would ask that the trip outcome carry the partial solutions and the partial status, with the tripped governor and whether the partial rows are certain, at most, or unknown, as the plain governed path already carries partial rows, and leave withholding to the caller. If the first version withholds, please keep the type able to carry the prefix later. We have not tested what the evidence status would contain for a prefix.

**Per call or shared.** The governor value per call, with the stop signal an `Arc` the caller shares: one operation of ours can make several evaluations under the same stop, and the stop is read at the start of each. The only thing we share across concurrent evaluations is a memory account, and it reaches the evaluator through the view's workspace reservation, not through the governors, so no shared counters are needed in the governor value for us. We use no other ceiling today.

**Fallible door and host exposure.** Only the fallible-view door matters to us; no infallible variant is wanted. We want no exposure through a C ABI or other host bindings: our callers are Rust.

Not checked: how the evidence status behaves for a prefix produced by a non-monotone operator, whether other consumers want other forms, and what per-governor behaviour (fuel, rows, cells) we would want beyond the stop signal, since we run with none.

Note on the trip outcome: this issue as first written asked for the partial answer to be withheld on any trip. For a trip caused by resource pressure we now ask for the delivered prefix with its evidence status, typed as incomplete, as set out above; that replaces the earlier ask. Withholding remains right when the caller has gone or the host is shutting down.


### paudley — 2026-11-12T03:29:11Z

Thank you for the answers. The specification is written. It meets your needs with several points that differ from what the issue first asked, listed here so you can confirm them.

**Met as asked.** A governed counterpart of the existing fallible-view evidence entry point, for SELECT and ASK. Governors per call, with the caller's stop signal shared across evaluations and read at the start of each. No host bindings, no infallible variant, no CONSTRUCT or DESCRIBE.

**Differences.**

1. **The callback's shape.** The entry point takes one closure over an enum. Its `Complete` arm is the existing three-argument callback, so your current callback is unchanged inside it. The other arm carries a trip: the tripped governor, the partial rows with their certainty (`Certain`, `AtMost` or `Unknown`, as on the plain governed path), and an optional prefix result.
2. **A new `finish_prefix`.** A status for a prefix does not exist today: finishing an answer without a complete enumeration is refused unless a verdict stop exists. The specification adds `finish_prefix`, an addition in the core crate. Its status says the answer is as observed over the delivered rows and is typed incomplete.
3. **A prefix and its status are delivered only when the partial rows are `Certain`.** `AtMost` rows can include non-answers, and `Unknown` carries no rows, so for those two the trip carries its bound but no status. An ASK that has found no witness yet is never reported as false.
4. **DISTINCT and REDUCED plans are withheld**, because a later duplicate would merge its evidence into a row that was already delivered. This rests on a reading of the code. The first implementation step is a test that confirms it, and the rule is dropped if the test shows it does not hold.
5. **A stop and a deadline are both withheld.** The prefix is delivered only for a resource trip.

**Question for you.** Please confirm that (a) the enum callback shape is acceptable, (b) delivering the prefix only for `Certain` rows meets your need (an `AtMost` or `Unknown` trip then gives you the bound but no prefix status), and (c) withholding on a deadline as well as on a stop is right for you. If a deadline should deliver the prefix like a resource trip, say so.


### paudley — 2026-11-12T05:21:23Z

The requester has confirmed the points where the spec differs from the request.

- The enum callback shape is acceptable.
- A prefix and status only when the delivered rows are certain is acceptable.
- Withholding the prefix for DISTINCT and REDUCED plans is acceptable.
- Withholding the prefix on both a stop and a deadline is acceptable.

One request: the trip type should be able to carry more later. It will be `#[non_exhaustive]` so that a field or variant can be added without a breaking change.

Not checked on its side: how many of its evidence queries use DISTINCT or REDUCED.

The requester also ranked the two parts. The stop is the part it needs, because a call that cannot be stopped goes on holding memory after its caller has gone. The prefix is secondary.


