# Vectors of PF-files

| vector | outcome | notes |
|---|---|---|
| `files-pass` | built and matches | `pass` is the canonical file of its fold (P7 4.5), the header declaring this profile |
| `files-pass-appended` | built and matches | `pass` is the canonical file of its fold (P7 4.5), the header declaring this profile |
| `files-hide-link-target` | built and matches | `pass` is the canonical file of its fold (P7 4.5), the header declaring this profile; the hard link lost its f:hardLinkOf statement |
| `files-hide-record-owner` | built and matches | `pass` is the canonical file of its fold (P7 4.5), the header declaring this profile; the two record nodes stay visible |
| `files-missing-parent` | built and matches | `pass` is the canonical file of its fold (P7 4.5), the header declaring this profile; the six entries below the directory each lack the parent |
| `files-entry-path-none` | built and matches |  |
| `files-entry-path-cycle` | built and matches | the head is the object of two statements; f:path's object is a blank node, so no files_value |
| `files-entry-path-empty` | derived from the text | an entry whose f:path is rdf:nil and whose one f:kind is f:Directory is the root entry: its path is the empty path, which is well formed and valid, and no check of the profile fails |
| `files-component-dotdot` | built and matches |  |
| `files-component-slash-kept` | built and matches | a path member 'a/b' and a path member holding the byte 00: no files_component, a component is one value whatever its bytes (PF-files 2.3) |
| `files-component-octets-text-bytes` | built and matches | the form does not depend on the bytes being text |
| `files-component-text-literal` | built and matches |  |
| `files-component-hex-text` | built and matches | written as text, with no pair of VALUES |
| `files-component-octets` | built and matches |  |
| `files-duplicate-path` | built and matches |  |
| `files-parent-not-directory` | built and matches |  |
| `files-kind-two` | built and matches | f:kind multiplicity is files_kind's alone |
| `files-kind-unknown` | built and matches |  |
| `files-content-unbound` | built and matches |  |
| `files-content-on-directory` | built and matches |  |
| `files-content-state` | built and matches |  |
| `files-cardinality-modified` | built and matches |  |
| `files-cardinality-link` | built and matches |  |
| `files-domain-device` | built and matches |  |
| `files-domain-shared-record` | built and matches | the record is counted once |
| `files-value-time` | built and matches | written as text, with no pair of VALUES |
| `files-value-time-typed` | built and matches |  |
| `files-value-count` | built and matches | written as text, with no pair of VALUES |
| `files-value-count-typed` | built and matches |  |
| `files-value-count-typed-and-text` | built and matches | the same literal written once as text and once as that typed value: VALUES holds it, so it is a count literal (the file's fold) |
| `files-value-octets-properties` | built and matches | bytes that are text, bytes that are not text, and the empty value for f:attributeValue and f:recordValue |
| `files-value-octets-as-text` | built and matches | each of the six properties as a text literal, and as FF41 hexBinary written as text with no pair of VALUES; per entry the entry, its attribute record and its archive record fail |
| `files-value-permission` | built and matches |  |
| `files-hard-link` | built and matches |  |
| `files-record-duplicate` | built and matches |  |
| `files-named-graph` | built and matches |  |
| `files-two-segments` | built and matches | three segments (files, unreadable, files), a failing check: one profile_violation in the third segment |
| `files-entry-order` | built and matches | PF-files-12: pass with docs/a, docs/a/x, docs/ab, three entries of the path docs/dup, an entry with no path and one with a '..' component: the entries with a valid path in the order of 2.3 |
| `files-set-order` | built and matches | PF-files-13: an entry with permissions written Sticky, OtherRead, OwnerRead, GroupWrite, two attribute records and two archive records written in the opposite order: listed in the order of 2.1 and of the names and keys |
| `files-entry-order-refused` | built and matches | files-entry-order with the two entries of equal paths written as the blank nodes \_:y then \_:x of one segment, read with labelled_lines below the size of the fold: labelling_refused in the file's findings; the equal pair listed with \_:x first, the other entries as in files-entry-order |
| `files-set-order-refused` | built and matches | files-set-order with two attribute records of equal names and two archive records of equal keys, their record terms blank nodes of one segment, the labelling refused: the equal records in ascending bytewise order of E([ordinal, label]) of their record terms |
| `files-two-layouts` | built and matches | one file tree as its canonical file, whose packs stand before the entries and the bindings, and as a file that is not canonical with the entries and the bindings before the blob frames: both read with no finding and with one content digest |
| `files-root-entry` | derived from the text | four files that declare files: no entry; the tree of the worked example with a root entry whose f:path is rdf:nil, a Directory with an owner and a time, which passes every check; the same with the kind RegularFile, which fails check 5 alone; and no entry with an owner and a time stated on a term that is no entry, which fails check 8 |
| `files-replicas-same-path` | built and matches | a base segment with a regular file at a.txt, then two segments of two replicas that each hide it by a term entry and write another entry at the same path: files_duplicate_path with count 2; then a further segment that hides one of the two: check 3 passes |
| `files-names-host-cannot-hold` | built and matches | six regular files in one directory whose last components are the octets 61 2f 62, 61 00 62, 2e 2e 2f 78, 52 65 61 64 6d 65, 72 65 61 64 6d 65 and ff 41: every check passes, since a component is refused only when it is empty, 2e or 2e 2e |
| `files-component-octets-regular-file` | built and matches | a regular file with its bytes bound and one component, the octets ff 41, written as the typed value of kind octets: the component is valid and every check passes |

Totals: built and matches: 44; derived from the text: 2 (46 entries).
