FROM scorpil/rust:stable

ADD . /source
WORKDIR /source

EXPOSE 3000

RUN rustc -V
RUN cargo -V
RUN cargo build

CMD cargo run
