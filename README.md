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
Copyright (c) 2019 Dongri Jin

996 License Version 1.0 (Draft)

Permission is hereby granted to any individual or legal entity obtaining a copy of this licensed work (including the source code, documentation and/or related items, hereinafter collectively referred to as the "licensed work"), free of charge, to deal with the licensed work for any purpose, including without limitation, the rights to use, reproduce, prepare derivative works of,  distribute and sublicense the licensed work, subject to the following conditions:

1. The individual or the legal entity must conspicuously display, without modification, this License on each redistributed or derivative copy of the Licensed Work.

2. The individual or the legal entity must strictly comply with all applicable laws, regulations, rules and standards of the jurisdiction relating to labor and employment where the individual is physically located or where the individual was born or naturalized; or where the legal entity is registered or is operating (whichever is stricter). In case that the jurisdiction has no such laws, regulations, rules and standards or its laws, regulations, rules and standards are unenforceable, the individual or the legal entity are required to comply with Core International Labor Standards.

3. The individual or the legal entity shall not induce or force its employee(s), whether full-time or part-time, or its independent contractor(s), in any methods, to agree in oral or written form, to directly or indirectly restrict, weaken or relinquish his or her rights or remedies under such laws, regulations, rules and standards relating to labor and employment as mentioned above, no matter whether such written or oral agreement are enforceable under the laws of the said jurisdiction, nor shall such individual or the legal entity limit, in any methods, the rights of its employee(s) or independent contractor(s) from reporting or complaining to the copyright holder or relevant authorities monitoring the compliance of the license about its violation(s) of the said license.
 
THE LICENSED WORK IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN ANY WAY CONNECTION WITH THE LICENSED WORK OR THE USE OR OTHER DEALINGS IN THE LICENSED WORK.
