//! Ultima IV transfer **comparison and status preview** screen.
//!
//! Provenance: `systems/u4-transfer.md §6.1` through `§6.6`, published
//! in answer to `cleak/u5-spec#73`. Everything here is geometry, text
//! and stage sequencing; no pixels are touched. The compositor that
//! consumes it lives in `u5-bevy`'s `u4_transfer` module.
//!
//! Retractions this module deliberately honours (`§6` preamble and the
//! `#73` answer):
//!
//! * there is **no** eight-column heading strip — it is an eight-*row*
//!   field-label column, drawn once into each of the two panels
//!   (`§6.2`);
//! * there is **no** double buffering, page swap or deferred flush
//!   anywhere on this path — what is written twice is written into two
//!   different *text windows*, not two display pages (`§6`);
//! * the panels carry exactly eight fields; no hit points, no magic
//!   points, no equipment, no status labels (`§6.2`);
//! * once the drive has been selected **no key aborts the transfer** —
//!   `Esc` at any confirmation prompt is silently ignored (`§6.5`).
//!
//! `§6.4`'s insert-disk instruction block is dead code in the shipped
//! build: the code jumps over it and nothing branches into it. Those
//! strings are deliberately absent from this module, and
//! `insert_disk_instructions_are_never_authored` pins that.

use crate::text_wrap::{TEXT_SCREEN_COLUMNS, TEXT_SCREEN_ROWS, TextWindowDescriptor};
use crate::u4_transfer::{
    U4TransferSource, u4_transfer_attribute_to_u5, u4_transfer_experience_to_u5,
    u4_transfer_strength_to_u5,
};

// ---------------------------------------------------------------------------
// `§6.1` windows and regions
// ---------------------------------------------------------------------------

const fn window(
    top_left_x: u8,
    top_left_y: u8,
    bottom_right_x: u8,
    bottom_right_y: u8,
) -> TextWindowDescriptor {
    TextWindowDescriptor {
        top_left_x,
        top_left_y,
        bottom_right_x,
        bottom_right_y,
        cursor_x: 0,
        cursor_y: 0,
        color: 0x0f,
        flags: 0,
    }
}

/// `§6.1`: the whole-screen window the path clears through and draws
/// the lower prompt frame on, before the three preview rectangles are
/// installed. `§6.3`'s two full-screen pages use it as well.
pub const U4_PREVIEW_FULL_SCREEN_WINDOW: TextWindowDescriptor =
    window(0, 0, TEXT_SCREEN_COLUMNS - 1, TEXT_SCREEN_ROWS - 1);

/// `§6.1`: left panel — "the character as Ultima IV supplied it".
pub const U4_PREVIEW_LEFT_PANEL_WINDOW: TextWindowDescriptor = window(0, 0, 19, 18);

/// `§6.1`: right panel — "the character as Ultima V will store it".
pub const U4_PREVIEW_RIGHT_PANEL_WINDOW: TextWindowDescriptor = window(21, 0, 39, 18);

/// `§6.1`: the one-row message line the stage machine prints into.
pub const U4_PREVIEW_MESSAGE_LINE_WINDOW: TextWindowDescriptor = window(3, 21, 37, 21);

/// `§6.6`: the message line is widened to columns `2..37`, rows
/// `21..22` for the commit notice, and only for it.
pub const U4_PREVIEW_COMMIT_MESSAGE_WINDOW: TextWindowDescriptor = window(2, 21, 37, 22);

/// Which of the two character-information panels a write targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U4PreviewPanel {
    /// Left, pixel origin `x = 0`: the imported Ultima IV values.
    Source,
    /// Right, pixel origin `x = 168`: the Ultima V values the commit
    /// will store.
    Result,
}

impl U4PreviewPanel {
    /// `§6.2`/`§6.5` move (a): the labels are printed, and highlighted,
    /// in **both** panels.
    pub const BOTH: [Self; 2] = [Self::Source, Self::Result];

    /// `§6.1`: the pixel origin the shared panel routine is called with.
    pub const fn origin_x(self) -> u16 {
        match self {
            Self::Source => 0,
            Self::Result => 168,
        }
    }

    pub const fn window(self) -> TextWindowDescriptor {
        match self {
            Self::Source => U4_PREVIEW_LEFT_PANEL_WINDOW,
            Self::Result => U4_PREVIEW_RIGHT_PANEL_WINDOW,
        }
    }
}

// ---------------------------------------------------------------------------
// `§6.1` lower prompt frame
// ---------------------------------------------------------------------------

/// `§6.1`, and `intro.md §6.1` for the glyph family: the frame is built
/// from the fixed-cell font's four rounded bevel corners plus **one
/// fully solid cell**. There is no separate horizontal-bar or
/// vertical-bar glyph — the runs and the side columns are the same
/// solid cell, so the frame reads as a thick band, not a line-drawn
/// box. Earlier wording calling them "bar" glyphs is withdrawn.
pub const U4_PREVIEW_FRAME_GLYPH_TOP_LEFT: u8 = 0x7b;
pub const U4_PREVIEW_FRAME_GLYPH_TOP_RIGHT: u8 = 0x7c;
pub const U4_PREVIEW_FRAME_GLYPH_BOTTOM_LEFT: u8 = 0x7d;
pub const U4_PREVIEW_FRAME_GLYPH_BOTTOM_RIGHT: u8 = 0x7e;
pub const U4_PREVIEW_FRAME_GLYPH_SOLID: u8 = 0x7f;

/// `§6.1`: top edge row of the prompt frame.
pub const U4_PREVIEW_FRAME_TOP_ROW: u8 = 19;
/// `§6.1`: bottom edge row of the prompt frame.
pub const U4_PREVIEW_FRAME_BOTTOM_ROW: u8 = 23;
/// `§6.1`: the three interior rows carrying only the two side cells.
/// The menu frame of `intro.md §6.1` has an eight-row interior; this is
/// a **separate descriptor with its own bounds**, not a reuse of it.
pub const U4_PREVIEW_FRAME_SIDE_ROWS: [u8; 3] = [20, 21, 22];
/// `§6.1`: thirty-eight solid cells run between the corners.
pub const U4_PREVIEW_FRAME_SOLID_RUN_LEN: u8 = 38;

/// `§6.1`: the closed four-segment accent rectangle, as the pixel
/// corners the driver's line primitive is called with. Every segment
/// is axis-aligned.
pub const U4_PREVIEW_FRAME_RULE_PATH: [(u16, u16); 5] =
    [(7, 159), (312, 159), (312, 184), (7, 184), (7, 159)];

/// One frame cell: absolute `(column, row)` and the glyph code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct U4PreviewFrameCell {
    pub column: u8,
    pub row: u8,
    pub glyph: u8,
}

/// `§6.1`: every cell of the lower prompt frame, in draw order.
///
/// Row `19` is corner + thirty-eight solid + corner; rows `20`, `21`
/// and `22` carry one solid cell in column `0` and one in column `39`
/// and nothing else — the interior is **not** cleared per prompt, only
/// the message-line window is; row `23` closes with the two bottom
/// corners.
pub fn u4_preview_prompt_frame_cells() -> Vec<U4PreviewFrameCell> {
    let last = TEXT_SCREEN_COLUMNS - 1;
    let run = U4_PREVIEW_FRAME_SOLID_RUN_LEN;
    let mut cells = Vec::new();

    for (row, left, right) in [
        (
            U4_PREVIEW_FRAME_TOP_ROW,
            U4_PREVIEW_FRAME_GLYPH_TOP_LEFT,
            U4_PREVIEW_FRAME_GLYPH_TOP_RIGHT,
        ),
        (
            U4_PREVIEW_FRAME_BOTTOM_ROW,
            U4_PREVIEW_FRAME_GLYPH_BOTTOM_LEFT,
            U4_PREVIEW_FRAME_GLYPH_BOTTOM_RIGHT,
        ),
    ] {
        if row == U4_PREVIEW_FRAME_BOTTOM_ROW {
            for side_row in U4_PREVIEW_FRAME_SIDE_ROWS {
                for column in [0, last] {
                    cells.push(U4PreviewFrameCell {
                        column,
                        row: side_row,
                        glyph: U4_PREVIEW_FRAME_GLYPH_SOLID,
                    });
                }
            }
        }
        cells.push(U4PreviewFrameCell {
            column: 0,
            row,
            glyph: left,
        });
        for offset in 0..run {
            cells.push(U4PreviewFrameCell {
                column: 1 + offset,
                row,
                glyph: U4_PREVIEW_FRAME_GLYPH_SOLID,
            });
        }
        cells.push(U4PreviewFrameCell {
            column: last,
            row,
            glyph: right,
        });
    }

    cells
}

// ---------------------------------------------------------------------------
// `§6.1` character-information panels
// ---------------------------------------------------------------------------

/// `§6.1`: the three filled bars, as inclusive pixel rectangles
/// `(x0, y0, x1, y1)`, for a panel drawn at pixel origin `origin_x`.
pub const fn u4_preview_panel_bars(origin_x: u16) -> [(u16, u16, u16, u16); 3] {
    [
        (origin_x, 0, origin_x + 6, 143),
        (origin_x + 143, 0, origin_x + 151, 137),
        (origin_x + 7, 137, origin_x + 150, 143),
    ]
}

