CREATE TABLE  "post" (
    id bigserial primary key,
    created_at bigint,
    updated_at bigint,
    uid bigint not null,
    cid bigint not null,
    pid bigint not null,
    kind int not null,
    title varchar(64),
    content text,
    tags text default '[]',
    comment_count int default '0',
    comment_allowable bool default 'true'
);
CREATE INDEX uid_of_post ON post(uid);
CREATE INDEX cid_pid ON post(cid, pid);

