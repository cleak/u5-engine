//! Chapter 9's sprite masks on the player's own TILES.16 (design 9.6).
//!
//! Counts only: no mask, pixel or tile image is printed or committed.
//! Skipped unless `U5_MOCKUP_PROFILE` names a validated game-data copy
//! (`game-dev-u5 test-assets` or `prepare-profile`).

use std::path::PathBuf;

use u5_runtime::enhancements::{
    DARK_BODY, KEEP_THE_BOX, NEAR_EMPTY_PIXELS, SpriteMasks, composite_eligible, mask_count,
};
use u5_runtime::{TileGraphicsDepth, load_tile_atlas};

#[test]
fn real_assets_give_256_non_empty_masks() {
    let Some(profile) = std::env::var_os("U5_MOCKUP_PROFILE").map(PathBuf::from) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE");
        return;
    };
    let atlas = load_tile_atlas(&profile, TileGraphicsDepth::Ega16).expect("load the atlas");
    let masks = SpriteMasks::from_atlas(&atlas);
    let non_empty = (256..512)
        .filter(|&t| mask_count(masks.background(t, false)) > 0)
        .count();
    assert_eq!(non_empty, 256, "every actor tile has some background");
    let fallback = (256..512)
        .filter(|t| !KEEP_THE_BOX.contains(t) && !DARK_BODY.contains(t))
        .filter(|&t| masks.kept_pixels(t) < NEAR_EMPTY_PIXELS)
        .count();
    let eligible = (256..512)
        .filter(|&t| composite_eligible(&masks, Some(t), Some(0x05)))
        .count();
    eprintln!(
        "sprite masks: {non_empty} non-empty; {fallback} near-empty fallback tile(s) outside the lists; {eligible} eligible on plain terrain"
    );
}