/// `§6.1`: the broken-top rule polyline. The deliberate gap between
/// `origin_x + 24` and `origin_x + 128` is where the panel title plate
/// sits, which is why the path starts and ends part-way along the top
/// edge instead of closing.
pub const fn u4_preview_panel_rule_polyline(origin_x: u16) -> [(u16, u16); 6] {
    [
        (origin_x + 24, 7),
        (origin_x + 7, 7),
        (origin_x + 7, 136),
        (origin_x + 143, 136),
        (origin_x + 143, 7),
        (origin_x + 128, 7),
    ]
}

/// `§6.1`: the string both panels are painted from.
pub const U4_PREVIEW_PANEL_TITLE_TEXT: &str = " Ultima IV ";
/// `§6.1`: panel cell the eleven title characters start at.
pub const U4_PREVIEW_PANEL_TITLE_FIRST_CELL: u8 = 4;
/// `§6.1`: immediately after the second panel is drawn the path selects
/// the **right** panel and writes a single space over its title cell
/// `12` — the `I` of `IV`. That one-cell edit is the only difference
/// between the two panel frames, and an implementation must reproduce
/// it.
pub const U4_PREVIEW_RIGHT_PANEL_TITLE_BLANKED_CELL: u8 = 12;
/// `§6.1`: the title plate's left cap cell (right-pointing bracket
/// glyph `0x02`, plus two short accent rules).
pub const U4_PREVIEW_PANEL_TITLE_LEFT_CAP_CELL: u8 = 3;
/// `§6.1`: the title plate's right cap cell (left-pointing bracket
/// glyph `0x01`, plus the mirrored rules).
pub const U4_PREVIEW_PANEL_TITLE_RIGHT_CAP_CELL: u8 = 15;
/// `§6.1`: the title row is nineteen cells wide.
pub const U4_PREVIEW_PANEL_TITLE_ROW_CELLS: u8 = 19;
/// `§6.1`: solid cells flanking the title plate.
pub const U4_PREVIEW_PANEL_TITLE_SOLID_CELLS: [u8; 4] = [1, 2, 16, 17];
/// `§6.1`: bottom-left corner glyph cell of a panel.
pub const U4_PREVIEW_PANEL_BOTTOM_LEFT_CELL: (u8, u8) = (0, 17);
/// `§6.1`: bottom-right corner glyph cell of a panel.
pub const U4_PREVIEW_PANEL_BOTTOM_RIGHT_CELL: (u8, u8) = (18, 17);

/// `§6.1`: the eleven title characters **as they end up on screen**.
///
/// The left panel keeps ` Ultima IV `; the right panel reads
/// ` Ultima  V `, with the doubled inner space the blanking write
/// leaves behind. Left is the Ultima IV source, right the Ultima V
/// result. Any claim that both panels read `Ultima IV` on screen is
/// withdrawn.
pub fn u4_preview_panel_title_text(panel: U4PreviewPanel) -> String {
    let mut title: Vec<u8> = U4_PREVIEW_PANEL_TITLE_TEXT.as_bytes().to_vec();
    if panel == U4PreviewPanel::Result {
        let index = usize::from(
            U4_PREVIEW_RIGHT_PANEL_TITLE_BLANKED_CELL - U4_PREVIEW_PANEL_TITLE_FIRST_CELL,
        );
        title[index] = b' ';
    }
    String::from_utf8(title).expect("panel title is ASCII")
}

// ---------------------------------------------------------------------------
// `§6.2` field-label strip
// ---------------------------------------------------------------------------

/// `§6.2`: every label is printed at column `3` of its panel.
pub const U4_PREVIEW_FIELD_LABEL_COLUMN: u8 = 3;

/// `§6.2`: the eight-**row** field-label column, with the exact leading
/// spaces that right-align the words so each begins at column `7`
/// (`Name:`), column `5` (`Sex:`, `Exp:`, `STR:`, `DEX:`, `INT:`) or
/// column `3` (`Class:`, `Level:`). Printed twice — once with the left
/// panel selected, once with the right — not once per display page.
pub const U4_PREVIEW_FIELD_LABELS: [(u8, &str); 8] = [
    (2, "    Name:"),
    (5, "  Sex:"),
    (6, "Class:"),
    (8, "  Exp:"),
    (9, "Level:"),
    (11, "  STR:"),
    (12, "  DEX:"),
    (13, "  INT:"),
];

/// `§6.5` move (b): converted values land at column `10` of the stage's
/// row, in the right panel.
pub const U4_PREVIEW_VALUE_COLUMN: u8 = 10;

/// `§6.5`: the name is centred at panel column `0` of this row.
pub const U4_PREVIEW_NAME_ROW: u8 = 3;
/// `§6.5`/`§6.6`: the Avatar verdict is centred at panel column `0` of
/// this row.
pub const U4_PREVIEW_AVATAR_VERDICT_ROW: u8 = 15;

/// The label text printed at a given field row, or `None` when the row
/// carries no label.
pub fn u4_preview_field_label(row: u8) -> Option<&'static str> {
    U4_PREVIEW_FIELD_LABELS
        .iter()
        .find(|(label_row, _)| *label_row == row)
        .map(|(_, text)| *text)
}

// ---------------------------------------------------------------------------
// `§6.3` full-screen pages
// ---------------------------------------------------------------------------

/// `§6.3`: the eight class names, in source class-index order.
pub const U4_PREVIEW_CLASS_NAMES: [&str; 8] = [
    "Mage", "Bard", "Fighter", "Druid", "Tinker", "Paladin", "Ranger", "Shepherd",
];

/// `§6.3`/`§6.5`: the class cell reads `Avatar` when the `§5.3`
/// Avatarhood test passed, otherwise the source class name.
pub const U4_PREVIEW_AVATAR_CLASS_TEXT: &str = "Avatar";
/// `§6.5`: the verdict line's negative form.
pub const U4_PREVIEW_NON_AVATAR_TEXT: &str = "Non-Avatar";

/// `§6.3`: first row of the rejected-source page.
pub const U4_PREVIEW_REJECTED_FIRST_ROW: u8 = 5;

/// `§6.3` rejected-source page, centred, printed from cell `(0, 5)`.
/// The empty entries are the published blank lines; the pair before the
/// closing line are its "two blank lines".
pub const U4_PREVIEW_REJECTED_LINES: [&str; 8] = [
    "Error:  Your Ultima IV game",
    "",
    "contains bad data.",
    "",
    "Unable to continue transfer.",
    "",
    "",
    "Press any key to return to the menu.",
];

/// `§6.3` "Found" page geometry.
pub const U4_PREVIEW_FOUND_LABEL_ROW: u8 = 11;
pub const U4_PREVIEW_FOUND_NAME_ROW: u8 = 12;
pub const U4_PREVIEW_FOUND_DESCRIPTION_CELL: (u8, u8) = (12, 13);
pub const U4_PREVIEW_FOUND_STAT_COLUMN: u8 = 17;
pub const U4_PREVIEW_FOUND_STAT_ROWS: [u8; 3] = [15, 16, 17];
pub const U4_PREVIEW_FOUND_VERDICT_CELL: (u8, u8) = (10, 20);
/// `§6.3`: the label the centred pair opens with.
pub const U4_PREVIEW_FOUND_LABEL_TEXT: &str = "Found:";
/// `§6.3`: the three stat rows' prefixes.
pub const U4_PREVIEW_FOUND_STAT_PREFIXES: [&str; 3] = ["STR:  ", "DEX:  ", "INT:  "];

/// One line of a page or panel fill: either centred by the active text
/// window's centring mode, or anchored at an explicit column.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct U4PreviewPageLine {
    pub row: u8,
    pub column: Option<u8>,
    pub text: String,
}

// ---------------------------------------------------------------------------
// `§6.5` stage machine
// ---------------------------------------------------------------------------

/// `§6.5`: the fixed sequence of confirmation and conversion stages.
///
/// [`U4PreviewStage::NameReplace`] is conditional — it is entered only
/// when [`U4PreviewStage::NameConfirm`] was answered `N`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U4PreviewStage {
    NameConfirm,
    NameReplace,
    SexConfirm,
    Class,
    Experience,
    Level,
    Strength,
    Dexterity,
    Intellect,
}

impl U4PreviewStage {
    /// `§6.5`: the panel row this stage's label and value occupy.
    pub const fn row(self) -> u8 {
        match self {
            Self::NameConfirm | Self::NameReplace => 2,
            Self::SexConfirm => 5,
            Self::Class => 6,
            Self::Experience => 8,
            Self::Level => 9,
            Self::Strength => 11,
            Self::Dexterity => 12,
            Self::Intellect => 13,
        }
    }

    /// `§6.5`: the two confirmation prompts accept only `Y` and `N`
    /// (case-folded); anything else is discarded silently — no beep, no
    /// message, no redraw. The informational stages accept any key.
    pub const fn accepts_only_yes_or_no(self) -> bool {
        matches!(self, Self::NameConfirm | Self::SexConfirm)
    }
}

/// Where a message-line string is placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U4PreviewMessagePlacement {
    /// Explicit window-relative cursor cell.
    Cell(u8, u8),
    /// Centred by the text window's own centring mode, at window home.
    Centred,
}

/// A stage's message-line content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct U4PreviewMessage {
    pub text: String,
    pub placement: U4PreviewMessagePlacement,
}

