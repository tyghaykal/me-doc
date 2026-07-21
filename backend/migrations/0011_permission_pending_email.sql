-- Sharing to an email with no matching account yet: the grant is recorded with
-- principal_id left null and the target email stashed here, then backfilled
-- (principal_id set, pending_email cleared) once that email registers.
alter table permissions add column pending_email text;
create index idx_permissions_pending_email on permissions(pending_email) where pending_email is not null;
