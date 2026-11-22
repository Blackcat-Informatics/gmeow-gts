# Vectors of PF-dist

| vector | outcome | notes |
|---|---|---|
| `dist-pass` | built and matches |  |
| `dist-not-canonical` | built and matches |  |
| `dist-not-canonical-claim-false` | built and matches |  |
| `dist-not-alone` | built and matches |  |
| `dist-decoded-size` | built and matches |  |
| `dist-decoded-size-pass` | built and matches |  |
| `dist-stored-size` | built and matches |  |
| `dist-storage-differs` | built and matches |  |
| `dist-sealed-frame` | built and matches |  |
| `dist-not-implemented` | built and matches |  |
| `dist-inconclusive-read-failed` | built and matches | dist-pass read through a source whose read fails at offset 12: read_failed in the file's findings, no segment, both content digests absent; INCONCLUSIVE, although the result has no segment; the same file with the read failing at offset 13: INCONCLUSIVE |
| `dist-refuted-not-declared` | built and matches | PF-dist 8: the canonical file of the fold of dist-pass with profile absent, and the input of zero bytes: no profile_violation in either; REFUTED in both (the segment does not declare the profile; the file has no segment) |
| `dist-verified` | built and matches | PF-dist 8: dist-pass is a VERIFIED DISTRIBUTION (PF-dist-10) |
| `dist-verified-storage-differs` | built and matches | PF-dist 8: dist-storage-differs is a VERIFIED DISTRIBUTION (the storage level does not enter condition 1) |
| `dist-refuted` | built and matches | PF-dist 8: dist-not-canonical-claim-false is REFUTED (PF-dist-11) |
| `dist-inconclusive-labelling` | built and matches | PF-dist 8: dist-pass read with labelled_lines one below the size of its fold: no profile_violation, labelling unchecked with labelling_refused, both content digests absent; INCONCLUSIVE |
| `dist-inconclusive-rebuilt` | built and matches | PF-dist 8: the canonical file of a fold with one multi-unit blob read with rebuilt_bytes_per_read one below the charge of its recipe member: exceeds_capacity with total_rebuilt, structure unchecked, integrity intact_unchecked; INCONCLUSIVE |
| `dist-inconclusive-total` | built and matches | PF-dist 8: dist-pass read with decoded_bytes_per_read below its total decoded length: a frame unchecked with exceeds_capacity and total_decoded; INCONCLUSIVE |
| `dist-inconclusive-not-implemented` | built and matches | PF-dist 8: dist-not-implemented is INCONCLUSIVE |
| `dist-sealed-version-1` | built and matches | PF-dist 8: a dist segment of registry version 2 with a sealed frame read by a reader of version 1: unsupported_registry_version, dist_sealed_frame; REFUTED (PF-dist-6) |
| `dist-sealed-version-2` | built and matches | PF-dist 8: the same file read by a reader of version 2 without the key: the sealed frame opaque, visible incomplete with unopened_sealed 1, dist_sealed_frame; REFUTED (PF-dist-6) |
| `dist-inconclusive-checks-prefix` | built and matches | dist-pass read by a reader that runs 1, 2, 3 or 4 of the five checks: no finding and canonical verified, as with five, and profiles states the number; INCONCLUSIVE, since checks is below the number of the checks of the profile |
| `dist-decoded-size-checks-prefix` | built and matches | dist-decoded-size read with decoded_bytes_per_frame 67108865 by a reader that runs 1 to 5 of the checks: the frame is decoded and canonical is verified; with 1 and 2 checks no profile_violation and INCONCLUSIVE, with 3, 4 and 5 dist_decoded_size at the terms frame with the count 67108865 and REFUTED |
| `dist-inconclusive-not-implemented-decoded-size` | built and matches | dist-decoded-size read at the minimum capacity by a reader that does not implement the profile: exceeds_capacity with decoded_size at the terms frame, one profile_not_checked and no profile_violation; INCONCLUSIVE |
| `dist-verified-external-blob` | built and matches | a distribution file whose fold declares one blob by an external frame, with its length and media type, and binds a term of the manifest to it: the bytes of the blob are not in the file, the binding resolves with no missing_dependency, integrity is intact; a VERIFIED DISTRIBUTION |
| `dist-verified-capsule-manifests` | built and matches | a distribution file with two versions of a module, a schema and a permission document as six blobs: with two manifests, one of which binds the module of version 2, the schema of version 1 and the permissions of version 2, and with one manifest whose three parts agree, each read gives no finding and is a VERIFIED DISTRIBUTION; the profile reads no manifest |

Totals: built and matches: 26 (26 entries).