/// `§6.5` message wording and cursor cells. Message-line cells are
/// window-relative to [`U4_PREVIEW_MESSAGE_LINE_WINDOW`].
pub const U4_PREVIEW_NAME_CONFIRM_TEXT: &str = "Keep this name?";
pub const U4_PREVIEW_NAME_CONFIRM_CURSOR: (u8, u8) = (10, 0);
pub const U4_PREVIEW_NAME_REPLACE_PROMPT: &str = "Enter new name: ";
pub const U4_PREVIEW_NAME_REPLACE_CURSOR: (u8, u8) = (1, 0);
/// `§6.5`: the typed-entry field starts immediately after the
/// sixteen-character prompt and takes at most eight characters.
pub const U4_PREVIEW_NAME_ENTRY_MAX_CHARS: usize = 8;
pub const U4_PREVIEW_SEX_CONFIRM_TEXT: &str = "Keep same sex?";
pub const U4_PREVIEW_CLASS_AVATAR_TEXT: &str = "Thou art now an Avatar:";
pub const U4_PREVIEW_CLASS_INTACT_TEXT: &str = "Class remains intact";
pub const U4_PREVIEW_CLASS_CURSOR: (u8, u8) = (2, 0);
pub const U4_PREVIEW_EXPERIENCE_TEXT: &str = "Experience has been converted";
pub const U4_PREVIEW_LEVEL_TEXT: &str = "Level has been converted";
pub const U4_PREVIEW_ATTRIBUTE_CURSOR: (u8, u8) = (1, 0);
/// `§6.5`: `(50)` and `(30)` are **literal text** — the Ultima IV and
/// Ultima V maxima printed after each number, not computed bounds.
pub const U4_PREVIEW_U4_MAXIMUM_TEXT: &str = "(50)";
pub const U4_PREVIEW_U5_MAXIMUM_TEXT: &str = "(30)";
pub const U4_PREVIEW_STRENGTH_LABEL: &str = "Strength:";
pub const U4_PREVIEW_DEXTERITY_LABEL: &str = "Dexterity:";
pub const U4_PREVIEW_INTELLECT_LABEL: &str = "Intellect:";

/// `§6.5`: `Strength: was ` + old value + `(50), now ` + new value +
/// `(30)`, and the same shape for Dexterity and Intellect.
pub fn u4_preview_attribute_message_text(label: &str, old: u16, new: u8) -> String {
    format!("{label} was {old}{U4_PREVIEW_U4_MAXIMUM_TEXT}, now {new}{U4_PREVIEW_U5_MAXIMUM_TEXT}")
}

/// `§6.6`: the commit notice, emitted into the widened message window.
///
/// The published string begins with two line breaks, then a single
/// leading space, then the words, then one more line break; because the
/// widened window is only two rows tall those breaks scroll it by one
/// row each under the standard overflow rule, so the settled result is
/// this line on screen row `21`, its leading space at column `2` and
/// its first letter at column `3`, with row `22` left blank. This
/// constant is that settled line.
pub const U4_PREVIEW_COMMIT_TEXT: &str = " Conversion complete, saving...";
/// `§6.6`: screen row the settled commit notice occupies.
pub const U4_PREVIEW_COMMIT_ROW: u8 = 21;
/// `§6.6`: screen column the notice's leading space lands on.
pub const U4_PREVIEW_COMMIT_COLUMN: u8 = 2;

/// One screen edit the compositor must apply.
///
/// `§6.5`: "Nothing else on the screen is redrawn at any point. The
/// panels are never rebuilt, the frames are never redrawn, and no
/// region is repainted after a name or gender edit beyond the single
/// cell run that changed." Modelling the edits as data is what lets the
/// tests pin that scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum U4PreviewEdit {
    /// `§6.5`: the four moves a stage repeats.
    Stage(Box<U4PreviewStageEdit>),
    /// `§6.5`: the live typed-entry echo, repainting only the run of
    /// cells after the sixteen-character prompt.
    NameField(String),
    /// `§6.5`: the accepted replacement name, written centred into the
    /// right panel at `(0, 3)`.
    AcceptedName(String),
    /// `§6.5`: a single right-panel value rewrite at column `10` of a
    /// row — the gender flip's published effect.
    RightValue { row: u8, text: String },
    /// `§6.6`: `Avatar` or `Non-Avatar` centred into right panel
    /// `(0, 15)`, then the widened-window commit notice.
    Finish { verdict: String, notice: String },
}

/// `§6.5`'s four moves, as data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct U4PreviewStageEdit {
    pub stage: U4PreviewStage,
    /// Move (a): the previous stage's label row, reprinted in normal
    /// video in **both** panels. `None` for the first stage.
    pub previous_label_row: Option<u8>,
    /// Move (a): the new stage's label row, reprinted in inverse video
    /// in **both** panels.
    pub inverse_label_row: u8,
    /// Move (b): the converted value, written into the **right** panel
    /// at column `10` of the stage's row. `None` where `§6.5`'s effect
    /// column publishes no value write.
    pub right_value: Option<String>,
    /// Move (c): the message-line window is cleared, then this is
    /// printed. Move (d) is waiting for input, which is the caller.
    pub message: U4PreviewMessage,
}

/// What the caller must do after applying the returned edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U4PreviewAction {
    /// Nothing beyond applying the edits — possibly none at all, when
    /// an invalid key was discarded silently.
    None,
    /// `§6.3`: the "Found" page is finished with; clear it, build the
    /// comparison screen, then apply the edits.
    BuildComparisonScreen,
    /// `§6.6`: write the save files. The notice is already on the
    /// visible page by the time the write starts.
    Commit,
    /// `§6.3`: the rejected-source page took its key. Return to the
    /// intro menu with nothing written.
    ReturnToMenu,
}

/// The response to one keystroke.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct U4PreviewResponse {
    pub edits: Vec<U4PreviewEdit>,
    pub action: U4PreviewAction,
}

impl U4PreviewResponse {
    fn ignored() -> Self {
        Self {
            edits: Vec::new(),
            action: U4PreviewAction::None,
        }
    }
}

// ---------------------------------------------------------------------------
// `§5.1`/`§5.2`/`§5.3` source view the preview needs
// ---------------------------------------------------------------------------

/// `§5.1`: the leading character record.
pub const U4_PREVIEW_RECORD_OFFSET: usize = 0x0008;
/// `§5.1`: the party-wide block begins immediately after the eighth
/// character record (`0x0008 + 8 * 39`).
pub const U4_PREVIEW_PARTY_BLOCK_OFFSET: usize = 0x0140;
/// `§5.1`: bytes read from the party-wide block.
pub const U4_PREVIEW_PARTY_BLOCK_LEN: usize = 182;
/// `§5.3`: the eight virtue standings are 16-bit, stride two, six bytes
/// into the party-wide block, i.e. file offsets `0x0146..=0x0155`.
pub const U4_PREVIEW_VIRTUE_STANDING_OFFSET: usize = 0x0146;
pub const U4_PREVIEW_VIRTUE_STANDING_COUNT: usize = 8;
/// `§5.2`: hit points, maximum hit points and experience are gated to
/// `0..9999`.
pub const U4_PREVIEW_PROGRESS_MAX: u16 = 9999;
/// `§5.2`: Strength, Dexterity and Intelligence are gated to `0..70`.
pub const U4_PREVIEW_ATTRIBUTE_MAX: u16 = 70;
/// `§5.2`: class index is gated to `0..7`.
pub const U4_PREVIEW_CLASS_INDEX_MAX: u8 = 7;
/// `§5.2`: only the first eight name bytes are validated.
pub const U4_PREVIEW_VALIDATED_NAME_BYTES: usize = 8;
/// `§7`: the source male marker; any other value becomes female.
pub const U4_PREVIEW_MALE_MARKER: u8 = 0x0b;
/// `§6.3`: the level shown on the "Found" page and in the left panel is
/// the staged value — the source record's maximum-hit-points field
/// divided by one hundred, truncating.
pub const U4_PREVIEW_FOUND_LEVEL_DIVISOR: u16 = 100;

const RECORD_HP: usize = 0x00;
const RECORD_MAX_HP: usize = 0x02;
const RECORD_EXPERIENCE: usize = 0x04;
const RECORD_STRENGTH: usize = 0x06;
const RECORD_DEXTERITY: usize = 0x08;
const RECORD_INTELLIGENCE: usize = 0x0a;
const RECORD_NAME: usize = 0x14;
const RECORD_SEX: usize = 0x24;
const RECORD_CLASS: usize = 0x25;

/// `§5.2`: why a source save was refused. Every variant lands the
/// player on `§6.3`'s bad-data page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U4PreviewSourceRejection {
    TooShort(usize),
    OutOfRange {
        field: &'static str,
        value: u16,
        max: u16,
    },
    NameByte(u8),
}

impl std::fmt::Display for U4PreviewSourceRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort(len) => write!(f, "PARTY.SAV is too short: {len} byte(s)"),
            Self::OutOfRange { field, value, max } => {
                write!(f, "PARTY.SAV {field} must be 0..{max}, got {value}")
            }
            Self::NameByte(byte) => write!(f, "PARTY.SAV name contains control byte {byte:#04x}"),
        }
    }
}

impl std::error::Error for U4PreviewSourceRejection {}

/// The imported character, exactly as `§6` needs to present it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct U4PreviewSource {
    pub name: String,
    pub male: bool,
    pub class_index: u8,
    pub strength: u16,
    pub dexterity: u16,
    pub intelligence: u16,
    pub experience: u16,
    pub max_hit_points: u16,
    /// `§5.3`: set when all eight virtue standings are individually
    /// zero. It never rejects a transfer; it selects a class override
    /// and some wording.
    pub is_avatar: bool,
}

