//! The offline report's plain-text formatter (Phase B design 5.2.11; build
//! plan M5-02).
//!
//! [`format_text`] prints a [`JournalModel`] as plain text: the same
//! sections, rows and gating as the page, with every row's detail lines
//! (the page shows them for the selected row only). There is no mantra
//! masking: at tier 4 an answer that passed the evidence rule is printed.
//! At tiers 3 and 4 the first line says `SPOILERS`.
//!
//! Colour is not available here, so each row keeps its marker glyph (and a
//! restored shrine's `r`); the marker is never the only cue on the page
//! either (5.2.5).

use super::{JournalModel, JournalRow, SectionId, text};

/// The journal as plain text, one `\n`-terminated line each.
pub fn format_text(model: &JournalModel) -> String {
    let mut out = format!(
        "{} - tier {} ({})",
        text::REPORT_TITLE,
        model.tier.number(),
        text::tier_name(model.tier)
    );
    if model.spoilers {
        out.push_str(" - ");
        out.push_str(text::SPOILERS);
    }
    out.push('\n');
    for section in &model.sections {
        out.push_str(section.id.title());
        if section.id == SectionId::Virtues {
            out.push_str(": ");
            out.push_str(&model.header);
        }
        out.push('\n');
        let width = |text: fn(&JournalRow) -> &str| {
            section
                .rows
                .iter()
                .map(|row| text(row).chars().count())
                .max()
                .unwrap_or(0)
        };
        let columns = Columns {
            label: width(|row| row.label.as_str()),
            value: width(|row| row.value.as_str()),
        };
        for row in &section.rows {
            push_row(&mut out, section.id, row, columns);
        }
    }
    out
}

/// A section's label and value column widths, so values and notes line up.
#[derive(Clone, Copy)]
struct Columns {
    label: usize,
    value: usize,
}

/// Detail lines start two columns in from the label.
const DETAIL_INDENT: &str = "       ";

fn push_row(out: &mut String, section: SectionId, row: &JournalRow, columns: Columns) {
    let mut line = String::from("  ");
    line.push(row.mark.glyph);
    line.push(row.suffix.map_or(' ', |mark| mark.glyph));
    line.push(' ');
    line.push_str(&format!(
        "{:<label$}  {:<value$}",
        row.label.as_str(),
        row.value.as_str(),
        label = columns.label,
        value = columns.value,
    ));
    if let Some((mark, note)) = &row.note {
        line.push_str("  ");
        line.push(mark.glyph);
        line.push(' ');
        line.push_str(note.as_str());
    }
    out.push_str(line.trim_end());
    out.push('\n');
    for detail in &row.detail {
        out.push_str(DETAIL_INDENT);
        out.push_str(detail.as_str());
        out.push('\n');
    }
    if let Some(secret) = row.secret {
        out.push_str(&format!(
            "{DETAIL_INDENT}{}: {secret}\n",
            text::secret_label(section)
        ));
    }
}
