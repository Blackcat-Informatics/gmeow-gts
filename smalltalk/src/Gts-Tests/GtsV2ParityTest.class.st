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
