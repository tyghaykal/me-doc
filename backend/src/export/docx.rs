//! DOCX (OOXML) exporter. Consumes the shared block model from `super::blocks`.

use std::io::Cursor;

use docx_rs::*;

use super::blocks::{parse_markdown, Block, Run as IRun};

const MONO: &str = "Courier New";

pub fn markdown_to_docx(md: &str) -> anyhow::Result<Vec<u8>> {
    let mut docx = Docx::new();

    for block in parse_markdown(md) {
        match block {
            Block::Heading { level, runs } => {
                // Half-points; render as bold + larger rather than relying on
                // named "HeadingN" styles (docx-rs doesn't define them by default).
                let size = match level {
                    1 => 36,
                    2 => 32,
                    3 => 28,
                    4 => 26,
                    5 => 24,
                    _ => 22,
                };
                let mut p = Paragraph::new();
                for r in &runs {
                    p = p.add_run(build_run(r).bold().size(size));
                }
                docx = docx.add_paragraph(p);
            }
            Block::Paragraph(runs) => {
                let mut p = Paragraph::new();
                for r in &runs {
                    p = p.add_run(build_run(r));
                }
                docx = docx.add_paragraph(p);
            }
            Block::Quote(runs) => {
                let mut p = Paragraph::new().add_run(Run::new().add_text("> ").italic());
                for r in &runs {
                    p = p.add_run(build_run(r).italic());
                }
                docx = docx.add_paragraph(p);
            }
            Block::Code(text) => {
                for line in text.lines() {
                    let p = Paragraph::new().add_run(
                        Run::new()
                            .add_text(line)
                            .fonts(RunFonts::new().ascii(MONO)),
                    );
                    docx = docx.add_paragraph(p);
                }
            }
            Block::ListItem { marker, depth, runs } => {
                // ponytail: real Word numbering needs AbstractNumbering wiring;
                // a marker-prefixed paragraph reads correctly and is far less code.
                let indent = "    ".repeat(depth);
                let mut p =
                    Paragraph::new().add_run(Run::new().add_text(format!("{indent}{marker}")));
                for r in &runs {
                    p = p.add_run(build_run(r));
                }
                docx = docx.add_paragraph(p);
            }
        }
    }

    let mut buf = Vec::new();
    docx.build().pack(Cursor::new(&mut buf))?;
    Ok(buf)
}

fn build_run(r: &IRun) -> Run {
    let mut run = Run::new().add_text(&r.text);
    if r.bold {
        run = run.bold();
    }
    if r.italic {
        run = run.italic();
    }
    if r.strike {
        run = run.strike();
    }
    if r.code {
        run = run.fonts(RunFonts::new().ascii(MONO));
    }
    run
}
