"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2WireEvent,
	#superclass : #Object,
	#instVars : [ 'kind', 'offset', 'end', 'problem', 'opener', 'frameType' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2WireEvent class >> kind: aKind from: start to: finish problem: aProblem [
	^ self new initializeKind: aKind from: start to: finish problem: aProblem
]

{ #category : #initialization }
GtsV2WireEvent >> initializeKind: aKind from: start to: finish problem: aProblem [
	kind := aKind.
	offset := start.
	end := finish.
	problem := aProblem.
	^ self
]

{ #category : #accessing }
GtsV2WireEvent >> kind [
	^ kind
]

{ #category : #accessing }
GtsV2WireEvent >> offset [
	^ offset
]

{ #category : #accessing }
GtsV2WireEvent >> end [
	^ end
]

{ #category : #accessing }
GtsV2WireEvent >> problem [
	^ problem
]

{ #category : #accessing }
GtsV2WireEvent >> opener [
	^ opener
]

{ #category : #accessing }
GtsV2WireEvent >> frameType [
	^ frameType
]

{ #category : #initialization }
GtsV2WireEvent >> opener: anOpener frameType: aFrameType [
	opener := anOpener.
	frameType := aFrameType.
	^ self
]

{ #category : #testing }
GtsV2WireEvent >> startsSegment [
	^ kind = #header or: [ kind = #resync ]
]
