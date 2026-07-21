//! Shared document model for the DOCX/PDF exporters.
//!
//! The Markdown exporter (`super::yjs_to_markdown`) already owns the one and
//! only walk of the Yjs XML fragment. Rather than re-traverse yrs a second and
//! third time for DOCX/PDF, both of those consume that Markdown through this
//! tiny block model: `parse_markdown` turns the exporter's own output back into
//! blocks + inline runs that `docx.rs`/`pdf.rs` render. pulldown-cmark does the
//! fiddly inline-mark parsing so we don't reinvent it.

use std::collections::HashMap;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub strike: bool,
}

#[derive(Debug, Clone)]
pub enum Block {
    Heading { level: u8, runs: Vec<Run> },
    Paragraph(Vec<Run>),
    Quote(Vec<Run>),
    Code(String),
    ListItem { marker: String, depth: usize, runs: Vec<Run> },
    Image { alt: String, url: String },
}

pub fn parse_markdown(md: &str) -> Vec<Block> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, opts);

    let mut blocks: Vec<Block> = Vec::new();
    let mut runs: Vec<Run> = Vec::new();

    let mut bold = 0i32;
    let mut italic = 0i32;
    let mut strike = 0i32;

    let mut heading: Option<u8> = None;
    let mut in_quote = false;
    let mut code_buf: Option<String> = None;
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut pending_item: Option<(String, usize)> = None;
    let mut in_image = false;
    let mut image_alt = String::new();
    let mut image_url = String::new();

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
                if pending_item.is_none() && list_stack.is_empty() {
                    runs.clear();
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if pending_item.is_some() || !list_stack.is_empty() {
                    // inside a list item: runs are flushed at End(Item).
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

            Event::Start(Tag::CodeBlock(_)) => code_buf = Some(String::new()),
            Event::End(TagEnd::CodeBlock) => {
                if let Some(code) = code_buf.take() {
                    blocks.push(Block::Code(code.trim_end_matches('\n').to_string()));
                }
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
                pending_item = Some((marker, depth));
                runs.clear();
            }
            Event::End(TagEnd::Item) => {
                flush_item(&mut blocks, &mut pending_item, &mut runs);
            }

            Event::Start(Tag::Strong) => bold += 1,
            Event::End(TagEnd::Strong) => bold -= 1,
            Event::Start(Tag::Emphasis) => italic += 1,
            Event::End(TagEnd::Emphasis) => italic -= 1,
            Event::Start(Tag::Strikethrough) => strike += 1,
            Event::End(TagEnd::Strikethrough) => strike -= 1,

            Event::Start(Tag::Image { dest_url, .. }) => {
                // Flush any pending paragraph text so the image stands alone.
                if !runs.is_empty() && pending_item.is_none() && list_stack.is_empty() && !in_quote {
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
                if pending_item.is_some() || !list_stack.is_empty() {
                    // Inside a list: keep a textual placeholder in the item runs.
                    let label = if alt.is_empty() {
                        "[image]".to_string()
                    } else {
                        format!("[image: {alt}]")
                    };
                    push_run(&mut runs, &label, bold, italic, false, strike);
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
                    push_run(&mut runs, t.as_ref(), bold, italic, false, strike);
                }
            }
            Event::Code(t) => {
                if !in_image {
                    push_run(&mut runs, t.as_ref(), bold, italic, true, strike);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(buf) = code_buf.as_mut() {
                    buf.push('\n');
                } else if !in_image {
                    push_run(&mut runs, " ", bold, italic, false, strike);
                }
            }
            Event::Rule => {
                blocks.push(Block::Paragraph(vec![Run {
                    text: "\u{2014}\u{2014}\u{2014}".to_string(),
                    bold: false,
                    italic: false,
                    code: false,
                    strike: false,
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

fn flush_item(blocks: &mut Vec<Block>, pending: &mut Option<(String, usize)>, runs: &mut Vec<Run>) {
    if let Some((marker, depth)) = pending.take() {
        blocks.push(Block::ListItem {
            marker,
            depth,
            runs: std::mem::take(runs),
        });
    }
}

fn push_run(runs: &mut Vec<Run>, text: &str, bold: i32, italic: i32, code: bool, strike: i32) {
    if text.is_empty() {
        return;
    }
    runs.push(Run {
        text: text.to_string(),
        bold: bold > 0,
        italic: italic > 0,
        code,
        strike: strike > 0,
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
}
