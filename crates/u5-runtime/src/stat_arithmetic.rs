//! Shared capped-add / floor-subtract helpers per `systems/stat-arithmetic.md`.
//!
//! Four observable shapes:
//! - `capped_add_u8`: unsigned byte capped add (gold/stock counters)
//! - `capped_add_word`: signed word capped add (HP, XP)
//! - `floor_sub_u8`: unsigned byte floor subtract (key/gem/torch spending)
//! - `floor_sub_word`: signed word floor subtract (damage at HP, food spending)
//!
//! All functions mutate the target field in place and return the actual
//! delta applied, so callers can drive narration without re-reading the
//! field. Caller owns the cap and any post-mutation effect (refusal, death,
//! starvation, etc.).

/// Unsigned byte capped add: increase `field` by `amount` unless the result
/// would reach or exceed `cap`; in that case store `cap`. Returns the actual
/// delta applied.
pub fn capped_add_u8(field: &mut u8, amount: u8, cap: u8) -> u8 {
    let before = *field;
    let proposed = before.saturating_add(amount);
    let next = proposed.min(cap);
    *field = next;
    next.saturating_sub(before)
}

/// Signed word capped add: increase `field` by `amount` unless the result
/// would reach or exceed `cap`; in that case store `cap`. Comparison is
/// signed so fields permitted to pass through negative states do not wrap.
/// Returns the actual delta applied as a signed value.
pub fn capped_add_word(field: &mut i16, amount: i16, cap: i16) -> i16 {
    let before = *field;
    let proposed = before.saturating_add(amount);
    let next = proposed.min(cap);
    *field = next;
    next - before
}

/// Unsigned byte floor subtract: decrease `field` by `amount` only when the
/// current value is greater than the amount; otherwise store zero. Returns
/// the actual amount subtracted.
pub fn floor_sub_u8(field: &mut u8, amount: u8) -> u8 {
    let before = *field;
    let next = before.saturating_sub(amount);
    *field = next;
    before - next
}

/// Signed word floor subtract: decrease `field` by `amount` only when the
/// current value is greater than the amount; otherwise store zero. Returns
/// the actual amount subtracted as a non-negative i16.
pub fn floor_sub_word(field: &mut i16, amount: i16) -> i16 {
    let before = *field;
    let next = before.saturating_sub(amount).max(0);
    *field = next;
    before - next
}
