# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

FROM ghcr.io/ba-st/pharo:v13.1.2 AS blake3-build

ARG BLAKE3_VERSION=1.8.5
ARG DEBIAN_FRONTEND=noninteractive

USER root

COPY native/gts_blake3.c /tmp/gts_blake3.c

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        gcc \
        libc6-dev \
    && curl -fsSL "https://github.com/BLAKE3-team/BLAKE3/archive/refs/tags/${BLAKE3_VERSION}.tar.gz" \
        | tar -xz -C /tmp \
    && gcc -O3 -fPIC -shared \
        -I"/tmp/BLAKE3-${BLAKE3_VERSION}/c" \
        -DBLAKE3_NO_AVX2 \
        -DBLAKE3_NO_AVX512 \
        -DBLAKE3_NO_NEON \
        -DBLAKE3_NO_SSE2 \
        -DBLAKE3_NO_SSE41 \
        -o /usr/local/lib/libblake3.so \
        "/tmp/BLAKE3-${BLAKE3_VERSION}/c/blake3.c" \
        "/tmp/BLAKE3-${BLAKE3_VERSION}/c/blake3_dispatch.c" \
        "/tmp/BLAKE3-${BLAKE3_VERSION}/c/blake3_portable.c" \
        /tmp/gts_blake3.c \
    && cp "/tmp/BLAKE3-${BLAKE3_VERSION}/c/blake3.h" /usr/local/include/blake3.h \
    && rm -rf "/tmp/BLAKE3-${BLAKE3_VERSION}" /tmp/gts_blake3.c /var/lib/apt/lists/*

FROM ghcr.io/ba-st/pharo:v13.1.2

ARG DEBIAN_FRONTEND=noninteractive

USER root

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        libsodium-dev \
        libzstd-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=blake3-build /usr/local/lib/libblake3.so /usr/local/lib/libblake3.so
COPY --from=blake3-build /usr/local/include/blake3.h /usr/local/include/blake3.h

RUN ldconfig

USER pharo
WORKDIR /workspace
