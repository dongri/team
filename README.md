# team

### 百闻不如一见

http://rust-team.herokuapp.com/
```
username: user
password: test
```

# Development

### env
```
$ vim ~/.bashrc

export TEAM_DATABASE_URL=postgres://root:@localhost:5432/team
export TEAM_DOMAIN=http://localhost:3000
export TEAM_SLACK=https://hooks.slack.com/services/xxxx/xxxxxxxx
```

### cargo-watch
```
$ cargo install cargo-watch
```

### run
```
$ cargo watch -x 'run'
```

# Production
```
$ ./run.sh
```
