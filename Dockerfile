FROM rust as builder
WORKDIR /root
COPY . /root
RUN cargo build --release --target x86_64-unknown-linux-gnu

FROM gcr.io/distroless/cc
EXPOSE 80 443
WORKDIR /
COPY --from=builder /root/target/x86_64-unknown-linux-gnu/release/see .
CMD ["./see", "-c", "see.conf"]
