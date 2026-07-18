//! PDF exporter via genpdf. Consumes the shared block model from `super::blocks`.
//!
//! genpdf needs TrueType font files on disk (it embeds no fonts of its own).
//! The production image installs `fonts-liberation`; we probe the standard
//! Debian font dirs and fail cleanly if none are present.

use genpdf::fonts::{FontData, FontFamily};
use genpdf::{elements, style::Style, Document};

use super::blocks::{parse_markdown, Block, Run as IRun};

const FONT_CANDIDATES: &[(&str, &str)] = &[
    ("/usr/share/fonts/truetype/liberation", "LiberationSans"),
    ("/usr/share/fonts/truetype/liberation2", "LiberationSans"),
    ("/usr/share/fonts/truetype/dejavu", "DejaVuSans"),
];

pub fn markdown_to_pdf(md: &str) -> anyhow::Result<Vec<u8>> {
    let mut doc = Document::new(load_font()?);
    doc.set_font_size(11);

    for block in parse_markdown(md) {
        match block {
            Block::Heading { level, runs } => {
                let size = match level {
                    1 => 20,
                    2 => 17,
                    3 => 15,
                    4 => 13,
                    5 => 12,
                    _ => 11,
                };
                let mut p = elements::Paragraph::default();
                for r in &runs {
                    p.push_styled(r.text.clone(), base_style(r).bold().with_font_size(size));
                }
                doc.push(p);
                doc.push(elements::Paragraph::default()); // blank-line spacer
            }
            Block::Paragraph(runs) => {
                doc.push(styled_par("", &runs));
                doc.push(elements::Paragraph::default());
            }
            Block::Quote(runs) => {
                doc.push(styled_par("> ", &runs));
            }
            Block::Code(text) => {
                // ponytail: no monospace font loaded; code renders in the body
                // font at a smaller size. Load a mono TTF if that matters.
                for line in text.lines() {
                    let mut p = elements::Paragraph::default();
                    p.push_styled(line.to_string(), Style::new().with_font_size(10));
                    doc.push(p);
                }
                doc.push(elements::Paragraph::default());
            }
            Block::ListItem { marker, depth, runs } => {
                let indent = "    ".repeat(depth);
                doc.push(styled_par(&format!("{indent}{marker}"), &runs));
            }
        }
    }

    let mut buf = Vec::new();
    doc.render(&mut buf)?;
    Ok(buf)
}

fn styled_par(prefix: &str, runs: &[IRun]) -> elements::Paragraph {
    let mut p = elements::Paragraph::default();
    if !prefix.is_empty() {
        p.push_styled(prefix.to_string(), Style::new());
    }
    for r in runs {
        p.push_styled(r.text.clone(), base_style(r));
    }
    p
}

// ponytail: genpdf's Style has no strikethrough/monospace — bold+italic only.
fn base_style(r: &IRun) -> Style {
    let mut s = Style::new();
    if r.bold {
        s = s.bold();
    }
    if r.italic {
        s = s.italic();
    }
    s
}

fn load_font() -> anyhow::Result<FontFamily<FontData>> {
    for (dir, name) in FONT_CANDIDATES {
        if let Ok(family) = genpdf::fonts::from_files(dir, name, None) {
            return Ok(family);
        }
    }
    anyhow::bail!("no PDF font family found (install fonts-liberation)")
}
