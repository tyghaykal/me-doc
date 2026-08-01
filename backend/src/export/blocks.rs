//! Shared document model for the DOCX/PDF exporters.
//!
//! The Markdown exporter (`super::yjs_to_markdown`) already owns the one and
//! only walk of the Yjs XML fragment. Rather than re-traverse yrs a second and
//! third time for DOCX/PDF, both of those consume that Markdown through this
//! tiny block model: `parse_markdown` turns the exporter's own output back into
//! blocks + inline runs that `docx.rs`/`pdf.rs` render. pulldown-cmark does the
//! fiddly inline-mark parsing so we don't reinvent it.

use std::collections::HashMap;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub strike: bool,
    /// Hex background color (e.g. `"#fef08a"`), from the Highlight mark.
    pub highlight: Option<String>,
    /// Hex text color (e.g. `"#be185d"`), from the Color extension's mark.
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TableRow {
    pub header: bool,
    pub cells: Vec<Vec<Run>>,
}

#[derive(Debug, Clone)]
pub enum Block {
    Heading { level: u8, runs: Vec<Run> },
    Paragraph(Vec<Run>),
    Quote(Vec<Run>),
    Code(String),
    /// Mermaid source from a ` ```mermaid ` fence — rendered as an image at
    /// export time (see `fetch_diagrams`), not printed as literal text.
    Diagram(String),
    ListItem { marker: String, depth: usize, checked: Option<bool>, runs: Vec<Run> },
    Image { alt: String, url: String },
    Table { rows: Vec<TableRow> },
}

