create table permissions (
    id uuid primary key default gen_random_uuid(),
    subject_type text not null check (subject_type in ('workspace', 'page')),
    subject_id uuid not null,
    principal_type text not null check (principal_type in ('user', 'link')),
    principal_id uuid,
    link_token text unique,
    role text not null check (role in ('viewer', 'editor')),
    expires_at timestamptz,
    created_at timestamptz not null default now()
);

create index idx_permissions_subject on permissions(subject_type, subject_id);
create index idx_permissions_link_token on permissions(link_token) where link_token is not null;
