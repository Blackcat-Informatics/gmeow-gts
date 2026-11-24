"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2GraphClaims,
	#superclass : #Object,
	#instVars : [ 'graphs', 'lastRanges', 'lastGraphs', 'rangeCount', 'comparable', 'identity', 'lastCanonicalRows' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2GraphClaims >> initialize [
	super initialize.
	graphs := Dictionary new.
	lastRanges := Array new: 5.
	lastGraphs := Array new: 5.
	lastCanonicalRows := Array new: 4.
	rangeCount := 0.
	comparable := true.
	identity := GtsV2TermIdentity new
]

{ #category : #recording }
GtsV2GraphClaims >> observe: event position: position rowsDo: rows report: report [
	| type bytes columns previous |
	type := event frameType.
	type = 1 ifTrue: [ ^ identity observe: event ].
	type <= 4 ifTrue: [
		previous := lastCanonicalRows at: type.
		lastCanonicalRows at: type put: nil ].
	bytes := event payload decoded.
	(event problem isNotNil or: [ (#(linked healed) includes: event link) not or: [ bytes isNil ] ])
		ifTrue: [ ^ self markUncompared ].
	[ [ columns := GtsV2RdfRows decodeType: type content: event frame content bytes: bytes.
		type = 4 ifTrue: [ GtsV2RdfRows checkAnnotations: columns identity: [ :id |
			identity isComplete ifTrue: [ identity identityOf: id ] ifFalse: [ id ] ] ] ]
		on: GtsV2FramingError do: [ :error | GtsV2EnvelopeError signal: 'wrong_length' ] ]
		on: GtsV2EnvelopeError do: [ :error |
			self markUncompared.
			report value: #malformed_value value: error messageText.
			^ self ].
	(type <= 4 and: [ event frame content list next uint = 1 ]) ifTrue: [
		self checkCanonical: columns type: type after: previous report: report ].
	rows value: columns.
	(type = 4 and: [ columns first size > 1 and: [ identity isComplete not ] ])
		ifTrue: [ ^ self markUncompared ].
	graphs ifNotNil: [
		columns first withIndexDo: [ :graph :row |
			self add: (graph ifNil: [ -1 ]) type: type position: position row: row - 1 ] ]
]

{ #category : #validation }
GtsV2GraphClaims >> checkCanonical: columns type: type after: prior report: report [
	| keyOrder previous violated |
	keyOrder := #((1 3 4 2) (1 2 4) (1 2 3 4)) at: type - 1.
	previous := prior.
	violated := false.
	1 to: columns first size do: [ :index |
		| row |
		row := keyOrder collect: [ :column |
			((columns at: column) at: index) ifNil: [ -1 ] ].
		(previous isNotNil and: [ (self row: previous strictlyPrecedes: row) not ])
			ifTrue: [ violated := true ].
		previous := row ].
	lastCanonicalRows at: type put: previous.
	violated ifTrue: [ report value: #declared_order_violated value: nil ]
]

{ #category : #private }
GtsV2GraphClaims >> row: previous strictlyPrecedes: row [
	1 to: row size do: [ :index |
		| left right |
		left := previous at: index.
		right := row at: index.
		left = right ifFalse: [ ^ left < right ] ].
	^ false
]

{ #category : #recording }
GtsV2GraphClaims >> observeDamagedRegion [
	self abandonTermIdentity.
	self markUncompared
]

{ #category : #recording }
GtsV2GraphClaims >> abandonTermIdentity [
	identity abandon
]

{ #category : #private }
GtsV2GraphClaims >> markUncompared [
	comparable := false.
	graphs := nil.
	lastRanges := nil
]

{ #category : #private }
GtsV2GraphClaims >> add: graph type: type position: position row: row [
	| range |
	graphs ifNil: [ ^ self ].
	(lastGraphs at: type) = graph ifTrue: [
		range := lastRanges at: type.
		range at: 4 put: range fourth + 1.
		^ self ].
	"An index range occupies at least five decoded bytes."
	rangeCount = ((1 bitShift: 26) // 5) ifTrue: [
		graphs := nil.
		lastRanges := nil.
		^ self ].
	range := { type. position. row. 1 }.
	(graphs at: graph ifAbsentPut: [ OrderedCollection new ]) add: range.
	lastGraphs at: type put: graph.
	lastRanges at: type put: range.
	rangeCount := rangeCount + 1
]

{ #category : #testing }
GtsV2GraphClaims >> isComparable [
	^ comparable
]

{ #category : #accessing }
GtsV2GraphClaims >> expectedNames: count [
	(graphs isNil or: [ graphs size ~= count ]) ifTrue: [ ^ nil ].
	^ graphs keys sorted
]

{ #category : #accessing }
GtsV2GraphClaims >> rangesOf: graph [
	^ graphs at: graph
]
