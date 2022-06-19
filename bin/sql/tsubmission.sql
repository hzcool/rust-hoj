
CREATE TABLE "tsubmission" (
    id bigserial primary key,
    created_at bigint,
    uid bigint not null,
    pid bigint not null,
    length int not null ,
    lang varchar(32) not null,
    code text,
    result text
);

CREATE INDEX pid ON tsubmission(pid);



