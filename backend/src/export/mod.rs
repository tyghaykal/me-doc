use axum::{
    extract::{Query, State},
    http::header,
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use yrs::updates::decoder::Decode;
use yrs::{Any, Doc, GetString, Out, ReadTxn, Text, Transact, Update, Xml, XmlElementRef, XmlFragment, XmlOut, XmlTextRef};

use crate::auth::error::AuthError;
use crate::sharing::{PagePermission, Role};
use crate::AppState;

mod blocks;
mod docx;
mod pdf;

pub fn router() -> Router<AppState> {
    Router::new().route("/pages/:id/export", get(export_page))
}

#[derive(Deserialize)]
struct ExportQuery {
    format: Option<String>,
}

async fn export_page(
    State(state): State<AppState>,
    perm: PagePermission,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, AuthError> {
    if perm.role != Role::Editor {
        return Err(AuthError::Forbidden);
    }

    let format = query.format.as_deref().unwrap_or("md");
    if !matches!(format, "md" | "docx" | "pdf") {
        return Err(AuthError::Validation(format!(
            "export format '{format}' not supported"
        )));
    }

    let row: Option<(String, Uuid, Option<Vec<u8>>)> = sqlx::query_as(
        "select p.slug, p.workspace_id, pc.yjs_state
         from pages p left join page_content pc on pc.page_id = p.id
         where p.id = $1",
    )
    .bind(perm.page_id)
    .fetch_optional(&state.db)
    .await?;

    let (slug, workspace_id, yjs_state) = row.ok_or(AuthError::NotFound)?;

    let markdown = yjs_to_markdown(&yjs_state.unwrap_or_default());
    // Embeds are placeholder links coming out of the Yjs walk (it has no DB
    // access) — resolve each to the target diagram page's actual Mermaid
    // source now that we do, so it flows through the same
    // ```mermaid``` -> Block::Diagram -> (image for docx/pdf) path as an
    // inline diagram.
    let markdown = resolve_diagram_embeds(&state.db, workspace_id, &markdown).await;

    // DOCX/PDF share one parse + image/diagram fetch so remote assets are pulled once.
    let (content_type, ext, bytes): (&str, &str, Vec<u8>) = match format {
        "docx" | "pdf" => {
            let parsed = blocks::parse_markdown(&markdown);
            let urls = blocks::collect_image_urls(&parsed);
            let images = blocks::fetch_images(&urls).await;
            let diagram_sources = blocks::collect_diagram_sources(&parsed);
            let diagrams = blocks::fetch_diagrams(&diagram_sources).await;
            if format == "docx" {
                (
                    "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                    "docx",
                    docx::blocks_to_docx(&parsed, &images, &diagrams).map_err(AuthError::Internal)?,
                )
            } else {
                (
                    "application/pdf",
                    "pdf",
                    pdf::blocks_to_pdf(&parsed, &images, &diagrams).map_err(AuthError::Internal)?,
                )
            }
        }
        _ => (
            "text/markdown; charset=utf-8",
            "md",
            markdown.into_bytes(),
        ),
    };

    Ok((
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{slug}.{ext}\""),
            ),
        ],
        bytes,
    ))
}

/// Scans `markdown` for the `[Embedded diagram](/app/<id>)` placeholders
/// `render_element_block` emits for `diagramEmbed` nodes and swaps each for a
/// ` ```mermaid ` fence holding that page's actual source — scoped to the
/// same workspace as the exported page so an embed can't be used to exfiltrate
/// another workspace's diagram by id.
async fn resolve_diagram_embeds(db: &PgPool, workspace_id: Uuid, markdown: &str) -> String {
    const PREFIX: &str = "[Embedded diagram](/app/";
    if !markdown.contains(PREFIX) {
        return markdown.to_string();
    }

    let mut out = String::with_capacity(markdown.len());
    let mut rest = markdown;
    while let Some(start) = rest.find(PREFIX) {
        out.push_str(&rest[..start]);
        let after_prefix = &rest[start + PREFIX.len()..];
        let Some(end) = after_prefix.find(')') else {
            out.push_str(&rest[start..]);
            rest = "";
            break;
        };
        let id_str = &after_prefix[..end];
        let source = match Uuid::parse_str(id_str) {
            Ok(id) => fetch_diagram_source(db, id, workspace_id).await,
            Err(_) => None,
        };
        match source {
            Some(src) => {
                out.push_str("```mermaid\n");
                out.push_str(src.trim_end_matches('\n'));
                out.push_str("\n```");
            }
            None => out.push_str("*Embedded diagram unavailable*"),
        }
        rest = &after_prefix[end + 1..];
    }
    out.push_str(rest);
    out
}

