# Vectors of the queryable snapshot proposal

These vectors belong to the proposal
[queryable snapshots](../../../../docs/proposals/GIP-queryable-snapshots.md).
`python/scripts/gen_snapshot_vectors.py` writes every file of this directory.

| vector | outcome | notes |
|---|---|---|
| `snapshot/spog-public` | built and matches | `source.bin` is a canonical file of one segment; `snapshot.bin` is the snapshot of the state that its last commit names, in the order `spog`, with pages of three rows. Cases: `certify`, `hit`, `absent-neighbour`, `absent-withheld` |
