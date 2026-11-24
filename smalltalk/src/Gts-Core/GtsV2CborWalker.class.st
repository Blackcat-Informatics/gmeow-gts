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
	| first major additional position width value minimum |
	start >= limit ifTrue: [ GtsV2FramingError signal: 'torn_tail' ].
	first := bytes at: start.
	major := first // 32.
	additional := first bitAnd: 31.
	position := start + 1.
	additional < 24 ifTrue: [
		^ GtsV2CborHead major: major argument: additional end: position shortest: true ].
	additional <= 27 ifTrue: [
		width := 1 bitShift: additional - 24.
		position + width > limit ifTrue: [ GtsV2FramingError signal: 'torn_tail' ].
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
	"Positions are one-based, and the limit and returned item end are exclusive."
	| pending position parent octet head major argument |
	pending := OrderedCollection with: (GtsV2CborContext major: 4 remaining: 1).
	position := start.
	[ pending notEmpty ] whileTrue: [
		parent := pending last.
		parent isComplete
			ifTrue: [ pending removeLast ]
			ifFalse: [
				position >= limit ifTrue: [ GtsV2FramingError signal: 'torn_tail' ].
				octet := bytes at: position.
				octet = 255
					ifTrue: [
						parent acceptsBreak ifFalse: [ GtsV2FramingError signal: 'undelimitable_input' ].
						pending removeLast.
						position := position + 1 ]
					ifFalse: [
						(parent acceptsHeadByte: octet)
							ifFalse: [ GtsV2FramingError signal: 'undelimitable_input' ].
						head := self headAt: position limit: limit.
						position := head end.
						parent acceptChild.
						major := head major.
						argument := head argument.
						(argument isNil or: [ major between: 4 and: 6 ])
							ifTrue: [
								pending add: (GtsV2CborContext
									major: major
									remaining: (major = 6 ifTrue: [ 1 ] ifFalse: [ argument ])) ]
							ifFalse: [
								(major between: 2 and: 3) ifTrue: [
									argument > (limit - position) ifTrue: [ GtsV2FramingError signal: 'torn_tail' ].
									position := position + argument ] ] ] ] ].
	^ position
]