pub fn parse_markdown(md: &str) -> Vec<Block> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(md, opts);

    let mut blocks: Vec<Block> = Vec::new();
    let mut runs: Vec<Run> = Vec::new();

    let mut bold = 0i32;
    let mut italic = 0i32;
    let mut strike = 0i32;
    let mut highlight_stack: Vec<String> = Vec::new();
    let mut color_stack: Vec<String> = Vec::new();

    let mut heading: Option<u8> = None;
    let mut in_quote = false;
    let mut code_buf: Option<String> = None;
    let mut code_lang: Option<String> = None;
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut pending_item: Option<(String, usize, Option<bool>)> = None;
    let mut in_image = false;
    let mut image_alt = String::new();
    let mut image_url = String::new();

    let mut table_rows: Vec<TableRow> = Vec::new();
    let mut row_cells: Vec<Vec<Run>> = Vec::new();
    let mut in_table_cell = false;

    macro_rules! color { () => { color_stack.last().map(String::as_str) } }
    macro_rules! highlight_color { () => { highlight_stack.last().map(String::as_str) } }

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                heading = Some(hlevel(level));
                runs.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                let level = heading.take().unwrap_or(1);
                blocks.push(Block::Heading {
                    level,
                    runs: std::mem::take(&mut runs),
                });
            }

            Event::Start(Tag::Paragraph) => {
                if pending_item.is_none() && list_stack.is_empty() && !in_table_cell {
                    runs.clear();
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if pending_item.is_some() || !list_stack.is_empty() || in_table_cell {
                    // inside a list item / table cell: runs are flushed by the caller.
                } else if in_quote {
                    blocks.push(Block::Quote(std::mem::take(&mut runs)));
                } else if !runs.is_empty() {
                    blocks.push(Block::Paragraph(std::mem::take(&mut runs)));
                } else {
                    runs.clear();
                }
            }

            Event::Start(Tag::BlockQuote(_)) => in_quote = true,
            Event::End(TagEnd::BlockQuote(_)) => in_quote = false,

            Event::Start(Tag::CodeBlock(kind)) => {
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.into_string()),
                    _ => None,
                };
                code_buf = Some(String::new());
            }
            Event::End(TagEnd::CodeBlock) => {
                if let Some(code) = code_buf.take() {
                    let text = code.trim_end_matches('\n').to_string();
                    if code_lang.as_deref() == Some("mermaid") {
                        blocks.push(Block::Diagram(text));
                    } else {
                        blocks.push(Block::Code(text));
                    }
                }
                code_lang = None;
            }

            Event::Start(Tag::List(start)) => {
                flush_item(&mut blocks, &mut pending_item, &mut runs);
                list_stack.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                let depth = list_stack.len().saturating_sub(1);
                let marker = match list_stack.last_mut() {
                    Some(Some(n)) => {
                        let m = format!("{n}. ");
                        *n += 1;
                        m
                    }
                    _ => "\u{2022} ".to_string(),
                };
                pending_item = Some((marker, depth, None));
                runs.clear();
            }
            Event::End(TagEnd::Item) => {
                flush_item(&mut blocks, &mut pending_item, &mut runs);
            }
            Event::TaskListMarker(checked) => {
                if let Some((_, _, c)) = pending_item.as_mut() {
                    *c = Some(checked);
                }
            }

            Event::Start(Tag::Table(_)) => {
                table_rows.clear();
            }
            Event::End(TagEnd::Table) => {
                blocks.push(Block::Table { rows: std::mem::take(&mut table_rows) });
            }
            Event::Start(Tag::TableHead) => {
                row_cells.clear();
            }
            Event::End(TagEnd::TableHead) => {
                table_rows.push(TableRow { header: true, cells: std::mem::take(&mut row_cells) });
            }
            Event::Start(Tag::TableRow) => {
                row_cells.clear();
            }
            Event::End(TagEnd::TableRow) => {
                table_rows.push(TableRow { header: false, cells: std::mem::take(&mut row_cells) });
            }
            Event::Start(Tag::TableCell) => {
                in_table_cell = true;
                runs.clear();
            }
            Event::End(TagEnd::TableCell) => {
                in_table_cell = false;
                row_cells.push(std::mem::take(&mut runs));
            }

            Event::Start(Tag::Strong) => bold += 1,
            Event::End(TagEnd::Strong) => bold -= 1,
            Event::Start(Tag::Emphasis) => italic += 1,
            Event::End(TagEnd::Emphasis) => italic -= 1,
            Event::Start(Tag::Strikethrough) => strike += 1,
            Event::End(TagEnd::Strikethrough) => strike -= 1,

            // `<mark style="background:...">`/`<span style="color:...">` — the
            // Highlight/Color marks have no native GFM syntax, so `render_text`
            // (export/mod.rs) rides them as raw inline HTML. Only these exact
            // tag shapes are recognized; anything else in `<...>` form is
            // silently ignored.
            Event::InlineHtml(html) => match html.as_ref() {
                "</mark>" => {
                    highlight_stack.pop();
                }
                "</span>" => {
                    color_stack.pop();
                }
                s => {
                    if let Some(hex) = s
                        .strip_prefix("<mark style=\"background:")
                        .and_then(|rest| rest.strip_suffix("\">"))
                    {
                        highlight_stack.push(hex.to_string());
                    } else if let Some(hex) = s
                        .strip_prefix("<span style=\"color:")
                        .and_then(|rest| rest.strip_suffix("\">"))
                    {
                        color_stack.push(hex.to_string());
                    }
                }
            },

            Event::Start(Tag::Image { dest_url, .. }) => {
                // Flush any pending paragraph text so the image stands alone —
                // but not from inside a list item / table cell, which keep
                // their own placeholder instead (see End(Image) below).
                if !runs.is_empty()
                    && pending_item.is_none()
                    && list_stack.is_empty()
                    && !in_quote
                    && !in_table_cell
                {
                    blocks.push(Block::Paragraph(std::mem::take(&mut runs)));
                }
                in_image = true;
                image_alt.clear();
                image_url = dest_url.into_string();
            }
            Event::End(TagEnd::Image) => {
                in_image = false;
                let alt = std::mem::take(&mut image_alt);
                let url = std::mem::take(&mut image_url);
                if pending_item.is_some() || !list_stack.is_empty() || in_table_cell {
                    // Inside a list/table cell: keep a textual placeholder.
                    let label = if alt.is_empty() {
                        "[image]".to_string()
                    } else {
                        format!("[image: {alt}]")
                    };
                    push_run(&mut runs, &label, bold, italic, false, strike, highlight_color!(), color!());
                } else {
                    blocks.push(Block::Image { alt, url });
                }
            }

            Event::Text(t) => {
                if let Some(buf) = code_buf.as_mut() {
                    buf.push_str(t.as_ref());
                } else if in_image {
                    image_alt.push_str(t.as_ref());
                } else {
                    push_run(&mut runs, t.as_ref(), bold, italic, false, strike, highlight_color!(), color!());
                }
            }
            Event::Code(t) => {
                if !in_image {
                    push_run(&mut runs, t.as_ref(), bold, italic, true, strike, highlight_color!(), color!());
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(buf) = code_buf.as_mut() {
                    buf.push('\n');
                } else if !in_image {
                    push_run(&mut runs, " ", bold, italic, false, strike, highlight_color!(), color!());
                }
            }
            Event::Rule => {
                blocks.push(Block::Paragraph(vec![Run {
                    text: "\u{2014}\u{2014}\u{2014}".to_string(),
                    bold: false,
                    italic: false,
                    code: false,
                    strike: false,
                    highlight: None,
                    color: None,
                }]));
            }
            _ => {}
        }
    }

    blocks
}

