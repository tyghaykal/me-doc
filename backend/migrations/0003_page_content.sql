create table page_content (
    page_id uuid primary key references pages(id) on delete cascade,
    yjs_state bytea not null default '\x',
    plain_text text,
    updated_at timestamptz not null default now()
);
