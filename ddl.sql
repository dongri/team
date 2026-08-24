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

create table tweet_likes (
  id            serial primary key,
  user_id       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  tweet_id      serial REFERENCES tweets (id) ON DELETE CASCADE NOT NULL,
  created       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated       timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(user_id, tweet_id)
);

create table notifications (
  id            serial primary key,
  path          varchar(255),
  from_user     serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  to_user       serial REFERENCES users (id) ON DELETE CASCADE NOT NULL,
  body          text NOT NULL,
  read          boolean DEFAULT false,
  kind          varchar(255) NOT NULL DEFAULT 'comment',
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

-- Indexes ------------------------------------------------------------------

-- Google sign-in looks users up by email
create index if not exists idx_users_email on users (email);

-- Post lists: where status/kind, order by id desc
create index if not exists idx_posts_kind_status_id on posts (kind, status, id desc);
create index if not exists idx_posts_user_id on posts (user_id);

-- Comments are fetched per post / gist / tweet
create index if not exists idx_post_comments_post_id on post_comments (post_id);
create index if not exists idx_post_comments_user_id on post_comments (user_id);
create index if not exists idx_gist_comments_gist_id on gist_comments (gist_id);
create index if not exists idx_gist_comments_user_id on gist_comments (user_id);
create index if not exists idx_tweet_comments_tweet_id on tweet_comments (tweet_id);
create index if not exists idx_tweet_comments_user_id on tweet_comments (user_id);

-- Likes are counted per tweet; (user_id, tweet_id) membership is covered
-- by the UNIQUE constraint
create index if not exists idx_tweet_likes_tweet_id on tweet_likes (tweet_id);

-- Tags: name lookup on save, taggings lookup per post and per tag
create index if not exists idx_tags_name on tags (name);
create index if not exists idx_taggings_post_id_tag_id on taggings (post_id, tag_id);
create index if not exists idx_taggings_tag_id on taggings (tag_id);

-- Stocks: per-user list and (user, post) membership check
create index if not exists idx_stocks_user_id_post_id on stocks (user_id, post_id);
create index if not exists idx_stocks_post_id on stocks (post_id);

-- Pinned lookup per post
create index if not exists idx_pinneds_post_id on pinneds (post_id);

-- Owner lookups / FK cascades
create index if not exists idx_gists_user_id on gists (user_id);
create index if not exists idx_tweets_user_id on tweets (user_id);

-- Notifications: unread badge is checked on every page load
create index if not exists idx_notifications_to_user_read on notifications (to_user, read);
create index if not exists idx_notifications_from_user on notifications (from_user);

-- Preferences: loaded for the signed-in user on every request
create index if not exists idx_preferences_user_id on preferences (user_id);

-- Search: ILIKE '%keyword%' over searchable columns needs trigram GIN indexes
create extension if not exists pg_trgm;
create index if not exists idx_posts_title_trgm on posts using gin (title gin_trgm_ops);
create index if not exists idx_posts_body_trgm on posts using gin (body gin_trgm_ops);
create index if not exists idx_gists_description_trgm on gists using gin (description gin_trgm_ops);
create index if not exists idx_gists_filename_trgm on gists using gin (filename gin_trgm_ops);
create index if not exists idx_gists_code_trgm on gists using gin (code gin_trgm_ops);
create index if not exists idx_tweets_body_trgm on tweets using gin (body gin_trgm_ops);
create index if not exists idx_post_comments_body_trgm on post_comments using gin (body gin_trgm_ops);
create index if not exists idx_gist_comments_body_trgm on gist_comments using gin (body gin_trgm_ops);
create index if not exists idx_tweet_comments_body_trgm on tweet_comments using gin (body gin_trgm_ops);
