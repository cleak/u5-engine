//! Helpers for `quest-flags.md`. Currently covers the per-scene TALK
//! branch-flag mask builder (§3) and the conversation `0x86`
//! letter-action table (§4).

/// `catalogs/quest-graph.md §1` semantic node class for a quest-graph
/// entry. The graph is data above the conversation system: a node
/// describes a fact, gate, item, or action the player can reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestGraphNodeClass {
    /// A speaking character from `catalogs/npc-roster.md`.
    Npc,
    /// A player-typed conversation topic.
    Keyword,
    /// A fact that can guide later action, such as a dungeon word.
    Knowledge,
    /// A typed answer that unlocks a branch.
    Password,
    /// A recoverable, buyable, or granted inventory object.
    Item,
    /// A named location from `catalogs/gazetteer.md`.
    Place,
    /// A condition such as Resistance trust, yes/no, gold, or virtue.
    Gate,
    /// A world command outside Talk (shrine meditation, Yell, ...).
    Action,
}

/// `catalogs/quest-graph.md §1`: ordered list of the eight node
/// classes the catalog uses.
pub const QUEST_GRAPH_NODE_CLASSES: [QuestGraphNodeClass; 8] = [
    QuestGraphNodeClass::Npc,
    QuestGraphNodeClass::Keyword,
    QuestGraphNodeClass::Knowledge,
    QuestGraphNodeClass::Password,
    QuestGraphNodeClass::Item,
    QuestGraphNodeClass::Place,
    QuestGraphNodeClass::Gate,
    QuestGraphNodeClass::Action,
];

/// `catalogs/quest-graph.md §3` typed gate identifying which password
/// the player typed into a TALK conversation prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationPassword {
    /// `DAWN` — Resistance trust token; opens anti-Blackthorn
    /// branches and Council-member help.
    Dawn,
    /// `IMPERA` — Oppression / Blackthorn-aligned password; routes
    /// the player into hostile or dangerous Blackthorn-side branches.
    Impera,
}

/// `catalogs/quest-graph.md §3`: classify a typed password input.
/// Comparison is case-insensitive ASCII; returns `None` for any other
/// input.
pub fn conversation_password(input: &str) -> Option<ConversationPassword> {
    if input.eq_ignore_ascii_case("DAWN") {
        Some(ConversationPassword::Dawn)
    } else if input.eq_ignore_ascii_case("IMPERA") {
        Some(ConversationPassword::Impera)
    } else {
        None
    }
}

/// `quest-flags.md §3`: build the 32-bit mask the TALK runner uses for
/// per-scene branch flags. The original implementation is a plain
/// left-shift with no wrap or clamp, so indices `>=32` produce a zero
/// mask (setter changes nothing, tester reports "not set").
pub const fn tlk_scene_branch_mask(bit_index: u8) -> u32 {
    if bit_index >= 32 {
        0
    } else {
        1u32 << bit_index
    }
}

/// `quest-flags.md §3`: TALK branch-flag tester used by the `0x8C`
/// IF/ELSE control code. Returns `true` when the bit is set in the
/// active scene's slot. A set bit selects the alternate/else arm; a
/// clear bit falls through to the normal/then arm.
pub const fn tlk_scene_branch_is_set(slot: u32, bit_index: u8) -> bool {
    let mask = tlk_scene_branch_mask(bit_index);
    mask != 0 && (slot & mask) != 0
}

/// `quest-flags.md §3`: TALK branch-flag setter (OR-into-slot semantics).
pub const fn tlk_scene_branch_set(slot: u32, bit_index: u8) -> u32 {
    slot | tlk_scene_branch_mask(bit_index)
}

/// `quest-flags.md §5` shared town/conversation sentinel value that
/// allows the post-conversation stolen-action warning + signal
/// reconciliation pass to run. Town setup writes `0` (the traced
/// town-produced state) for scenes whose Shadowlord-location slot
/// matches index 0; slot indices `1` and `2`, plus the no-slot
/// marker, suppress the cleanup pass.
pub const CONVERSATION_CLEANUP_SENTINEL_ALLOW: u8 = 0;

/// `quest-flags.md §5`: returns `true` when the sentinel allows
/// the post-conversation stolen-action warning + reconciliation
/// pass to run (sentinel byte equals zero). Nonzero values
/// suppress the pass entirely. The shop surcharge helper uses the
/// same sentinel: nonzero also suppresses the post-transaction
/// gold debit there.
pub const fn conversation_cleanup_runs_warning(sentinel: u8) -> bool {
    sentinel == CONVERSATION_CLEANUP_SENTINEL_ALLOW
}

/// `quest-flags.md §5` random gold-debit upper bound. When no
/// byte-sized signal was decremented in the cleanup, the pass
/// subtracts a random `1..=15` gold from the party's gold total
/// (floored at zero).
pub const CONVERSATION_CLEANUP_GOLD_DEBIT_MAX: u8 = 15;
/// `quest-flags.md §5` random gold-debit lower bound. The cleanup
/// always debits at least one gold when it reaches the gold
/// fallback branch.
pub const CONVERSATION_CLEANUP_GOLD_DEBIT_MIN: u8 = 1;

