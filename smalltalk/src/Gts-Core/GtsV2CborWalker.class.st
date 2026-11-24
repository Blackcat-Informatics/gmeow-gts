"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2CborWalker,
	#superclass : #Object,
	#instVars : [ 'bytes' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2CborWalker class >> on: bytes [
	^ self new initializeOn: bytes
]

{ #category : #initialization }
GtsV2CborWalker >> initializeOn: aByteArray [
	bytes := aByteArray.
	^ self
]

{ #category : #parsing }
GtsV2CborWalker >> headAt: start limit: limit [
	^ (self headIfCompleteAt: start limit: limit)
		ifNil: [ GtsV2FramingError signal: 'torn_tail' ]
]

{ #category : #parsing }
GtsV2CborWalker >> headIfCompleteAt: start limit: limit [
	| first major additional position width value minimum |
	start >= limit ifTrue: [ ^ nil ].
	first := bytes at: start.
	major := first // 32.
	additional := first bitAnd: 31.
	position := start + 1.
	additional < 24 ifTrue: [
		^ GtsV2CborHead major: major argument: additional end: position shortest: true ].
	additional <= 27 ifTrue: [
		width := 1 bitShift: additional - 24.
		position + width > limit ifTrue: [ ^ nil ].
		value := 0.
		position to: position + width - 1 do: [ :index |
			value := (value bitShift: 8) + (bytes at: index) ].
		(major = 7 and: [ additional = 24 and: [ value < 32 ] ])
			ifTrue: [ GtsV2FramingError signal: 'undelimitable_input' ].
		minimum := #(24 256 65536 4294967296) at: additional - 23.
		^ GtsV2CborHead major: major argument: value end: position + width shortest: value >= minimum ].
	(additional = 31 and: [ (major between: 2 and: 5) or: [ major = 7 ] ])
		ifTrue: [ ^ GtsV2CborHead major: major argument: nil end: position shortest: false ].
	^ GtsV2FramingError signal: 'undelimitable_input'
]

{ #category : #parsing }
GtsV2CborWalker >> itemEndAt: start limit: limit [
	^ ((self itemAt: start) endUpTo: limit)
		ifNil: [ GtsV2FramingError signal: 'torn_tail' ]
]

{ #category : #parsing }
GtsV2CborWalker >> itemAt: start [
	^ GtsV2CborItem walker: self at: start
]

{ #category : #accessing }
GtsV2CborWalker >> bytes [
	^ bytes
]
