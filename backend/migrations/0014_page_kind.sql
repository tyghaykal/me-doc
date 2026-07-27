-- Diagrams reuse the whole page subsystem (CRUD, collab room, sharing,
-- versions, sidebar) instead of a parallel table. A page is either a normal
-- rich-text 'document' or a Mermaid 'diagram'; the frontend picks the editor
-- from this column. Existing rows are documents.
alter table pages add column kind text not null default 'document';
