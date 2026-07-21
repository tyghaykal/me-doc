//! PDF exporter via genpdf. Consumes the shared block model from `super::blocks`.
//!
//! genpdf needs TrueType font files on disk (it embeds no fonts of its own).
//! The production image installs `fonts-liberation`; we probe the standard
//! Debian font dirs and fail cleanly if none are present.
//!
//! Spacing is tuned to roughly match the in-app editor:
//! - page padding ≈ Tailwind `p-8` on the main column
//! - body `text-base leading-7` → 12pt + 1.55 line spacing
//! - paragraphs `my-3`, headings' `margin-top` from `main.css`

use std::collections::HashMap;
use std::io::Cursor;

use genpdf::error::Error as PdfError;
use genpdf::fonts::{FontData, FontFamily};
use genpdf::render::Area;
use genpdf::style::Style;
use genpdf::{
    elements, Alignment, Context, Document, Element, Margins, Mm, RenderResult, Scale, Size,
    SimplePageDecorator,
};

use super::blocks::{Block, Run as IRun};

const FONT_CANDIDATES: &[(&str, &str)] = &[
    ("/usr/share/fonts/truetype/liberation", "LiberationSans"),
    ("/usr/share/fonts/truetype/liberation2", "LiberationSans"),
    ("/usr/share/fonts/truetype/dejavu", "DejaVuSans"),
];

// --- Page layout (mm) -------------------------------------------------------
// Editor shell: main has Tailwind `p-8` (2rem). With the app's 18px root that
// is ~9.5mm; print needs a bit more air, so we land near 18–20mm — still
// reading as "document padding" rather than edge-to-edge.
const MARGIN_TOP_MM: f64 = 20.0;
const MARGIN_RIGHT_MM: f64 = 22.0;
const MARGIN_BOTTOM_MM: f64 = 20.0;
const MARGIN_LEFT_MM: f64 = 22.0;

/// Content width on A4 after the margins above (210 − 22 − 22 ≈ 166).
const MAX_IMAGE_WIDTH_MM: f64 = 160.0;
/// A4 height minus vertical margins — images taller than this are scaled down
/// so they always fit on a single page once moved there.
const PAGE_CONTENT_HEIGHT_MM: f64 = 297.0 - MARGIN_TOP_MM - MARGIN_BOTTOM_MM;
/// Leave a little air under a full-page image so it doesn't kiss the margin.
const MAX_IMAGE_HEIGHT_MM: f64 = PAGE_CONTENT_HEIGHT_MM - 4.0;
/// Screen-sourced images are treated as 96 dpi (matches the editor / CSS px).
const IMAGE_DPI: f64 = 96.0;

// --- Type scale (pt) — mirrors `.ProseMirror h*` + body text-base ------------
const BODY_PT: u8 = 12;
const H1_PT: u8 = 22; // ~1.875rem at 18px root
const H2_PT: u8 = 18; // ~1.5rem
const H3_PT: u8 = 15; // ~1.25rem
const H4_PT: u8 = 13; // ~1.125rem
const H5_PT: u8 = 12;
const H6_PT: u8 = 11;
const CODE_PT: u8 = 10;

// --- Vertical rhythm in "lines" of BODY_PT (genpdf Break unit) --------------
// Editor: leading-7 / 1rem ≈ 1.75; print reads better a touch tighter.
const LINE_SPACING: f64 = 1.55;
// `[&_p]:my-3` → ~0.75rem between paragraphs ≈ 0.9 body lines.
const GAP_PARAGRAPH: f64 = 0.9;
// Heading top margins from main.css (1.5 / 1.25 / 1.0 / 0.75 rem).
const GAP_H1_BEFORE: f64 = 1.4;
const GAP_H2_BEFORE: f64 = 1.15;
const GAP_H3_BEFORE: f64 = 0.95;
const GAP_H4_BEFORE: f64 = 0.85;
const GAP_HEADING_AFTER: f64 = 0.45;
const GAP_QUOTE: f64 = 0.75;
const GAP_CODE: f64 = 0.75;
const GAP_LIST_ITEM: f64 = 0.35;
const GAP_IMAGE: f64 = 0.75;

