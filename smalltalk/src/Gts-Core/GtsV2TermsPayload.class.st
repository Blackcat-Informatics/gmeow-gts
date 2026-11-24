"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2TermsPayload,
	#superclass : #Object,
	#instVars : [ 'blocks', 'languages', 'restart' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2TermsPayload class >> read: bytes content: content newer: newer [
	^ self new read: bytes content: content newer: newer
]

{ #category : #decoding }
GtsV2TermsPayload >> read: bytes content: content newer: newer [
	| walker limit finish body runs |
	walker := GtsV2CborWalker on: bytes.
	limit := bytes size + 1.
	[
		finish := walker itemEndAt: 1 limit: limit.
		body := (GtsV2Field walker: walker from: 1 to: finish) fields: 2.
		blocks := GtsV2RdfColumns read: body next with: [ :block | self readBlock: block newer: newer ].
		languages := GtsV2RdfColumns read: body next with: [ :language | self readLanguage: language ]
	] on: GtsV2FramingError do: [ :error | GtsV2EnvelopeError signal: 'wrong_length' ].
	finish = limit ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	runs := content runs.
	restart := content restart.
	GtsV2RdfColumns requireCount: runs size in: blocks.
	blocks withIndexDo: [ :block :index | self checkBlock: block count: (runs at: index) second ].
	blocks withIndexDo: [ :block :index |
		block first = (runs at: index) first ifFalse: [ GtsV2EnvelopeError signal: 'forbidden_combination' ] ].
	^ self
]

{ #category : #private }
GtsV2TermsPayload >> readBlock: block newer: newer [
	| fields kind roles |
	fields := block list.
	fields hasNext ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	kind := fields next uint.
	(kind > 7 and: [ newer ]) ifTrue: [ GtsV2EnvelopeError signal: 'unsupported_critical' ].
	(kind between: 1 and: 7) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	roles := #(
		(uints uints suffixes)
		(uints uints suffixes forms uints uints)
		(labels)
		(uints uints uints)
		(digests uints forms uints uints)
		(values)
		(digests uints)) at: kind.
	fields := block fields: roles size + 1.
	fields next.
	^ { kind. roles collect: [ :role | self read: fields next role: role newer: newer ] }
]

{ #category : #private }
GtsV2TermsPayload >> read: field role: role newer: newer [
	^ role caseOf: {
		[ #uints ] -> [ GtsV2RdfColumns uints: field ].
		[ #suffixes ] -> [ GtsV2RdfColumns body: field size: nil ].
		[ #labels ] -> [
			GtsV2RdfColumns read: field with: [ :label | GtsV2RdfColumns body: label size: nil ] ].
		[ #digests ] -> [
			GtsV2RdfColumns read: field with: [ :digest | GtsV2RdfColumns body: digest size: 32 ] ].
		[ #forms ] -> [
			GtsV2RdfColumns read: field with: [ :item |
				| code |
				code := item uint.
				(code > 4 and: [ newer ]) ifTrue: [ GtsV2EnvelopeError signal: 'unsupported_critical' ].
				(code between: 1 and: 4) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
				code ] ].
		[ #values ] -> [
			GtsV2RdfColumns read: field with: [ :value | GtsV2RdfTypedValue decode: value newer: newer ] ] }
]

{ #category : #private }
GtsV2TermsPayload >> readLanguage: field [
	| start |
	start := (field typedHead: 3) end.
	[ ^ GtsV2RdfColumns utf8: (field bytesStartingAt: start) ]
		on: GtsV2EnvelopeError do: [ :error | GtsV2EnvelopeError signal: 'wrong_type' ]
]

{ #category : #private }
GtsV2TermsPayload >> checkBlock: block count: count [
	| kind columns typed formsAt |
	kind := block first.
	columns := block second.
	kind <= 2
		ifTrue: [
			GtsV2RdfColumns requireCount: count in: columns first.
			GtsV2RdfColumns requireCount: count in: columns second ]
		ifFalse: [
			(#(3 4 6 7) includes: kind)
				ifTrue: [ columns do: [ :column | GtsV2RdfColumns requireCount: count in: column ] ]
				ifFalse: [
					GtsV2RdfColumns requireCount: count in: columns first.
					GtsV2RdfColumns requireCount: count in: columns second ] ].
	(#(2 5) includes: kind) ifTrue: [
		formsAt := kind = 2 ifTrue: [ 4 ] ifFalse: [ 3 ].
		GtsV2RdfColumns requireCount: count in: (columns at: formsAt).
		typed := (columns at: formsAt) count: [ :form | form = 1 ].
		GtsV2RdfColumns requireCount: typed in: (columns at: formsAt + 1).
		GtsV2RdfColumns requireCount: count - typed in: columns last ]
]

{ #category : #accessing }
GtsV2TermsPayload >> formedCost [
	^ blocks inject: 0 into: [ :sum :block |
		block first <= 2
			ifTrue: [ block second first inject: sum into: [ :total :prefix | total + prefix ] ]
			ifFalse: [ sum ] ]
]

{ #category : #validation }
GtsV2TermsPayload >> validateChargingText: charge [
	self validateChargingText: charge textDo: [ :block :base :index :octets | ]
]

{ #category : #validation }
GtsV2TermsPayload >> validateChargingText: charge textDo: observe [
	| next base |
	base := 0.
	blocks do: [ :block |
		block first <= 2 ifTrue: [
			self textsOf: block second charging: charge do: [ :index :text |
				observe value: block value: base value: index value: text ] ].
		base := base + block second first size ].
	self languageIndexesDo: [ :index |
		index < languages size ifFalse: [ GtsV2EnvelopeError signal: 'out_of_range' ] ].
	next := 0.
	self languageIndexesDo: [ :index |
		index > next ifTrue: [ GtsV2EnvelopeError signal: 'not_deterministic' ].
		index = next ifTrue: [ next := next + 1 ] ].
	(next = languages size and: [ languages asSet size = languages size ])
		ifFalse: [ GtsV2EnvelopeError signal: 'not_deterministic' ]
]

{ #category : #private }
GtsV2TermsPayload >> languageIndexesDo: aBlock [
	blocks do: [ :block |
		(#(2 5) includes: block first) ifTrue: [ block second last do: aBlock ] ]
]

{ #category : #decoding }
GtsV2TermsPayload >> textsOf: columns do: deliver [
	self textsOf: columns charging: [ :length | ] do: deliver
]

{ #category : #private }
GtsV2TermsPayload >> textsOf: columns charging: charge do: deliver [
	| prefixes lengths suffixes cursor previous |
	prefixes := columns first.
	lengths := columns second.
	suffixes := columns third.
	(lengths inject: 0 into: [ :sum :length | sum + length ]) = suffixes size
		ifFalse: [ GtsV2EnvelopeError signal: 'inconsistent_counts' ].
	cursor := 1.
	previous := #[].
	1 to: prefixes size do: [ :index |
		| prefix length end isRestart |
		prefix := prefixes at: index.
		length := lengths at: index.
		end := cursor + length - 1.
		isRestart := (index - 1) \\ restart = 0.
		(index > 1 and: [ prefix > previous size ])
			ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		(isRestart and: [ prefix ~= 0 ]) ifTrue: [ GtsV2EnvelopeError signal: 'forbidden_combination' ].
		(isRestart not and: [ prefix < previous size and: [ length > 0 and: [
			(previous at: prefix + 1) = (suffixes at: cursor) ] ] ])
			ifTrue: [ GtsV2EnvelopeError signal: 'not_deterministic' ].
		charge value: prefix + length.
		previous := (previous copyFrom: 1 to: prefix), (suffixes copyFrom: cursor to: end).
		cursor := end + 1.
		deliver value: index value: previous ]
]

{ #category : #accessing }
GtsV2TermsPayload >> blocks [
	^ blocks
]

{ #category : #accessing }
GtsV2TermsPayload >> languages [
	^ languages
]
