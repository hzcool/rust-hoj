
CREATE TABLE "problem" (
    id bigserial primary key,
    index varchar(10) unique not null,
    is_open bool,
    created_at bigint,
    author varchar(32) default 'super_admin',
    source varchar(32) default 'happy online judge',
    title varchar(32) ,
    background text,
    statement text,
    input text,
    output text,
    hint text,
    examples_in text,
    examples_out text,
    time_limit int,
    memory_limit int,
    tags text default '[]',
    accepted_count int default 0,
    all_count int default 0,
    spj_config text,
    test_cases text,
    checker varchar(64)
);
CREATE UNIQUE INDEX idx ON "problem"(index)

