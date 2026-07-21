-- Threaded replies under a root comment. Replies share the parent's mark_id
-- so the editor highlight still scrolls to the same range. Deleting a root
-- cascades to its replies via parent_id ON DELETE CASCADE.
alter table comments
    add column parent_id uuid references comments(id) on delete cascade;

create index idx_comments_parent_id on comments(parent_id);

-- One root comment per mark; replies may share that mark_id.
drop index if exists idx_comments_mark_id;
create unique index idx_comments_root_mark_id on comments(mark_id) where parent_id is null;
