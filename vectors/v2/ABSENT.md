# Inputs not stored in this directory

These inputs do not compress under the size limit for added files. They are in the archive
`gts-v2-corpus-absent-inputs.tar.zst` of the release `corpus-v2-inputs-1`:
<https://github.com/blackcat-informatics/gmeow-gts/releases/download/corpus-v2-inputs-1/gts-v2-corpus-absent-inputs.tar.zst>
(SHA-256 `49a874dccefa583f40cc75f74ab0fa1bc146ee0f6782c7b9b3da81f1a6e4464c`). Extract it at the root of a checkout with
`zstd -dc --long=27 gts-v2-corpus-absent-inputs.tar.zst | tar -x`; each file lands at its path below `vectors/v2/`.
Compare the SHA-256 of each file with the table before use.

| file | bytes | SHA-256 |
|---|---:|---|
| `P4/chunker-golden.bin` | 4194304 | `f07f91e1376601061ced3044d6f6f0bee5470211fc381bf3d720a34fc0f1d7d7` |
| `P4/rebuild-charge-not-charged.bin` | 1054076 | `a6e0f3442ee0a0a6022a97a8a08fa90466c009d8889fdae117e371eea1b55d79` |
| `P5/parity-stale-equal.case.bin` | 10683878 | `d24d8dbf8b80f4108fead892649a7ca5d7ea73035cb565ea9760504c4003fbf4` |
| `P5/parity-stale-equal.nothing-repaired.bin` | 10683878 | `d24d8dbf8b80f4108fead892649a7ca5d7ea73035cb565ea9760504c4003fbf4` |
| `P5/parity-stale-geometry.case.bin` | 10683877 | `43f8662cadb24048e8e1ff7cb3e1c9051608da789826b32090d876df44fa0089` |
| `P5/parity-stale-geometry.nothing-repaired.bin` | 10683877 | `43f8662cadb24048e8e1ff7cb3e1c9051608da789826b32090d876df44fa0089` |
| `P5/recover-a.case.bin` | 10683878 | `3828b21e6a2da30944fd9ae26feef4ded5f600470d00674eed90c03b63a7c319` |
| `P5/recover-a.repaired-copy.bin` | 10683878 | `d4978f01c3fa62e58afc27a5fb5428561750ad6c0c8e9a0027c8624326ecd59f` |
| `P5/recover-b.case.bin` | 10683878 | `efc09f37c75d6fc739ec1592a0b47216aa8c82d6b26df3613b60e77d7275624a` |
| `P5/recover-b.no-repaired-copy.bin` | 10683878 | `efc09f37c75d6fc739ec1592a0b47216aa8c82d6b26df3613b60e77d7275624a` |
| `P5/recover-c.case.bin` | 10683878 | `9f5f2c3495a03444afe9fdc458d01163271099e6b4e85ab8b62ed10430832f7e` |
| `P5/recover-c.repaired-copy.bin` | 10683878 | `d4978f01c3fa62e58afc27a5fb5428561750ad6c0c8e9a0027c8624326ecd59f` |
| `P5/recover-check-charge.bin` | 3373634 | `bfc04d2abbf3d206b6b259a8c108811aedd64a97561878c25c848b7ecaa2e6c8` |
| `P5/recover-far.bin` | 71607688 | `7a01eb91f099b487362727682633b3d503f4f7449f5377afb5b56c97b5d0493c` |
| `P5/recover-rounds-straddle.bin` | 4482897 | `8c6a9ce757c922b483cd22057ac2062fee1399f42566a8e0b81c9fd7a5d00b6c` |
| `P5/tail-live.bin` | 9561641 | `b349a4d6c656ea0763b56197d26e98a0f66d720b01b53066bf0c846d65b0dc36` |
| `P7/read__resume.bin` | 10683878 | `9f5f2c3495a03444afe9fdc458d01163271099e6b4e85ab8b62ed10430832f7e` |
| `P7/stream-parity.parity-stale-equal.bin` | 10683878 | `d24d8dbf8b80f4108fead892649a7ca5d7ea73035cb565ea9760504c4003fbf4` |
| `P7/stream-parity.parity-stale-geometry.bin` | 10683877 | `43f8662cadb24048e8e1ff7cb3e1c9051608da789826b32090d876df44fa0089` |
| `P8/p8-members-layout-cut.bin` | 100663853 | `70b40bfb7079c10c419c03cdaff868b16dbd451a4ba3264a793b75ef391d30bd` |
| `P8/p8-members-layout-pass.bin` | 101450563 | `64165bf554cdc453a85ca2fed4f65b8e7972322da60abe38cbd34e25deb8bd1f` |
| `PF-dist/dist-stored-size.bin` | 71603220 | `281650daa324225d2ff2f47945bdde9db162cfd6b687daf1e22bf66a135d7a61` |
| `PF-stream/stream-concat-two.bin` | 25137202 | `598f8d2f8c4b8a6e566da121ac5a36d602ba27dea87ee3a12213be5d5e6e33f7` |
| `PF-stream/stream-heal-pending-by-parity.bin` | 4476183 | `0e4e045da4050ae6f55ff3a84bcdcc16638de7ab0d949530ca746b0293d0b112` |
| `PF-stream/stream-parity-last-item.bin` | 4194331 | `f93c5d6d575762fd2025d5e2a32ac8f33b7b0d8acdd61b120abdff4485eded2d` |
| `PF-stream/stream-pass.bin` | 12568601 | `f510be37f2b5f7d2f0d2feb7736d9ab7e2fb5757949bd6372cb5956b03920e87` |
| `PF-stream/stream-stream-read.whole.bin` | 12568601 | `f510be37f2b5f7d2f0d2feb7736d9ab7e2fb5757949bd6372cb5956b03920e87` |
