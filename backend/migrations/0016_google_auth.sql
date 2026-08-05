-- Google OAuth sign-in: users may now authenticate with a Google account
-- instead of a password. google_sub is the stable Google user identifier.
alter table users
    add column google_sub text,
    alter column password_hash drop not null,
    alter column email set not null;

alter table users
    add constraint users_google_sub_unique unique (google_sub);
