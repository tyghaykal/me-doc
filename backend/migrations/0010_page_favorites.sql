create table page_favorites (
    user_id uuid not null references users(id) on delete cascade,
    page_id uuid not null references pages(id) on delete cascade,
    created_at timestamptz not null default now(),
    primary key (user_id, page_id)
);
