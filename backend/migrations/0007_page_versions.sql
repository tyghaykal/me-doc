create table page_versions (
    id uuid primary key default gen_random_uuid(),
    page_id uuid not null references pages(id) on delete cascade,
    yjs_state bytea not null,
    created_at timestamptz not null default now()
);

create index idx_page_versions_page_id on page_versions(page_id, created_at desc);
