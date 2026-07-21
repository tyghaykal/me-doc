//! DOCX (OOXML) exporter. Consumes the shared block model from `super::blocks`.

use std::collections::HashMap;
use std::io::Cursor;

use docx_rs::*;

use super::blocks::{Block, Run as IRun};

const BODY: &str = "Calibri";
const MONO: &str = "Courier New";
/// 96dpi px → EMU (English Metric Units). 1 inch = 914400 EMU; 96 px/inch.
const PX_TO_EMU: u32 = 9525;
/// Cap embedded images at ~6.5" wide so they fit a standard page with margins.
const MAX_IMAGE_WIDTH_PX: u32 = 624;

pub fn blocks_to_docx(blocks: &[Block], images: &HashMap<String, Vec<u8>>) -> anyhow::Result<Vec<u8>> {
    let mut docx = Docx::new();

    for block in blocks {
        match block {
            Block::Heading { level, runs } => {
                // Half-points; bold + larger rather than relying on named HeadingN styles.
                let size = match level {
                    1 => 36,
                    2 => 32,
                    3 => 28,
                    4 => 26,
                    5 => 24,
                    _ => 22,
                };
                let mut p = Paragraph::new();
                if runs.is_empty() {
                    p = p.add_run(body_run("").bold().size(size));
                } else {
                    for r in runs {
                        p = p.add_run(build_run(r).bold().size(size));
                    }
                }
                docx = docx.add_paragraph(p);
            }
            Block::Paragraph(runs) => {
                let mut p = Paragraph::new();
                if runs.is_empty() {
                    p = p.add_run(body_run(""));
                } else {
                    for r in runs {
                        p = p.add_run(build_run(r));
                    }
                }
                docx = docx.add_paragraph(p);
            }
            Block::Quote(runs) => {
                let mut p = Paragraph::new().add_run(body_run("> ").italic());
                for r in runs {
                    p = p.add_run(build_run(r).italic());
                }
                docx = docx.add_paragraph(p);
            }
            Block::Code(text) => {
                if text.is_empty() {
                    docx = docx.add_paragraph(Paragraph::new().add_run(mono_run("")));
                } else {
                    for line in text.lines() {
                        let p = Paragraph::new().add_run(mono_run(line));
                        docx = docx.add_paragraph(p);
                    }
                }
            }
            Block::ListItem { marker, depth, runs } => {
                let indent = "    ".repeat(*depth);
                let mut p = Paragraph::new().add_run(body_run(format!("{indent}{marker}")));
                for r in runs {
                    p = p.add_run(build_run(r));
                }
                docx = docx.add_paragraph(p);
            }
            Block::Image { alt, url } => {
                if let Some(bytes) = images.get(url) {
                    match embed_image(bytes) {
                        Ok(pic) => {
                            let p = Paragraph::new().add_run(Run::new().add_image(pic));
                            docx = docx.add_paragraph(p);
                        }
                        Err(_) => {
                            let label = image_fallback(alt);
                            docx = docx.add_paragraph(Paragraph::new().add_run(body_run(label).italic()));
                        }
                    }
                } else {
                    let label = image_fallback(alt);
                    docx = docx.add_paragraph(Paragraph::new().add_run(body_run(label).italic()));
                }
            }
        }
    }

    // Word rejects completely empty documents in some viewers — ensure at least
    // one paragraph exists when the page has no content.
    // (docx-rs always produces a valid package even with zero paragraphs, but
    // LibreOffice/Word can open blank; leave as-is.)

    let mut buf = Vec::new();
    docx.build().pack(Cursor::new(&mut buf))?;
    Ok(buf)
}

#[allow(dead_code)]
pub fn markdown_to_docx(md: &str) -> anyhow::Result<Vec<u8>> {
    let blocks = super::blocks::parse_markdown(md);
    blocks_to_docx(&blocks, &HashMap::new())
}

fn image_fallback(alt: &str) -> String {
    if alt.is_empty() {
        "[image]".to_string()
    } else {
        format!("[image: {alt}]")
    }
}

fn embed_image(bytes: &[u8]) -> anyhow::Result<Pic> {
    // Validate the bytes first — Pic::new panics on undecodable input.
    image::load_from_memory(bytes).map_err(|e| anyhow::anyhow!("decode image: {e}"))?;

    // Pic::new re-encodes as PNG and computes pixel dimensions in EMUs.
    // Scale down if wider than the page content width.
    let mut pic = Pic::new(bytes);
    let (w_emu, h_emu) = pic.size;
    let w_px = w_emu / PX_TO_EMU;
    if w_px > MAX_IMAGE_WIDTH_PX && w_px > 0 {
        let scale = MAX_IMAGE_WIDTH_PX as f64 / w_px as f64;
        let new_w = (w_emu as f64 * scale) as u32;
        let new_h = (h_emu as f64 * scale) as u32;
        pic = pic.size(new_w.max(1), new_h.max(1));
    }
    Ok(pic)
}

fn body_fonts() -> RunFonts {
    RunFonts::new().ascii(BODY).hi_ansi(BODY).cs(BODY)
}

fn mono_fonts() -> RunFonts {
    RunFonts::new().ascii(MONO).hi_ansi(MONO).cs(MONO)
}

fn body_run(text: impl Into<String>) -> Run {
    Run::new().add_text(text).fonts(body_fonts())
}

fn mono_run(text: impl Into<String>) -> Run {
    Run::new().add_text(text).fonts(mono_fonts()).size(18) // 9pt
}

fn build_run(r: &IRun) -> Run {
    let mut run = if r.code {
        mono_run(&r.text)
    } else {
        body_run(&r.text)
    };
    if r.bold {
        run = run.bold();
    }
    if r.italic {
        run = run.italic();
    }
    if r.strike {
        run = run.strike();
    }
    run
}