impl U4PreviewSource {
    /// `§6.3`/`§6.5`: the class cell's text — `Avatar` when the
    /// Avatarhood test passed, otherwise the source class name.
    pub fn class_text(&self) -> &'static str {
        if self.is_avatar {
            U4_PREVIEW_AVATAR_CLASS_TEXT
        } else {
            U4_PREVIEW_CLASS_NAMES[usize::from(self.class_index)]
        }
    }

    pub const fn sex_text(&self) -> &'static str {
        u4_preview_sex_text(self.male)
    }

    /// `§6.3`: the staged level, `max_hp / 100` truncating.
    ///
    /// This is **not** `§7`'s experience-derived level; `§7`'s level and
    /// the `30 * level` hit points overwrite it later, so this number
    /// and the comparison screen's right-hand panel can legitimately
    /// differ. The left panel shows this one because it is the
    /// unconverted Ultima IV figure.
    pub const fn staged_level(&self) -> u16 {
        self.max_hit_points / U4_PREVIEW_FOUND_LEVEL_DIVISOR
    }

    /// The commit-side record this preview hands to the save writer.
    pub fn to_transfer_source(&self) -> U4TransferSource {
        U4TransferSource {
            name: self.name.as_bytes().to_vec(),
            male: self.male,
            class_index: self.class_index,
            strength: self.strength,
            dexterity: self.dexterity,
            intelligence: self.intelligence,
            experience: u32::from(self.experience),
        }
    }
}

pub const fn u4_preview_sex_text(male: bool) -> &'static str {
    if male { "Male" } else { "Female" }
}

fn u16_at(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

fn gate(field: &'static str, value: u16, max: u16) -> Result<u16, U4PreviewSourceRejection> {
    if value <= max {
        Ok(value)
    } else {
        Err(U4PreviewSourceRejection::OutOfRange { field, value, max })
    }
}

/// `§5.1`/`§5.2`/`§5.3`: read the leading transferable record and the
/// Avatarhood block out of a `PARTY.SAV` image.
///
/// Only the regions `§5.1` publishes are touched. **No party-wide
/// counter is validated**: gold, food, gems, torches, keys, sextants,
/// move count, moon phase and dungeon progress are never read on this
/// path, and an implementation must not reject a transfer because of
/// them. `§5.3`'s all-zero standings are the *success* condition for
/// Avatarhood, never a rejection.
pub fn parse_u4_preview_source(bytes: &[u8]) -> Result<U4PreviewSource, U4PreviewSourceRejection> {
    let required = U4_PREVIEW_VIRTUE_STANDING_OFFSET + U4_PREVIEW_VIRTUE_STANDING_COUNT * 2;
    if bytes.len() < required {
        return Err(U4PreviewSourceRejection::TooShort(bytes.len()));
    }

    let record = U4_PREVIEW_RECORD_OFFSET;
    gate(
        "current hit points",
        u16_at(bytes, record + RECORD_HP),
        U4_PREVIEW_PROGRESS_MAX,
    )?;
    let max_hit_points = gate(
        "maximum hit points",
        u16_at(bytes, record + RECORD_MAX_HP),
        U4_PREVIEW_PROGRESS_MAX,
    )?;
    let experience = gate(
        "experience",
        u16_at(bytes, record + RECORD_EXPERIENCE),
        U4_PREVIEW_PROGRESS_MAX,
    )?;
    let strength = gate(
        "strength",
        u16_at(bytes, record + RECORD_STRENGTH),
        U4_PREVIEW_ATTRIBUTE_MAX,
    )?;
    let dexterity = gate(
        "dexterity",
        u16_at(bytes, record + RECORD_DEXTERITY),
        U4_PREVIEW_ATTRIBUTE_MAX,
    )?;
    let intelligence = gate(
        "intelligence",
        u16_at(bytes, record + RECORD_INTELLIGENCE),
        U4_PREVIEW_ATTRIBUTE_MAX,
    )?;

    let class_index = bytes[record + RECORD_CLASS];
    if class_index > U4_PREVIEW_CLASS_INDEX_MAX {
        return Err(U4PreviewSourceRejection::OutOfRange {
            field: "class index",
            value: u16::from(class_index),
            max: u16::from(U4_PREVIEW_CLASS_INDEX_MAX),
        });
    }

    // `§5.2`: each of the first eight name bytes must be NUL or at
    // least `0x20`; any other control byte rejects the transfer.
    let name_start = record + RECORD_NAME;
    let name_bytes = &bytes[name_start..name_start + U4_PREVIEW_VALIDATED_NAME_BYTES];
    for &byte in name_bytes {
        if byte != 0 && byte < 0x20 {
            return Err(U4PreviewSourceRejection::NameByte(byte));
        }
    }
    let name: String = name_bytes
        .iter()
        .copied()
        .take_while(|&byte| byte != 0)
        .map(|byte| byte as char)
        .collect();

    // `§5.3`: satisfied only when each of the eight 16-bit values is
    // individually zero — not a sum, not a total, not a wider
    // aggregate.
    let is_avatar = (0..U4_PREVIEW_VIRTUE_STANDING_COUNT)
        .all(|index| u16_at(bytes, U4_PREVIEW_VIRTUE_STANDING_OFFSET + index * 2) == 0);

    Ok(U4PreviewSource {
        name,
        male: bytes[record + RECORD_SEX] == U4_PREVIEW_MALE_MARKER,
        class_index,
        strength,
        dexterity,
        intelligence,
        experience,
        max_hit_points,
        is_avatar,
    })
}

/// `§7`: the converted values the right panel shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct U4PreviewConverted {
    pub experience: u16,
    pub level: u8,
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
}

impl U4PreviewConverted {
    pub fn from_source(source: &U4PreviewSource) -> Self {
        let experience = u4_transfer_experience_to_u5(u32::from(source.experience));
        Self {
            experience,
            level: crate::party::recompute_level_from_experience(experience),
            strength: u4_transfer_strength_to_u5(source.strength),
            dexterity: u4_transfer_attribute_to_u5(source.dexterity),
            intelligence: u4_transfer_attribute_to_u5(source.intelligence),
        }
    }
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

/// Which page the preview is showing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum U4PreviewPhase {
    /// `§6.3`: the bad-data page. Any key returns to the menu.
    Rejected,
    /// `§6.3`: the "Found" summary page. Any key clears it and builds
    /// the comparison screen.
    Found,
    /// `§6.5`: the comparison screen, mid-stage.
    Stage(U4PreviewStage),
    /// `§6.6`: the commit notice is up and the save has been written.
    Complete,
}

/// The `§6` preview, as a keystroke-driven state machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct U4PreviewSession {
    source: U4PreviewSource,
    converted: U4PreviewConverted,
    phase: U4PreviewPhase,
    /// The name the commit will store: the imported name until the
    /// replacement stage accepts a new one.
    name: String,
    /// The gender the commit will store.
    male: bool,
    /// Live typed-entry buffer for the replacement-name field.
    entry: String,
}

impl U4PreviewSession {
    /// `§6.3`: a validated source opens on the "Found" page.
    pub fn new(source: U4PreviewSource) -> Self {
        let converted = U4PreviewConverted::from_source(&source);
        let name = source.name.clone();
        let male = source.male;
        Self {
            source,
            converted,
            phase: U4PreviewPhase::Found,
            name,
            male,
            entry: String::new(),
        }
    }

    /// `§6.3`: a rejected source opens on the bad-data page and never
    /// reaches the comparison screen.
    pub fn rejected() -> Self {
        let source = U4PreviewSource {
            name: String::new(),
            male: true,
            class_index: 0,
            strength: 0,
            dexterity: 0,
            intelligence: 0,
            experience: 0,
            max_hit_points: 0,
            is_avatar: false,
        };
        let converted = U4PreviewConverted::from_source(&source);
        Self {
            source,
            converted,
            phase: U4PreviewPhase::Rejected,
            name: String::new(),
            male: true,
            entry: String::new(),
        }
    }

    pub const fn phase(&self) -> U4PreviewPhase {
        self.phase
    }

    pub const fn source(&self) -> &U4PreviewSource {
        &self.source
    }

    pub const fn converted(&self) -> U4PreviewConverted {
        self.converted
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn male(&self) -> bool {
        self.male
    }

    pub fn entry(&self) -> &str {
        &self.entry
    }

    /// `§6.5`/`§6.6`: the verdict written at the end of the run.
    pub const fn verdict_text(&self) -> &'static str {
        if self.source.is_avatar {
            U4_PREVIEW_AVATAR_CLASS_TEXT
        } else {
            U4_PREVIEW_NON_AVATAR_TEXT
        }
    }

    /// The commit-side record, with the player's name and gender
    /// corrections already applied.
    pub fn committed_source(&self) -> U4TransferSource {
        let mut source = self.source.to_transfer_source();
        source.name = self.name.as_bytes().to_vec();
        source.male = self.male;
        source
    }

    /// `§6.5`: the left panel's unconverted contents, as panel-relative
    /// placements. Filled once, when the comparison screen is built.
    pub fn left_panel_lines(&self) -> Vec<U4PreviewPageLine> {
        let source = &self.source;
        vec![
            centred_line(U4_PREVIEW_NAME_ROW, source.name.clone()),
            value_line(5, source.sex_text().to_string()),
            value_line(6, source.class_text().to_string()),
            value_line(8, source.experience.to_string()),
            value_line(9, source.staged_level().to_string()),
            value_line(11, source.strength.to_string()),
            value_line(12, source.dexterity.to_string()),
            value_line(13, source.intelligence.to_string()),
            centred_line(
                U4_PREVIEW_AVATAR_VERDICT_ROW,
                self.verdict_text().to_string(),
            ),
        ]
    }

