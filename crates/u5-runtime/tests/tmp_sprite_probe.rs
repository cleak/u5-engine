// Temporary diagnostic (not for commit): dump the decoded dungeon object sprites.
use u5_runtime::*;

#[test]
fn dump_object_sprites() {
    let Some(dir) = std::env::var_os("U5_TEST_ASSET_DIR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    let banks = load_optional_dungeon_sprite_banks(&dir, TileGraphicsDepth::Ega16).unwrap();
    let Some(banks) = banks else {
        println!("no sprite banks");
        return;
    };
    let Some(objects) = banks.objects() else {
        println!("no object sheet");
        return;
    };
    println!("sprite count = {}", objects.sprites.len());
    for slot in 0..objects.sprites.len().min(20) {
        match objects.sprites.get(slot).and_then(|s| s.as_ref()) {
            Some(s) => {
                let opaque = s.transparent_mask.iter().filter(|b| **b == 0).count();
                println!(
                    "slot {slot:2}: {}x{} opaque={opaque}",
                    s.image.width, s.image.height
                );
                if slot == 0 {
                    // ASCII art of the band-0 ladder, opaque pixels only.
                    for row in 0..s.image.height {
                        let mut line = String::new();
                        for col in 0..s.image.width {
                            let i = row * s.image.width + col;
                            line.push(if s.transparent_mask[i] != 0 {
                                '.'
                            } else if s.image.pixels[i] >= 8 {
                                '#'
                            } else {
                                '+'
                            });
                        }
                        println!("{row:3} {line}");
                    }
                }
            }
            None => println!("slot {slot:2}: absent"),
        }
    }
}
