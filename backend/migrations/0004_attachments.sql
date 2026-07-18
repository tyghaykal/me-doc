create table attachments (
    id uuid primary key default gen_random_uuid(),
    workspace_id uuid not null references workspaces(id) on delete cascade,
    page_id uuid references pages(id) on delete cascade,
    s3_key text not null,
    filename text not null,
    mime_type text not null,
    size bigint not null,
    uploaded_by uuid not null references users(id),
    created_at timestamptz not null default now()
);
create index idx_attachments_workspace_id on attachments(workspace_id);
create index idx_attachments_page_id on attachments(page_id);
