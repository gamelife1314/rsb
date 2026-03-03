FROM rust:latest as build

WORKDIR /usr/src/rsb
COPY . .

# for local build
#ENV RUSTUP_DIST_SERVER="https://rsproxy.cn"
#ENV RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
#RUN cargo --config ./docker/cargo.config.toml install --path .

RUN cargo install --path .

FROM ubuntu

#RUN sed -i 's@deb.debian.org@repo.huaweicloud.com@g' /etc/apt/sources.list && apt update \
#    && apt-get install -y apt-transport-https ca-certificates
#
#RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*

COPY --from=build /usr/local/cargo/bin/rsb /usr/local/bin/rsb

ENTRYPOINT ["/usr/local/bin/rsb"]

CMD ["-n", "500", "-l", "-c", "50", "https://httpbin.org"]