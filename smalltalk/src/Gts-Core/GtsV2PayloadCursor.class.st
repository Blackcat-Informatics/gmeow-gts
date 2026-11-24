"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2PayloadCursor,
	#superclass : #Object,
	#instVars : [ 'walker', 'offset', 'limit' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2PayloadCursor class >> on: bytes [
	^ self new walker: (GtsV2CborWalker on: bytes) from: 1 to: bytes size + 1
]

{ #category : #initialization }
GtsV2PayloadCursor >> walker: aWalker from: first to: end [
	walker := aWalker.
	offset := first.
	limit := end.
	^ self
]

{ #category : #accessing }
GtsV2PayloadCursor >> offset [
	^ offset
]

{ #category : #accessing }
GtsV2PayloadCursor >> bytes [
	^ walker bytes
]

{ #category : #accessing }
GtsV2PayloadCursor >> fieldFrom: first [
	^ GtsV2Field walker: walker from: first to: offset
]

{ #category : #accessing }
GtsV2PayloadCursor >> sliceFrom: first to: end [
	^ self class new walker: walker from: first to: end
]

{ #category : #validation }
GtsV2PayloadCursor >> requireEnd [
	offset = limit ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ]
]

{ #category : #parsing }
GtsV2PayloadCursor >> readHead: major [
	| head |
	(offset < limit and: [ (walker bytes at: offset) = 255 ]) ifTrue: [
		GtsV2EnvelopeError signal: 'wrong_length' ].
	head := (GtsV2Field walker: walker from: offset to: limit) typedHead: major.
	offset := head end.
	^ head argument
]

{ #category : #parsing }
GtsV2PayloadCursor >> requireArray: arity [
	(self readHead: 4) = arity ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ]
]

{ #category : #parsing }
GtsV2PayloadCursor >> skip: length [
	offset := offset + length.
	offset > limit ifTrue: [ GtsV2EnvelopeError signal: 'wrong_length' ]
]

{ #category : #parsing }
GtsV2PayloadCursor >> readDigest [
	| start |
	(self readHead: 2) = 32 ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	start := offset.
	self skip: 32.
	^ start
]

{ #category : #parsing }
GtsV2PayloadCursor >> readText [
	| length start |
	length := self readHead: 3.
	start := offset.
	self skip: length.
	(GtsCborText isValidUtf8: walker bytes from: start to: offset) ifFalse: [
		GtsV2EnvelopeError signal: 'wrong_type' ]
]

{ #category : #parsing }
GtsV2PayloadCursor >> readOptionDo: block [
	| count tag |
	count := self readHead: 4.
	count = 0 ifTrue: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	tag := self readHead: 0.
	(tag between: 1 and: 2) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	tag = count ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	tag = 2 ifTrue: [ block value ]
]
