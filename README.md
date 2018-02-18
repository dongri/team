# team

### 百闻不如一见

https://rust-team.herokuapp.com/
```
username: user
password: test
```

### env
```
$ vim ~/.bashrc

export TEAM_DATABASE_URL=postgres://root:@localhost:5432/team
export TEAM_DOMAIN=http://localhost:3000
export TEAM_SLACK=https://hooks.slack.com/services/xxxx/xxxxxxxx
export TEAM_GOOGLE_CLIENT_ID=*********.apps.googleusercontent.com
export TEAM_GOOGLE_CLIENT_SECRET=**********
export TEAM_GOOGLE_REDIRECT_URL=http://localhost:3000/auth/google
export TEAM_GOOGLE_ALLOW_DOMAIN=yourcompany.com
```

# Development (Docker)

### up
```
$ docker-compose up
```
http://localhost:3000

# Development (Mac)

### database
```
$ createdb team
```
ddl.sql

### cargo-watch
```
$ cargo install cargo-watch
```

### run
```
$ cargo watch -x 'run'
```
http://localhost:3000

# Production
```
$ ./run.sh
```

# docker file
https://github.com/dongri/docker-rust