async fn fetch_diagram_source(db: &PgPool, page_id: Uuid, workspace_id: Uuid) -> Option<String> {
    let row: Option<(Option<Vec<u8>>,)> = sqlx::query_as(
        "select pc.yjs_state
         from pages p join page_content pc on pc.page_id = p.id
         where p.id = $1 and p.workspace_id = $2 and p.kind = 'diagram' and p.archived_at is null",
    )
    .bind(page_id)
    .bind(workspace_id)
    .fetch_optional(db)
    .await
    .ok()?;

    let source = yjs_named_text(&row?.0?, "source");
    if source.trim().is_empty() {
        None
    } else {
        Some(source)
    }
}

/// Decode a Yjs v1 update into a `Doc` and serialize its "default" XML fragment
/// (the shared type Tiptap's Collaboration extension binds to) as Markdown.
pub(crate) fn yjs_to_markdown(state: &[u8]) -> String {
    let doc = Doc::new();
    if !state.is_empty() {
        if let Ok(update) = Update::decode_v1(state) {
            let mut txn = doc.transact_mut();
            let _ = txn.apply_update(update);
        }
    }

    let fragment = doc.get_or_insert_xml_fragment("default");
    let txn = doc.transact();
    let mut out = String::new();
    for node in fragment.children(&txn) {
        render_block(&node, &txn, &mut out);
    }

    let trimmed = out.trim_end();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

/// Reads a top-level named `Y.Text` out of an encoded Yjs update — used for
/// diagram pages, whose Mermaid source lives in a `Text` (not the XML fragment
/// documents use). Returns "" when absent/empty.
pub(crate) fn yjs_named_text(state: &[u8], name: &str) -> String {
    let doc = Doc::new();
    if !state.is_empty() {
        if let Ok(update) = Update::decode_v1(state) {
            let mut txn = doc.transact_mut();
            let _ = txn.apply_update(update);
        }
    }
    let text = doc.get_or_insert_text(name);
    let txn = doc.transact();
    text.get_string(&txn)
}

fn render_block<T: ReadTxn>(node: &XmlOut, txn: &T, out: &mut String) {
    match node {
        XmlOut::Element(el) => render_element_block(el, txn, out),
        XmlOut::Text(t) => {
            render_text(t, txn, out);
            out.push_str("\n\n");
        }
        XmlOut::Fragment(f) => {
            for child in f.children(txn) {
                render_block(&child, txn, out);
            }
        }
    }
}

fn render_element_block<T: ReadTxn>(el: &XmlElementRef, txn: &T, out: &mut String) {
    match el.tag().as_ref() {
        "paragraph" => {
            let mut line = String::new();
            render_inline_children(el, txn, &mut line);
            out.push_str(line.trim_end());
            out.push_str("\n\n");
        }
        "heading" => {
            out.push_str(&"#".repeat(heading_level(el, txn)));
            out.push(' ');
            let mut line = String::new();
            render_inline_children(el, txn, &mut line);
            out.push_str(line.trim_end());
            out.push_str("\n\n");
        }
        "bulletList" => {
            render_list(el, txn, out, ListKind::Bullet);
            out.push('\n');
        }
        "orderedList" => {
            render_list(el, txn, out, ListKind::Ordered(1));
            out.push('\n');
        }
        "taskList" => {
            render_list(el, txn, out, ListKind::Task);
            out.push('\n');
        }
        "table" => render_table(el, txn, out),
        "codeBlock" => {
            let lang = attr_str(el, txn, "language").unwrap_or_default();
            let mut code = String::new();
            for child in el.children(txn) {
                if let XmlOut::Text(t) = child {
                    code.push_str(&t.get_string(txn));
                }
            }
            out.push_str("```");
            out.push_str(&lang);
            out.push('\n');
            out.push_str(code.trim_end_matches('\n'));
            out.push_str("\n```\n\n");
        }
        "blockquote" => {
            let mut inner = String::new();
            for child in el.children(txn) {
                render_block(&child, txn, &mut inner);
            }
            for line in inner.trim_end().lines() {
                out.push_str("> ");
                out.push_str(line);
                out.push('\n');
            }
            out.push('\n');
        }
        "horizontalRule" => out.push_str("---\n\n"),
        "image" => {
            render_image(el, txn, out);
            out.push_str("\n\n");
        }
        // Inline diagram block: Mermaid source lives in the `source` attribute.
        "diagram" => {
            let source = attr_str(el, txn, "source").unwrap_or_default();
            out.push_str("```mermaid\n");
            out.push_str(source.trim_end_matches('\n'));
            out.push_str("\n```\n\n");
        }
        // Live embed of a standalone diagram page. The walker has no DB access,
        // so it can't inline the referenced source — emit a link placeholder.
        "diagramEmbed" => {
            let id = attr_str(el, txn, "diagramId").unwrap_or_default();
            out.push_str(&format!("[Embedded diagram](/app/{id})\n\n"));
        }
        // Unknown block: recurse so nested text is never silently dropped.
        _ => {
            for child in el.children(txn) {
                render_block(&child, txn, out);
            }
        }
    }
}

enum ListKind {
    Bullet,
    Ordered(usize),
    Task,
}

fn render_list<T: ReadTxn>(el: &XmlElementRef, txn: &T, out: &mut String, kind: ListKind) {
    let mut index = match kind {
        ListKind::Ordered(n) => n,
        _ => 0,
    };
    for item in el.children(txn) {
        let XmlOut::Element(li) = item else { continue };

        let marker = match kind {
            ListKind::Ordered(_) => {
                let m = format!("{index}. ");
                index += 1;
                m
            }
            ListKind::Bullet => "- ".to_string(),
            ListKind::Task => {
                let checked = matches!(li.get_attribute(txn, "checked"), Some(Out::Any(Any::Bool(true))));
                if checked { "- [x] ".to_string() } else { "- [ ] ".to_string() }
            }
        };

        let mut item_buf = String::new();
        for block in li.children(txn) {
            render_block(&block, txn, &mut item_buf);
        }

        let text = item_buf.trim_end();
        let mut lines = text.lines();
        match lines.next() {
            Some(first) => {
                out.push_str(&marker);
                out.push_str(first);
                out.push('\n');
                for line in lines {
                    out.push_str("  ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
            None => {
                out.push_str(&marker);
                out.push('\n');
            }
        }
    }
}

/// Renders a `table` node as a GFM pipe table. Tiptap's table model always
/// starts with a header row (`tableHeader` cells), so the separator row goes
/// right after the first row unconditionally. Multi-paragraph cell content is
/// flattened onto one line — GFM table cells can't span lines.
fn render_table<T: ReadTxn>(el: &XmlElementRef, txn: &T, out: &mut String) {
    let mut rows: Vec<Vec<String>> = Vec::new();
    for row in el.children(txn) {
        let XmlOut::Element(row_el) = row else { continue };
        if row_el.tag().as_ref() != "tableRow" {
            continue;
        }
        let mut cells = Vec::new();
        for cell in row_el.children(txn) {
            let XmlOut::Element(cell_el) = cell else { continue };
            let mut text = String::new();
            for block in cell_el.children(txn) {
                render_block(&block, txn, &mut text);
            }
            cells.push(text.trim().replace('\n', " ").replace('|', "\\|"));
        }
        rows.push(cells);
    }

    let col_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    if rows.is_empty() || col_count == 0 {
        return;
    }

    for (i, cells) in rows.iter().enumerate() {
        out.push('|');
        for c in 0..col_count {
            out.push(' ');
            out.push_str(cells.get(c).map(String::as_str).unwrap_or(""));
            out.push_str(" |");
        }
        out.push('\n');
        if i == 0 {
            out.push('|');
            for _ in 0..col_count {
                out.push_str(" --- |");
            }
            out.push('\n');
        }
    }
    out.push('\n');
}

fn render_inline_children<T: ReadTxn>(el: &XmlElementRef, txn: &T, out: &mut String) {
    for child in el.children(txn) {
        match child {
            XmlOut::Text(t) => render_text(&t, txn, out),
            XmlOut::Element(e) => match e.tag().as_ref() {
                "hardBreak" => out.push_str("  \n"),
                "image" => render_image(&e, txn, out),
                _ => render_inline_children(&e, txn, out),
            },
            XmlOut::Fragment(f) => {
                for c in f.children(txn) {
                    render_block(&c, txn, out);
                }
            }
        }
    }
}

/// Render an XmlText run, wrapping each uniformly-formatted chunk in Markdown
/// marks. y-prosemirror stores ProseMirror marks as Yjs text formatting attributes
/// keyed by mark name (`bold`, `italic`, `code`, `strike`, `highlight`, `textStyle`).
///
/// `highlight`/`textStyle` (the Color extension's mark) have no standard GFM
/// syntax, so they ride as literal inline HTML (`<mark style="background:...">`,
/// `<span style="color:...">`) — `blocks::parse_markdown` recognizes exactly
/// these two tag shapes on the way back in. They wrap outermost so
/// `**`/`*`/`` ` `` inside still parses normally (CommonMark only treats the
/// tag itself as raw HTML, not its contents).
fn render_text<T: ReadTxn>(t: &XmlTextRef, txn: &T, out: &mut String) {
    for diff in t.diff(txn, |change| change) {
        let Out::Any(Any::String(s)) = &diff.insert else {
            continue;
        };
        let mut text = s.to_string();
        if let Some(attrs) = &diff.attributes {
            // `code` innermost so its backticks hug the text, not the ** / * wrappers.
            if attrs.contains_key("code") {
                text = format!("`{text}`");
            }
            if attrs.contains_key("bold") {
                text = format!("**{text}**");
            }
            if attrs.contains_key("italic") {
                text = format!("*{text}*");
            }
            if attrs.contains_key("strike") {
                text = format!("~~{text}~~");
            }
            if attrs.contains_key("highlight") {
                // Highlight's `color` attr can be unset (default swatch) even
                // when the mark is present — fall back to a representative
                // yellow so highlighted text is never silently dropped.
                let color = mark_color(attrs, "highlight").unwrap_or_else(|| "#fef08a".to_string());
                text = format!("<mark style=\"background:{color}\">{text}</mark>");
            }
            if let Some(color) = mark_color(attrs, "textStyle") {
                text = format!("<span style=\"color:{color}\">{text}</span>");
            }
        }
        out.push_str(&text);
    }
}

/// Reads a mark's `color` attribute (Yjs stores mark attrs as an `Any::Map`)
/// out of a text diff's attribute set. A mark applied via its default swatch
/// (e.g. plain `<mark>` from a Markdown paste, no explicit color picked) still
/// has a `color` key but with an empty string, not an absent one — treat that
/// the same as "unset" rather than emitting an empty `background:` value.
fn mark_color(attrs: &std::collections::HashMap<std::sync::Arc<str>, Any>, mark: &str) -> Option<String> {
    match attrs.get(mark)? {
        Any::Map(m) => match m.get("color")? {
            Any::String(s) if !s.is_empty() => Some(s.to_string()),
            _ => None,
        },
        _ => None,
    }
}

fn render_image<T: ReadTxn>(el: &XmlElementRef, txn: &T, out: &mut String) {
    let src = attr_str(el, txn, "src").unwrap_or_default();
    let alt = attr_str(el, txn, "alt").unwrap_or_default();
    out.push_str(&format!("![{alt}]({src})"));
}

fn heading_level<T: ReadTxn>(el: &XmlElementRef, txn: &T) -> usize {
    let level = match el.get_attribute(txn, "level") {
        Some(Out::Any(Any::Number(n))) => n as usize,
        Some(Out::Any(Any::BigInt(n))) => n as usize,
        Some(Out::Any(Any::String(s))) => s.parse().unwrap_or(1),
        _ => 1,
    };
    level.clamp(1, 6)
}

fn attr_str<T: ReadTxn>(el: &XmlElementRef, txn: &T, name: &str) -> Option<String> {
    match el.get_attribute(txn, name) {
        Some(Out::Any(Any::String(s))) => Some(s.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yrs::{StateVector, XmlElementPrelim, XmlTextPrelim};

    #[test]
    fn serializes_headings_paragraphs_and_lists() {
        let doc = Doc::new();
        let frag = doc.get_or_insert_xml_fragment("default");
        {
            let mut txn = doc.transact_mut();

            let h = frag.push_back(&mut txn, XmlElementPrelim::empty("heading"));
            h.insert_attribute(&mut txn, "level", "2");
            h.push_back(&mut txn, XmlTextPrelim::new("Title"));

            let p = frag.push_back(&mut txn, XmlElementPrelim::empty("paragraph"));
            p.push_back(&mut txn, XmlTextPrelim::new("hello world"));

            let list = frag.push_back(&mut txn, XmlElementPrelim::empty("bulletList"));
            let li = list.push_back(&mut txn, XmlElementPrelim::empty("listItem"));
            let lip = li.push_back(&mut txn, XmlElementPrelim::empty("paragraph"));
            lip.push_back(&mut txn, XmlTextPrelim::new("one"));
        }

        let update = doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default());
        let md = yjs_to_markdown(&update);

        assert!(md.contains("## Title"), "heading: {md:?}");
        assert!(md.contains("hello world"), "paragraph: {md:?}");
        assert!(md.contains("- one"), "list: {md:?}");
    }

    #[test]
    fn empty_state_is_empty_string() {
        assert_eq!(yjs_to_markdown(&[]), "");
    }
}
