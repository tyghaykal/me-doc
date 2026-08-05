create table if not exists user_ai_settings (
    user_id uuid primary key references users(id) on delete cascade,
    api_url text not null,
    api_key_encrypted bytea not null,
    api_key_nonce bytea not null,
    model text not null,
    updated_at timestamptz not null default now()
);
