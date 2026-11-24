"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2FrameEnvelope,
	#superclass : #Object,
	#instVars : [ 'frameType', 'problem' ],
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

{ #category : #parsing }
GtsV2FrameEnvelope >> read: item registry: registry [
	[ self check: (item fields: 11) newer: registry > 1 ]
		on: GtsV2EnvelopeError
		do: [ :error | problem := error messageText ].
	^ self
]

{ #category : #validation }
GtsV2FrameEnvelope >> check: fields newer: newer [
	| known parity critical transparent length storage |
	fields next uint.
	frameType := fields next uint.
	(frameType = 0 or: [ newer not and: [ frameType > 16 ] ])
		ifTrue: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	known := frameType <= 16.
	parity := frameType = 16.
	critical := fields next flag.
	(known and: [ critical ~= (frameType < 14) ])
		ifTrue: [ GtsV2EnvelopeError signal: 'forbidden_combination' ].
	transparent := fields next flag.
	(known and: [ transparent ~= parity ])
		ifTrue: [ GtsV2EnvelopeError signal: 'forbidden_combination' ].
	self checkContent: fields next.
	length := fields next uint.
	fields next byteBodyOfSize: 32.
	storage := self storage: fields next newer: newer.
	((storage = 2 and: [ length = 0 ]) or: [ parity and: [ storage ~= 1 ] ])
		ifTrue: [ GtsV2EnvelopeError signal: 'forbidden_combination' ].
	fields next byteBodyOfSize: nil.
	fields next byteBodyOfSize: 32.
	self checkSignatures: fields next newer: newer parity: parity.
	((frameType = 8 and: [ length = 0 ])
		or: [ (#(12 14) includes: frameType) and: [ length ~= 0 ] ])
		ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ]
]

{ #category : #validation }
GtsV2FrameEnvelope >> checkContent: item [
	| shape fields |
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
GtsV2FrameEnvelope >> storage: field newer: newer [
	| head fields tag codec |
	head := field typedHead: 4.
	head argument = 0 ifTrue: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	fields := field list.
	tag := fields next uint.
	(tag = 0 or: [ newer not and: [ tag > 2 ] ])
		ifTrue: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	tag = 1 ifTrue: [
		head argument = 1 ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
		^ tag ].
	tag = 2 ifTrue: [
		head argument = 3 ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
		codec := fields next uint.
		(codec = 0 or: [ newer not and: [ codec ~= 1 ] ])
			ifTrue: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
		fields next byteBodyOfSize: 16.
		^ tag ].
	field checkGrammarUpTo: field end.
	^ tag
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