/// `quest-flags.md §5`: deterministic mapping from a uniform
/// `0..=255` random seed to the `1..=15` gold-debit amount the
/// conversation-cleanup gold fallback subtracts. Computed as
/// `(seed % CONVERSATION_CLEANUP_GOLD_DEBIT_MAX) +
/// CONVERSATION_CLEANUP_GOLD_DEBIT_MIN`, which produces every value
/// in the inclusive `1..=15` range exactly seventeen times across
/// the 255-value domain and value 1 eighteen times (255 = 15*17).
/// Caller is responsible for floor-at-zero gold subtraction after
/// computing the amount.
pub const fn conversation_cleanup_gold_debit_amount(roll_seed: u8) -> u8 {
    (roll_seed % CONVERSATION_CLEANUP_GOLD_DEBIT_MAX) + CONVERSATION_CLEANUP_GOLD_DEBIT_MIN
}

/// `quest-flags.md §5` reconciliation branch taken by the zero-sentinel
/// post-conversation cleanup pass, in the published priority order.
/// The cleanup decrements at most one byte-sized signal per call; if
/// every signal array is empty, it falls back to the random gold debit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationCleanupReconciliation {
    /// At least one of the three resource/special band slots is
    /// nonzero. The cleanup re-rolls one of three slot indices until
    /// it lands on a nonzero entry and decrements that slot.
    ResourceBand,
    /// All three resource-band slots are zero but the generic
    /// conversation signal array has at least one nonzero entry. The
    /// cleanup scans high-to-low and decrements the first nonzero entry.
    GenericSignalArray,
    /// The resource band and generic signal array are empty but at
    /// least one of the two eight-slot conversation signal arrays has
    /// a nonzero entry. The cleanup scans both arrays high-to-low and
    /// decrements the first nonzero entry.
    EightSlotSignalArrays,
    /// No byte-sized signal remained anywhere; the cleanup falls back
    /// to subtracting a random `1..=15` gold from party gold,
    /// floored at zero.
    GoldDebitFallback,
}

/// `quest-flags.md §5`: choose the reconciliation branch the
/// zero-sentinel cleanup pass should take, in the published priority
/// order. Caller passes per-array "any nonzero" predicates rather
/// than the array contents, which keeps the helper independent of
/// the storage representation.
pub const fn conversation_cleanup_reconciliation(
    resource_band_any_nonzero: bool,
    generic_signal_any_nonzero: bool,
    eight_slot_signals_any_nonzero: bool,
) -> ConversationCleanupReconciliation {
    if resource_band_any_nonzero {
        ConversationCleanupReconciliation::ResourceBand
    } else if generic_signal_any_nonzero {
        ConversationCleanupReconciliation::GenericSignalArray
    } else if eight_slot_signals_any_nonzero {
        ConversationCleanupReconciliation::EightSlotSignalArrays
    } else {
        ConversationCleanupReconciliation::GoldDebitFallback
    }
}

/// `quest-flags.md §4`: confirmed letter effects for the `0x86`
/// action-dispatch control code's letter-argument family. Returns
/// `None` for letters not in the published table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversationLetterAction {
    /// `A` — Food counter raised to the capped grant value.
    GrantFood,
    /// `B` — Gold counter raised to the capped grant value.
    GrantGold,
    /// `C` — Ordinary key counter raised to the capped grant value.
    GrantKeys,
    /// `D` — Gem counter raised to the capped grant value.
    GrantGems,
    /// `E` — Torch counter raised to the capped grant value.
    GrantTorches,
    /// `F` — Outdoor Klimb gear / Grapple gate set.
    SetGrappleGate,
    /// `G` — Magic-carpet carried counter raised to the capped grant
    /// value.
    GrantMagicCarpet,
    /// `H` — Sextant carried-item flag set.
    SetSextant,
    /// `I` — Spyglass carried-item flag set.
    SetSpyglass,
    /// `J` — Black Badge carried-item flag set.
    SetBlackBadge,
    /// `K` — Skull/special-key counter raised to the capped grant value.
    GrantSkullKeys,
}

/// `quest-flags.md §4`: classify a `0x86` letter argument.
pub const fn conversation_letter_action(letter: u8) -> Option<ConversationLetterAction> {
    Some(match letter {
        b'A' => ConversationLetterAction::GrantFood,
        b'B' => ConversationLetterAction::GrantGold,
        b'C' => ConversationLetterAction::GrantKeys,
        b'D' => ConversationLetterAction::GrantGems,
        b'E' => ConversationLetterAction::GrantTorches,
        b'F' => ConversationLetterAction::SetGrappleGate,
        b'G' => ConversationLetterAction::GrantMagicCarpet,
        b'H' => ConversationLetterAction::SetSextant,
        b'I' => ConversationLetterAction::SetSpyglass,
        b'J' => ConversationLetterAction::SetBlackBadge,
        b'K' => ConversationLetterAction::GrantSkullKeys,
        _ => return None,
    })
}
