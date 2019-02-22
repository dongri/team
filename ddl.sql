create database team;
\c team

-- Tables

create table users (
  id            serial primary key,
  username      varchar(255) NOT NULL,
  password      varchar(255),
  icon_url      varchar(2048),
  email         varchar(255),
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(username)
);

create table posts (
  id            serial primary key,
  kind          varchar(255) NOT NULL,
  user_id       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  title         varchar(255) NOT NULL,
  body          text NOT NULL,
  status        varchar(255) NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE posts ADD COLUMN shared boolean DEFAULT false;

create table post_comments (
  id            serial primary key,
  user_id       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  post_id       serial REFERENCES posts (id) ON DELETE CASCADE NOT NULL,
  body          text NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table tags (
  id            serial primary key,
  name          varchar(255) NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table taggings (
  id            serial primary key,
  tag_id        serial REFERENCES tags (id) ON DELETE CASCADE NOT NULL,
  post_id       serial REFERENCES posts (id) ON DELETE CASCADE NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table stocks (
  id            serial primary key,
  user_id        serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  post_id       serial REFERENCES posts (id) ON DELETE CASCADE NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table gists (
  id            serial primary key,
  user_id       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  description   varchar(255),
  filename      varchar(255),
  code          text NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table gist_comments (
  id            serial primary key,
  user_id       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  gist_id       serial REFERENCES gists (id) ON DELETE CASCADE NOT NULL,
  body          text NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table tweets (
  id            serial primary key,
  user_id       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  body          text NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table tweet_comments (
  id            serial primary key,
  user_id       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  tweet_id      serial REFERENCES tweets (id) ON DELETE CASCADE NOT NULL,
  body          text NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table notifications (
  id            serial primary key,
  path          varchar(255),
  from_user     serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  to_user       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  body          text NOT NULL,
  read          boolean DEFAULT false,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table preferences (
  id            serial primary key,
  user_id       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  menu          varchar(1024),
  theme         varchar(1024),
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);

create table pinneds (
  id            serial primary key,
  post_id       serial REFERENCES posts (id) ON DELETE CASCADE NOT NULL,
  user_id       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  deleted       boolean DEFAULT false,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP
);
