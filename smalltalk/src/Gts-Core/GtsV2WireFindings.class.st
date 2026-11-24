"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2WireFindings,
	#superclass : #Object,
	#instVars : [ 'fileFindings', 'segmentFindings', 'frameFindings', 'segment', 'positions', 'position', 'currentItem', 'previousItem', 'inputLength', 'itemLimit', 'fileEnd', 'leftOut' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2WireFindings class >> inputLength: inputLength itemLimit: itemLimit [
	^ self new initializeLength: inputLength itemLimit: itemLimit
]

{ #category : #initialization }
GtsV2WireFindings >> initializeLength: length itemLimit: limit [
	inputLength := length.
	itemLimit := limit.
	fileFindings := OrderedCollection new.
	segmentFindings := OrderedCollection new.
	frameFindings := Dictionary new.
	positions := Dictionary new.
	fileEnd := 4.
	leftOut := 0.
	^ self
]

{ #category : #accessing }
GtsV2WireFindings >> fileFindings [
	^ self ordered: fileFindings
]

{ #category : #accessing }
GtsV2WireFindings >> segmentFindingsAt: ordinal [
	^ self ordered: (segmentFindings at: ordinal + 1)
]

{ #category : #accessing }
GtsV2WireFindings >> frameFindingsAt: offset [
	^ self ordered: (frameFindings at: offset ifAbsent: [ #() ])
]

{ #category : #accessing }
GtsV2WireFindings >> fileResultWithoutSegments [
	segmentFindings ifNotEmpty: [ GtsV2EnvelopeError signal: 'the file result requires segment records' ].
	^ { 2. #(). self fileFindings. #(1). #(1 0). fileEnd. leftOut }
]

{ #category : #observing }
GtsV2WireFindings >> rejectedGeneration: format firstLength: length [
	format = #empty ifTrue: [
		fileFindings add: (self code: 1 reason: nil position: nil offset: nil length: nil count: nil).
		^ self ].
	fileFindings add: (self code: (format = #unsupported ifTrue: [ 3 ] ifFalse: [ 2 ])
		reason: nil position: nil offset: 0 length: length count: nil)
]

{ #category : #observing }
GtsV2WireFindings >> framingStoppedAt: offset reason: reason [
	reason = 'items_per_read' ifTrue: [
		fileEnd := 7.
		fileFindings add: (self code: 20 reason: 87 position: nil offset: offset
			length: inputLength - offset count: itemLimit + 1).
		^ self ].
	fileEnd := reason = 'torn_tail' ifTrue: [ 5 ] ifFalse: [ 6 ].
	self recordGapAt: offset to: inputLength reason: reason
]

{ #category : #observing }
GtsV2WireFindings >> observe: event [
	event startsSegment ifTrue: [
		position := 0.
		segment := OrderedCollection new.
		segmentFindings add: segment.
		self remember: event.
		^ self ].
	event kind = #stop ifTrue: [ ^ self framingStoppedAt: event offset reason: event problem ].
	(#(parityRefused resyncRefused) includes: event kind) ifTrue: [ ^ self recordRefusal: event ].
	event kind = #framingGap ifTrue: [
		self recordGapAt: event offset to: event end reason: event problem.
		event problem = 'torn_tail' ifTrue: [ ^ self ] ].
	segment ifNil: [ ^ self ].
	position := position + 1.
	self remember: event.
	event frameType = 16 ifTrue: [ self recordParity: event ]
]

{ #category : #private }
GtsV2WireFindings >> recordGapAt: offset to: end reason: reason [
	| torn |
	torn := reason = 'torn_tail'.
	(torn or: [ end < inputLength ]) ifTrue: [ leftOut := leftOut + 1 ].
	fileFindings add: (self code: (torn ifTrue: [ 9 ] ifFalse: [ 8 ])
		reason: nil position: nil offset: offset length: end - offset count: nil)
]

{ #category : #private }
GtsV2WireFindings >> recordRefusal: event [
	| owner at findings |
	owner := { currentItem. previousItem } detect: [ :item |
		item notNil and: [ event offset >= item first and: [ event offset < item second ] ] ] ifNone: [ nil ].
	findings := owner
		ifNil: [ segment ifNil: [ fileFindings ] ]
		ifNotNil: [ owner fourth ].
	at := owner
		ifNil: [ segment ifNil: [ 0 ] ifNotNil: [ position + 1 ] ]
		ifNotNil: [ owner third ].
	findings add: (self code: 20 reason: 86 position: at offset: event offset
		length: event end - event offset count: event payload count)
]

{ #category : #private }
GtsV2WireFindings >> remember: event [
	"On a rejected scan of two complete items, the candidate can begin in the first."
	previousItem := currentItem.
	currentItem := { event offset. event end. position. segment }
]

{ #category : #private }
GtsV2WireFindings >> recordParity: event [
	| records |
	positions at: event offset put: position.
	records := OrderedCollection new.
	event isDamagedParity ifTrue: [
		records add: (self positionalCode: 30 reason: nil position: position count: nil) ].
	event parityShardFindings do: [ :finding |
		| mismatch reason |
		mismatch := finding first = 'parity_mismatch'.
		reason := mismatch ifTrue: [ finding second = 'data' ifTrue: [ 40 ] ifFalse: [ 41 ] ].
		records add: (self positionalCode: (mismatch ifTrue: [ 31 ] ifFalse: [ 32 ])
			reason: reason position: position count: finding third) ].
	event parityFinding = 'parity_stale' ifTrue: [
		records add: (self positionalCode: 33 reason: nil position: position count: nil) ].
	records ifNotEmpty: [ frameFindings at: event offset put: records ]
]

{ #category : #observing }
GtsV2WireFindings >> addRepair: finding [
	| records at |
	at := positions at: finding second.
	records := frameFindings at: finding second ifAbsentPut: [ OrderedCollection new ].
	finding first = 'repaired'
		ifTrue: [ records add: (self positionalCode: 35 reason: nil position: at count: finding third) ]
		ifFalse: [ records add: (self positionalCode: 34
			reason: (finding third = 'too_many_erasures' ifTrue: [ 42 ] ifFalse: [ 43 ])
			position: at count: nil) ]
]

{ #category : #observing }
GtsV2WireFindings >> tailAt: offset length: length [
	segment add: (self code: 36 reason: nil position: nil offset: offset length: length count: nil)
]

{ #category : #private }
GtsV2WireFindings >> positionalCode: code reason: reason position: at count: count [
	^ self code: code reason: reason position: at offset: nil length: nil count: count
]

{ #category : #private }
GtsV2WireFindings >> code: code reason: reason position: at offset: offset length: length count: count [
	^ { code. self option: reason. #(1). self option: at.
		self option: offset. self option: length. self option: count }
]

{ #category : #private }
GtsV2WireFindings >> option: value [
	^ value ifNil: [ #(1) ] ifNotNil: [ { 2. value } ]
]

{ #category : #private }
GtsV2WireFindings >> ordered: findings [
	| keyed |
	keyed := findings asArray collect: [ :finding |
		| at |
		at := finding fourth.
		{ self stepOf: finding. (at size = 1 ifTrue: [ -1 ] ifFalse: [ at second ]).
			GtsCborWriter encode: finding. finding } ].
	keyed sort: [ :left :right |
		left first = right first
			ifTrue: [
				left second = right second
					ifTrue: [ (GtsCose compareBytes: left third with: right third) <= 0 ]
					ifFalse: [ left second < right second ] ]
			ifFalse: [ left first < right first ] ].
	^ keyed collect: [ :entry | entry fourth ]
]

{ #category : #private }
GtsV2WireFindings >> stepOf: finding [
	| code |
	code := finding first.
	code <= 3 ifTrue: [ ^ 1 ].
	(#(8 9) includes: code) ifTrue: [ ^ 2 ].
	code = 20 ifTrue: [ ^ finding second second = 87 ifTrue: [ 2 ] ifFalse: [ 7 ] ].
	code <= 33 ifTrue: [ ^ 8 ].
	code <= 35 ifTrue: [ ^ 9 ].
	^ 10
]
