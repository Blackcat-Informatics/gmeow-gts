"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2CborItem,
	#superclass : #Object,
	#instVars : [ 'walker', 'pending', 'position' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2CborItem class >> walker: aWalker at: start [
	^ self new initializeWalker: aWalker at: start
]

{ #category : #initialization }
GtsV2CborItem >> initializeWalker: aWalker at: start [
	walker := aWalker.
	pending := OrderedCollection with: (GtsV2CborContext major: 4 remaining: 1).
	position := start.
	^ self
]

{ #category : #copying }
GtsV2CborItem >> postCopy [
	super postCopy.
	pending := pending collect: [ :context | context copy ]
]

{ #category : #parsing }
GtsV2CborItem >> endUpTo: limit [
	"Positions are one-based. Keep the item state if this exclusive limit cuts it."
	| bytes parent octet |
	bytes := walker bytes.
	[ position <= limit and: [ pending notEmpty ] ] whileTrue: [
		parent := pending last.
		parent isComplete
			ifTrue: [ pending removeLast ]
			ifFalse: [
				position = limit ifTrue: [ ^ nil ].
				octet := bytes at: position.
				octet = 255
					ifTrue: [
						parent acceptsBreak ifFalse: [ GtsV2FramingError signal: 'undelimitable_input' ].
						pending removeLast.
						position := position + 1 ]
					ifFalse: [
						(parent acceptsHeadByte: octet)
							ifFalse: [ GtsV2FramingError signal: 'undelimitable_input' ].
						(self readChildOf: parent limit: limit) ifFalse: [ ^ nil ] ] ] ].
	^ (pending isEmpty and: [ position <= limit ]) ifTrue: [ position ]
]

{ #category : #private }
GtsV2CborItem >> readChildOf: parent limit: limit [
	| head major argument |
	head := walker headIfCompleteAt: position limit: limit.
	head ifNil: [ ^ false ].
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
			(major between: 2 and: 3) ifTrue: [ position := position + argument ] ].
	^ true
]
