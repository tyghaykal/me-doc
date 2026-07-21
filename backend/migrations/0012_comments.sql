-- The anchor (which text is commented on) lives in the Yjs doc as a "comment"
-- mark keyed by mark_id, not as stored positions here — positions would drift
-- as the doc is concurrently edited; the Yjs mark survives that for free.
create table comments (
    id uuid primary key default gen_random_uuid(),
    page_id uuid not null references pages(id) on delete cascade,
    mark_id uuid not null,
    author_id uuid not null references users(id),
    assignee_id uuid references users(id),
    body text not null,
    resolved boolean not null default false,
    created_at timestamptz not null default now()
);

create index idx_comments_page_id on comments(page_id);
create unique index idx_comments_mark_id on comments(mark_id);
