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
	| events |
	events := OrderedCollection new.
	(GtsV2Wire inspect: bytes) eventsDo: [ :event | events add: event ].
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
