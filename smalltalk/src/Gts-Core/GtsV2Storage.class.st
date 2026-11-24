"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2Storage,
	#superclass : #Object,
	#instVars : [ 'tag', 'codec', 'encodedCodec', 'storedCheck' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2Storage class >> read: field newer: newer [
	^ self new read: field newer: newer
]

{ #category : #parsing }
GtsV2Storage >> read: field newer: newer [
	| head fields check |
	head := field typedHead: 4.
	head argument = 0 ifTrue: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	fields := field list.
	tag := fields next uint.
	(tag = 0 or: [ newer not and: [ tag > 2 ] ])
		ifTrue: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	tag = 1 ifTrue: [
		head argument = 1 ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
		^ self ].
	tag = 2 ifTrue: [
		head argument = 3 ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
		encodedCodec := fields next.
		codec := encodedCodec uint.
		(codec = 0 or: [ newer not and: [ codec ~= 1 ] ])
			ifTrue: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
		check := fields next.
		storedCheck := check bytesStartingAt: (check byteBodyOfSize: 16).
		^ self ].
	field checkGrammarUpTo: field end.
	^ self
]

{ #category : #accessing }
GtsV2Storage >> tag [
	^ tag
]

{ #category : #testing }
GtsV2Storage >> isZstd [
	^ codec = 1
]

{ #category : #validation }
GtsV2Storage >> storedMatches: field [
	| digest |
	storedCheck ifNil: [ ^ nil ].
	digest := field storedHashWithPrefix: #[129], encodedCodec encodedBytes.
	^ digest ifNotNil: [ (digest copyFrom: 1 to: 16) = storedCheck ]
]
