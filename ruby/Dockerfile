# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

FROM ruby@sha256:06759c3a6688bb51e9f754d53901b08533b2183f16c3f48de6f279f79a4f8513

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential libffi-dev \
    && gem install ffi --version '~> 1.17' --no-document \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 appuser

USER appuser
