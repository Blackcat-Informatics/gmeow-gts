# Julia General Registration

<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->

`GmeowGTS` is registered from the `julia/` subdirectory of the monorepo. Keep
the package source-only: users provide `libgts` through `GTS_LIBGTS`,
`GTS_LIB_DIR`, or the platform dynamic loader.

## Package Identity

- Name: `GmeowGTS`
- UUID: `2d7fe44c-1957-4481-aa09-d6d0150c36ae`
- Version: `0.9.4`
- Repository: `https://github.com/Blackcat-Informatics/gmeow-gts`
- Subdirectory: `julia`
- License: MIT OR Apache-2.0, with copied license files in this subdirectory

Do not change the UUID after registration. Keep the Julia wrapper version in
lockstep with the GTS release family unless a Julia-only patch is needed.

## First Registration

After the implementation PR for issue #245 is merged to `main`, trigger
Registrator from a repository issue or commit comment. Prefer the issue comment
so the registration discussion stays linked to the work item:

```text
@JuliaRegistrator register subdir=julia

Release notes:
Initial source-only Julia wrapper for the GTS C ABI. The package requires an
externally available libgts shared library through GTS_LIBGTS, GTS_LIB_DIR, or
the platform dynamic loader.
```

Registrator should open a pull request against `JuliaRegistries/General`. New
packages have a General registry waiting period before AutoMerge; monitor the
General pull request and respond to registry feedback there.

## Future Julia Releases

1. Update `julia/Project.toml` and `julia/src/GmeowGTS.jl` version constants.
2. Run `bash julia/scripts/smoke.sh`.
3. Merge the release-prep PR to `main`.
4. Trigger `@JuliaRegistrator register subdir=julia` with release notes.
5. Let TagBot create the Git tag and GitHub release after the General registry
   PR is merged. The configured tag prefix produces tags such as
   `julia-v0.9.4`.

Do not introduce a `GTS_jll` or bundled native binary in the Julia package
without a separate BinaryBuilder/JLL design issue.
