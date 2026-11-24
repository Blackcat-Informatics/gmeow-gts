"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2CborContext,
	#superclass : #Object,
	#instVars : [ 'major', 'remaining', 'needsValue' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2CborContext class >> major: aMajor remaining: count [
	^ self new initializeMajor: aMajor remaining: count
]

{ #category : #initialization }
GtsV2CborContext >> initializeMajor: aMajor remaining: count [
	major := aMajor.
	remaining := count.
	needsValue := false.
	^ self
]

{ #category : #testing }
GtsV2CborContext >> isComplete [
	^ remaining = 0
]

{ #category : #testing }
GtsV2CborContext >> acceptsBreak [
	^ remaining isNil and: [ needsValue not ]
]

{ #category : #testing }
GtsV2CborContext >> acceptsHeadByte: byte [
	(major between: 2 and: 3) ifFalse: [ ^ true ].
	^ byte // 32 = major and: [ (byte bitAnd: 31) ~= 31 ]
]

{ #category : #parsing }
GtsV2CborContext >> acceptChild [
	"A map holds pairs. Decrement after its value without doubling the claimed count."
	major = 5 ifTrue: [ needsValue := needsValue not ].
	(remaining isNotNil and: [ needsValue not ]) ifTrue: [ remaining := remaining - 1 ]
]
