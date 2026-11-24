"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2RdfValues,
	#superclass : #Object,
	#instVars : [ 'units', 'values', 'texts', 'octets', 'rebuilt', 'retainedBytes', 'pinned' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2RdfValues >> initialize [
	super initialize.
	units := Dictionary new.
	values := Dictionary new.
	texts := Dictionary new.
	octets := Dictionary new.
	rebuilt := 0.
	retainedBytes := 0.
	pinned := IdentitySet new
]

{ #category : #initialization }
GtsV2RdfValues >> beginSegment [
	units valuesDo: [ :unit |
		(pinned includes: unit) ifFalse: [ retainedBytes := retainedBytes - unit size ] ].
	units removeAll
]

{ #category : #decoding }
GtsV2RdfValues >> readType: type content: content bytes: bytes [
	| fields digest length count |
	type = 13 ifTrue: [ ^ self readPack: content bytes: bytes ].
	fields := content fields: (#(1 2 4 3) at: type - 7).
	digest := GtsV2RdfColumns body: fields next size: 32.
	type <= 9 ifTrue: [
		type = 9 ifTrue: [ self checkMedia: fields next ].
		^ self addUnit: digest bytes: bytes ].
	length := fields next uint.
	type = 10 ifTrue: [ self checkMedia: fields next ].
	count := fields next uint.
	self declare: type digest: digest size: length count: count bytes: bytes
]

{ #category : #private }
GtsV2RdfValues >> checkMedia: field [
	field optionUpTo: field end do: [ :media | GtsV2RdfColumns text: media ]
]

{ #category : #private }
GtsV2RdfValues >> addUnit: digest bytes: bytes [
	(GtsBlake3 deriveWith: 'gts2 unit' bytes: bytes) = digest
		ifFalse: [ GtsV2EnvelopeError signal: 'unit_digest' ].
	units at: digest ifAbsentPut: [
		retainedBytes := retainedBytes + bytes size.
		retainedBytes > 268435456 ifTrue: [
			GtsV2EnvelopeError signal: 'retained units exceed projection capacity' ].
		bytes ]
]

{ #category : #private }
GtsV2RdfValues >> declare: type digest: digest size: length count: count bytes: bytes [
	| entries parts total actual key |
	count < (type = 10 ifTrue: [ 2 ] ifFalse: [ 1 ])
		ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	entries := GtsV2RdfColumns read: (GtsV2RdfColumns item: bytes) with: [ :entry |
		| pair |
		pair := entry fields: 2.
		{ (GtsV2RdfColumns body: pair next size: 32). pair next uint } ].
	GtsV2RdfColumns requireCount: count in: entries.
	total := 0.
	entries do: [ :entry |
		entry second = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		total := total + entry second ].
	total = length ifFalse: [ GtsV2EnvelopeError signal: 'inconsistent_counts' ].
	rebuilt := rebuilt + length.
	rebuilt > 137438953472 ifTrue: [ GtsV2EnvelopeError signal: 'rebuilt_bytes_per_read' ].
	parts := entries collect: [ :entry |
		| unit |
		unit := units at: entry first ifAbsent: [ ^ self ].
		unit size = entry second ifFalse: [ ^ self ].
		unit ].
	actual := GtsBlake3 deriveWith: 'gts2 unit' updatingWith: [ :update |
		parts do: [ :part | update value: part value: 1 value: part size + 1 ] ].
	actual = digest ifFalse: [ GtsV2EnvelopeError signal: 'unit_digest' ].
	type = 11 ifTrue: [
		key := { digest. length }.
		values at: key ifAbsentPut: [
			pinned addAll: parts.
			parts ] ]
]

{ #category : #private }
GtsV2RdfValues >> readPack: content bytes: bytes [
	| pack |
	pack := GtsV2Pack read: content bytes: bytes.
	pack membersDo: [ :type :fields :start :size :index |
		type <= 9 ifTrue: [ self readType: type content: fields bytes: (bytes copyFrom: start to: start + size - 1) ] ].
	pack membersDo: [ :type :fields :start :size :index |
		type >= 10 ifTrue: [ self readType: type content: fields bytes: (bytes copyFrom: start to: start + size - 1) ] ]
]

{ #category : #accessing }
GtsV2RdfValues >> textFor: digest length: length [
	| key |
	key := { digest. length }.
	^ texts at: key ifAbsentPut: [
		| parts bytes |
		parts := values at: key ifAbsent: [ GtsV2EnvelopeError signal: 'missing_value' ].
		length > 268435456 ifTrue: [ GtsV2EnvelopeError signal: 'referenced text exceeds projection capacity' ].
		bytes := ByteArray streamContents: [ :stream | parts do: [ :part | stream nextPutAll: part ] ].
		GtsV2RdfColumns utf8: bytes ]
]

{ #category : #accessing }
GtsV2RdfValues >> octetsFor: digest length: length [
	| key |
	key := { digest. length }.
	^ octets at: key ifAbsentPut: [
		| parts |
		parts := values at: key ifAbsent: [ GtsV2EnvelopeError signal: 'missing_value' ].
		GtsV2RdfTypedValue hexOf: parts length: length ]
]
