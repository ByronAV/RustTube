FROM rust:latest

WORKDIR /usr/src/backend
COPY Cargo.* ./
COPY ./src ./src
COPY ./videos ./videos
RUN cargo build --release

CMD cargo run