FROM rust:latest AS builder

WORKDIR /app

COPY . .

RUN cargo build --release

FROM gcr.io/distroless/cc

COPY --from=builder /app/target/release/smo_server /

EXPOSE 1207

ENV RUST_LOG info

CMD ["./smo_server"]