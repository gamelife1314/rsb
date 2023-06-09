FROM rust:alpine as build

WORKDIR /usr/src/rsb
COPY . .
RUN apk add --no-cache musl-dev pkgconfig openssl-dev git

# for local build
#ENV RUSTUP_DIST_SERVER="https://rsproxy.cn"
#ENV RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
#RUN cargo --config ./docker/cargo.config.toml install --path .

RUN cargo install --path .

FROM alpine:latest

COPY --from=build /usr/local/cargo/bin/rsb /usr/local/bin/rsb
RUN apk add libc6-compat

ENTRYPOINT ["/usr/local/bin/rsb"]

CMD ["-n", "500", "-l", "-c", "50", "https://httpbin.org"]