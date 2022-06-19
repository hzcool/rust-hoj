CREATE TABLE  "contest" (
    id bigserial primary key,
    created_at bigint,
    title varchar(32),
    begin bigint not null,
    length int not null,
    description text,
    author varchar(32),
    is_open bool not null,
    password varchar(32),
    format varchar(16),
    status smallint,
    team_count int,
    problems text,
    clarifications text
);


