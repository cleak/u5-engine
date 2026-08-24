# Review Heuristics

Three mechanical checks that between them caught every real defect found in the
2026-08-23 audit pass. All three are counts or greps. **None of them requires
reading the code attentively, and none of the day's real finds came from doing
so** — including the ones found on the specification side, in a file its author
had been working in all day.

Each check catches a claim that is doing a reviewer's job for them.

| Check | Catches | Mechanical because |
|---|---|---|
| Does anything read this? | Contracts that are not real | Reference count |
| Is this byte inside the save window? | Real contracts at the wrong scope | One subtraction |
| Does this name assert something nobody checked? | Wrong owner, polarity, units, provenance | Names decompose into claims |

## 1. Does anything read this?

A contract whose output has no production consumer is not a weakly-supported
contract — it is not a contract at all, and tests asserting it are vacuous.

Two opposite defects present identically as "a well-tested module with a
confident name", and **only the reference count separates them**:

- **Real code implementing an unreal contract.** `combat.md` §7's "post-round
  maintenance pass": a row-major arena sweep whose report both call sites
  discarded, mutating one write-only field. Removed in `60ec07c`; route-smoke's
  493 cases were unchanged, because it had never done anything.
- **Unreal code implementing a real contract.** The active-object eviction and
  prune predicates: correct, published, ~80 test references, and **zero
  production call sites**. The table's occupancy silently diverged from the
  original. Likewise the spell scene allow-mask, fully modelled and never
  enforced, and `cast_dispatcher_gate`, a parallel implementation of the live
  cast gate that had drifted from it.

Run it as routine, not on suspicion. Exclude `tests_inline/`, `test_fixtures.rs`
and `#[cfg(test)]` helpers, or you will drown in false positives.

A sweep that comes back **empty** is the valuable result: it licenses the word
*isolated* ("152 of 154 fields have real consumers"), which is a far stronger
claim about the tree than a careful search finding nothing.

## 2. State the lifetime

Ask of every counter and flag: per-call, per-turn, per-round, or persisted? For
persistence there is a decisive test that depends on nobody's judgement —
**is the byte inside the save window?**

This catches a different failure from check 1: not an invented contract, but a
*real* mechanism hung at the wrong scope. That is the more dangerous of the two,
because the behaviour is present and every local check passes; only the scope is
wrong, and scope is invisible from any single call site.

Worked example: the moon-gate presence counter was described to us first as
per-turn, then as a call-scoped animation counter, before being established as
persistent save-backed state at `SAVED.GAM` offset `0x02E1`. Modelling it
per-call destroys the rise and sink outright; modelling it turn-scoped breaks
save/load round-trip and reloads a gate at the wrong height.

**Prefer deleting dead state to relocating it** on an unverified placement. A
plausibly-named dead field is how the first invention survived review.

## 3. Decompose the name

A name is an assertion that no review ever checks. Reviewers read a name as a
label and check the code beneath it, so false claims in a name live *above* the
level being audited — and they are load-bearing, because the next reader reasons
from the name when the code is unobvious.

`MOONGATE_ANIMATOR_DAYTIME_THRESHOLD` asserted four things: there is an animator
(false — withdrawn in full), it belongs to moongates (false — the mechanism is
the night-time light beacon), it is gated on daytime (false — that beacon runs
only after dark), and the gate is a threshold (true).

Classes to check, worst last:

- **Owner + mechanism.** Does the named subsystem really own it?
- **Polarity/direction.** `MIN`/`MAX`, `FLOOR`/`CEIL`, `DAYTIME`/`NIGHT`. We
  shipped `TORCH_LIGHT_FLOOR` and `LIGHT_SPELL_FLOOR` inverted.
- **Units.** `_RADIUS` is a known-bad class here: the ambient light byte is a
  **squared-distance threshold**, and local light a squared-distance disc, not
  the Chebyshev square first implemented. Unit errors are quiet and produce
  plausible output at every distance; polarity errors at least invert visibly.
- **Provenance.** `PUBLISHED_`, `MEASURED_`, `NATIVE_`, `STOCK_`. The worst
  class, because the other names make claims about the *referent* and this one
  makes a claim about the *evidence* — so a false one suppresses the check that
  would catch it.

### Rename on discovery — especially when the code is right

The intuitive danger ordering is backwards. A wrong name over **wrong** code is
self-consistent and merely wrong; someone eventually notices the behaviour. A
wrong name over **correct** code is a trap primed for the next diligent person,
and it detonates precisely when someone does careful work.

This nearly happened: an agent was briefed to suspect the active-object
per-axis distance test and convert it if the spec disagreed, on the strength of
the lighting precedent. The published contract turned out to specify a **square
window** — both axes tested separately, no disc — so the code was right and only
the `_RADIUS` names were wrong. A disc would have pruned corners the original
keeps. Correct code, a wrong name, a true precedent and an instruction to act:
three of four inputs sound, and the fourth was a name.

Renaming is defusing, not tidying. Do it even when the logic is untouched.

