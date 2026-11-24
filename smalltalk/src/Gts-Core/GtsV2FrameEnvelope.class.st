"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2FrameEnvelope,
	#superclass : #Object,
	#instVars : [ 'frameType', 'problem', 'identityStart', 'identityEnd', 'critical', 'transparent', 'storedIdentifier', 'payload', 'content', 'signatureRecords' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2FrameEnvelope class >> read: item registry: registry [
	^ self new read: item registry: registry
]

{ #category : #accessing }
GtsV2FrameEnvelope >> frameType [
	^ frameType
]

{ #category : #accessing }
GtsV2FrameEnvelope >> problem [
	^ problem
]

{ #category : #accessing }
GtsV2FrameEnvelope >> storedIdentifier [
	^ storedIdentifier
]

{ #category : #accessing }
GtsV2FrameEnvelope >> payload [
	^ payload
]

{ #category : #accessing }
GtsV2FrameEnvelope >> content [
	^ content
]

{ #category : #accessing }
GtsV2FrameEnvelope >> signatureRecords [
	^ signatureRecords
]

{ #category : #testing }
GtsV2FrameEnvelope >> isTransparent [
	^ transparent = true
]

{ #category : #testing }
GtsV2FrameEnvelope >> hasUnknownCriticalType [
	^ frameType notNil and: [ frameType > 16 and: [ critical = true and: [ transparent notNil ] ] ]
]

{ #category : #hashing }
GtsV2FrameEnvelope >> identifierOn: bytes predecessor: predecessor [
	identityEnd ifNil: [ ^ nil ].
	^ GtsBlake3
		deriveWith: 'gts2 frame'
		prefix: #[135 88 32], predecessor
		bytes: bytes
		from: identityStart
		to: identityEnd
]

{ #category : #parsing }
GtsV2FrameEnvelope >> read: item registry: registry [
	self check: (item fields: 11) newer: registry > 1.
	^ self
]

{ #category : #validation }
GtsV2FrameEnvelope >> check: fields newer: newer [
	| known parity contentWellTyped length digest storage stored bodyStart identifier signatures |
	fields next uint.
	identityStart := fields offset.
	frameType := self observe: [ fields next uint ].
	(frameType isNotNil and: [ frameType = 0 or: [ newer not and: [ frameType > 16 ] ] ])
		ifTrue: [ self recordProblem: 'unregistered_value' ].
	known := frameType isNotNil and: [ frameType between: 1 and: 16 ].
	parity := frameType = 16.
	critical := self observe: [ fields next flag ].
	(known and: [ critical ~= (frameType < 14) ])
		ifTrue: [ self recordProblem: 'forbidden_combination' ].
	transparent := self observe: [ fields next flag ].
	(known and: [ transparent ~= parity ])
		ifTrue: [ self recordProblem: 'forbidden_combination' ].
	content := fields next.
	contentWellTyped := self observe: [ self checkContent: content. true ].
	length := self observe: [ fields next uint ].
	digest := self observe: [
		| field |
		field := fields next.
		field bytesStartingAt: (field byteBodyOfSize: 32) ].
	(frameType isNotNil and: [ critical isNotNil and: [ transparent isNotNil and: [
		contentWellTyped = true and: [ length isNotNil and: [ digest isNotNil ] ] ] ] ])
		ifTrue: [ identityEnd := fields offset ].
	storage := self observe: [ GtsV2Storage read: fields next newer: newer ].
	(storage isNotNil and: [
		(storage tag = 2 and: [ length = 0 ]) or: [ parity and: [ storage tag ~= 1 ] ] ])
		ifTrue: [ self recordProblem: 'forbidden_combination' ].
	stored := fields next.
	bodyStart := self observe: [ stored byteBodyOfSize: nil ].
	identifier := fields next.
	self observe: [ identifier byteBodyOfSize: 32 ].
	storedIdentifier := identifier observedIdentifier.
	signatures := fields next.
	signatureRecords := self observe: [
		self checkSignatures: signatures newer: newer parity: parity.
		signatures size <= (1 bitShift: 26) ifTrue: [ signatures ] ].
	(length isNotNil and: [ (frameType = 8 and: [ length = 0 ])
		or: [ (#(12 14) includes: frameType) and: [ length ~= 0 ] ] ])
		ifTrue: [ self recordProblem: 'out_of_range' ].
	payload := GtsV2FramePayload length: length digest: digest storage: storage stored: stored
		bodyStart: bodyStart contentSize: content size signaturesSize: signatures size
]

{ #category : #validation }
GtsV2FrameEnvelope >> observe: aBlock [
	^ aBlock on: GtsV2EnvelopeError do: [ :error |
		self recordProblem: error messageText.
		nil ]
]

{ #category : #validation }
GtsV2FrameEnvelope >> recordProblem: reason [
	problem ifNil: [ problem := reason ]
]

{ #category : #validation }
GtsV2FrameEnvelope >> checkContent: item [
	| shape fields |
	frameType ifNil: [ ^ item checkGrammarUpTo: item end ].
	shape := #(
		(uint runs uint uint)
		(uint uint) (uint uint) (uint uint)
		(uint) (uint) (uint)
		(digest) (digest media) (digest uint media uint) (digest uint uint)
		(name uint media) (uint uint) (uint digest) (uint digest)
		(uint uint uint digest digest)) at: frameType ifAbsent: [ nil ].
	shape ifNil: [ ^ item checkGrammarUpTo: item end ].
	fields := item fields: shape size.
	shape do: [ :role | self checkField: fields next role: role ]
]

{ #category : #validation }
GtsV2FrameEnvelope >> checkField: field role: role [
	role = #uint ifTrue: [ ^ field uint ].
	role = #digest ifTrue: [ ^ field byteBodyOfSize: 32 ].
	role = #media ifTrue: [ ^ self checkMediaOption: field ].
	role = #name ifTrue: [
		| pair algorithm |
		pair := field fields: 2.
		algorithm := pair next uint.
		^ pair next byteBodyOfSize: (algorithm = 1 ifTrue: [ 32 ] ifFalse: [ nil ]) ].
	self checkRuns: field
]

{ #category : #validation }
GtsV2FrameEnvelope >> checkRuns: field [
	| runs pair |
	runs := field list.
	[ runs hasNext ] whileTrue: [
		pair := runs next fields: 2.
		pair next uint.
		pair next uint ]
]

{ #category : #validation }
GtsV2FrameEnvelope >> checkMediaOption: field [
	| head fields tag |
	head := field typedHead: 4.
	head argument = 0 ifTrue: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	fields := field list.
	tag := fields next uint.
	(tag between: 1 and: 2) ifTrue: [
		tag = head argument ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
		tag = 2 ifTrue: [ fields next typedHead: 3 ] ].
	field checkGrammarUpTo: field end
]

{ #category : #validation }
GtsV2FrameEnvelope >> checkSignatures: field newer: newer parity: parity [
	| signatures previous record fields algorithm |
	signatures := field list.
	[ signatures hasNext ] whileTrue: [
		record := signatures next.
		fields := record fields: 3.
		algorithm := fields next uint.
		(algorithm = 0 or: [ newer not and: [ algorithm > 2 ] ])
			ifTrue: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
		fields next byteBodyOfSize: 32.
		fields next byteBodyOfSize: nil.
		(previous isNotNil and: [ (previous compare: record) >= 0 ])
			ifTrue: [ GtsV2EnvelopeError signal: 'unordered_or_repeated' ].
		previous := record ].
	(parity and: [ previous isNotNil ]) ifTrue: [ GtsV2EnvelopeError signal: 'forbidden_combination' ]
]
