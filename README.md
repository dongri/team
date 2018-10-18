# team

### 百闻不如一见

https://teamx.herokuapp.com/
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
export TEAM_GOOGLE_CLIENT_ID=xxxxxxxxxx.apps.googleusercontent.com
export TEAM_GOOGLE_CLIENT_SECRET=xxxxxxxxxx
export TEAM_GOOGLE_REDIRECT_URL=http://localhost:3000/auth/google
export TEAM_GOOGLE_ALLOW_DOMAIN=yourcompany.com
export TEAM_SECRET_COOKIE=zqXjwojD9MMnbAoL2mT3o
export TEAM_RUST_BACKTRACE=1
```

# Development (Docker)

### up
```
$ rm /tmp/postgres/*
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

# Screen Shot

<a data-flickr-embed="true"  href="https://www.flickr.com/photos/140596581@N07/27373734497/in/dateposted-public/" title="1"><img src="https://farm1.staticflickr.com/978/27373734497_a45fa1a8ed_c.jpg" width="800" height="528" alt="1"></a>

<a data-flickr-embed="true"  href="https://www.flickr.com/photos/140596581@N07/42244553111/in/dateposted-public/" title="2"><img src="https://farm1.staticflickr.com/830/42244553111_6da609ae89_c.jpg" width="800" height="528" alt="2"></a>

<a data-flickr-embed="true"  href="https://www.flickr.com/photos/140596581@N07/42244553011/in/dateposted-public/" title="3"><img src="https://farm1.staticflickr.com/973/42244553011_306ff88d55_c.jpg" width="800" height="528" alt="3"></a>

<a data-flickr-embed="true"  href="https://www.flickr.com/photos/140596581@N07/42244552961/in/dateposted-public/" title="4"><img src="https://farm1.staticflickr.com/903/42244552961_1f467f751c_c.jpg" width="800" height="528" alt="4"></a>

<a data-flickr-embed="true"  href="https://www.flickr.com/photos/140596581@N07/42244552801/in/dateposted-public/" title="5"><img src="https://farm1.staticflickr.com/952/42244552801_d8c2848163_c.jpg" width="800" height="528" alt="5"></a>

# License
MIT License

Copyright (c) 2018 Dongri Jin

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