pub fn collect_image_urls(blocks: &[Block]) -> Vec<String> {
    let mut urls = Vec::new();
    for b in blocks {
        if let Block::Image { url, .. } = b {
            if !url.is_empty() && !urls.iter().any(|u| u == url) {
                urls.push(url.clone());
            }
        }
    }
    urls
}

pub fn collect_diagram_sources(blocks: &[Block]) -> Vec<String> {
    let mut sources = Vec::new();
    for b in blocks {
        if let Block::Diagram(source) = b {
            if !source.trim().is_empty() && !sources.iter().any(|s| s == source) {
                sources.push(source.clone());
            }
        }
    }
    sources
}

/// Best-effort render of Mermaid sources to PNG via the public mermaid.ink
/// service — there's no Mermaid renderer in the Rust ecosystem, and this repo's
/// Rust-only backend has no headless browser to run the real thing. Failures
/// (network, unreachable, bad diagram) are omitted from the map so callers can
/// fall back to printing the source as text.
/// ponytail: third-party dependency at export time, so a self-hosted/air-gapped
/// deployment silently loses diagram images in exports — upgrade path is a
/// local Mermaid CLI/headless-Chrome render service if that matters later.
pub async fn fetch_diagrams(sources: &[String]) -> HashMap<String, Vec<u8>> {
    let mut out = HashMap::new();
    if sources.is_empty() {
        return out;
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return out,
    };

    const MAX_BYTES: usize = 8 * 1024 * 1024;

    for source in sources {
        let encoded = URL_SAFE_NO_PAD.encode(source.as_bytes());
        let url = format!("https://mermaid.ink/img/{encoded}?bgColor=white");
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(bytes) = resp.bytes().await {
                    if !bytes.is_empty() && bytes.len() <= MAX_BYTES {
                        out.insert(source.clone(), bytes.to_vec());
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Best-effort HTTP fetch of image URLs. Failures are omitted from the map so
/// renderers can fall back to alt text. Caps each body at 8 MiB.
pub async fn fetch_images(urls: &[String]) -> HashMap<String, Vec<u8>> {
    let mut out = HashMap::new();
    if urls.is_empty() {
        return out;
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return out,
    };

    const MAX_BYTES: usize = 8 * 1024 * 1024;

    for url in urls {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            continue;
        }
        match client.get(url).send().await {
            Ok(resp) if resp.status().is_success() => {
                match resp.bytes().await {
                    Ok(bytes) if bytes.len() <= MAX_BYTES && !bytes.is_empty() => {
                        out.insert(url.clone(), bytes.to_vec());
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    out
}

fn flush_item(
    blocks: &mut Vec<Block>,
    pending: &mut Option<(String, usize, Option<bool>)>,
    runs: &mut Vec<Run>,
) {
    if let Some((marker, depth, checked)) = pending.take() {
        blocks.push(Block::ListItem {
            marker,
            depth,
            checked,
            runs: std::mem::take(runs),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn push_run(
    runs: &mut Vec<Run>,
    text: &str,
    bold: i32,
    italic: i32,
    code: bool,
    strike: i32,
    highlight: Option<&str>,
    color: Option<&str>,
) {
    if text.is_empty() {
        return;
    }
    runs.push(Run {
        text: text.to_string(),
        bold: bold > 0,
        italic: italic > 0,
        code,
        strike: strike > 0,
        highlight: highlight.map(str::to_string),
        color: color.map(str::to_string),
    });
}

fn hlevel(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_headings_paragraphs_lists_and_marks() {
        let md = "# Title\n\nhello **bold** and *em* and `co`\n\n- one\n- two\n";
        let blocks = parse_markdown(md);

        assert!(matches!(blocks.first(), Some(Block::Heading { level: 1, .. })));

        let bold_run = blocks
            .iter()
            .any(|b| matches!(b, Block::Paragraph(runs) if runs.iter().any(|r| r.bold)));
        assert!(bold_run, "expected a bold run: {blocks:?}");

        let code_run = blocks
            .iter()
            .any(|b| matches!(b, Block::Paragraph(runs) if runs.iter().any(|r| r.code)));
        assert!(code_run, "expected an inline code run: {blocks:?}");

        let items: Vec<_> = blocks
            .iter()
            .filter(|b| matches!(b, Block::ListItem { .. }))
            .collect();
        assert_eq!(items.len(), 2, "expected two list items: {blocks:?}");
    }

    #[test]
    fn parses_code_block_and_quote() {
        let md = "> quoted\n\n```rust\nfn main() {}\n```\n";
        let blocks = parse_markdown(md);
        assert!(blocks.iter().any(|b| matches!(b, Block::Quote(_))));
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::Code(c) if c.contains("fn main"))));
    }

    #[test]
    fn parses_image_block() {
        let md = "before\n\n![diagram](https://example.com/a.png)\n\nafter\n";
        let blocks = parse_markdown(md);
        let img = blocks.iter().find_map(|b| match b {
            Block::Image { alt, url } => Some((alt.as_str(), url.as_str())),
            _ => None,
        });
        assert_eq!(img, Some(("diagram", "https://example.com/a.png")));
        assert_eq!(collect_image_urls(&blocks), vec!["https://example.com/a.png".to_string()]);
    }

    #[test]
    fn parses_table() {
        let md = "| Name | Value |\n| --- | --- |\n| Alpha | 1 |\n| Beta | 2 |\n";
        let blocks = parse_markdown(md);
        let rows = blocks.iter().find_map(|b| match b {
            Block::Table { rows } => Some(rows),
            _ => None,
        });
        let rows = rows.expect("expected a table block");
        assert_eq!(rows.len(), 3, "1 header + 2 body rows: {rows:?}");
        assert!(rows[0].header);
        assert_eq!(rows[0].cells[0][0].text, "Name");
        assert!(!rows[1].header);
        assert_eq!(rows[1].cells[0][0].text, "Alpha");
        assert_eq!(rows[2].cells[1][0].text, "2");
    }

    #[test]
    fn parses_task_list() {
        let md = "- [x] done\n- [ ] not done\n";
        let blocks = parse_markdown(md);
        let items: Vec<_> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::ListItem { checked, runs, .. } => Some((*checked, runs[0].text.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(items, vec![(Some(true), "done".to_string()), (Some(false), "not done".to_string())]);
    }

    #[test]
    fn parses_highlight_and_color_marks() {
        let md = "before <mark style=\"background:#fef08a\">marked</mark> and <span style=\"color:#be185d\">colored</span> after";
        let blocks = parse_markdown(md);
        let runs = blocks.iter().find_map(|b| match b {
            Block::Paragraph(runs) => Some(runs),
            _ => None,
        }).expect("expected a paragraph");

        let highlighted = runs.iter().any(|r| r.highlight.as_deref() == Some("#fef08a") && r.text == "marked");
        assert!(highlighted, "expected a highlighted run: {runs:?}");

        let colored = runs.iter().any(|r| r.color.as_deref() == Some("#be185d") && r.text == "colored");
        assert!(colored, "expected a colored run: {runs:?}");
    }
}
