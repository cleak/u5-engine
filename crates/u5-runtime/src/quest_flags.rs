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
