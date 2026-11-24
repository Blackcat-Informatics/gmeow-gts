"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2CommitChecks,
	#superclass : #Object,
	#instVars : [ 'range', 'position', 'committedFrames', 'tailFrames', 'tailPosition', 'hasIndex' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2CommitChecks class >> openedBy: identifier [
	^ self new initializeOpener: identifier
]

{ #category : #initialization }
GtsV2CommitChecks >> initializeOpener: identifier [
	range := GtsV2Range new.
	range add: identifier.
	position := 0.
	committedFrames := 0.
	tailFrames := 0.
	hasIndex := false.
	^ self
]

{ #category : #accessing }
GtsV2CommitChecks >> committedFrames [
	^ committedFrames
]

{ #category : #accessing }
GtsV2CommitChecks >> tailFrames [
	^ tailFrames
]

{ #category : #accessing }
GtsV2CommitChecks >> tailPosition [
	^ tailPosition
]

{ #category : #accessing }
GtsV2CommitChecks >> hasIndex [
	^ hasIndex
]

{ #category : #validating }
GtsV2CommitChecks >> observe: event findingsDo: aBlock [
	| identifier |
	position := position + 1.
	identifier := event effectiveIdentifier.
	event frameType = 15
		ifTrue: [ hasIndex := true ]
		ifFalse: [
			identifier ifNotNil: [
				tailFrames = 0 ifTrue: [ tailPosition := position ].
				tailFrames := tailFrames + 1 ] ].
	(event frameType = 14 and: [ event problem isNil ]) ifTrue: [
		self check: event frame commitContent link: event link findingsDo: aBlock ].
	identifier ifNotNil: [ range add: identifier ]
]

{ #category : #validating }
GtsV2CommitChecks >> check: content link: link findingsDo: aBlock [
	| fields expected countMatches root rootMatches |
	fields := content fields: 2.
	expected := range leafCount - 1.
	countMatches := fields next uint = expected.
	root := fields next.
	rootMatches := (root bytesStartingAt: (root byteBodyOfSize: 32)) = range root.
	countMatches ifFalse: [ aBlock value: 'commit_count_mismatch' value: position value: expected ].
	rootMatches ifFalse: [ aBlock value: 'commit_root_mismatch' value: position value: nil ].
	(countMatches and: [ rootMatches and: [ link = #linked or: [ link = #healed ] ] ]) ifTrue: [
		committedFrames := range leafCount.
		tailFrames := 0.
		tailPosition := nil ]
]
