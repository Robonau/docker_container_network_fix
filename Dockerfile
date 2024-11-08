FROM rust:1.82.0 AS builder
ENV SYSROOT=/dummy
WORKDIR /wd
COPY . .
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo build --bins --release --target=x86_64-unknown-linux-musl

FROM docker:27
COPY --from=builder /wd/target/x86_64-unknown-linux-musl/release/docker_container_network_fix /
CMD /docker_container_network_fix