
CREATE TABLE "submission" (
    id bigserial primary key,
    created_at bigint,
    uid bigint not null,
    pid bigint not null,
    lang varchar(32) not null,
    code text,
    length int,
    time int,
    memory int,
    total_time int,
    status int,
    compile_info text,
    case_count smallint,
    pass_count smallint,
    is_open bool not null  default 'false',
    error varchar(255),
    details text
);
CREATE INDEX uid ON submission(uid);
CREATE INDEX pid_uid ON submission(pid, uid);



