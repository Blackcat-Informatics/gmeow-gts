# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

FROM debian@sha256:4e401d95de7083948053197a9c3913343cd06b706bf15eb6a0c3ccd26f436a0e

RUN apt-get update \
    && apt-get install -y --no-install-recommends git luajit luarocks \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 appuser

USER appuser
