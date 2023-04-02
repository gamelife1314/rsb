FROM rust:alpine as build

WORKDIR /usr/src/rsb
COPY . .
RUN cargo build --release

FROM alpine:latest

COPY --from=build /usr/src/rsb/target/release/rsb .

ENTRYPOINT ["rsb"]

CMD ["-n", "500", "-l", "-c", "50", "https://httpbin.org"]