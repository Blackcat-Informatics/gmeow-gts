"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2Fields,
	#superclass : #Object,
	#instVars : [ 'walker', 'offset', 'end', 'bound' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2Fields class >> walker: aWalker from: start to: finish bound: exclusiveBound [
	^ self new initializeWalker: aWalker from: start to: finish bound: exclusiveBound
]

{ #category : #initialization }
GtsV2Fields >> initializeWalker: aWalker from: start to: finish bound: exclusiveBound [
	walker := aWalker.
	offset := start.
	end := finish.
	bound := exclusiveBound.
	^ self
]

{ #category : #testing }
GtsV2Fields >> hasNext [
	^ offset < end
]

{ #category : #parsing }
GtsV2Fields >> next [
	| start |
	start := offset.
	start >= bound ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	offset := walker itemEndAt: start limit: end.
	^ GtsV2Field walker: walker from: start to: offset
]
