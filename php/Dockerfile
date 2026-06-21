# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

FROM php@sha256:966621a53c8e75f062fad4e043ffec507cb793822aee3422110e0127fe53952d

RUN apt-get update \
    && apt-get install -y --no-install-recommends libffi-dev \
    && docker-php-ext-install ffi \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --uid 10001 appuser

USER appuser
