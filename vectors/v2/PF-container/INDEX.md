# Vectors of PF-container

| vector | outcome | notes |
|---|---|---|
| `container-pass` | built and matches |  |
| `container-suppress` | built and matches | a suppress frame with one entry of kind term on the tombstone's node hides every line of the tombstone |
| `container-export-unbound` | built and matches | the binding of an export is check 2's alone (check 4) |
| `container-export-early` | built and matches | X1 is bound to a commit C0 written after o1 to o3, the graphs and the blob and before the statements of o4: the state of C0 lacks (o4, rdf:type, c:Observation) |
| `container-chain-two-first` | built and matches | checks 11 to 13 not judged; the count is the number of exports |
| `container-chain-fork` | built and matches | X2 and X3 both name X1; the count is the number of exports |
| `container-export-root` | built and matches |  |
| `container-observation-time` | built and matches | c:retrievedAt written as text with its datatype, with no pair of VALUES |
| `container-observation-status-typed` | built and matches | c:status of an Unreadable observation written as the typed value [2, 200]: no finding |
| `container-observation-status-text` | built and matches | c:status written as the text 200 with its datatype, with no pair of VALUES |
| `container-export-complete-text` | built and matches | c:complete of X1 written as text with its datatype, with no pair of VALUES (the second export holds false, a different pair) |
| `container-observation-relation` | built and matches |  |
| `container-observation-duplicate` | built and matches | o6 is a second observation of the root in X1 |
| `container-kind-nonrdf-unbound` | built and matches | the blob is declared and nothing binds it |
| `container-kind-absent-graph` | built and matches |  |
| `container-graph-shared` | built and matches | o4 names the graph of o2; the graph g2 is not written, so that no graph is orphaned |
| `container-graph-orphan` | built and matches |  |
| `container-blank-shared` | built and matches |  |
| `container-pass-blank-graph` | built and matches | the RDF resource graph of `pass` is named by the blank node _:g1 (section 7) |
| `container-blank-graph-mentioned` | built and matches |  |
| `container-dangling-auxiliary` | built and matches |  |
| `container-dangling-containment` | built and matches |  |
| `container-outside-root` | built and matches |  |
| `container-tombstone-unlisted` | built and matches | in X2 the root is observed again and lists only n |
| `container-header-only` | built and matches | a file of the profile with no export: check 3 counts 1 when there is no export |
| `container-late-property` | built and matches | a valid export, then after its commit one more statement with the observation o2 as subject: container_observation_late |
| `container-late-graph-line` | built and matches | after the commit, one more line in the graph of o2 (a graph of export 1): container_graph_late |
| `container-late-blob-binding` | built and matches | the non-RDF observation o3 of export 1 has its blob binding after the commit of export 1: container_blob_late |
| `container-media-type-text` | built and matches | a text literal is not an octets literal |
| `container-media-type-hex-text` | built and matches | written as text, with no pair of VALUES |
| `container-octets-non-utf8` | built and matches | the c:entityTag of o3 is the bytes 22 ff 41 22 (not UTF-8) and the c:relationType of the auxiliary resource o4 is the bytes 41 31 (text): no finding, the form does not depend on the bytes being text |
| `container-entity-tag-empty` | built and matches |  |
| `container-compacted` | built and matches | the file of container-pass compacted at the profile: the canonical file of its fold, whose header declares the profile; the compacted file is not stored |
| `container-blob-entry-grows` | built and matches | the blob of o3 is declared in the state of the first export and again after its commit with a smaller length: the entry differs (17) and the core reports the conflict |
| `container-blob-present-late-unit` | built and matches | the recipe of the blob of o3 and its binding stand in the state of the first export; the second unit stands after the commit: the blob is not complete in the state. The core reports missing_dependency with chunk at the recipe: a unit serves only what stands after it (P4 6.1), so the blob is incomplete in the fold of the file too |
| `container-blob-external` | built and matches | an observation bound to an external blob declared in the state |
| `container-checks-segment` | built and matches | two segments declaring the profile: the first has canonical present (check 14) and a graph that no observation names (check 9, reported once, in the last readable segment declaring the profile) |
| `container-current-latest` | built and matches | d is observed in three exports: live, a container listing a member with no observation, then the Absent tombstone: the current observation is the tombstone |
| `container-current-undefined-duplicate` | built and matches | two observations of d in the last export and a live root listing a missing member: only check 6; checks 11 to 13 are not judged |
| `container-current-undefined-two-exports` | built and matches | o2 has two c:export statements and the root lists a missing member: only check 5; checks 11 to 13 are not judged |
| `container-export-two-segments-no-lineage` | built and matches | the observation of the export stands in an earlier segment than its commit, the commit's segment with lineage absent: the state is local to the last segment |
| `container-export-two-segments-lineage` | built and matches | the same with a checked lineage: the state spans both segments |
| `container-export-lineage-mismatch` | built and matches | the same with a lineage present and mismatched: the commit stands in the file and names no state |
| `container-export-state-unknown-commit` | built and matches | the export's referent has an identifier that no commit of the file has |
| `container-export-state-broken-frame` | built and matches | a valid export whose segment holds, before the commit, a frame with a broken link: the commit stands in the file and names no state |
| `container-late-content` | built and matches | the state named by the first commit holds one line, the type statement of the observation; its other properties and its graph follow the commit; the whole file holds twelve lines |
| `container-late-empty-graph` | built and matches | after the commit, an empty_graph line of the observation's graph name |
| `container-late-reifier` | built and matches | after the commit, a reifier_binding line in the default graph whose triple has the observation as subject |
| `container-listing-order` | built and matches | an export of three observations written in descending order of c:resource and a container observation with three c:contains objects: the listing is in ascending bytewise order, the shorter IRI first when it is a prefix of the other |
| `container-outside-root-mutual` | built and matches | two live resources, each naming the other with c:parent, reached from the root by no listing |
| `container-pass-export-metadata-after-commit` | built and matches | pass: the export node and its binding follow the commit by design and are not the content of any observation |
| `container-export-state-opaque-frame` | built and matches | before the commit, a quads frame whose link holds and whose content has the wrong shape: the commit names no state; no container_export_state |
| `container-export-state-signature-grammar` | built and matches | before the commit, a frame whose signatures field holds two equal records: the commit names no state |
| `container-export-state-sealed` | built and matches | a valid export of registry version 2 whose observation stands in a sealed frame, read without the segment key and with it |
| `container-blob-unchecked` | built and matches | a valid export whose observation is bound to a blob declared by one recipe in the state, every unit before the commit, read with rebuilt_bytes_per_read one below the charge of the recipe: exceeds_capacity with total_rebuilt at the recipe, the blob UNCHECKED, no container_blob_late; read at the minimum capacity: no finding |
| `container-listing-order-refused` | built and matches | an export with two observations of one resource (check 6 fails), both blank nodes of one segment with the labels y and x, written in that order, read with labelled_lines below the size of the fold: labelling_refused in the file's findings; the two observations listed with x first; checks 5 and 6 report as in a reading whose labelling succeeds |
| `container-listing-order-canonical` | built and matches | the same file read with a capacity that labels its fold: the two observations in the order of their ids in the canonical dictionary |
| `container-export-after-resync` | built and matches | an export whose content stands in a segment before a resync item with a checked lineage, the commit in the segment the item opens: the state is local to that segment (container_observation_late), as with no lineage |
| `container-observation-status-503` | built and matches | c:status of an Unreadable observation written as the typed value [2, 503], the status that the server returned: no finding |
| `container-observation-status-absent` | derived from the text | an Unreadable observation with no c:status, the record of a retrieval that received no response: no finding, and a content digest that differs from that of the same export with a status |

Totals: built and matches: 59; derived from the text: 1 (60 entries).
