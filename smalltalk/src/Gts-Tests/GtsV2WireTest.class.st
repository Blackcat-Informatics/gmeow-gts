"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2WireTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsV2WireTest >> vector: name [
	^ ('/workspace/vectors/v2/P1/', name, '.bin') asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #private }
GtsV2WireTest >> events: bytes [
	^ self events: bytes budget: GtsV2PayloadBudget new
]

{ #category : #private }
GtsV2WireTest >> events: bytes budget: budget [
	| events |
	events := OrderedCollection new.
	(GtsV2Wire inspect: bytes) eventsDo: [ :event | events add: event ] budget: budget.
	^ events
]

{ #category : #tests }
GtsV2WireTest >> testDispatchUsesTheFirstCompleteItemAndFirstVersionKey [
	| wire |
	self assert: (GtsV2Wire inspect: (self vector: 'gen-first-generation-tagged')) format equals: #legacy.
	self assert: (GtsV2Wire inspect: (self vector: 'gen-first-generation-v-twice')) format equals: #unknown.
	self assert: (GtsV2Wire inspect: (self vector: 'gen-first-generation-two')) format equals: #unsupported.
	self assert: (GtsV2Wire inspect: (self vector: 'gen-record-one')) format equals: #unknown.
	self assert: (GtsV2Wire inspect: (self vector: 'gen-record-two')) format equals: #v2.
	wire := GtsV2Wire inspect: #[130 1 27 255 255 255 255 255 255 255 255 28].
	self assert: wire format equals: #unsupported.
	self assert: wire generation equals: 18446744073709551615.
	self assert: (GtsV2Wire inspect: #[191 127 96 97 118 255 1 255]) format equals: #legacy
]

{ #category : #tests }
GtsV2WireTest >> testWalkRetainsZeroBasedRangesBeforeTornTail [
	| events |
	events := self events: (self vector: 'torn-after-frames').
	self assert: (events collect: [ :event | event kind ]) asArray equals: #(header frame frame frame stop).
	self assert: (events at: 4) offset equals: 239.
	self assert: (events at: 4) end equals: 353.
	self assert: events last offset equals: 353.
	self assert: events last problem equals: 'torn_tail'
]

{ #category : #tests }
GtsV2WireTest >> testMalformedPrefixKeepsItemsUncheckedUntilNextOpener [
	| events opener |
	opener := self vector: 'gen-record-two'.
	events := self events: (#[130 1 2 0], opener, #[0]).
	self assert: (events collect: [ :event | event kind ]) asArray equals: #(header unchecked header opaque).
	self assert: events first problem equals: 'wrong_length'.
	self assert: events last problem equals: 'wrong_type'.
	self assert: (self events: opener, #[129 0]) last problem equals: 'unregistered_value'.
	self assert: (self events: (self vector: 'header-long-form-first-item')) first problem equals: 'not_deterministic'.
	self assert: (self events: (self vector: 'frame-test-array-long-form')) last problem equals: 'not_deterministic'
]

{ #category : #tests }
GtsV2WireTest >> testWalkDistinguishesTornPrefixesFromImpossibleHeads [
	| opener entries wire |
	opener := self vector: 'gen-record-two'.
	entries := {
		#[248] -> 'torn_tail'.
		#[248 0] -> 'undelimitable_input'.
		#[191 1 255] -> 'undelimitable_input'.
		#[95 122] -> 'undelimitable_input'.
		#[155 255 255 255 255 255 255 255 255] -> 'torn_tail' }.
	entries do: [ :entry |
		| last |
		last := (self events: opener, entry key) last.
		self assert: last kind equals: #stop.
		self assert: last offset equals: opener size.
		self assert: last problem equals: entry value ].
	wire := GtsV2Wire inspect: (self vector: 'file-first-incomplete').
	self assert: wire format equals: #framingError.
	self assert: wire stopReason equals: 'torn_tail'
]

{ #category : #tests }
GtsV2WireTest >> testWalksNestedAndChunkedItemsWithoutRecursiveDecoding [
	| opener nested events |
	opener := self vector: 'gen-record-two'.
	nested := (ByteArray new: 4096 withAll: 129), #[0].
	events := self events: (opener, nested, #[95 65 1 255], opener).
	self assert: (events collect: [ :event | event kind ]) asArray equals: #(header opaque opaque header).
	self assert: events last offset equals: opener size + nested size + 4
]

{ #category : #tests }
GtsV2WireTest >> testOpenerRegistryAndResyncGrammar [
	| events |
	events := self events: (self vector: 'header-hash-newer').
	self assert: events first opener registry equals: 2.
	self assert: events first opener hashAlgorithm equals: 9.
	self assert: events last kind equals: #unchecked.
	self assert: (self events: (self vector: 'header-hash-unregistered')) first problem
		equals: 'unregistered_value'.
	#(
		('marks-order-at-one-byte.head-before-order' 'not_deterministic')
		('marks-order-at-one-byte.65th-and-below' 'out_of_range')
		('marks-order-at-one-byte.order-before-deeper-head' 'unordered_or_repeated')
		('marks-order-at-one-byte.text-first-below' 'wrong_type')
		('resync-too-long.walked' 'out_of_range')
		('resync-too-long.exactly-4096' nil)
	) do: [ :entry |
		| resync |
		resync := (self events: (self vector: entry first)) detect: [ :event | event kind = #resync ].
		self assert: resync problem = entry second description: entry first ]
]

{ #category : #tests }
GtsV2WireTest >> testResyncLimitChecksTheStartOfNestedFields [
	| prefix |
	prefix := GtsHex decode: '8b050250fcf3eb32a2f0267dd16a53f09bff879202098101820282'.
	#((4061 'wrong_type') (4062 'out_of_range')) do: [ :entry |
		| size digest bytes |
		size := entry first.
		digest := (ByteArray with: 89 with: size // 256 with: size \\ 256), (ByteArray new: size).
		bytes := prefix, digest, #[1 0 130 2 0 128 64].
		self assert: (self events: bytes) first problem equals: entry second ]
]

{ #category : #tests }
GtsV2WireTest >> testFrameEnvelopeReportsFirstProblemInWireOrder [
	| parity |
	#(
		('frame-flags-order' 'forbidden_combination')
		('envelope-field-before-content-value' 'wrong_type')
		('frame-compressed-empty' 'forbidden_combination')
		('stored-check-short' 'wrong_length')
		('frame-unknown-content-map' 'wrong_type')
		('frame-unknown-content-bad-utf8' 'wrong_type')
		('frame-signatures-unordered' 'unordered_or_repeated')
		('frame-storage-tag-newer' nil)
	) do: [ :entry |
		| frame |
		frame := (self events: (self vector: entry first)) last.
		self assert: frame kind equals: #frame.
		self assert: frame problem = entry second description: entry first ].
	parity := (self events: (self vector: 'parity-signature')) detect: [ :event | event frameType = 16 ].
	self assert: parity problem equals: 'forbidden_combination'
]

{ #category : #tests }
GtsV2WireTest >> testLinkUsesOnlyTheNextItemAndKeepsTransparentFramesOutOfTheChain [
	#(
		('link-healed-effective' (healed linked))
		('link-healed-before-transparent' (healed linked))
		('link-heal-opaque-between' (linked broken broken linked))
		('link-transparent-damaged' (linked linked linked broken linked))
		('resync-identifier-field-differs' (linked linked linked linked))
	) do: [ :entry |
		| links |
		links := self linksIn: (self vector: entry first).
		self assert: links = entry second description: entry first ]
]

{ #category : #tests }
GtsV2WireTest >> testMissingStoredIdentifierFallsBackOnlyToAWellTypedIdentity [
	self assert: (self linksIn: (self vector: 'identifier-ill-typed-identity-well-typed'))
		equals: #(broken linked linked linked).
	self assert: (self linksIn: (self vector: 'identifier-ill-typed-identity-ill-typed'))
		equals: #(broken broken linked linked)
]

{ #category : #tests }
GtsV2WireTest >> testLinkUsesBytesOfIndefiniteIdentifierDespiteEnvelopeFinding [
	| bytes firstFrame body modified frames |
	bytes := self vector: 'link-ok'.
	firstFrame := (self events: bytes) second.
	body := firstFrame end - 32.
	modified := (bytes copyFrom: 1 to: body - 3), #[95 80],
		(bytes copyFrom: body to: body + 15), #[80],
		(bytes copyFrom: body + 16 to: body + 31), #[255],
		(bytes copyFrom: firstFrame end to: bytes size).
	frames := self events: modified.
	self assert: frames second problem equals: 'not_deterministic'.
	self assert: (self linksIn: modified) equals: #(linked linked linked linked)
]

{ #category : #tests }
GtsV2WireTest >> testEventsCannotAlterThePredecessorForLaterFrames [
	| links wire |
	links := OrderedCollection new.
	wire := GtsV2Wire inspect: (self vector: 'link-ok').
	wire eventsDo: [ :event |
		event link ifNotNil: [ :link | links add: link ].
		event effectiveIdentifier ifNotNil: [ :identifier |
			identifier at: 1 put: ((identifier at: 1) bitXor: 255) ] ].
	self assert: links asArray equals: #(linked linked linked linked)
]

{ #category : #private }
GtsV2WireTest >> linksIn: bytes [
	| frames |
	frames := (self events: bytes) select: [ :event | event kind = #frame ].
	^ (frames collect: [ :event | event link ]) asArray
]

{ #category : #tests }
GtsV2WireTest >> testStoredCheckAndPayloadFindingAreIndependent [
	#(
		('stored-mismatch' false damaged 'decode_failed')
		('zstd-ok' true verified nil)
		('zstd-header-short' true damaged 'codec_constraint')
		('zstd-two-frames' true damaged 'decode_failed')
		('zstd-reserved-block' true damaged 'decode_failed')
		('zstd-output-short' true damaged 'decode_failed')
		('payload-length' nil damaged 'decoded_length')
		('payload-digest' nil damaged 'payload_digest')
		('payload-unknown-codec' true unchecked 'unknown_codec')
	) do: [ :entry |
		| frame |
		frame := (self events: (self vector: entry first)) detect: [ :event | event kind = #frame ].
		self assert: frame storedMatches = entry second description: entry first.
		self assert: frame payload state = entry third description: entry first.
		self assert: frame payload reason = entry fourth description: entry first ].
	(self events: (self vector: 'payload-codec-constraint-twice')) do: [ :event |
		event kind = #frame ifTrue: [
			self assert: event problem equals: 'forbidden_combination'.
			self assert: event payload state equals: #damaged.
			self assert: event payload reason isNil ] ]
]

{ #category : #tests }
GtsV2WireTest >> testStoredCheckHashesTheBytesOfAnIndefinitePayload [
	| bytes frame walker fields stored chunked first changed |
	bytes := self vector: 'zstd-ok'.
	frame := (self events: bytes) second.
	walker := GtsV2CborWalker on: bytes.
	fields := (GtsV2Field walker: walker from: frame offset + 1 to: frame end + 1) fields: 11.
	8 timesRepeat: [ fields next ].
	stored := fields next.
	first := stored byteBodyOfSize: nil.
	chunked := ByteArray streamContents: [ :stream |
		stream nextPut: 95.
		first to: stored end - 1 do: [ :index |
			stream nextPut: 65.
			stream nextPut: (bytes at: index) ].
		stream nextPut: 255 ].
	changed := (bytes copyFrom: 1 to: stored start - 1), chunked,
		(bytes copyFrom: stored end to: bytes size).
	frame := (self events: changed) second.
	self assert: frame problem equals: 'not_deterministic'.
	self assert: frame storedMatches equals: true.
	self assert: frame payload state equals: #damaged.
	self assert: frame payload reason isNil
]

{ #category : #tests }
GtsV2WireTest >> testTotalDecodedRefusalRemainsAcrossSegments [
	| compressed events frames |
	compressed := self vector: 'zstd-ok'.
	events := self events: compressed, compressed, (self vector: 'link-ok')
		budget: (GtsV2PayloadBudget withReadLimit: 599).
	frames := events select: [ :event | event kind = #frame ].
	self assert: frames first payload state equals: #verified.
	self assert: frames second storedMatches equals: true.
	self assert: frames second payload reason equals: 'total_decoded'.
	self assert: frames second payload count equals: 600.
	self assert: frames third payload reason equals: 'total_decoded'.
	self assert: frames third payload count equals: 300
]

{ #category : #tests }
GtsV2WireTest >> testClosingClaimsUseObservedItemsAndTheLastTypedIndex [
	#(
		('offsets-counts-opaque' offsets nil nil)
		('offsets-count-short' offsets nil 'index_offsets')
		('offsets-count-long' offsets nil 'index_offsets')
		('offsets-length-differs' offsets nil 'index_offsets')
		('locator-last-index.first' locator nil 'index_locator')
		('locator-last-index.second' locator nil nil)
		('locator-index-type-long-form' locator nil 'index_locator')
		('locator-short-form' locator 'not_deterministic' nil)
	) do: [ :entry |
		| event |
		event := (self events: (self vector: entry first)) last.
		self assert: event kind = entry second description: entry first.
		self assert: event problem = entry third description: entry first.
		self assert: event closingClaimFailure = entry fourth description: entry first ]
]

{ #category : #tests }
GtsV2WireTest >> testClosingPlacementComesBeforeClaimsAndResetsAtAnOpener [
	| header offsets locator events |
	header := self vector: 'gen-record-two'.
	offsets := #[130 4 129 11].
	locator := #[130 3 27 0 0 0 0 0 0 0 1].
	events := self events: header, offsets, locator, locator, header, offsets.
	self assert: (events collect: [ :event | event kind ]) asArray
		equals: #(header offsets locator locator header offsets).
	self assert: (events collect: [ :event | event problem ]) asArray
		equals: #(nil 'unordered_or_repeated' 'unordered_or_repeated' 'unordered_or_repeated' nil nil).
	self assert: (events allSatisfy: [ :event | event closingClaimFailure isNil ]).
	events := self events: header, offsets, locator, #[27].
	self assert: (events at: 2) closingClaimFailure isNil.
	self assert: (events at: 3) closingClaimFailure equals: 'index_locator'.
	self assert: events last problem equals: 'torn_tail'
]

{ #category : #tests }
GtsV2WireTest >> testMalformedLengthAfterMismatchStillReportsGrammarError [
	| event |
	event := (self events: (self vector: 'gen-record-two'), #[130 4 130 0 96]) last.
	self assert: event closingClaimFailure isNil.
	self assert: event problem equals: 'wrong_type'
]
