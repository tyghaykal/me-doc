create extension if not exists pgcrypto;

create table users (
    id uuid primary key default gen_random_uuid(),
    email text not null unique,
    password_hash text not null,
    email_verified_at timestamptz,
    created_at timestamptz not null default now()
);

create table workspaces (
    id uuid primary key default gen_random_uuid(),
    name text not null,
    slug text not null unique,
    owner_id uuid not null references users(id) on delete cascade,
    created_at timestamptz not null default now()
);

create table workspace_members (
    workspace_id uuid not null references workspaces(id) on delete cascade,
    user_id uuid not null references users(id) on delete cascade,
    role text not null check (role in ('owner', 'admin', 'member', 'guest')),
    created_at timestamptz not null default now(),
    primary key (workspace_id, user_id)
);

create table refresh_tokens (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id) on delete cascade,
    token_hash text not null unique,
    user_agent text,
    ip text,
    expires_at timestamptz not null,
    revoked_at timestamptz,
    created_at timestamptz not null default now()
);

create index idx_workspace_members_user_id on workspace_members(user_id);
create index idx_refresh_tokens_user_id on refresh_tokens(user_id);
