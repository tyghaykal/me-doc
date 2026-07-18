alter table page_content add column search_vector tsvector
    generated always as (to_tsvector('english', coalesce(plain_text, ''))) stored;
create index idx_page_content_search on page_content using gin(search_vector);
