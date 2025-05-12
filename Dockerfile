FROM rust:1.86 as builder

COPY . .

RUN cargo build --release

ENTRYPOINT ["./target/release/cypi"]
