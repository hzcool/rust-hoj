
CREATE TABLE "user"(
    id bigserial primary key,
    created_at bigint not null default floor(extract(epoch from((current_timestamp - timestamp '1970-01-01 00:00:00')*1000))),
    username varchar(32) unique not null,
    password varchar(16) not null,
    email varchar(32) unique not null,
    school varchar(32) default '',
    description text default '什么都没留下',
    role varchar(16) not null default 'user',
    avatar varchar(128),
    rating int default 1600,
    privilege bigint not null default 0,
    solved int not null default 0,
    all_count int not null default 0,
    accepted_count int not null default 0,
    solved_problems text default '[]',
    failed_problems text default '[]'
);
CREATE UNIQUE INDEX name_index ON "user"(username);
CREATE INDEX email_index ON "user" USING HASH (email);
INSERT INTO "user"(username, password, email, school,  role,  avatar)
    VALUES('super_admin', '123456', '562954019@qq.com', 'nuaa', 'super_admin', 'https://www.helloimg.com/images/2020/07/27/1041d84b6996f3e71.jpg')