    /// `§6.3`: the "Found" page, in draw order. The first two lines are
    /// centred; centring is turned off after the name.
    pub fn found_page_lines(&self) -> Vec<U4PreviewPageLine> {
        let source = &self.source;
        let mut lines = vec![
            centred_line(
                U4_PREVIEW_FOUND_LABEL_ROW,
                U4_PREVIEW_FOUND_LABEL_TEXT.to_string(),
            ),
            centred_line(U4_PREVIEW_FOUND_NAME_ROW, source.name.clone()),
            U4PreviewPageLine {
                row: U4_PREVIEW_FOUND_DESCRIPTION_CELL.1,
                column: Some(U4_PREVIEW_FOUND_DESCRIPTION_CELL.0),
                text: format!(
                    "a level {} {} {}",
                    source.staged_level(),
                    source.sex_text(),
                    U4_PREVIEW_CLASS_NAMES[usize::from(source.class_index)]
                ),
            },
        ];
        let stats = [source.strength, source.dexterity, source.intelligence];
        for ((row, prefix), value) in U4_PREVIEW_FOUND_STAT_ROWS
            .iter()
            .zip(U4_PREVIEW_FOUND_STAT_PREFIXES)
            .zip(stats)
        {
            lines.push(U4PreviewPageLine {
                row: *row,
                column: Some(U4_PREVIEW_FOUND_STAT_COLUMN),
                text: format!("{prefix}{value}"),
            });
        }
        lines.push(U4PreviewPageLine {
            row: U4_PREVIEW_FOUND_VERDICT_CELL.1,
            column: Some(U4_PREVIEW_FOUND_VERDICT_CELL.0),
            text: format!(
                "{} is {}",
                source.name,
                if source.is_avatar {
                    "an Avatar."
                } else {
                    "not an Avatar"
                }
            ),
        });
        lines
    }

    /// `§6.3`: the rejected-source page, centred from cell `(0, 5)`.
    pub fn rejected_page_lines(&self) -> Vec<U4PreviewPageLine> {
        U4_PREVIEW_REJECTED_LINES
            .iter()
            .enumerate()
            .filter(|(_, text)| !text.is_empty())
            .map(|(index, text)| {
                centred_line(
                    U4_PREVIEW_REJECTED_FIRST_ROW + index as u8,
                    (*text).to_string(),
                )
            })
            .collect()
    }

    /// `§6.5` move (b): the value this stage writes into the right
    /// panel, or `None` where the published effect column has no value
    /// write. Name confirm's effect is published as "none".
    fn right_value(&self, stage: U4PreviewStage) -> Option<String> {
        match stage {
            U4PreviewStage::NameConfirm | U4PreviewStage::NameReplace => None,
            U4PreviewStage::SexConfirm => Some(u4_preview_sex_text(self.male).to_string()),
            U4PreviewStage::Class => Some(self.source.class_text().to_string()),
            U4PreviewStage::Experience => Some(self.converted.experience.to_string()),
            U4PreviewStage::Level => Some(self.converted.level.to_string()),
            U4PreviewStage::Strength => Some(self.converted.strength.to_string()),
            U4PreviewStage::Dexterity => Some(self.converted.dexterity.to_string()),
            U4PreviewStage::Intellect => Some(self.converted.intelligence.to_string()),
        }
    }

    /// `§6.5`: the message-line content for a stage.
    pub fn stage_message(&self, stage: U4PreviewStage) -> U4PreviewMessage {
        let cell = |(x, y): (u8, u8)| U4PreviewMessagePlacement::Cell(x, y);
        match stage {
            U4PreviewStage::NameConfirm => U4PreviewMessage {
                text: U4_PREVIEW_NAME_CONFIRM_TEXT.to_string(),
                placement: cell(U4_PREVIEW_NAME_CONFIRM_CURSOR),
            },
            U4PreviewStage::NameReplace => U4PreviewMessage {
                text: U4_PREVIEW_NAME_REPLACE_PROMPT.to_string(),
                placement: cell(U4_PREVIEW_NAME_REPLACE_CURSOR),
            },
            U4PreviewStage::SexConfirm => U4PreviewMessage {
                text: U4_PREVIEW_SEX_CONFIRM_TEXT.to_string(),
                placement: U4PreviewMessagePlacement::Centred,
            },
            U4PreviewStage::Class => U4PreviewMessage {
                text: if self.source.is_avatar {
                    U4_PREVIEW_CLASS_AVATAR_TEXT.to_string()
                } else {
                    U4_PREVIEW_CLASS_INTACT_TEXT.to_string()
                },
                placement: cell(U4_PREVIEW_CLASS_CURSOR),
            },
            U4PreviewStage::Experience => U4PreviewMessage {
                text: U4_PREVIEW_EXPERIENCE_TEXT.to_string(),
                placement: U4PreviewMessagePlacement::Centred,
            },
            U4PreviewStage::Level => U4PreviewMessage {
                text: U4_PREVIEW_LEVEL_TEXT.to_string(),
                placement: U4PreviewMessagePlacement::Centred,
            },
            U4PreviewStage::Strength => U4PreviewMessage {
                text: u4_preview_attribute_message_text(
                    U4_PREVIEW_STRENGTH_LABEL,
                    self.source.strength,
                    self.converted.strength,
                ),
                placement: cell(U4_PREVIEW_ATTRIBUTE_CURSOR),
            },
            U4PreviewStage::Dexterity => U4PreviewMessage {
                text: u4_preview_attribute_message_text(
                    U4_PREVIEW_DEXTERITY_LABEL,
                    self.source.dexterity,
                    self.converted.dexterity,
                ),
                placement: cell(U4_PREVIEW_ATTRIBUTE_CURSOR),
            },
            U4PreviewStage::Intellect => U4PreviewMessage {
                text: u4_preview_attribute_message_text(
                    U4_PREVIEW_INTELLECT_LABEL,
                    self.source.intelligence,
                    self.converted.intelligence,
                ),
                placement: cell(U4_PREVIEW_ATTRIBUTE_CURSOR),
            },
        }
    }

    /// `§6.5`: the four moves for entering `stage` from `previous`.
    fn stage_edit(&self, stage: U4PreviewStage, previous: Option<U4PreviewStage>) -> U4PreviewEdit {
        U4PreviewEdit::Stage(Box::new(U4PreviewStageEdit {
            stage,
            previous_label_row: previous.map(U4PreviewStage::row),
            inverse_label_row: stage.row(),
            right_value: self.right_value(stage),
            message: self.stage_message(stage),
        }))
    }

    /// `§6.5`: the first stage's four moves, issued once the comparison
    /// screen exists. There is no previous stage to un-highlight.
    pub fn opening_stage_edit(&self) -> U4PreviewEdit {
        self.stage_edit(U4PreviewStage::NameConfirm, None)
    }

    fn enter(&mut self, stage: U4PreviewStage, previous: U4PreviewStage) -> U4PreviewEdit {
        self.phase = U4PreviewPhase::Stage(stage);
        self.stage_edit(stage, Some(previous))
    }

    /// Feed one keystroke.
    pub fn key(&mut self, key: char) -> U4PreviewResponse {
        match self.phase {
            // `§6.3`: waits for any key and returns to the menu with
            // nothing written.
            U4PreviewPhase::Rejected => U4PreviewResponse {
                edits: Vec::new(),
                action: U4PreviewAction::ReturnToMenu,
            },
            // `§6.3`: the page waits for any key and is then cleared.
            U4PreviewPhase::Found => {
                self.phase = U4PreviewPhase::Stage(U4PreviewStage::NameConfirm);
                U4PreviewResponse {
                    edits: vec![self.opening_stage_edit()],
                    action: U4PreviewAction::BuildComparisonScreen,
                }
            }
            U4PreviewPhase::Stage(stage) => self.stage_key(stage, key),
            U4PreviewPhase::Complete => U4PreviewResponse::ignored(),
        }
    }

    fn stage_key(&mut self, stage: U4PreviewStage, key: char) -> U4PreviewResponse {
        // `§6.5`: `Esc` at any of these prompts is simply ignored and
        // there is no cancel path back to the menu. It is not special
        // cased here — it is just a key no stage accepts as an answer.
        if stage.accepts_only_yes_or_no() {
            return match key.to_ascii_uppercase() {
                'Y' => self.confirm(stage, true),
                'N' => self.confirm(stage, false),
                // Discarded silently: no beep, no message, no redraw.
                _ => U4PreviewResponse::ignored(),
            };
        }

        if stage == U4PreviewStage::NameReplace {
            return self.name_entry_key(key);
        }

        // `§6.5`: the informational stages accept any key.
        let next = match stage {
            U4PreviewStage::Class => U4PreviewStage::Experience,
            U4PreviewStage::Experience => U4PreviewStage::Level,
            U4PreviewStage::Level => U4PreviewStage::Strength,
            U4PreviewStage::Strength => U4PreviewStage::Dexterity,
            U4PreviewStage::Dexterity => U4PreviewStage::Intellect,
            U4PreviewStage::Intellect => return self.finish(),
            U4PreviewStage::NameConfirm
            | U4PreviewStage::NameReplace
            | U4PreviewStage::SexConfirm => {
                unreachable!("confirmation and typed-entry stages are handled above")
            }
        };
        U4PreviewResponse {
            edits: vec![self.enter(next, stage)],
            action: U4PreviewAction::None,
        }
    }

