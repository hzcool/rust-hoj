
CREATE TABLE "team"(
    id bigserial primary key,
    cid bigint not null,
    uid bigint not null,
    name varchar(32) not null,
    result text not null default '{}'
);

CREATE INDEX cid_uid on team(cid, uid);
CREATE INDEX user_id on team(uid);