pub fn blocks_to_pdf(blocks: &[Block], images: &HashMap<String, Vec<u8>>) -> anyhow::Result<Vec<u8>> {
    let mut doc = Document::new(load_font()?);
    doc.set_font_size(BODY_PT);
    doc.set_line_spacing(LINE_SPACING);

    let mut decorator = SimplePageDecorator::new();
    decorator.set_margins(Margins::trbl(
        MARGIN_TOP_MM,
        MARGIN_RIGHT_MM,
        MARGIN_BOTTOM_MM,
        MARGIN_LEFT_MM,
    ));
    doc.set_page_decorator(decorator);

    for (i, block) in blocks.iter().enumerate() {
        let is_first = i == 0;
        match block {
            Block::Heading { level, runs } => {
                if !is_first {
                    doc.push(elements::Break::new(heading_before(*level)));
                }
                let size = heading_size(*level);
                let mut p = elements::Paragraph::default();
                if runs.is_empty() {
                    p.push_styled(" ", Style::new().bold().with_font_size(size));
                } else {
                    for r in runs {
                        p.push_styled(r.text.clone(), base_style(r).bold().with_font_size(size));
                    }
                }
                doc.push(p);
                doc.push(elements::Break::new(GAP_HEADING_AFTER));
            }
            Block::Paragraph(runs) => {
                doc.push(styled_par("", runs));
                doc.push(elements::Break::new(GAP_PARAGRAPH));
            }
            Block::Quote(runs) => {
                // Editor: pl-4 italic quote — indent via a few spaces + italic.
                let mut q = elements::Paragraph::default();
                q.push_styled("    ", Style::new());
                if runs.is_empty() {
                    q.push_styled(" ", Style::new().italic());
                } else {
                    for r in runs {
                        q.push_styled(r.text.clone(), base_style(r).italic());
                    }
                }
                doc.push(q);
                doc.push(elements::Break::new(GAP_QUOTE));
            }
            Block::Code(text) => {
                // genpdf is single-family; code uses smaller size + indent like
                // the editor's padded pre block.
                if text.is_empty() {
                    let mut p = elements::Paragraph::default();
                    p.push_styled("    ", Style::new().with_font_size(CODE_PT));
                    doc.push(p);
                } else {
                    for line in text.lines() {
                        let mut p = elements::Paragraph::default();
                        let rendered = if line.is_empty() {
                            "    ".to_string()
                        } else {
                            format!("    {line}")
                        };
                        p.push_styled(rendered, Style::new().with_font_size(CODE_PT));
                        doc.push(p);
                    }
                }
                doc.push(elements::Break::new(GAP_CODE));
            }
            Block::ListItem { marker, depth, runs } => {
                // Editor: pl-6 lists — four spaces per depth level + marker.
                let indent = "    ".repeat(depth.saturating_add(1));
                doc.push(styled_par(&format!("{indent}{marker}"), runs));
                doc.push(elements::Break::new(GAP_LIST_ITEM));
            }
            Block::Image { alt, url } => {
                if let Some(bytes) = images.get(url) {
                    match make_pdf_image(bytes) {
                        Ok((img, height_mm)) => {
                            // KeepTogether moves the image to the next page when
                            // the remaining space on this page is too short —
                            // genpdf's bare Image would otherwise paint past the
                            // bottom margin.
                            doc.push(KeepTogether::new(img, height_mm));
                            doc.push(elements::Break::new(GAP_IMAGE));
                        }
                        Err(_) => {
                            doc.push(alt_paragraph(alt));
                            doc.push(elements::Break::new(GAP_IMAGE));
                        }
                    }
                } else {
                    doc.push(alt_paragraph(alt));
                    doc.push(elements::Break::new(GAP_IMAGE));
                }
            }
        }
    }

    let mut buf = Vec::new();
    doc.render(&mut buf)?;
    Ok(buf)
}

#[allow(dead_code)]
pub fn markdown_to_pdf(md: &str) -> anyhow::Result<Vec<u8>> {
    let blocks = super::blocks::parse_markdown(md);
    blocks_to_pdf(&blocks, &HashMap::new())
}

fn heading_size(level: u8) -> u8 {
    match level {
        1 => H1_PT,
        2 => H2_PT,
        3 => H3_PT,
        4 => H4_PT,
        5 => H5_PT,
        _ => H6_PT,
    }
}

fn heading_before(level: u8) -> f64 {
    match level {
        1 => GAP_H1_BEFORE,
        2 => GAP_H2_BEFORE,
        3 => GAP_H3_BEFORE,
        _ => GAP_H4_BEFORE,
    }
}

fn alt_paragraph(alt: &str) -> elements::Paragraph {
    let label = if alt.is_empty() {
        "[image]".to_string()
    } else {
        format!("[image: {alt}]")
    };
    let mut p = elements::Paragraph::default();
    p.push_styled(label, Style::new().italic());
    p
}

