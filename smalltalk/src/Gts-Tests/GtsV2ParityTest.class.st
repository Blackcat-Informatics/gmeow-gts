"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ParityTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsV2ParityTest >> bytesFor: name [
	^ ('/workspace/vectors/v2/P5/', name) asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #private }
GtsV2ParityTest >> parityIn: bytes [
	| events |
	events := OrderedCollection new.
	(GtsV2Wire inspect: bytes) eventsDo: [ :event |
		event frameType = 16 ifTrue: [ events add: event ] ].
	^ events
]

{ #category : #tests }
GtsV2ParityTest >> testParityFrameAndTableFindingsMatchP5Vectors [
	#(
		'parity-base' 'parity-damaged-each' 'parity-above-capacity'
		'parity-above-capacity-and-damaged' 'parity-above-capacity-wrong-table'
		'parity-broken-copy-no-first' 'parity-condition-six-first-by-case'
		'parity-later-version' 'parity-span-262143' 'parity-span-262144'
		'parity-span-4096' 'parity-span-65536' 'parity-span-short'
		'parity-wrong-transparent'
	) do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P5/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry | self assertParityOf: entry ] ]
]

{ #category : #private }
GtsV2ParityTest >> assertParityOf: entry [
	| input decoded expected actual |
	input := entry at: 'input_file'.
	decoded := GtsCborReader decode: (GtsHex decode: (entry at: 'read_result')).
	expected := OrderedCollection new.
	(decoded at: 7) second do: [ :segment |
		(segment at: 17) do: [ :frame |
			(frame at: 4) = #(2 16) ifTrue: [
				| findings |
				findings := (frame at: 11) collect: [ :finding | finding first ].
				expected add: { frame second. (findings includes: 30). (findings includes: 5).
					(findings includes: 20) } ] ] ].
	actual := (self parityIn: (self bytesFor: input)) collect: [ :event |
		{ event offset. event isDamagedParity. (event parityFinding = 'unknown_frame_type').
			(event isDamagedParity not and: [ event payload state = #unchecked ]) } ].
	self assert: actual = expected description: input, ': ', actual printString, ' expected ', expected printString
]

{ #category : #tests }
GtsV2ParityTest >> testShortParitySpanMustEndAtTheLastStreamItem [
	| bytes frame modified result |
	bytes := self bytesFor: 'parity-base.bin'.
	frame := (self parityIn: bytes) first.
	self assert: frame parityFinding isNil.
	modified := (bytes first: frame offset), #[0], (bytes copyFrom: frame offset + 1 to: bytes size).
	result := (self parityIn: modified) first.
	self deny: result isDamagedParity.
	self assert: result parityFinding equals: 'parity_stale'
]

{ #category : #tests }
GtsV2ParityTest >> testDamagedTransparentParityDoesNotStopTheFold [
	| bytes source ordinary parity |
	bytes := self bytesFor: 'parity-broken-copy-no-first.bin'.
	parity := bytes allButFirst: (self parityIn: bytes) first offset.
	source := '<https://e/s> <https://e/p> "v" .', String lf.
	ordinary := GtsFromNQuads v2FromString: source.
	self assert: (self parityIn: ordinary, parity) first isDamagedParity.
	self assert: (GtsNQuads toNQuads: (GtsV2RdfFold read: (GtsV2Wire inspect: ordinary, parity))) equals: source
]

{ #category : #tests }
GtsV2ParityTest >> testLaterGeometryAfterUnknownCriticalTypeAddsNoParityFinding [
	| bytes parity |
	bytes := '/workspace/vectors/v2/P1/parity-later-version-after-stop.bin' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ].
	parity := (self parityIn: bytes) first.
	self deny: parity isDamagedParity.
	self assert: parity parityFinding isNil
]

{ #category : #tests }
GtsV2ParityTest >> testParityShardFindingsMatchP5Vectors [
	#('parity-base' 'parity-damaged-shard' 'parity-unexplained'
		'parity-unexplained-two-stripes' 'recover-no-signature') do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P5/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry | self assertShardFindingsOf: entry ] ]
]

{ #category : #private }
GtsV2ParityTest >> assertShardFindingsOf: entry [
	| input decoded expected actual |
	input := entry at: 'input_file'.
	decoded := GtsCborReader decode: (GtsHex decode: (entry at: 'read_result')).
	expected := OrderedCollection new.
	(decoded at: 7) second do: [ :segment |
		(segment at: 17) do: [ :frame |
			(frame at: 4) = #(2 16) ifTrue: [
				| findings |
				findings := frame at: 11.
				expected add: { frame second.
					((findings select: [ :finding | #(31 32) includes: finding first ])
						collect: [ :finding | self shardFindingFrom: finding ]) asArray.
					(findings anySatisfy: [ :finding | finding first = 33 ]) } ] ] ].
	actual := (self parityIn: (self bytesFor: input)) collect: [ :event |
		{ event offset. event parityShardFindings asArray. (event parityFinding = 'parity_stale') } ].
	self assert: actual = expected description: input, ': ', actual printString, ' expected ', expected printString
]

{ #category : #private }
GtsV2ParityTest >> shardFindingFrom: finding [
	| count |
	count := (finding at: 7) second.
	finding first = 32 ifTrue: [ ^ { 'parity_unexplained'. nil. count } ].
	^ { 'parity_mismatch'. (finding second second = 40 ifTrue: [ 'data' ] ifFalse: [ 'parity' ]). count }
]

{ #category : #tests }
GtsV2ParityTest >> testChangedSignatureBytesMakeParityStaleWithoutFailingTheFrame [
	| bytes signatures end parity |
	bytes := self bytesFor: 'recover-no-signature.signature-zero.repaired.bin'.
	(GtsV2Wire inspect: bytes) eventsDo: [ :event |
		event frame ifNotNil: [ :frame |
			frame signatureRecords ifNotNil: [ :field |
				field list hasNext ifTrue: [ signatures := field ] ] ] ].
	end := signatures end - 1.
	bytes at: end put: ((bytes at: end) bitXor: 1).
	parity := (self parityIn: bytes) first.
	self deny: parity isDamagedParity.
	self assert: parity parityFinding equals: 'parity_stale'.
	self assert: parity parityShardFindings asArray equals: #(('parity_mismatch' 'data' 1))
]

{ #category : #tests }
GtsV2ParityTest >> testOpenerShardExplanationWaitsForTheFirstOrdinaryLink [
	| writer header identifier parity prefix events |
	writer := GtsV2SegmentWriter new.
	header := writer bytes.
	identifier := GtsBlake3 deriveWith: 'gts2 header' bytes: header.
	parity := self mismatchingParityForHeader: header identifier: identifier.
	prefix := header, parity.
	self assert: (self parityIn: prefix) first parityFinding isNil.
	writer commit.
	events := OrderedCollection new.
	(GtsV2Wire inspect: prefix, (writer bytes allButFirst: header size))
		eventsDo: [ :event | events add: event ].
	self assert: (events collect: [ :event | event frameType ]) asArray equals: #(nil 16 14).
	self assert: events second parityFinding equals: 'parity_stale'.
	self assert: events second parityShardFindings asArray equals: #(('parity_mismatch' 'data' 1))
]

{ #category : #private }
GtsV2ParityTest >> mismatchingParityForHeader: header identifier: identifier [
	| data parity digests table payload identity encoded frameId |
	data := { ByteArray new: 16 }.
	parity := ByteArray new: 256.
	digests := Array new: 4 withAll: ((GtsBlake3 deriveWith: 'gts2 shard' bytes: (ByteArray new: 64)) first: 16).
	table := GtsBlake3 deriveWith: 'gts2 parity table' bytes: (GtsCborWriter encode: { identifier. data. digests }).
	payload := GtsCborWriter encode: { data. digests. parity }.
	identity := { 16. false. true. { 1. 0. header size. identifier. table }.
		payload size. (GtsBlake3 deriveWith: 'gts2 payload' bytes: payload) }.
	encoded := (GtsCborWriter encode: identity) allButFirst.
	frameId := GtsBlake3 deriveWith: 'gts2 frame' prefix: #[135 88 32], identifier
		bytes: encoded from: 1 to: encoded size + 1.
	^ GtsCborWriter encode: { 2 }, identity, { #(1). payload. frameId. #() }
]
