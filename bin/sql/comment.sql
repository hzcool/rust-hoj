CREATE TABLE  "comment" (
     id bigserial primary key,
     created_at bigint,
     uid bigint not null,
     post_id bigint not null,
     reply_id bigint not null default '0',
     content text
);
CREATE INDEX post_id_of_comment ON comment(post_id);

