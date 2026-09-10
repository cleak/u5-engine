#!/usr/bin/env python3
"""Compare the message window of both sides of a paired run, beat by beat.

`game-dev-u5-paired` records `"result": "pass"` when the *run* completed; it
does not compare the two sides. `qa/paired/README.md` said comparison had to be
done by eye because "the engine side cannot be [decoded], at any window size:
the Bevy shell presents its 320x200 frame at the 4:3 display aspect
(`DISPLAY_PIXEL_ASPECT`, 1.20), stretching the 200 rows and letterboxing the
remainder".

That is true of a naive grid, and false of the frame's own geometry. The
letterbox is deterministic: the frame is `4:3` and centred, so a capture of
height `h` holds the 320x200 frame in a `4h/3` by `h` rect at
`x = (w - 4h/3) / 2`. Sampling each glyph cell's own pixels through that map
recovers the same text `IBM.CH`/`RUNES.CH` matching reads off the DOSBox side.

So this compares them. Only the sixteen-by-thirteen message window: the
viewport carries host-clock NPC positions and the wind banner drifts on its own
roll, both of which the README documents as legitimately different.

Reads captures and prints agreement; writes nothing. The decoded text stays in
the process - it is game text, and only the verdict leaves.

Usage: paired_compare.py <ARTIFACT_DIR>...
       paired_compare.py --latest [SCENARIO]...

`--latest` compares the newest artifact directory for each scenario - every
scenario in `qa/paired` when none are named - which is what a suite pass
leaves behind. Runs that captured nothing are reported rather than skipped
silently.
"""

import builtins
import json
import pathlib
import re
import sys

from PIL import Image

ASSETS = pathlib.Path("/srv/u5-clean/assets/gog-1.0-cs-28045")
FONTS = {"ibm": ASSETS / "IBM.CH", "rune": ASSETS / "RUNES.CH"}
LEFT, TOP, COLS, ROWS = 24, 11, 16, 13
# `gameplay_chrome.rs`: the four-frame barber-pole prompt cursor. It animates
# from a free-running counter on both sides, so a cell holding one is not a
# difference.
CURSOR_GLYPHS = {0x05, 0x06, 0x07, 0x08}


