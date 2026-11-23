<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

# GTS Kotlin/JVM Engine

This directory contains the Kotlin/JVM implementation of Graph Transport Substrate (GTS).
It is a native JVM engine, not a wrapper around another `gts` binary.

The implementation target is Go-equal/common parity:

- read, fold, and verify the committed conformance corpus;
- deterministic writer and `from-nq`;
- files-profile `pack`, `unpack`, and `diff`;
- COSE Sign1/Encrypt0 helpers, zstd/gzip codec handling, OpenPGP key extraction,
  detached MMR proof verification, streamable compaction, and replication verbs;
- the common `gts` CLI verbs used by the cross-engine interop gate.

Build and test with a local JDK/Gradle installation:

```bash
cd kotlin
gradle test
gradle run --args='fold ../vectors/01-minimal.gts'
```

On systems without a host JVM, use Docker:

```bash
docker run --rm -v "$PWD:/workspace" -w /workspace/kotlin gradle:jdk21 gradle test
```
