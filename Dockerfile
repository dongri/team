FROM rust:1

ADD . /source
WORKDIR /source

EXPOSE 3000

RUN rustc -V
RUN cargo -V
RUN cargo build --release

CMD ["./target/release/team"]
