set -ex

main() {
    local version=1.1.1w
    local os=$1 \
          triple=$2

    local dependencies=(
        ca-certificates
        curl
        m4
        make
        perl
    )

    # NOTE cross toolchain must be already installed
    apt-get update
    local purge_list=()
    # shellcheck disable=SC2068
    for dep in ${dependencies[@]}; do
        if ! dpkg -L "$dep"; then
            apt-get install --no-install-recommends -y "$dep"
            # shellcheck disable=SC2206
            purge_list+=( $dep )
        fi
    done

    td=$(mktemp -d)

    pushd "$td"
    
    # Download with error checking
    curl -fSL "https://www.openssl.org/source/openssl-$version.tar.gz" -o openssl.tar.gz
    
    # Verify it's a valid gzip file before extracting
    if ! gzip -t openssl.tar.gz; then
        echo "Error: Downloaded file is not in gzip format"
        exit 1
    fi
    
    tar --strip-components=1 -xzf openssl.tar.gz
    
    # shellcheck disable=SC2068
    AR=${triple}ar CC=${triple}gcc ./Configure \
      --prefix=/openssl \
      no-dso \
      "$os" \
      -fPIC \
      ${@:3}
    # shellcheck disable=SC2046
    nice make -j$(nproc)
    make install

    # clean up
    # shellcheck disable=SC2068
    apt-get purge --auto-remove -y ${purge_list[@]}

    popd

    rm -rf "$td"
    rm "$0"
}

main "${@}"
