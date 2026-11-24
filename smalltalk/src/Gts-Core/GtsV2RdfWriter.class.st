"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2RdfWriter,
	#superclass : #Object,
	#instVars : [ 'segment', 'references', 'formedBytes', 'batchFormedBytes' ],
	#category : #'Gts-Core'
}

{ #category : #writing }
GtsV2RdfWriter class >> terms: terms quads: quads [
	^ self new writeTerms: terms quads: quads
]

{ #category : #writing }
GtsV2RdfWriter >> writeTerms: terms quads: quads [
	| prepared |
	segment := GtsV2SegmentWriter new.
	references := Dictionary new.
	formedBytes := 0.
	prepared := terms collect: [ :term |
		{ term. term kind = #triple ifTrue: [ #[] ] ifFalse: [ term value utf8Encoded ] } ].
	self writeReferencedValues: prepared.
	self writeDictionary: prepared.
	1 to: quads size by: 16384 do: [ :start |
		self writeQuads: (quads copyFrom: start to: (start + 16383 min: quads size)) ].
	segment commit.
	^ segment bytes
]

{ #category : #private }
GtsV2RdfWriter >> writeReferencedValues: entries [
	| units rebuiltBytes |
	units := Set new.
	rebuiltBytes := 0.
	entries do: [ :entry |
		| term lexical rawSize |
		term := entry first.
		lexical := entry second.
		rawSize := lexical size + (self languageSizeOf: term).
		(term kind = #literal and: [ lexical isNotEmpty and: [ rawSize > 67108800 ] ]) ifTrue: [
			references at: term value ifAbsentPut: [
				rebuiltBytes := rebuiltBytes + lexical size.
				rebuiltBytes > 268435456 ifTrue: [ GtsArchiveError signal: 'RDF values exceed rebuild capacity' ].
				self writeValue: lexical units: units ] ] ]
]

{ #category : #private }
GtsV2RdfWriter >> writeValue: lexical units: units [
	| digest recipe |
	digest := GtsBlake3 deriveWith: 'gts2 unit' bytes: lexical.
	recipe := OrderedCollection new.
	1 to: lexical size by: 262144 do: [ :start |
		| unit unitDigest |
		unit := lexical copyFrom: start to: (start + 262143 min: lexical size).
		unitDigest := GtsBlake3 deriveWith: 'gts2 unit' bytes: unit.
		(units includes: unitDigest) ifFalse: [
			segment appendType: 8 content: { unitDigest } payload: unit.
			units add: unitDigest ].
		recipe add: { unitDigest. unit size } ].
	segment appendType: 11 content: { digest. lexical size. recipe size } payload: (GtsCborWriter encode: recipe).
	^ { digest. lexical size }
]

{ #category : #private }
GtsV2RdfWriter >> writeDictionary: entries [
	| base batch estimate |
	base := 0.
	batch := OrderedCollection new.
	estimate := 0.
	entries do: [ :entry |
		| size |
		size := (self languageSizeOf: entry first) + 64.
		(self kindOf: entry) = 5 ifFalse: [ size := size + entry second size ].
		(batch isNotEmpty and: [ batch size = 4096 or: [ estimate + size > 67108864 ] ]) ifTrue: [
			self writeTerms: batch base: base.
			base := base + batch size.
			batch := OrderedCollection new.
			estimate := 0 ].
		batch add: entry.
		estimate := estimate + size ].
	batch isNotEmpty ifTrue: [ self writeTerms: batch base: base ]
]

{ #category : #private }
GtsV2RdfWriter >> writeTerms: entries base: base [
	| runs languages languageIds blocks payload counts |
	runs := self runsOf: entries by: [ :entry | self kindOf: entry ].
	languages := OrderedCollection new.
	languageIds := Dictionary new.
	batchFormedBytes := 0.
	blocks := runs collect: [ :run |
		self termBlock: run languageId: [ :tag |
			languageIds at: tag ifAbsentPut: [
				| index |
				index := languages size.
				languages add: tag.
				index ] ] ].
	payload := GtsCborWriter encode: { blocks. languages }.
	payload size > 67108864 ifTrue: [
		| middle |
		entries size = 1 ifTrue: [ GtsArchiveError signal: 'RDF term exceeds frame capacity' ].
		middle := entries size // 2.
		self writeTerms: (entries copyFrom: 1 to: middle) base: base.
		^ self writeTerms: (entries copyFrom: middle + 1 to: entries size) base: base + middle ].
	formedBytes := formedBytes + batchFormedBytes.
	formedBytes > 137438953472 ifTrue: [ GtsArchiveError signal: 'RDF terms exceed formed bytes capacity' ].
	counts := runs collect: [ :run | { run key. run value size } ].
	segment appendType: 1 content: { base. counts. 2. 64 } payload: payload
]

{ #category : #private }
GtsV2RdfWriter >> languageSizeOf: term [
	^ term language ifNil: [ 0 ] ifNotNil: [ :tag | tag utf8Encoded size ]
]

{ #category : #private }
GtsV2RdfWriter >> kindOf: entry [
	| term |
	term := entry first.
	term kind = #literal ifTrue: [ ^ (references includesKey: term value) ifTrue: [ 5 ] ifFalse: [ 2 ] ].
	^ #(iri literal blankNode triple) indexOf: term kind
]

{ #category : #private }
GtsV2RdfWriter >> termBlock: run languageId: languageId [
	| kind entries referencesInRun fields |
	kind := run key.
	entries := run value.
	kind = 3 ifTrue: [ ^ { 3. (entries collect: [ :entry | entry second ]) } ].
	kind = 4 ifTrue: [
		^ { 4 }, ((1 to: 3) collect: [ :column |
			entries collect: [ :entry | entry first triple at: column ] ]) ].
	fields := kind = 5
		ifTrue: [
			referencesInRun := entries collect: [ :entry | references at: entry first value ].
			{ (referencesInRun collect: [ :reference | reference first ]).
			(referencesInRun collect: [ :reference | reference second ]) } ]
		ifFalse: [ self frontCode: entries ].
	(kind = 2 or: [ kind = 5 ]) ifTrue: [ fields := fields, (self literalColumns: entries languageId: languageId) ].
	^ { kind }, fields
]

{ #category : #private }
GtsV2RdfWriter >> frontCode: entries [
	| previous prefixes lengths suffixes |
	previous := #[].
	prefixes := OrderedCollection new.
	lengths := OrderedCollection new.
	suffixes := ByteArray streamContents: [ :stream |
		entries withIndexDo: [ :entry :index |
			| bytes prefix limit |
			bytes := entry second.
			prefix := 0.
			(index - 1) \\ 64 = 0 ifFalse: [
				limit := previous size min: bytes size.
				[ prefix < limit and: [ (previous at: prefix + 1) = (bytes at: prefix + 1) ] ]
					whileTrue: [ prefix := prefix + 1 ] ].
			prefixes add: prefix.
			lengths add: bytes size - prefix.
			batchFormedBytes := batchFormedBytes + prefix.
			stream next: bytes size - prefix putAll: bytes startingAt: prefix + 1.
			previous := bytes ] ].
	^ { prefixes. lengths. suffixes }
]

{ #category : #private }
GtsV2RdfWriter >> literalColumns: entries languageId: languageId [
	| forms datatypes languageIndexes |
	forms := OrderedCollection new.
	datatypes := OrderedCollection new.
	languageIndexes := OrderedCollection new.
	entries do: [ :entry |
		| term |
		term := entry first.
		term language
			ifNil: [ forms add: 1. datatypes add: term datatype ]
			ifNotNil: [ :tag |
				forms add: (term direction = 'ltr' ifTrue: [ 3 ] ifFalse: [
					term direction = 'rtl' ifTrue: [ 4 ] ifFalse: [ 2 ] ]).
				languageIndexes add: (languageId value: tag) ] ].
	^ { forms. datatypes. languageIndexes }
]

{ #category : #private }
GtsV2RdfWriter >> writeQuads: rows [
	| graphs predicates payload |
	graphs := (self runsOf: rows by: [ :row | row at: 4 ifAbsent: [ nil ] ]) collect: [ :run |
		{ (run key ifNil: [ #(1) ] ifNotNil: [ :graph | { 2. graph } ]). run value size } ].
	predicates := (self runsOf: rows by: [ :row | row second ]) collect: [ :run |
		{ run key. run value size } ].
	payload := GtsCborWriter encode: { graphs. predicates. (self deltas: rows column: 3). (self deltas: rows column: 1) }.
	segment appendType: 2 content: { 2. rows size } payload: payload
]

{ #category : #private }
GtsV2RdfWriter >> deltas: rows column: column [
	| previous |
	previous := 0.
	^ rows collect: [ :row |
		| next delta |
		next := row at: column.
		delta := next - previous.
		previous := next.
		delta ]
]

{ #category : #private }
GtsV2RdfWriter >> runsOf: entries by: keyOf [
	| runs run |
	runs := OrderedCollection new.
	entries do: [ :entry |
		| key |
		key := keyOf value: entry.
		(run isNil or: [ run key ~= key ]) ifTrue: [
			run := key -> OrderedCollection new.
			runs add: run ].
		run value add: entry ].
	^ runs
]
