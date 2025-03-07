FROM rust:latest

WORKDIR /usr/src/storage
COPY Cargo.* ./
COPY ./src ./src
RUN cargo build --release

CMD ["cargo", "run", "--release"]