## The scope caveat, and a fourth class

**Negative claims have no local evidence.** A positive claim is establishable
from a fragment — one instruction that writes a byte proves the byte is written,
and no amount of unexamined code unproves it. A negative claim ("nothing reads
this", "no branch is conditional", "this is the only test") is a claim about the
*complement* of what you looked at, so its strength is exactly the boundary of
the scan and nothing else.

All three checks above are negative claims. So:

**A mechanical check settles a claim only within its stated scope, and a check
whose scope is unstated settles nothing.** Cheapness makes a check *runnable*;
stated scope makes its result *mean* something. A one-line grep with an
unstated blind spot is not stronger evidence than careful reading — it is the
same weak evidence wearing a false air of rigour, which is worse, because that
is the shape that stops the next person checking.

State the blind spot when you report the result. Ours: the reference-count sweep
matches direct identifier references across the three crates, so it would miss a
read through a trait object, a function pointer, or a macro-generated call site.
It found two real defects; it is still not a proof of absence.

**A fourth claim class: the cross-reference.** A name claims the referent, a
citation claims the evidence, a test claims the exercise — and a cross-reference
claims *a relationship that nobody checks in either direction*. It is probably
the most common of the four, because a cross-reference does not read as a claim
at all. It reads as context. "The same generator the shrine effect uses" slips
past review that would have caught the same assertion stated plainly.

**A fifth, and the worst: the disclaimer.** A disclaimer claims
**unavailability** — "this is published but does not reproduce, so it is not
available to this workspace" — and it is the only one of the five that
**forecloses** rather than misinforms.

A wrong fact invites correction: someone eventually checks and finds it wrong. A
wrong *"you can't get this"* removes the reason to try. It does not compete with
the truth; it prevents anyone going to look. And it **strengthens with age**,
because its own persistence reads as corroboration — if it were wrong, surely
someone would have noticed by now. Nobody noticed because nobody could.

We carried exactly that on the Spyglass renderer. The table it declared
unavailable fetched in one command, and the sentence had been standing between
the engine and a fully published contract that retracts five separate things we
had built.

**The check must be the same shape as the claim: re-attempt the thing the
disclaimer says is impossible, on a schedule, rather than reasoning about
whether it is still true.** A stale disclaimer is a stale figure whose subject
is a capability instead of a number.

### The silent-discard default

The same foreclosing shape appears in code as a catch-all that skips:

```rust
_ => { /* unrecognised byte - skip it */ }
```

**A missing case cannot announce itself.** Two published control codes were
living in one such arm: `0x89` (raise moral standing) discarded both its shipped
runs, and `0x81` (print the Avatar's name) silently dropped a word from **60**
shipped dialogue lines, rendering "Greetings, , nice". Nothing failed, because
no test asserted rendered text against shipped blobs.

If the thing being dispatched is exhaustive — and that arm's own comment said
the original's dispatcher *is* — then an unrecognised input is a defect in the
implementation, not defensive input, and skipping it is the wrong response. A
defensive default is only defensible when the input is genuinely untrusted;
against a fixed shipped corpus it converts every implementation gap into
silence.

**It does not only hide a missing case — it hides an already-answered one.**
`0x81`'s behaviour was correctly published, word-boundary handling and all, in a
document we were actively building from. The information was *there*. The skip
arm meant nothing ever asked for it. So the cost of a silent default is not
measured in unanswered questions; it is measured in answers nobody fetched.

**Enumerate mechanically, and mind which set you enumerate against.** Checking
handled arms against the *published constant list* can only find cases someone
already thought to name. The complete set is *every value the dispatcher can
receive*, and the gap between those two sets is exactly where an unnamed case
hides.

### Absence of failure is not evidence

Both forms of this failure strengthen with age, in different currencies. A
disclaimer strengthens because its persistence reads as corroboration — if it
were wrong, surely someone would have noticed. A silent-discard arm strengthens
because every day it does not crash reads as evidence it has nothing in it.

Neither is evidence. **A quiet arm is only ever evidence that nothing asked.**

## 4. Which of the things I just changed does this suite actually touch?

A suite reports what it **ran** and is read as reporting what **exists**. Ask of
any green integration run: *which of the changes I just made does this actually
reach?* The answer is "none of them" more often than anyone guesses.

Two features landed on 2026-08-23 and route-smoke's 493 cases reached **neither**
— for two completely unrelated structural reasons:

- **Outdoor creature ranged attacks.** No route ever brings a pirate ship or a
  dragon into range, so the trigger cannot fire. This is *why the absence
  survived*: the subsystem had ~80 unit tests on predicates nothing called, and
  an integration suite that structurally could not notice.
- **The moongate transit presentation.** The harness captures one frame per
  script step and the transit is blocking, so it plays entirely *between* two
  captures. Its one hash-visible side effect is discarded when the warp rebuilds
  state from `PlayOptions`.

In both cases the suite was **correct**. 493 cases really did pass. Nothing was
wrong except the sentence about what that meant.

### The general shape

This is the same defect as the three checks above, arriving from a fourth
direction. A measurement's boundary is invisible in its own output, so the
reader supplies "everything" by default:

| Artefact | Reported | Read as |
|---|---|---|
| A bounded survey without its bound | "found N" | "there are N" |
| A bounded scan without its scope | "found zero" | "there are none" |
| A suite | "493 passed" | "493 things work" |

**Postscript, and the sharpest instance of this page's own rule.** The number in
that table was `493` when this was written. Two features later it was **495** —
and the author of this page went on quoting `493` for hours, in status reports
and in agent briefs, because the figure had been copied forward from an earlier
run rather than re-measured. Two agents caught it independently, from the
assertion in `crates/u5-tui/tests/play_loop.rs`.

The same guard now measures **511** cases; the lesson is unchanged.

Nothing was wrong with the suite. The number was simply **stale**, and staleness
is invisible in a figure the way a boundary is invisible in a measurement. So
the check has a companion: *when did I last actually run the thing I am quoting?*

Every one of those artefacts is accurate. The failure is entirely in the
sentence wrapped around it — which is why none of them can be caught by
re-running the thing that produced it.

**When you add a feature, add the route that reaches it**, or state plainly that
the integration suite does not cover it. A feature whose only coverage is unit
tests over its own helpers is in exactly the position the eviction cascade and
the ranged attacks were in before they were found.

## 5. Does the *reason* transfer, or only the conclusion?

When a contract says **"X cannot happen"**, the claim is about the original's
*structure*, not about its behaviour. An implementation can reproduce the
behaviour exactly and lose the property that made X impossible.

**Worked example.** The spec authors established that the ordering between a
location's marker scrub and the beacon's indoor source harvest **cannot matter**,
and the reason was structural: one 32x32 walk, positions converted to resident
words at load time, and a beacon that never re-reads the map afterwards. All
true, and a correct statement about the original.

It did not transfer. Our harvest re-read `self.grid`, and five of our six
location-entry paths handed it an **already-scrubbed** buffer — and our scrub
rewrites the very byte the beacon looks for. The failure the ordering question
was about was impossible in the original and real in our tree at the same time.

Worse, **the only path reachable in play was the broken one**: the four floors
carrying that tile are lighthouse lantern rooms, and a lantern room is reached
by climbing. The correct entry path was dead code from the player's point of
view.

**So: a safety property of the original is not inherited by an implementation
that reproduces the behaviour but not the structure that made it safe.**

Nothing about matching observable behaviour would have surfaced this. Only
asking *why* the original was safe, and then checking whether that reason still
holds of our construction.

### Consequence for reading a spec

A contract of the form **"X cannot happen"** is more dangerous than one of the
form **"X happens when Y"**, because the reader takes the reassurance and skips
the mechanism. When you meet one, find out *why* — and if the spec does not say
why, ask. Then check whether your own construction preserves it.

The same applies in reverse when writing: publishing "ordering cannot matter"
without publishing the reason hands the next implementer a guarantee they cannot
check.

## Corollaries

- **Internal consistency is not evidence.** Three mutually-agreeing tests that
  all descend from one wrong premise agree about nothing that matters. Verify
  against shipped data (decode the file, hash it) or published spec text — never
  against the neighbouring test.

  The sharper form: two statements corroborate only to the extent that their
  errors are **independent**. So the question to ask of a second source is not
  *"does it agree?"* but **"could it have inherited the first's error?"** If yes,
  the agreement is close to worthless. The same word carries opposite weight
  either side of that line:

  | Agreement | Worth |
  |---|---|
  | Three tests written from one spec section | nothing — they share every premise it carries |
  | Our tests agreeing with our own constants | nothing — same ancestor |
  | Two independently-written scans of the same shipped bytes | strong |
  | Two spec statements about one routine, different triggers, different subsystems, months apart, neither written to check the other | strongest available |

  This is exactly how a green suite certified a chain of three wrong `.TLK`
  constants: every test in the chain descended from the same wrong boundary.
  Conversely it is why a helper could be pinned by matching it against an
  already-published behaviour on an unrelated trigger — the two had no
  opportunity to influence each other.

  Worth hunting deliberately: **cross-trigger, cross-subsystem restatements of
  the same mechanism.** They are the cheapest strong evidence available, and
  they mostly get noticed by accident rather than sought.
- **A green test can pin something that never touches the world.** We had a
  passing assertion that a seed constant equalled `"BRIT.GAM"` — a correct
  string naming a file that does not ship. It could not have failed either way.
- **Audit repair passes against the same rules as the thing they repair.** A
  repair arrives with elevated trust exactly where it is least warranted: it
  touches things already known to be wrong, at scale. Corrections in this
  project have introduced wrong names while fixing wrong names, regressed a
  section via a retracted correction, and leaked private paths into a public
  document. **A correction can breach a boundary the original error didn't.**
- **Ask a contract to name its consumers**, and to say what does *not* read a
  piece of state. A contract that names its consumers is one an implementer can
  falsify.
