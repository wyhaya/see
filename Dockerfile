

FROM rust as builder
WORKDIR /root
COPY . /root
RUN apt update && apt install -y musl-tools
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --release --target x86_64-unknown-linux-musl


FROM scratch
EXPOSE 80 443
WORKDIR /
COPY --from=builder /root/target/x86_64-unknown-linux-musl/release/see .
CMD ["./see", "-c", "see.conf"]