/// Returns `(image, rendered_height_mm)`. Height is what genpdf will reserve
/// after width- and page-height scaling, so `KeepTogether` can decide whether
/// the remaining page space is enough.
fn make_pdf_image(bytes: &[u8]) -> anyhow::Result<(elements::Image, f64)> {
    // Decode with our image crate, strip alpha (genpdf rejects it), re-encode as
    // PNG, then hand the bytes to genpdf via from_reader — genpdf pins image
    // 0.23 so we cannot pass our 0.25 DynamicImage across the crate boundary.
    let img = image::load_from_memory(bytes)
        .map_err(|e| anyhow::anyhow!("decode image: {e}"))?;
    let rgb = img.to_rgb8();
    let (w_px, h_px) = (rgb.width(), rgb.height());
    if w_px == 0 || h_px == 0 {
        anyhow::bail!("empty image");
    }

    let mut png_bytes = Vec::new();
    image::DynamicImage::ImageRgb8(rgb)
        .write_to(&mut Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .map_err(|e| anyhow::anyhow!("encode png: {e}"))?;

    // Natural size at IMAGE_DPI (mm).
    let nat_w = (w_px as f64) * 25.4 / IMAGE_DPI;
    let nat_h = (h_px as f64) * 25.4 / IMAGE_DPI;

    // Fit width first, then shrink further if the image is taller than a page.
    let mut scale = if nat_w > MAX_IMAGE_WIDTH_MM {
        MAX_IMAGE_WIDTH_MM / nat_w
    } else {
        1.0
    };
    let mut height_mm = nat_h * scale;
    if height_mm > MAX_IMAGE_HEIGHT_MM {
        scale *= MAX_IMAGE_HEIGHT_MM / height_mm;
        height_mm = MAX_IMAGE_HEIGHT_MM;
    }

    let element = elements::Image::from_reader(Cursor::new(png_bytes))
        .map_err(|e| anyhow::anyhow!("pdf image: {e}"))?
        .with_alignment(Alignment::Left)
        .with_dpi(IMAGE_DPI)
        .with_scale(Scale::new(scale, scale));

    Ok((element, height_mm))
}

/// Wraps an image so that if it would not fit in the remaining space on the
/// current page, rendering defers to the next page instead of painting into
/// the bottom margin. genpdf's bare `Image` always draws and never sets
/// `has_more`, which is what caused overflow near the page foot.
///
/// Images are pre-scaled to at most one page tall, so a deferred image is
/// guaranteed to fit once a fresh page is started.
struct KeepTogether {
    child: Option<elements::Image>,
    height_mm: f64,
}

impl KeepTogether {
    fn new(image: elements::Image, height_mm: f64) -> Self {
        Self {
            child: Some(image),
            height_mm,
        }
    }
}

impl Element for KeepTogether {
    fn render(
        &mut self,
        context: &Context,
        area: Area<'_>,
        style: Style,
    ) -> Result<RenderResult, PdfError> {
        let Some(child) = self.child.as_mut() else {
            return Ok(RenderResult::default());
        };

        // 0.5mm slack so floating-point noise doesn't force a spurious break.
        if Mm::from(self.height_mm) > area.size().height + Mm::from(0.5) {
            // Nothing fitted on this page — LinearLayout will keep prior
            // content's size (non-zero) and request a new page; we'll be
            // called again with a full content area.
            return Ok(RenderResult {
                size: Size::new(0, 0),
                has_more: true,
            });
        }

        let result = child.render(context, area, style)?;
        self.child = None;
        Ok(result)
    }
}

fn styled_par(prefix: &str, runs: &[IRun]) -> elements::Paragraph {
    let mut p = elements::Paragraph::default();
    if !prefix.is_empty() {
        p.push_styled(prefix.to_string(), Style::new());
    }
    if runs.is_empty() && prefix.is_empty() {
        p.push_styled(" ", Style::new());
        return p;
    }
    for r in runs {
        p.push_styled(r.text.clone(), base_style(r));
    }
    p
}

// genpdf Style supports bold/italic/size — no strikethrough/monospace switch.
fn base_style(r: &IRun) -> Style {
    let mut s = Style::new();
    if r.bold {
        s = s.bold();
    }
    if r.italic {
        s = s.italic();
    }
    if r.code {
        s = s.with_font_size(CODE_PT);
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
