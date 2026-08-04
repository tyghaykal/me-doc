"""Thin HTTP wrapper around microsoft/markitdown for MeDoc's document import.

The Rust backend has no maintained MarkItDown port, so conversion lives here as
its own internal-only service (docker-compose gives it no published port) —
the backend proxies uploads to it and hands the resulting Markdown back to the
frontend, which already knows how to turn Markdown into a page (same path the
existing .md import uses).
"""
import os
import tempfile
from pathlib import Path

from fastapi import FastAPI, File, HTTPException, UploadFile
from markitdown import MarkItDown

app = FastAPI()
converter = MarkItDown()

# Matches nginx's client_max_body_size and the backend's own upload cap.
MAX_BYTES = 20 * 1024 * 1024


@app.get("/health")
def health():
    return {"status": "ok"}


# Mirrors the formats the frontend's import file picker offers — keeps
# markitdown's format sniffing (which leans on the extension) working while
# never handing an attacker-controlled string straight to a filesystem API.
ALLOWED_SUFFIXES = {
    ".md", ".txt", ".docx", ".doc", ".pdf", ".xlsx", ".xls",
    ".pptx", ".ppt", ".epub", ".html", ".htm", ".csv",
}


def _safe_suffix(filename: str) -> str:
    suffix = Path(filename).suffix.lower()
    return suffix if suffix in ALLOWED_SUFFIXES else ""


@app.post("/convert")
async def convert(file: UploadFile = File(...)):
    data = await file.read()
    if not data:
        raise HTTPException(status_code=400, detail="empty file")
    if len(data) > MAX_BYTES:
        raise HTTPException(status_code=413, detail="file too large")

    suffix = _safe_suffix(file.filename or "")
    with tempfile.NamedTemporaryFile(suffix=suffix, delete=False) as tmp:
        tmp.write(data)
        tmp_path = tmp.name

    try:
        try:
            result = converter.convert(tmp_path)
        except Exception as exc:  # markitdown raises a variety of format-specific errors
            raise HTTPException(status_code=422, detail=f"couldn't convert this file: {exc}") from exc

        # ponytail: markitdown's result attribute has moved before across
        # releases (text_content -> markdown) — fall back rather than pin an
        # exact version.
        markdown = getattr(result, "markdown", None)
        if markdown is None:
            markdown = getattr(result, "text_content", "")

        if suffix.lower() == ".pdf":
            markdown = _pdf_with_tables(tmp_path) or markdown
    finally:
        os.unlink(tmp_path)

    return {"markdown": markdown}


def _pdf_with_tables(path: str) -> str | None:
    """markitdown's own PDF table detection is a best-effort heuristic over
    word positions that regularly finds nothing (its PDF path is otherwise
    just a flat `pdfminer.extract_text()` dump — no layout awareness at all).
    pdfplumber (already a transitive dep via markitdown[pdf]) knows each
    table's bounding box, so tables can be spliced in where they actually sit
    on the page instead of guessed at or appended separately. Returns None on
    any failure so the caller falls back to markitdown's plain text.
    """
    import pdfplumber

    try:
        with pdfplumber.open(path) as pdf:
            pages = [_page_to_markdown(page) for page in pdf.pages]
    except Exception:
        return None

    pages = [p for p in pages if p and p.strip()]
    return "\n\n".join(pages) if pages else None


def _page_to_markdown(page) -> str:
    tables = sorted(page.find_tables(), key=lambda t: t.bbox[1])
    if not tables:
        return page.extract_text() or ""

    segments = []
    cursor = 0.0
    for t in tables:
        top = max(cursor, 0.0)
        if t.bbox[1] > top:
            text = _text_in_band(page, top, t.bbox[1])
            if text:
                segments.append(text)
        md = _table_to_markdown(t.extract())
        if md:
            segments.append(md)
        cursor = t.bbox[3]

    if cursor < page.height:
        text = _text_in_band(page, cursor, page.height)
        if text:
            segments.append(text)

    return "\n\n".join(segments)


def _text_in_band(page, top: float, bottom: float) -> str:
    if bottom <= top:
        return ""
    region = page.within_bbox((0, top, page.width, bottom), relative=False)
    text = region.extract_text() if region else ""
    return text.strip() if text else ""


def _table_to_markdown(table: list[list[str | None]]) -> str | None:
    rows = [
        [(cell or "").strip().replace("|", "\\|").replace("\n", " ") for cell in row]
        for row in table
        if any(cell for cell in row)
    ]
    if not rows:
        return None
    header, *body = rows
    lines = [
        "| " + " | ".join(header) + " |",
        "| " + " | ".join("---" for _ in header) + " |",
    ]
    lines.extend("| " + " | ".join(row) + " |" for row in body)
    return "\n".join(lines)
