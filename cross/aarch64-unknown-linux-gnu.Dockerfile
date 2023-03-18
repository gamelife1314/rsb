ARG CROSS_BASE_IMAGE
FROM $CROSS_BASE_IMAGE

# this issue remove oopenssl, mr: https://github.com/cross-rs/cross/pull/322/files
COPY cross/openssl.sh /
RUN bash /openssl.sh linux-aarch64 aarch64-linux-gnu-

ENV OPENSSL_DIR=/openssl \
    OPENSSL_INCLUDE_DIR=/openssl/include \
    OPENSSL_LIB_DIR=/openssl/lib