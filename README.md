# team

### 百闻不如一见

http://rust-team.herokuapp.com/
```
username: user
password: test
```

# Development (Docker)

### up
```
$ docker-compose up
```
http://localhost:3000

# Development (Mac)

### env
```
$ vim ~/.bashrc

export TEAM_DATABASE_URL=postgres://root:@localhost:5432/team
export TEAM_DOMAIN=http://localhost:3000
export TEAM_SLACK=https://hooks.slack.com/services/xxxx/xxxxxxxx
```

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

