
create table users (
  id            serial primary key,
  email         varchar(255) NOT NULL,
  username      varchar(255) NOT NULL,
  password      varchar(255) NOT NULL,
  icon_url      varchar(2048),
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table posts (
  id            serial primary key,
  kind          int NOT NULL,
  user_id       serial REFERENCES users (id) NOT NULL,
  title         varchar(255) NOT NULL,
  body          text NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table post_comments (
  id            serial primary key,
  user_id       serial REFERENCES users (id) NOT NULL,
  post_id      serial REFERENCES posts (id) NOT NULL,
  body          text NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- create table nippos (
--   id            serial primary key,
--   user_id       serial REFERENCES users (id) NOT NULL,
--   title         varchar(255) NOT NULL,
--   body          text NOT NULL,
--   created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
--   updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
-- );
--
-- create table nippo_comments (
--   id            serial primary key,
--   user_id       serial REFERENCES users (id) NOT NULL,
--   nippo_id      serial REFERENCES nippos (id) NOT NULL,
--   body          text NOT NULL,
--   created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
--   updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
-- );
