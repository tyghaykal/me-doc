create table pages (
    id uuid primary key default gen_random_uuid(),
    workspace_id uuid not null references workspaces(id) on delete cascade,
    parent_page_id uuid references pages(id) on delete cascade,
    title text not null default 'Untitled',
    slug text not null,
    order_index integer not null default 0,
    archived_at timestamptz,
    created_by uuid not null references users(id),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);

create index idx_pages_workspace_id on pages(workspace_id);
create index idx_pages_parent_page_id on pages(parent_page_id);