def glyph_table(path: pathlib.Path) -> dict[int, tuple]:
    data = path.read_bytes()
    return {
        code: tuple(
            tuple((row >> (7 - bit)) & 1 for bit in range(8))
            for row in data[code * 8 : (code + 1) * 8]
        )
        for code in range(len(data) // 8)
    }


TABLES = {name: glyph_table(path) for name, path in FONTS.items()}


def frame_rect(size: tuple[int, int]) -> tuple[int, int, float, float]:
    """Origin and per-pixel scale of the 320x200 frame inside a capture."""
    width, height = size
    if width == 640 and height == 400:
        # A DOSBox capture is an exact 2:1 downscale with no letterbox.
        return 0, 0, width / 320.0, height / 200.0
    frame_w = round(height * 4 / 3)
    return (width - frame_w) // 2, 0, frame_w / 320.0, height / 200.0


# `text-output.md §10.1`: window 2 is the message window at `(24, 11)`-`(39,
# 23)`. The panel above it, rows `0..10` of the same columns, is window 1 - the
# party roster, the food/gold line and the date. It is as deterministic as the
# message window given one save, and until now nothing compared it: every
# "matches" this tool has ever reported meant *the message window* matched.
PANEL_TOP, PANEL_ROWS = 0, 11


def decode(path: pathlib.Path) -> list[list[str]] | None:
    return decode_region(path, LEFT, TOP, COLS, ROWS)


def decode_panel(path: pathlib.Path) -> list[list[str]] | None:
    return decode_region(path, LEFT, PANEL_TOP, COLS, PANEL_ROWS)


def decode_region(
    path: pathlib.Path, left: int, top: int, cols: int, rows_count: int
) -> list[list[str]] | None:
    image = Image.open(path).convert("RGB")
    width, height = image.size
    if width < 320 or height < 200:
        return None
    origin_x, origin_y, scale_x, scale_y = frame_rect(image.size)
    pixels = image.load()
    rows = []
    for row in range(rows_count):
        line = []
        for col in range(cols):
            bits = []
            for j in range(8):
                for i in range(8):
                    x = int(origin_x + ((left + col) * 8 + i + 0.5) * scale_x)
                    y = int(origin_y + ((top + row) * 8 + j + 0.5) * scale_y)
                    r, g, b = pixels[
                        min(max(x, 0), width - 1), min(max(y, 0), height - 1)
                    ]
                    bits.append(1 if r + g + b > 200 else 0)
            cell = tuple(tuple(bits[j * 8 : (j + 1) * 8]) for j in range(8))
            # `inventory.md §4.4`: the picker draws its selected row "with its
            # ordinary label and padding in inverted glyph pixels", and
            # `text-output.md §5` gives the same inverse flag to the text
            # system generally. Matching only the upright font turned every
            # such cell into whichever glyph scored least badly, so a selected
            # picker row and its neighbours decoded as runs of `<7f>` and the
            # comparison of those rows meant nothing. Score the inverted cell
            # too and keep whichever reads better.
            inverted = tuple(tuple(1 - bit for bit in row_bits) for row_bits in cell)
            best = None
            for name, table in TABLES.items():
                for code, glyph in table.items():
                    score = max(
                        sum(
                            1
                            for j in range(8)
                            for i in range(8)
                            if glyph[j][i] == cell[j][i]
                        ),
                        sum(
                            1
                            for j in range(8)
                            for i in range(8)
                            if glyph[j][i] == inverted[j][i]
                        ),
                    )
                    if best is None or score > best[0]:
                        best = (score, name, code)
            score, name, code = best
            if score < 60:
                line.append("?")
            elif name == "ibm" and code in CURSOR_GLYPHS:
                line.append("~")
            elif name == "ibm":
                line.append(chr(code) if 32 <= code < 127 else f"<{code:02x}>")
            else:
                line.append(f"[r{code:02x}]")
        rows.append(line)
    return rows


# The window's empty cell is glyph code zero, which `decode` renders as its
# escaped form rather than as a space.
BLANK_CELLS = {"<00>", "~", "?"}


def row_text(row: list[str]) -> str:
    """A decoded row as comparable text, with blanks and the cursor dropped."""
    return "".join(" " if cell in BLANK_CELLS else cell for cell in row).rstrip()


# Published equal-probability variant pools. Each group is one draw; a beat
# that differs only by which member was selected is not a conformance failure.
# Decoded rows drop the cell the font renders as `?`, so the entries below end
# where the capture does.
VARIANT_GROUPS: list[tuple[str, ...]] = [
    # `shops.md` §8.B, the arms entry greeting: two equal-probability variants.
    (
        "Hail, friend! Wouldst thou Buy or Sell",
        "Greetings, traveller! Wish ye to Buy, or hast thou wares to Sell",
    ),
    # §8.B, on Buy: "one uniformly selected affirmation".
    ("Very good!", "Excellent!", "Fine, fine!", "But of course!"),
    # §8.B, then "one independently selected stock introduction".
    ("We have:", "We stock:", "Thou canst buy:", "We've got:"),
    # §8.1's stock-call pool, "chosen uniformly", printed after each list.
    (
        "What may I show thee",
        "Which wouldst thou like to see",
        "What is thine interest",
        "Which would ye see",
    ),
    # §8.1's four buy confirmation prompts, "chosen uniformly".
    (
        "Wouldst thou buy one",
        "Wilt thou take it",
        "Wish ye it",
        "May I get one for thee",
    ),
    # The arms sell-entry pool, measured 2026-09-07 and carried by the engine.
    (
        "Which item wouldst thou like to sell",
        "What dost thou wish to sell",
        "Show me what ye got...",
        "What dost thou have for me to buy",
    ),
    # The arms sell-continuation pool, measured the same way.
    (
        "What else can ye offer me",
        "What else hath ye to sell",
        "What else doth thou wish to sell",
        "What other arms wilt thou sell",
    ),
    # The arms sell-browser goodbye pool, the local farewell that ends a
    # browser visit. Carried by the engine as `arms_sell_goodbye`.
    (
        "Good-bye...",
        "Mayhap another time...",
        "Godspeed...",
        "Fare thee well...",
    ),
    # §8's arms no-credit bark pool, "one chosen uniformly".
    (
        "Can't pay?! Out with ye, orc-face!",
        "What be ye trying to pull? OUT!",
        "OUT, SLIME!",
        "BEAT IT!",
    ),
]


def _flatten(rows: list[str]) -> str:
    """The beat's visible text as one wrap-independent string."""
    return " ".join(" ".join(rows).split())


# `shops.md §4` clusters `SHOPPE.DAT` by consumer, and several of those
# clusters are uniform draw pools: an overlay picks one record per visit. Two
# sides with different host-clock seeds draw different records, which is not a
# conformance difference.
#
# The pools are read from a local profile at run time rather than transcribed
# here: they are the original's text and this repository does not carry it.
# Only the published record ranges live in the source.
# `shops.md` §4 clusters `SHOPPE.DAT` by consumer, and those clusters are draw
# pools: an overlay picks one record per visit. Two sides with different
# host-clock seeds draw different records, which is not a conformance
# difference.
#
# The clusters below are per shop kind, because that is the granularity the
# spec publishes. §8.A describes each kind as owning *rows* within its cluster
# - a preamble, an entry greeting and a farewell, each a uniform `0..3` draw -
# but only publishes the row bases for the arms closing bark, whose two halves
# have opposite sentiment and are split here. Modelling the other kinds' rows
# from the engine's own base table was tried and is wrong: `bd-shipwright`'s
# greeting draws from a wider span than any single four-record row, which is
# the open question in `cleak/u5-spec#254`. Until that is published, a
# kind-wide cluster is what the evidence supports.
SHOPPE_POOLS = {
    # The two arms closing-bark rows. `shops.md` line 693: "a visit that bought
    # nothing draws from a curt no-sale row, a completed purchase draws from a
    # courteous farewell row". Measured 2026-09-10: a paid-up
    # `shop-arms-buy-confirm` visit closed on record 7 and the no-purchase
    # visits have only ever drawn `0..3`. Separate pools, so a cross-row draw
    # stays a real difference - it was a real engine defect.
    "arms-bark-nothing-bought": range(0, 4),
    "arms-bark-purchase-completed": range(4, 8),
    # `shops.md` §8.1: an ordinary sellable selection draws "uniform `0..7`
    # over `SHOPPE.DAT` records `49..56`".
    "arms-sell-back": range(49, 57),
    # The tavern, meal and sage band. This was missing entirely, so every
    # tavern arrival greeting - §8.A's "one shared tavern greeting record
    # selected uniformly from `57..60`" - read as a wording difference.
    "tavern-sage": range(57, 92),
    "horse-trader": range(92, 105),
    "ship-broker": range(105, 127),
    "reagent": range(127, 148),
    "guild": range(148, 165),
    "healer": range(165, 174),
    "innkeeper": range(174, 194),
}

ASSET_PROFILE = pathlib.Path.home() / ".local/share/u5/engine/codex-seed"
_SIGNATURES: dict[str, list[tuple[str, ...]]] | None = None


# A run shorter than this is noise. A record identified by several runs at once
# gets the lower floor, because the conjunction supplies the specificity that a
# single run needs length for.
_SIGNATURE_FLOOR = 8
_CONJUNCTION_FLOOR = 5

# Ordinary Ultima phrasing that occurs in prompts and resident literals, not
# only in the pools. A record whose *only* handle is one of these cannot be
# told apart from a window that contains the phrase for another reason, and
# excusing a beat on that basis would mask a real wording difference. Such a
# record simply has no usable signature, exactly as a too-short one has none.
# A record identified by a conjunction of runs is not subject to this: the
# other runs already rule out the coincidence.
_GENERIC_RUNS = frozenset(
    {
        "wilt thou",
        "use them",
        "do for thee",
        "can i show",
        "anything else",
    }
)


def _record_signature(text: str) -> tuple[str, ...]:
    """The plain-letter runs that together identify one pool record.

    A record carries `@`/`#`/`$` substitution placeholders and its own line
    breaks, so it never matches the wrapped window text directly; the runs
    between those are stable under both. The original rule took the single
    longest run and required 14 characters of it, which silently dropped every
    short record - `SHOPPE.DAT`'s `Harrumph!` bark among them - so a visit that
    drew one reported the pool draw as a wording difference (measured
    2026-09-10 on `shop-arms-menus/exit`). Length alone is the wrong axis: some
    short runs are perfectly distinctive, some long ones are stock phrases, and
    several runs together are specific however short each one is.
    """
    runs = sorted(
        {
            " ".join(run.split())
            for run in re.split(r"[^A-Za-z ]+", text)
            if len(" ".join(run.split())) >= _CONJUNCTION_FLOOR
        }
    )
    if len(runs) >= 2:
        return tuple(runs)
    solo = [
        run
        for run in runs
        if len(run) >= _SIGNATURE_FLOOR and run.casefold() not in _GENERIC_RUNS
    ]
    return tuple(solo)


def _pool_signatures() -> dict[str, list[tuple[str, ...]]]:
    """Per cluster, the identifying fragments of each record.

    A record carries `@`/`#`/`$` substitution placeholders and its own line
    breaks, so it never matches the wrapped window text directly. The longest
    run of plain letters and spaces is stable under both.
    """
    global _SIGNATURES
    if _SIGNATURES is not None:
        return _SIGNATURES
    _SIGNATURES = {}
    source = ASSET_PROFILE / "SHOPPE.DAT"
    if not source.is_file():
        return _SIGNATURES
    records = source.read_bytes().split(b"\x00")
    for name, span in SHOPPE_POOLS.items():
        signatures = []
        for index in span:
            if index >= len(records):
                break
            text = records[index].decode("latin-1")
            usable = _record_signature(text)
            if usable:
                signatures.append(usable)
        if signatures:
            _SIGNATURES[name] = signatures
    return _SIGNATURES


def _same_pool_different_record(left: str, right: str) -> bool:
    """Did the two sides draw different records from one published pool?"""
    for signatures in _pool_signatures().values():
        # A record counts as drawn only when *every* one of its usable runs is
        # present, so a lone generic fragment cannot stand in for a record that
        # has more to it.
        hit_left = {sig for sig in signatures if all(run in left for run in sig)}
        hit_right = {sig for sig in signatures if all(run in right for run in sig)}
        if hit_left and hit_right and hit_left != hit_right:
            return True
    return False


def _canonical(text: str) -> str:
    """Replace every published variant with a token for its pool.

    One beat can spend several draws at once - `shops.md` §8.B's Buy entry
    prints an affirmation and a stock introduction, and §8.1 then adds the
    stock-call question, three independent uniform choices in one window - so
    this canonicalises all of them rather than trying one substitution.
    Longest first, so a short entry cannot eat part of a longer one.
    """
    for index, group in enumerate(VARIANT_GROUPS):
        for member in sorted(group, key=len, reverse=True):
            text = text.replace(member, f"<v{index}>")
    return text


def variant_only(stock: list[str], engine: list[str]) -> bool:
    """Do the two sides differ only by which published variants were drawn?"""
    left, right = _flatten(stock), _flatten(engine)
    if left == right:
        return False
    if _same_pool_different_record(left, right):
        return True
    canon_left, canon_right = _canonical(left), _canonical(right)
    # Both sides must actually carry a variant token, or two unrelated windows
    # that happen to canonicalise alike would be excused.
    return canon_left == canon_right and "<v" in canon_left


def classify(left: list[list[str]], right: list[list[str]], rows: list[int]) -> str:
    """Why a beat differs, so a suite pass can be triaged without eyeballing it.

    The buckets are the ones that actually recur. `stock-idle` and `engine-idle`
    are not conformance differences at all: one side simply had not reached the
    beat when the shot was taken, which the walk-up scenarios do routinely
    because NPC positions run off the host clock. `offset` is the same text a
    row or two adrift - a row-accounting difference, not a wording one. `cursor`
    is a single trailing cell. Only `text` is a wording difference.
    """
    stock = [row_text(row) for row in left]
    engine = [row_text(row) for row in right]
    if not any(stock):
        return "stock-idle"
    if not any(engine):
        return "engine-idle"
    # The differing-row list is built from raw cells, so a row whose only
    # disagreement is where the input cursor sits lands in it. `row_text`
    # already drops the cursor glyph, so once every differing row reads the
    # same the beat's *wording* agrees and only the cursor is misplaced.
    if all(stock[index] == engine[index] for index in rows):
        return "cursor"
    # Two sides can also be in different *places*: a walk-up scenario whose
    # NPC did not reach the counter on one side leaves that side in the world
    # loop pressing its scripted shop keys as world commands, and every beat
    # after it disagrees on every row. That is a scenario-reliability problem,
    # not a wording one, and counting it as `text` overstates the conformance
    # queue. Rows the two sides share are the signal: a real wording difference
    # still has most of the window in common.
    # `systems/prng.md` §3: the generator is seeded from the host clock at the
    # intro menu, so a line the game *chooses at random* cannot be expected to
    # agree between the two sides. A beat whose two sides differ only by which
    # published equal-probability variant was drawn is `variant`: the engine
    # printed a legal line, just not the one the original happened to draw.
    #
    # This runs before the `diverged` heuristic below, because two greeting
    # templates from one pool can share almost no rows and would otherwise be
    # reported as the two sides being in different places. `bd-shipwright`
    # read `diverged 4` that way while both sides were in the same shop
    # drawing different records from the published shipwright band.
    if variant_only(stock, engine):
        return "variant"
    # The window is `ROWS` tall, so a transcript that is out of step can be
    # adrift by almost all of it - an 8-row shift turned up in the arms shop,
    # and probing only +-3 reported it as a wording difference. Nearest shifts
    # first, so the smallest explanation wins.
    for shift in sorted(
        (s for s in range(-(ROWS - 1), ROWS) if s), key=lambda s: (abs(s), s)
    ):
        lo, hi = max(0, shift), min(ROWS, ROWS + shift)
        window = range(lo, hi)
        if not any(stock[index] for index in window):
            continue
        if all(stock[index] == engine[index - shift] for index in window):
            return f"offset{shift:+d}"
    # The window is a scrolling stream. Two sides that printed the same text
    # but are showing a different amount of it - because one spent a row the
    # other did not, or because a length-changing variant re-wrapped a line -
    # disagree on every row without disagreeing on a word. The row-shift probe
    # above only catches that when the wrap is identical, so compare the
    # flattened streams too: if one side's visible text is a contiguous run of
    # the other's, the content agrees and the row accounting does not.
    left, right = _flatten(stock), _flatten(engine)
    if len(left) >= 24 and len(right) >= 24:
        # Containment is tested on the *canonicalised* streams, so which side
        # is shorter must be decided there too. A long variant draw collapses
        # to a short token - `shops.md` §8.B's greeting variant 2 is 63 raw
        # characters and becomes 4 - which can invert the raw-length order and
        # run the containment test backwards. That reported the whole arms
        # sell-keys family as `text` when it was one greeting coin re-wrapping
        # the scrollback. Try both directions rather than guessing an order.
        # Compared without spaces. The decoder drops the cell the font renders
        # as `?`, so a pool member that ends in a question mark leaves a gap
        # where one that does not leaves none - `...to sell "` against
        # `...got..."` - and that one-space difference broke containment on
        # beats whose letters agreed exactly. Letter order alone is still
        # strong evidence, and the row accounting is what `scroll` reports on.
        canon_left = _canonical(left).replace(" ", "")
        canon_right = _canonical(right).replace(" ", "")
        if canon_left in canon_right or canon_right in canon_left:
            return "scroll"
        # Containment fails when the row that scrolled part-way off the top is
        # itself a variant: only a *fragment* of the drawn member is left, and
        # a fragment is not what `_canonical` replaces, so the two heads disagree while
        # everything since agrees. The window is a scrolling stream, so the
        # tail is what both sides most recently printed - if they share most of
        # it, the content agrees and only the row accounting does not. A real
        # wording difference in the newest text shortens the shared tail, which
        # is why this is a proportion of the whole rather than a fixed length.
        # The newest row can also be cut off by the window edge on one side
        # only, when that side drew the longer member of a pool - the original
        # showed `...to sell?" ` and this engine's longer draw ran out of row
        # before its closing quote. That is a truncation, not a wording
        # difference, so the trailing punctuation is not part of the tail.
        canon_left = canon_left.rstrip('".,')
        canon_right = canon_right.rstrip('".,')
        shortest = min(len(canon_left), len(canon_right))
        shared_tail = 0
        while (
            shared_tail < shortest
            and canon_left[-1 - shared_tail] == canon_right[-1 - shared_tail]
        ):
            shared_tail += 1
        if shared_tail * 2 >= shortest:
            return "scroll"
    # Two sides can also be in different *places*: a walk-up scenario whose
    # NPC did not reach the counter on one side leaves that side in the world
    # loop pressing its scripted shop keys as world commands, and every beat
    # after it disagrees on every row. That is a scenario-reliability problem,
    # not a wording one, and counting it as `text` overstates the conformance
    # queue. Rows the two sides share are the signal: a real wording difference
    # still has most of the window in common.
    #
    # This runs *after* the benign explanations above, not before them. A
    # variant draw that re-wraps the scrollback moves nearly every row, so the
    # shared-row fraction collapses and two sides standing in the same shop
    # printing the same thing were being reported as being in different places
    # - five of `shop-arms-sell-flow`'s eight beats read `diverged` that way.
    # Divergence is what is left when nothing benign fits, so it is tested
    # last.
    stock_lines = {line for line in stock if line}
    engine_lines = {line for line in engine if line}
    if stock_lines and engine_lines:
        shared = len(stock_lines & engine_lines) / len(stock_lines | engine_lines)
        if shared < 0.2:
            return "diverged"
    if len(rows) == 1:
        a, b = stock[rows[0]], engine[rows[0]]
        if a.rstrip() == b.rstrip():
            return "cursor"
        if abs(len(a) - len(b)) <= 1 and (a.startswith(b) or b.startswith(a)):
            return "cursor"
    return "text"


# One side had not reached the beat when the shot was taken. Not a difference.
IDLE_KINDS = {"stock-idle", "engine-idle"}


# Decoding a beat matches every cell against two fonts, so a suite-wide pass
# costs minutes. An artifact directory never changes after its run finishes, so
# its verdict is cacheable: keyed by the directory name and the newest capture's
# timestamp, a re-run of the same artifacts is a lookup. This is what makes the
# suite total a query rather than a three-hour sweep - and the sweep is not only
# slow but *less* accurate, because the machine is under load throughout and the
# DOSBox side boots late more often.
CACHE = pathlib.Path.home() / ".cache/u5-qa/paired-compare.json"


def _cache_load() -> dict:
    try:
        return json.loads(CACHE.read_text())
    except Exception:
        return {}


def _cache_save(cache: dict) -> None:
    try:
        CACHE.parent.mkdir(parents=True, exist_ok=True)
        CACHE.write_text(json.dumps(cache))
    except OSError:
        pass


def _stamp(artifact: pathlib.Path) -> str:
    newest = max(
        (p.stat().st_mtime for p in artifact.glob("*.png")), default=0.0
    )
    return f"{newest:.0f}"


def compare_cached(artifact: pathlib.Path, cache: dict) -> tuple:
    key = artifact.name
    stamp = _stamp(artifact)
    hit = cache.get(key)
    if hit and hit.get("stamp") == stamp and hit.get("version") == CACHE_VERSION:
        for kind, count in hit["kinds"].items():
            KINDS[kind] = KINDS.get(kind, 0) + count
        # Replay the per-beat lines too. Without them a cached pass cannot be
        # used to *find* anything - only to total it - and the first thing I
        # wanted from the cache was which beats carry a given classification.
        for line in hit.get("lines", []):
            print(line)
        return tuple(hit["totals"])
    before = dict(KINDS)
    buffer: list[str] = []
    real_print = builtins.print

    def capturing(*args, **kwargs):
        text = " ".join(str(a) for a in args)
        buffer.append(text)
        real_print(*args, **kwargs)

    builtins.print = capturing
    try:
        totals = compare(artifact)
    finally:
        builtins.print = real_print
    kinds = {k: KINDS[k] - before.get(k, 0) for k in KINDS if KINDS[k] - before.get(k, 0)}
    cache[key] = {"stamp": stamp, "totals": list(totals), "kinds": kinds,
                  "lines": buffer, "version": CACHE_VERSION}
    return totals


# Bump when a classifier change would alter a cached verdict.
CACHE_VERSION = 15


# Some scenarios are explicitly a lottery: their own headers say so. The night
# encounter roll of `encounters.md` §2.1 runs every overworld turn, so a run
# where one side meets a creature and the other does not "has measured nothing
# and must be re-run" - `night-cast` says exactly that. Scoring such a run as a
# wording difference counts the dice, not the engine.
LOTTERY_MARKERS = ("lottery", "measured nothing", "must be re-run")


def is_lottery(scenario: str) -> bool:
    path = SCENARIOS / f"{scenario}.tsv"
    try:
        header = [l for l in path.read_text().splitlines() if l.startswith("#")]
    except OSError:
        return False
    text = " ".join(header).lower()
    return any(marker in text for marker in LOTTERY_MARKERS)


def compare(artifact: pathlib.Path) -> tuple[int, int, int, int, int]:
    record = json.loads((artifact / "record.json").read_text())
    scenario = record.get("scenario", artifact.name)
    same = differ = skipped = idle = panel = 0
    for capture in record.get("captures", []):
        label = capture.get("label")
        stock = artifact / f"dosbox-{label}.png"
        engine = artifact / f"engine-{label}.png"
        if not stock.exists() or not engine.exists():
            skipped += 1
            continue
        left, right = decode(stock), decode(engine)
        if left is None or right is None:
            skipped += 1
            continue
        # The roster panel above the message window is compared too, and
        # counted on its own: it is a different window with its own contract,
        # and folding it into the message-window total would make two years of
        # earlier numbers incomparable.
        panel_left, panel_right = decode_panel(stock), decode_panel(engine)
        if panel_left is not None and panel_right is not None:
            panel_rows = [
                index + PANEL_TOP
                for index, (a, b) in enumerate(zip(panel_left, panel_right))
                if row_text(a) != row_text(b)
            ]
            if panel_rows:
                panel += 1
                print(f"  panel  {scenario}/{label}: rows {panel_rows}")
        rows = [
            index
            for index, (a, b) in enumerate(zip(left, right))
            # `?` is an unmatched cell on either side - a mid-resize capture or
            # a partially drawn row - and is not a claim about the other side.
            if any(x != y and "?" not in (x, y) for x, y in zip(a, b))
        ]
        if rows:
            kind = classify(left, right, rows)
            KINDS[kind] = KINDS.get(kind, 0) + 1
            # A side that never reached the beat is not a difference: the DOSBox
            # side sometimes boots too slowly for the scripted Journey Onward,
            # and the whole run then compares a blank window against a working
            # one. Counting that as a conformance failure is how `cove-herbalist`
            # once read 0 of 7 and then matched 7 of 7 unchanged on a re-run.
            if kind in IDLE_KINDS:
                idle += 1
                print(f"  idle   {scenario}/{label}: {kind}, re-run needed")
                continue
            differ += 1
            print(
                f"  differ {scenario}/{label}: {kind}, rows {[r + TOP for r in rows]}"
            )
        else:
            same += 1
    return same, differ, skipped, idle, panel


KINDS: dict[str, int] = {}

ARTIFACTS = pathlib.Path.home() / "artifacts/u5/paired"
SCENARIOS = pathlib.Path(__file__).resolve().parent.parent / "paired"


def latest_artifacts(names: list[str]) -> list[pathlib.Path]:
    """The newest artifact directory for each named scenario."""
    if not names:
        names = sorted(
            path.stem
            for path in SCENARIOS.glob("*.tsv")
            if path.stem != "seeds"
        )
    found = []
    for name in names:
        # The artifact directory is `<scenario>-<YYYYMMDD>-<HHMMSS>`, so a bare
        # `<name>-*` glob also matches every longer scenario sharing the
        # prefix: `nb-inn` picked up `nb-inn-refusals`, and `cove-healer`
        # picked up `cove-healer-services`, silently comparing the wrong
        # scenario. Require the timestamp.
        runs = sorted(
            (
                path
                for path in ARTIFACTS.glob(f"{name}-*")
                if (path / "record.json").is_file()
                and re.fullmatch(rf"{re.escape(name)}-\d{{8}}-\d{{6}}", path.name)
            ),
            key=lambda path: path.name,
        )
        if runs:
            found.append(runs[-1])
        else:
            print(f"  no run  {name}")
    return found


def main() -> None:
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    args = sys.argv[1:]
    if args[0] == "--latest":
        args = [str(path) for path in latest_artifacts(args[1:])]
    total = [0, 0, 0, 0, 0]
    cache = _cache_load()
    for arg in args:
        same, differ, skipped, idle, panel = compare_cached(pathlib.Path(arg), cache)
        # A run carrying idle beats is not a verdict either way: it needs
        # re-running before its differences mean anything.
        # A cached verdict replays totals but not the per-beat detail lines, so
        # the panel count belongs on the status line: without it a cached
        # scenario reads as clean whatever its roster panel did.
        scenario = re.sub(r"-\d{8}-\d{6}$", "", pathlib.Path(arg).name)
        lottery = differ and is_lottery(scenario)
        status = (
            "RERUN"
            if idle or lottery
            else ("match" if differ == 0 and panel == 0 else "DIFFER")
        )
        print(
            f"{status} {pathlib.Path(arg).name}: {same} beat(s) agree, "
            f"{differ} differ, {skipped} skipped, {idle} idle, {panel} panel"
        )
        for index, value in enumerate((same, differ, skipped, idle, panel)):
            total[index] += value
    _cache_save(cache)
    print(
        f"\n{total[0]} beat(s) agree, {total[1]} differ, "
        f"{total[2]} skipped, {total[3]} idle (re-run); "
        f"{total[4]} beat(s) differ in the roster panel"
    )
    if KINDS:
        # `stock-idle`/`engine-idle` are re-run candidates, `offset`/`cursor`
        # are row accounting, and `text` is the conformance queue.
        print(
            "by kind: "
            + ", ".join(f"{kind} {count}" for kind, count in sorted(KINDS.items()))
        )
    sys.exit(1 if total[1] else 0)


if __name__ == "__main__":
    main()
