// Regressions for the pass-11 exploratory-playtest findings (p11combat) and
// the u5-spec#302 answers in the combat area.
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `active-objects.md §3`: class `0xFC` (the Shadow Lord actor) takes the
/// default script A (frames 2, 3, 4) and "the displayed frame is class + n -
/// 1", which is the legal tile byte `0xFF` for frame 4. The animator added
/// `class + n` first and overflowed the byte, so a debug build panicked a few
/// turns after entering Stonegate (S06).
#[test]
fn playtest_s06_shadow_lord_frame_four_reaches_tile_ff_without_overflow() {
    let mut reached = false;
    for seed in 0..64u16 {
        let mut state = test_state(open_grid(), 1, 1);
        state.prng_state = seed.wrapping_mul(977).wrapping_add(3);
        state.active_objects.push(ActiveObject {
            type_byte: 0xfc,
            tile: 0xfe,
            x: 5,
            y: 5,
            z: 0,
            // Script position 2 (script A's `Frame(4)`), delay 0: the next
            // pass is a decision point.
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });
        state.animate_active_objects();
        let object = state.active_objects[1];
        if object.phase >> 4 == 3 {
            assert_eq!(object.tile, 0xff, "frame 4 of class 0xFC is 0xFC + 4 - 1");
            reached = true;
        } else {
            assert_eq!(object.tile, 0xfe, "a lost coin leaves the frame alone");
        }
    }
    assert!(reached, "some seed passes the eligibility coin");
}
