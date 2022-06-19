
CREATE TABLE "csubmission" (
      id bigserial primary key,
      run_id bigint not null,
      created_at bigint,
      tid bigint not null,
      cid bigint not null,
      pid bigint not null,
      author varchar(32) not null,
      label varchar(10) not null,
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

CREATE INDEX cid_tid ON csubmission(cid, tid);
CREATE INDEX tid ON csubmission(tid);
