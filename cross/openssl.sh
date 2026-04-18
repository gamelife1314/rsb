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
        wget
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
    
    # Try multiple mirrors for downloading openssl
    local urls=(
        "https://www.openssl.org/source/openssl-$version.tar.gz"
        "https://github.com/openssl/openssl/releases/download/openssl-$version/openssl-$version.tar.gz"
        "https://mirrors.dotsrc.org/openssl/source/openssl-$version.tar.gz"
    )
    
    local downloaded=false
    for url in "${urls[@]}"; do
        echo "Trying to download from: $url"
        if curl -fSL "$url" -o openssl.tar.gz 2>/dev/null; then
            echo "Download successful from: $url"
            downloaded=true
            break
        fi
    done
    
    if [ "$downloaded" = false ]; then
        echo "Error: Failed to download openssl from all mirrors"
        exit 1
    fi
    
    # Verify it's a valid gzip file before extracting
    if ! gzip -t openssl.tar.gz; then
        echo "Error: Downloaded file is not in gzip format"
        echo "File content preview:"
        head -c 500 openssl.tar.gz || true
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