    fn confirm(&mut self, stage: U4PreviewStage, yes: bool) -> U4PreviewResponse {
        match stage {
            U4PreviewStage::NameConfirm => {
                // `§6.5`: name confirm's published effect is "none" —
                // nothing is written to the right panel on either
                // branch. `N` opens the replacement field, `Y` moves on.
                let next = if yes {
                    U4PreviewStage::SexConfirm
                } else {
                    self.entry.clear();
                    U4PreviewStage::NameReplace
                };
                U4PreviewResponse {
                    edits: vec![self.enter(next, stage)],
                    action: U4PreviewAction::None,
                }
            }
            U4PreviewStage::SexConfirm => {
                // `Y` keeps the imported gender, `N` flips it; the
                // resulting `Male` or `Female` is written to right-panel
                // `(10, 5)`.
                if !yes {
                    self.male = !self.male;
                }
                let edits = vec![
                    U4PreviewEdit::RightValue {
                        row: stage.row(),
                        text: u4_preview_sex_text(self.male).to_string(),
                    },
                    self.enter(U4PreviewStage::Class, stage),
                ];
                U4PreviewResponse {
                    edits,
                    action: U4PreviewAction::None,
                }
            }
            _ => unreachable!("only the two confirmation stages take Y/N"),
        }
    }

    fn name_entry_key(&mut self, key: char) -> U4PreviewResponse {
        match key {
            '\r' | '\n' => {
                // `§6.5`: the stage repeats while the entered name is
                // empty, so a blank name can never be accepted.
                if self.entry.trim().is_empty() {
                    self.entry.clear();
                    return U4PreviewResponse {
                        edits: vec![U4PreviewEdit::NameField(String::new())],
                        action: U4PreviewAction::None,
                    };
                }
                self.name = self.entry.clone();
                let edits = vec![
                    U4PreviewEdit::AcceptedName(self.name.clone()),
                    self.enter(U4PreviewStage::SexConfirm, U4PreviewStage::NameReplace),
                ];
                U4PreviewResponse {
                    edits,
                    action: U4PreviewAction::None,
                }
            }
            '\x08' | '\x7f' => {
                self.entry.pop();
                U4PreviewResponse {
                    edits: vec![U4PreviewEdit::NameField(self.entry.clone())],
                    action: U4PreviewAction::None,
                }
            }
            key if key == ' ' || key.is_ascii_graphic() => {
                if self.entry.chars().count() >= U4_PREVIEW_NAME_ENTRY_MAX_CHARS {
                    return U4PreviewResponse::ignored();
                }
                self.entry.push(key);
                U4PreviewResponse {
                    edits: vec![U4PreviewEdit::NameField(self.entry.clone())],
                    action: U4PreviewAction::None,
                }
            }
            _ => U4PreviewResponse::ignored(),
        }
    }

    fn finish(&mut self) -> U4PreviewResponse {
        self.phase = U4PreviewPhase::Complete;
        U4PreviewResponse {
            edits: vec![U4PreviewEdit::Finish {
                verdict: self.verdict_text().to_string(),
                notice: U4_PREVIEW_COMMIT_TEXT.to_string(),
            }],
            action: U4PreviewAction::Commit,
        }
    }
}

fn centred_line(row: u8, text: String) -> U4PreviewPageLine {
    U4PreviewPageLine {
        row,
        column: None,
        text,
    }
}

fn value_line(row: u8, text: String) -> U4PreviewPageLine {
    U4PreviewPageLine {
        row,
        column: Some(U4_PREVIEW_VALUE_COLUMN),
        text,
    }
}

