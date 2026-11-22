# Vectors of PF-bundle

| vector | outcome | notes |
|---|---|---|
| `bundle-pass` | built and matches |  |
| `bundle-pass-shared` | built and matches |  |
| `bundle-binding-none` | built and matches |  |
| `bundle-binding-frame` | built and matches | the referent names the terms frame of the segment |
| `bundle-binding-two` | built and matches |  |
| `bundle-external` | built and matches |  |
| `bundle-missing-chunk` | built and matches |  |
| `bundle-size` | built and matches | check 2 precedes check 3 |
| `bundle-size-boundary` | built and matches |  |
| `bundle-not-a-file` | built and matches |  |
| `bundle-not-a-file-torn` | built and matches | the small file without its last byte: torn_tail in the nested read |
| `bundle-depth-pass` | built and matches |  |
| `bundle-depth` | built and matches | the file of depth 4 declares a nested file: depth 5, not read |
| `bundle-depth-other-profile` | built and matches | the file of depth 2 declares no profile: its nested file is not read and the files below it are not judged |
| `bundle-hidden` | built and matches |  |
| `bundle-named-graph` | built and matches |  |
| `bundle-two-segments` | built and matches | three segments (bundle, unreadable, bundle), a failing check: one profile_violation in the third (PF-bundle-13) |
| `bundle-depth-shared-smaller` | built and matches | PF-bundle-10: a nested-file term in a file of depth 4 bound to a blob that the outer file binds at depth 1: no bundle_depth; the control binds a blob nowhere else: depth 5 |
| `bundle-read-order` | built and matches | PF-bundle-12: three nested files bound in the order opposite to ascending E(name): read in ascending E(name); one bound again at depth 2 is read once, at depth 1; and a smaller name of depth 2 is read after them |
| `bundle-rebuild-refused` | built and matches | a nested-file term bound to a blob whose only declaration is a recipe, read with rebuilt_bytes_per_read one below its charge: exceeds_capacity with total_rebuilt at the recipe, no bundle_bytes_unavailable, no nested read (PF-bundle-15) |
| `bundle-shared-allowance` | built and matches | two nested files of depth 1, each holding one recipe of charge T, the outer file's own recipe of charge R, read with rebuilt_bytes_per_read R + T: the first nested read made with T and its recipe admitted; the second made with 0, its recipe refused, so it is capacity-limited; no bundle_not_a_file and no bundle_depth for its term; each nested result states its capacity (PF-bundle-14) |
| `bundle-shared-items` | built and matches | a nested file whose items exceed what the outer read leaves of items_per_read: its result holds exceeds_capacity with item_count; no bundle_not_a_file for its term (PF-bundle-15) |
| `bundle-shared-formed` | built and matches | two nested files of depth 1, each holding one terms frame of charge T, the outer file's own terms frames of charge R, read with formed_bytes_per_read R + T: the first nested read made with T and its frame admitted; the second made with 0, its frame refused with total_formed, so it is capacity-limited; each nested result states its capacity (PF-bundle-14, PF-bundle-15) |
| `bundle-shared-labelled` | built and matches | two nested files of depth 1, each of one segment whose fold weighs W (a total labelled weight of 2W when its own fold and its file's fold are labelled), the outer file with a total labelled weight V, read with labelled_bytes_per_read V + 2W: the first nested read made with labelled_bytes_per_read 2W and both of its folds labelled; the second made with 0, its own fold and its file's fold refused with refusal_weight; each nested result states its capacity (PF-bundle-14) |
| `bundle-refused-order` | built and matches | bundle-pass with two blank nested-file terms, read with labelled_lines below the size: the same profile_violation findings (none) and counts as bundle-pass, plus labelling_refused in the file's findings; nested reads in ascending E(name) of the blob names, unchanged |
| `bundle-refused-violations` | built and matches | three blank nested-file terms failing checks 1, 3 and 4, labelling refused: the findings and counts equal those of the same file read with enough capacity (the order decides no count) |
| `bundle-signed-file-whole` | derived from the text | a nested file whose last commit carries two signatures, carried whole as a blob: the blob is complete, no check fails, and the result of the nested read holds both signature results |

Totals: built and matches: 26; derived from the text: 1 (27 entries).
