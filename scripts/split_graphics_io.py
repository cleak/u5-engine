"""Split graphics_io.rs into smaller files (the bulk move overshot 1000 lines)."""
from pathlib import Path
import sys
sys.path.insert(0, "scripts")
from carve_items import carve_to_module

SRC = Path("crates/u5-runtime/src")

# Move LZW codec to its own module.
carve_to_module(
    dest=SRC / "lzw.rs",
    summary="GIF-style LZW bit reader and codec used by tile/graphic asset envelopes.",
    sources=[SRC / "graphics_io.rs"],
    items=["LzwBitReader", "reset_lzw_dictionary", "decode_lzw_envelope", "decode_gif_lzw_payload"],
)

# Move font + monochrome bitmap loaders to their own module.
carve_to_module(
    dest=SRC / "fonts_io.rs",
    summary="Loaders/parsers for fixed and proportional fonts plus monochrome bitmaps (BIT/CH/HCS/PCS).",
    sources=[SRC / "graphics_io.rs"],
    items=[
        "load_title_bit",
        "parse_title_bit",
        "parse_title_bit_body",
        "load_british_bit",
        "parse_british_bit",
        "load_wd_bit",
        "parse_wd_bit",
        "parse_single_image_bit_body",
        "parse_monochrome_bitmap_block",
        "parse_monochrome_bitmap_payload",
        "monochrome_bitmap_payload_len",
        "unpack_monochrome_bits",
        "load_ch_font",
        "load_hcs_font",
        "parse_fixed_font_body",
        "load_proportional_font",
        "parse_proportional_font",
        "parse_proportional_font_body",
        "monochrome_row_stride",
        "unpack_monochrome_rows",
        "prepare_fixed_text_cell",
        "rasterize_fixed_text_line",
        "measure_proportional_text",
        "rasterize_proportional_text_line",
        "blit_monochrome_bitmap",
    ],
)