/// Build a **synthetic** `PARTY.SAV` image from the published `§5.1`
/// layout.
///
/// This is a *test input* constructed from a published file format, not
/// a captured, copied or reconstructed Ultima IV save. No Ultima IV
/// install or genuine `PARTY.SAV` is used anywhere in this repository,
/// and nothing ships this as real data.
pub fn synthetic_party_sav_fixture(source: &U4PreviewSource) -> Vec<u8> {
    let mut bytes = vec![0u8; U4_PREVIEW_PARTY_BLOCK_OFFSET + U4_PREVIEW_PARTY_BLOCK_LEN];
    let record = U4_PREVIEW_RECORD_OFFSET;
    for (offset, value) in [
        (RECORD_HP, source.max_hit_points),
        (RECORD_MAX_HP, source.max_hit_points),
        (RECORD_EXPERIENCE, source.experience),
        (RECORD_STRENGTH, source.strength),
        (RECORD_DEXTERITY, source.dexterity),
        (RECORD_INTELLIGENCE, source.intelligence),
    ] {
        let at = record + offset;
        bytes[at..at + 2].copy_from_slice(&value.to_le_bytes());
    }
    for (index, byte) in source
        .name
        .bytes()
        .take(U4_PREVIEW_VALIDATED_NAME_BYTES)
        .enumerate()
    {
        bytes[record + RECORD_NAME + index] = byte;
    }
    bytes[record + RECORD_SEX] = if source.male {
        U4_PREVIEW_MALE_MARKER
    } else {
        0
    };
    bytes[record + RECORD_CLASS] = source.class_index;
    if !source.is_avatar {
        // `§5.3`: one nonzero standing is enough to fail the test.
        bytes[U4_PREVIEW_VIRTUE_STANDING_OFFSET] = 1;
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn avatar_source() -> U4PreviewSource {
        U4PreviewSource {
            name: "Dupre".to_string(),
            male: true,
            class_index: 5,
            strength: 35,
            dexterity: 20,
            intelligence: 22,
            experience: 1500,
            max_hit_points: 300,
            is_avatar: true,
        }
    }

    fn plain_source() -> U4PreviewSource {
        U4PreviewSource {
            is_avatar: false,
            class_index: 2,
            ..avatar_source()
        }
    }

    fn session_at_first_stage(source: U4PreviewSource) -> U4PreviewSession {
        let mut session = U4PreviewSession::new(source);
        assert_eq!(session.phase(), U4PreviewPhase::Found);
        let response = session.key(' ');
        assert_eq!(response.action, U4PreviewAction::BuildComparisonScreen);
        session
    }

    fn stage_edit(edit: &U4PreviewEdit) -> &U4PreviewStageEdit {
        match edit {
            U4PreviewEdit::Stage(edit) => edit,
            other => panic!("expected a stage edit, got {other:?}"),
        }
    }

    #[test]
    fn panel_titles_differ_by_the_single_blanked_cell() {
        assert_eq!(
            u4_preview_panel_title_text(U4PreviewPanel::Source),
            " Ultima IV "
        );
        assert_eq!(
            u4_preview_panel_title_text(U4PreviewPanel::Result),
            " Ultima  V "
        );
    }

    #[test]
    fn panel_title_cells_and_caps_match_the_published_row() {
        assert_eq!(U4_PREVIEW_PANEL_TITLE_ROW_CELLS, 19);
        assert_eq!(U4_PREVIEW_PANEL_TITLE_LEFT_CAP_CELL, 3);
        assert_eq!(U4_PREVIEW_PANEL_TITLE_FIRST_CELL, 4);
        assert_eq!(U4_PREVIEW_PANEL_TITLE_TEXT.len(), 11);
        assert_eq!(U4_PREVIEW_PANEL_TITLE_RIGHT_CAP_CELL, 15);
        assert_eq!(U4_PREVIEW_PANEL_TITLE_SOLID_CELLS, [1, 2, 16, 17]);
        assert_eq!(U4_PREVIEW_PANEL_BOTTOM_LEFT_CELL, (0, 17));
        assert_eq!(U4_PREVIEW_PANEL_BOTTOM_RIGHT_CELL, (18, 17));
    }

    #[test]
    fn panel_origins_bars_and_polyline_match_the_published_geometry() {
        assert_eq!(U4PreviewPanel::Source.origin_x(), 0);
        assert_eq!(U4PreviewPanel::Result.origin_x(), 168);
        assert_eq!(
            u4_preview_panel_bars(168),
            [(168, 0, 174, 143), (311, 0, 319, 137), (175, 137, 318, 143)]
        );
        assert_eq!(
            u4_preview_panel_rule_polyline(0),
            [(24, 7), (7, 7), (7, 136), (143, 136), (143, 7), (128, 7)]
        );
    }

    #[test]
    fn field_labels_carry_their_exact_leading_spaces() {
        assert_eq!(
            U4_PREVIEW_FIELD_LABELS,
            [
                (2, "    Name:"),
                (5, "  Sex:"),
                (6, "Class:"),
                (8, "  Exp:"),
                (9, "Level:"),
                (11, "  STR:"),
                (12, "  DEX:"),
                (13, "  INT:"),
            ]
        );
        assert_eq!(U4_PREVIEW_FIELD_LABEL_COLUMN, 3);
    }

    #[test]
    fn field_label_words_land_on_the_published_columns() {
        // `§6.2`: the padding right-aligns the words so each begins at
        // column 7, 5 or 3 of the panel.
        let word_column = |label: &str| {
            U4_PREVIEW_FIELD_LABEL_COLUMN as usize + label.len() - label.trim_start().len()
        };
        assert_eq!(word_column("    Name:"), 7);
        for label in ["  Sex:", "  Exp:", "  STR:", "  DEX:", "  INT:"] {
            assert_eq!(word_column(label), 5, "{label}");
        }
        for label in ["Class:", "Level:"] {
            assert_eq!(word_column(label), 3, "{label}");
        }
    }

    #[test]
    fn there_are_exactly_eight_fields_and_no_hp_mp_or_equipment_labels() {
        assert_eq!(U4_PREVIEW_FIELD_LABELS.len(), 8);
        let joined = U4_PREVIEW_FIELD_LABELS
            .iter()
            .map(|(_, text)| text.trim())
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(joined, "Name: Sex: Class: Exp: Level: STR: DEX: INT:");
    }

    #[test]
    fn window_bounds_match_the_published_rectangles() {
        let bounds = |w: TextWindowDescriptor| {
            (
                w.top_left_x,
                w.top_left_y,
                w.bottom_right_x,
                w.bottom_right_y,
            )
        };
        assert_eq!(bounds(U4_PREVIEW_LEFT_PANEL_WINDOW), (0, 0, 19, 18));
        assert_eq!(bounds(U4_PREVIEW_RIGHT_PANEL_WINDOW), (21, 0, 39, 18));
        assert_eq!(bounds(U4_PREVIEW_MESSAGE_LINE_WINDOW), (3, 21, 37, 21));
        assert_eq!(bounds(U4_PREVIEW_COMMIT_MESSAGE_WINDOW), (2, 21, 37, 22));
        assert_eq!(bounds(U4_PREVIEW_FULL_SCREEN_WINDOW), (0, 0, 39, 24));
    }

    #[test]
    fn prompt_frame_cells_are_the_published_glyph_positions() {
        let cells = u4_preview_prompt_frame_cells();
        // Two 40-cell edge rows plus two side cells on each of rows
        // 20, 21 and 22.
        assert_eq!(cells.len(), 40 + 40 + 6);

        let at = |column: u8, row: u8| {
            cells
                .iter()
                .find(|cell| cell.column == column && cell.row == row)
                .unwrap_or_else(|| panic!("no frame cell at ({column}, {row})"))
                .glyph
        };
        assert_eq!(at(0, 19), U4_PREVIEW_FRAME_GLYPH_TOP_LEFT);
        assert_eq!(at(39, 19), U4_PREVIEW_FRAME_GLYPH_TOP_RIGHT);
        assert_eq!(at(0, 23), U4_PREVIEW_FRAME_GLYPH_BOTTOM_LEFT);
        assert_eq!(at(39, 23), U4_PREVIEW_FRAME_GLYPH_BOTTOM_RIGHT);
        for column in 1..=38 {
            assert_eq!(at(column, 19), U4_PREVIEW_FRAME_GLYPH_SOLID);
            assert_eq!(at(column, 23), U4_PREVIEW_FRAME_GLYPH_SOLID);
        }
        for row in [20, 21, 22] {
            assert_eq!(at(0, row), U4_PREVIEW_FRAME_GLYPH_SOLID);
            assert_eq!(at(39, row), U4_PREVIEW_FRAME_GLYPH_SOLID);
            // The interior is not cleared or written per prompt.
            assert!(
                !cells
                    .iter()
                    .any(|cell| cell.row == row && cell.column != 0 && cell.column != 39)
            );
        }
    }

    #[test]
    fn prompt_frame_interior_is_three_rows_not_the_menu_frames_eight() {
        assert_eq!(U4_PREVIEW_FRAME_SIDE_ROWS.len(), 3);
        assert_eq!(
            U4_PREVIEW_FRAME_RULE_PATH,
            [(7, 159), (312, 159), (312, 184), (7, 184), (7, 159)]
        );
    }

    #[test]
    fn stage_rows_match_the_label_rows() {
        use U4PreviewStage::*;
        for (stage, row) in [
            (NameConfirm, 2),
            (NameReplace, 2),
            (SexConfirm, 5),
            (Class, 6),
            (Experience, 8),
            (Level, 9),
            (Strength, 11),
            (Dexterity, 12),
            (Intellect, 13),
        ] {
            assert_eq!(stage.row(), row, "{stage:?}");
            assert!(u4_preview_field_label(row).is_some(), "{stage:?}");
        }
    }

    #[test]
    fn stage_messages_and_cursors_match_the_published_table() {
        let session = session_at_first_stage(avatar_source());
        let expect = |stage, text: &str, placement| {
            assert_eq!(
                session.stage_message(stage),
                U4PreviewMessage {
                    text: text.to_string(),
                    placement,
                },
                "{stage:?}"
            );
        };
        use U4PreviewMessagePlacement::{Cell, Centred};
        expect(U4PreviewStage::NameConfirm, "Keep this name?", Cell(10, 0));
        expect(U4PreviewStage::NameReplace, "Enter new name: ", Cell(1, 0));
        expect(U4PreviewStage::SexConfirm, "Keep same sex?", Centred);
        expect(U4PreviewStage::Class, "Thou art now an Avatar:", Cell(2, 0));
        expect(
            U4PreviewStage::Experience,
            "Experience has been converted",
            Centred,
        );
        expect(U4PreviewStage::Level, "Level has been converted", Centred);
        expect(
            U4PreviewStage::Strength,
            "Strength: was 35(50), now 21(30)",
            Cell(1, 0),
        );
        expect(
            U4PreviewStage::Dexterity,
            "Dexterity: was 20(50), now 15(30)",
            Cell(1, 0),
        );
        expect(
            U4PreviewStage::Intellect,
            "Intellect: was 22(50), now 16(30)",
            Cell(1, 0),
        );
    }

    #[test]
    fn name_replace_prompt_is_sixteen_characters_so_the_field_starts_after_it() {
        assert_eq!(U4_PREVIEW_NAME_REPLACE_PROMPT.len(), 16);
        assert_eq!(U4_PREVIEW_NAME_ENTRY_MAX_CHARS, 8);
    }

    #[test]
    fn non_avatar_class_stage_says_class_remains_intact() {
        let session = session_at_first_stage(plain_source());
        assert_eq!(
            session.stage_message(U4PreviewStage::Class).text,
            "Class remains intact"
        );
    }

    #[test]
    fn confirmation_prompts_discard_every_key_but_y_and_n() {
        let mut session = session_at_first_stage(avatar_source());
        for key in ['A', 'z', '1', ' ', '\r', '\x1b', '\x08'] {
            let response = session.key(key);
            assert!(response.edits.is_empty(), "{key:?} redrew something");
            assert_eq!(response.action, U4PreviewAction::None);
            assert_eq!(
                session.phase(),
                U4PreviewPhase::Stage(U4PreviewStage::NameConfirm),
                "{key:?} advanced the stage machine"
            );
        }
        assert_eq!(session.key('y').edits.len(), 1);
        assert_eq!(
            session.phase(),
            U4PreviewPhase::Stage(U4PreviewStage::SexConfirm)
        );
    }

    #[test]
    fn escape_never_aborts_the_transfer_once_the_screen_is_up() {
        let mut session = session_at_first_stage(avatar_source());
        for _ in 0..4 {
            let response = session.key('\x1b');
            assert_eq!(response.action, U4PreviewAction::None);
            assert!(response.edits.is_empty());
        }
        assert_eq!(
            session.phase(),
            U4PreviewPhase::Stage(U4PreviewStage::NameConfirm)
        );
    }

    #[test]
    fn a_blank_replacement_name_is_never_accepted() {
        let mut session = session_at_first_stage(avatar_source());
        session.key('N');
        assert_eq!(
            session.phase(),
            U4PreviewPhase::Stage(U4PreviewStage::NameReplace)
        );
        for _ in 0..3 {
            let response = session.key('\r');
            assert_eq!(response.action, U4PreviewAction::None);
            assert_eq!(
                session.phase(),
                U4PreviewPhase::Stage(U4PreviewStage::NameReplace),
                "an empty name was accepted"
            );
        }
        // Spaces alone are still empty.
        session.key(' ');
        session.key('\r');
        assert_eq!(
            session.phase(),
            U4PreviewPhase::Stage(U4PreviewStage::NameReplace)
        );
        for key in "Shamino".chars() {
            session.key(key);
        }
        session.key('\r');
        assert_eq!(session.name(), "Shamino");
        assert_eq!(
            session.phase(),
            U4PreviewPhase::Stage(U4PreviewStage::SexConfirm)
        );
    }

    #[test]
    fn the_replacement_name_field_stops_at_eight_characters() {
        let mut session = session_at_first_stage(avatar_source());
        session.key('N');
        for key in "ABCDEFGHIJ".chars() {
            session.key(key);
        }
        assert_eq!(session.entry(), "ABCDEFGH");
    }

    #[test]
    fn a_name_edit_repaints_only_the_typed_run() {
        let mut session = session_at_first_stage(avatar_source());
        session.key('N');
        let response = session.key('A');
        assert_eq!(response.edits, vec![U4PreviewEdit::NameField("A".into())]);
    }

    #[test]
    fn every_stage_redraw_is_the_four_published_moves_and_nothing_else() {
        let mut session = session_at_first_stage(avatar_source());
        let opening = session.opening_stage_edit();
        let opening = stage_edit(&opening);
        assert_eq!(opening.previous_label_row, None);
        assert_eq!(opening.inverse_label_row, 2);
        // `§6.5`: name confirm's published effect is "none".
        assert_eq!(opening.right_value, None);

        let mut previous = U4PreviewStage::NameConfirm;
        let mut keys = vec!['Y', 'Y'];
        keys.extend(['\r'; 6]);
        let expected = [
            (U4PreviewStage::SexConfirm, Some("Male")),
            (U4PreviewStage::Class, Some("Avatar")),
            (U4PreviewStage::Experience, Some("150")),
            (U4PreviewStage::Level, Some("2")),
            (U4PreviewStage::Strength, Some("21")),
            (U4PreviewStage::Dexterity, Some("15")),
            (U4PreviewStage::Intellect, Some("16")),
        ];
        let mut seen = 0;
        for key in keys {
            let response = session.key(key);
            for edit in &response.edits {
                let U4PreviewEdit::Stage(edit) = edit else {
                    continue;
                };
                let (stage, value) = expected[seen];
                assert_eq!(edit.stage, stage);
                assert_eq!(edit.previous_label_row, Some(previous.row()));
                assert_eq!(edit.inverse_label_row, stage.row());
                assert_eq!(edit.right_value.as_deref(), value);
                assert_eq!(edit.message, session.stage_message(stage));
                previous = stage;
                seen += 1;
            }
            if seen == expected.len() {
                break;
            }
        }
        assert_eq!(seen, expected.len());
    }

    #[test]
    fn the_sex_flip_rewrites_only_one_right_panel_cell_run() {
        let mut session = session_at_first_stage(avatar_source());
        session.key('Y');
        let response = session.key('N');
        assert_eq!(
            response.edits[0],
            U4PreviewEdit::RightValue {
                row: 5,
                text: "Female".to_string(),
            }
        );
        assert!(!session.male());
    }

    #[test]
    fn the_last_keypress_commits_immediately_with_the_notice_on_screen() {
        let mut session = session_at_first_stage(avatar_source());
        session.key('Y');
        let mut response = session.key('Y');
        // Class, Experience, Level, Strength, Dexterity, Intellect.
        for _ in 0..6 {
            response = session.key('\r');
        }
        assert_eq!(response.action, U4PreviewAction::Commit);
        assert_eq!(
            response.edits,
            vec![U4PreviewEdit::Finish {
                verdict: "Avatar".to_string(),
                notice: " Conversion complete, saving...".to_string(),
            }]
        );
        assert_eq!(session.phase(), U4PreviewPhase::Complete);
        assert_eq!(U4_PREVIEW_COMMIT_ROW, 21);
        assert_eq!(U4_PREVIEW_COMMIT_COLUMN, 2);
    }

    #[test]
    fn rejected_source_page_is_the_published_text_and_takes_any_key() {
        let mut session = U4PreviewSession::rejected();
        let lines = session.rejected_page_lines();
        assert_eq!(
            lines
                .iter()
                .map(|line| (line.row, line.column, line.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (5, None, "Error:  Your Ultima IV game"),
                (7, None, "contains bad data."),
                (9, None, "Unable to continue transfer."),
                (12, None, "Press any key to return to the menu."),
            ]
        );
        let response = session.key('\x1b');
        assert_eq!(response.action, U4PreviewAction::ReturnToMenu);
        assert!(response.edits.is_empty());
    }

    #[test]
    fn found_page_matches_the_published_cells() {
        let session = U4PreviewSession::new(plain_source());
        let lines = session.found_page_lines();
        assert_eq!(
            lines
                .iter()
                .map(|line| (line.column, line.row, line.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (None, 11, "Found:"),
                (None, 12, "Dupre"),
                (Some(12), 13, "a level 3 Male Fighter"),
                (Some(17), 15, "STR:  35"),
                (Some(17), 16, "DEX:  20"),
                (Some(17), 17, "INT:  22"),
                (Some(10), 20, "Dupre is not an Avatar"),
            ]
        );
    }

    #[test]
    fn found_page_level_is_the_staged_max_hp_figure_not_the_converted_level() {
        let source = avatar_source();
        assert_eq!(source.staged_level(), 3);
        assert_eq!(U4PreviewConverted::from_source(&source).level, 2);
    }

    #[test]
    fn left_panel_shows_the_unconverted_source_values() {
        let session = U4PreviewSession::new(avatar_source());
        let lines = session.left_panel_lines();
        assert_eq!(
            lines
                .iter()
                .map(|line| (line.column, line.row, line.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (None, 3, "Dupre"),
                (Some(10), 5, "Male"),
                (Some(10), 6, "Avatar"),
                (Some(10), 8, "1500"),
                (Some(10), 9, "3"),
                (Some(10), 11, "35"),
                (Some(10), 12, "20"),
                (Some(10), 13, "22"),
                (None, 15, "Avatar"),
            ]
        );
    }

    #[test]
    fn synthetic_fixture_round_trips_through_the_published_parser() {
        let expected = avatar_source();
        let bytes = synthetic_party_sav_fixture(&expected);
        assert_eq!(parse_u4_preview_source(&bytes).unwrap(), expected);

        let plain = plain_source();
        let bytes = synthetic_party_sav_fixture(&plain);
        assert_eq!(parse_u4_preview_source(&bytes).unwrap(), plain);
    }

    #[test]
    fn all_zero_virtue_standings_mark_an_avatar_and_never_reject() {
        let mut bytes = synthetic_party_sav_fixture(&plain_source());
        for index in 0..U4_PREVIEW_VIRTUE_STANDING_COUNT {
            let at = U4_PREVIEW_VIRTUE_STANDING_OFFSET + index * 2;
            bytes[at] = 0;
            bytes[at + 1] = 0;
        }
        let parsed = parse_u4_preview_source(&bytes).expect("all-zero standings never reject");
        assert!(parsed.is_avatar);
    }

    #[test]
    fn out_of_range_source_fields_reject_the_transfer() {
        let mut bytes = synthetic_party_sav_fixture(&avatar_source());
        let strength = U4_PREVIEW_RECORD_OFFSET + RECORD_STRENGTH;
        bytes[strength..strength + 2].copy_from_slice(&71u16.to_le_bytes());
        assert!(matches!(
            parse_u4_preview_source(&bytes),
            Err(U4PreviewSourceRejection::OutOfRange {
                field: "strength",
                value: 71,
                max: 70,
            })
        ));

        let mut bytes = synthetic_party_sav_fixture(&avatar_source());
        bytes[U4_PREVIEW_RECORD_OFFSET + RECORD_CLASS] = 8;
        assert!(matches!(
            parse_u4_preview_source(&bytes),
            Err(U4PreviewSourceRejection::OutOfRange {
                field: "class index",
                ..
            })
        ));

        let mut bytes = synthetic_party_sav_fixture(&avatar_source());
        bytes[U4_PREVIEW_RECORD_OFFSET + RECORD_NAME] = 0x07;
        assert!(matches!(
            parse_u4_preview_source(&bytes),
            Err(U4PreviewSourceRejection::NameByte(0x07))
        ));
    }

    #[test]
    fn party_wide_counters_never_reject_a_transfer() {
        // `§5.2`: gold, food, gems, torches, keys, sextants, moves,
        // moon phase and dungeon progress are never read on this path.
        let mut bytes = synthetic_party_sav_fixture(&avatar_source());
        // The two leading counters `§5.1` says are skipped.
        for byte in &mut bytes[..U4_PREVIEW_RECORD_OFFSET] {
            *byte = 0xff;
        }
        // The party-wide block's food and gold head, and every item
        // counter that sits after the eight standings.
        for byte in &mut bytes[U4_PREVIEW_PARTY_BLOCK_OFFSET..U4_PREVIEW_VIRTUE_STANDING_OFFSET] {
            *byte = 0xff;
        }
        let after_standings =
            U4_PREVIEW_VIRTUE_STANDING_OFFSET + U4_PREVIEW_VIRTUE_STANDING_COUNT * 2;
        for byte in &mut bytes[after_standings..] {
            *byte = 0xff;
        }
        let parsed = parse_u4_preview_source(&bytes).expect("party-wide counters are never read");
        assert!(parsed.is_avatar, "the standings themselves are untouched");
    }

    #[test]
    fn insert_disk_instructions_are_never_authored() {
        // `§6.4`: the shipped build's insert-disk block is unreachable
        // dead code. None of its strings may appear anywhere on this
        // path.
        let module = include_str!("u4_transfer_preview.rs");
        for dead in [
            "Please insert the Ultima IV Player Disk",
            "and press drive letter",
            "or press <Esc> to abort transfer",
            "Transfer Character from Ultima IV",
        ] {
            let occurrences = module.matches(dead).count();
            assert!(
                occurrences <= 1,
                "{dead:?} is drawn somewhere instead of only being named in this guard"
            );
        }
        let authored = [
            U4_PREVIEW_NAME_CONFIRM_TEXT,
            U4_PREVIEW_NAME_REPLACE_PROMPT,
            U4_PREVIEW_SEX_CONFIRM_TEXT,
            U4_PREVIEW_CLASS_AVATAR_TEXT,
            U4_PREVIEW_CLASS_INTACT_TEXT,
            U4_PREVIEW_EXPERIENCE_TEXT,
            U4_PREVIEW_LEVEL_TEXT,
            U4_PREVIEW_COMMIT_TEXT,
        ];
        for text in authored.iter().chain(U4_PREVIEW_REJECTED_LINES.iter()) {
            assert!(!text.contains("insert"), "{text:?}");
            assert!(!text.contains("drive letter"), "{text:?}");
        }
    }
}
