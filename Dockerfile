FROM alpine:3.24.2@sha256:294b683cb724975bec92580e1e685676bd4b50bda910ddb8c51d4cabeaec77e6
COPY --from=oci.17166969.xyz/owned-tools@sha256:aaec19e7a44cbc53b40db2d22093600ee888689b89be94da90e5398ed02f1883 /opt/laputa /opt/laputa
ENV PATH=$PATH:/opt/laputa/bin

ARG TARGETARCH

RUN apk add --no-cache \
    dav1d-dev libgcc linux-headers make musl-dev nasm perl pkgconf python3 rustup \
    && ln -s libgcc_s.so.1 /usr/lib/libgcc_s.so \
    && ln -s /opt/laputa/bin/clang /usr/local/bin/cc

ENV PATH="/root/.cargo/bin:$PATH" \
    CC=clang \
    CXX=clang++ \
    AR=llvm-ar \
    CXXSTDLIB=c++ \
    RUSTFLAGS="-C link-arg=-fuse-ld=mold -C target-feature=-crt-static" \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=clang \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_RUNNER=/usr/bin/env \
    CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=clang

COPY rust-toolchain.toml /img/rust-toolchain.toml
WORKDIR /img

RUN rustup-init -y --default-toolchain none --no-modify-path \
    && channel="$(sed -n 's/^channel *= *"\(.*\)"/\1/p' rust-toolchain.toml)" \
    && test -n "$channel" \
    && rustup toolchain install "$channel" --component rust-src --component clippy

RUN case "$TARGETARCH" in \
        amd64) arch=x86_64; kache_sum=d47c5344b34366370347a831307a07fd0cc395cdf8facc390b97ab071e0640ae; engine_sum=c51daed8c621087ec737e399ece70784dc0cdda4ca9271b76ca83328fdfe4730 ;; \
        arm64) arch=aarch64; kache_sum=a1e8622143add4b81647b6073d98e985d0e22fff57cf5a583c632746946b036b; engine_sum=502ea8f9fd4587263ee6b9098fa8aca07e9a76b391759fe179309d09e98a1c61 ;; \
        *) echo "unsupported architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac \
    && mkdir -p /img/.work/tmp \
    && for binary in kache kache-engine; do \
        archive="/img/.work/tmp/$binary.zip"; \
        case "$binary" in kache) sum=$kache_sum ;; *) sum=$engine_sum ;; esac; \
        wget -O "$archive" "https://github.com/joshuarli/kache/releases/download/release-88537005c6c5d90968c6064585ec51a7096093e5/${binary}-${arch}-unknown-linux-musl-static.zip"; \
        echo "$sum  $archive" | sha256sum -c - || exit 1; \
        unzip -o "$archive" -d /img/.work/tmp || exit 1; \
        mv "/img/.work/tmp/${binary}-${arch}-unknown-linux-musl-static" "/usr/local/bin/$binary" || exit 1; \
        chmod +x "/usr/local/bin/$binary"; \
        rm "$archive"; \
    done

ENV RUSTC_WRAPPER=kache
CMD ["make", "test"]
