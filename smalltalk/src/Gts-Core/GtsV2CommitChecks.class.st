"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2CommitChecks,
	#superclass : #Object,
	#instVars : [ 'range', 'head', 'typeRuns', 'runType', 'runLength', 'packs', 'graphs', 'position', 'committedFrames', 'tailFrames', 'tailPosition', 'hasIndex', 'uncomparedDirectories', 'incompletePrefix', 'dictionaryInDoubt' ],
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
	head := identifier.
	typeRuns := ByteArray new writeStream.
	runLength := 0.
	packs := GtsV2PackClaims new.
	graphs := GtsV2GraphClaims new.
	uncomparedDirectories := OrderedCollection new.
	position := 0.
	committedFrames := 0.
	tailFrames := 0.
	hasIndex := false.
	incompletePrefix := false.
	dictionaryInDoubt := false.
	^ self
]

{ #category : #validating }
GtsV2CommitChecks >> observeDamagedRegion [
	position := position + 1.
	dictionaryInDoubt := true.
	graphs markUncompared
]

{ #category : #validating }
GtsV2CommitChecks >> observeDetachedFrame [
	incompletePrefix := true.
	packs markUncompared
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

{ #category : #accessing }
GtsV2CommitChecks >> uncomparedDirectories [
	^ uncomparedDirectories
]

{ #category : #validating }
GtsV2CommitChecks >> observe: event findingsDo: aBlock [
	| identifier validCommit |
	validCommit := false.
	position := position + 1.
	identifier := event effectiveIdentifier.
	(dictionaryInDoubt not and: [ identifier isNotNil and: [
		event frameType isNotNil and: [ event frameType between: 1 and: 5 ] ] ]) ifTrue: [
		graphs observe: event position: range leafCount - 1 report: [ :reason |
			aBlock value: reason value: position value: nil ] ].
	(incompletePrefix not and: [ identifier isNotNil and: [ event frameType = 13 ] ]) ifTrue: [
		packs observe: event position: range leafCount - 1 report: [ :reason |
			aBlock value: reason value: position value: nil ] ].
	event frameType = 15
		ifTrue: [
			hasIndex := true.
			(incompletePrefix and: [ event problem isNil ]) ifTrue: [
				#('index_head' 'index_root' 'index_directory') do: [ :reason |
					aBlock value: 'index_claim_failed (', reason, ')' value: position value: nil ] ].
			(event problem isNil and: [ (#(linked healed) includes: event link) and: [
				event payload state = #verified and: [ event payload decoded isNotNil ] ] ]) ifTrue: [
				self checkIndex: event findingsDo: aBlock ] ]
		ifFalse: [
			identifier ifNotNil: [
				tailFrames = 0 ifTrue: [ tailPosition := position ].
				tailFrames := tailFrames + 1 ] ].
	(event frameType = 14 and: [ event problem isNil and: [ event frame content size <= (1 bitShift: 26) ] ]) ifTrue: [
		validCommit := self check: event frame content link: event link findingsDo: aBlock ].
	identifier ifNotNil: [
		range add: identifier.
		head := identifier.
		self recordType: event frameType ].
	^ validCommit
]

{ #category : #private }
GtsV2CommitChecks >> recordType: type [
	typeRuns ifNil: [ ^ self ].
	(type isNil or: [ (type between: 1 and: 16) not ]) ifTrue: [
		typeRuns := nil.
		^ self ].
	type = runType ifTrue: [ runLength := runLength + 1. ^ self ].
	runLength > 0 ifTrue: [
		GtsCborWriter write: { runType. runLength } on: typeRuns.
		"A larger run list cannot fit in a verified index payload."
		typeRuns position >= (1 bitShift: 26) ifTrue: [ typeRuns := nil ] ].
	runType := type.
	runLength := 1
]

{ #category : #private }
GtsV2CommitChecks >> expectedTypeRuns [
	^ typeRuns ifNotNil: [
		{ typeRuns originalContents. typeRuns position.
			(runLength = 0 ifTrue: [ #[] ] ifFalse: [ GtsCborWriter encode: { runType. runLength } ]) } ]
]

{ #category : #validating }
GtsV2CommitChecks >> checkIndex: event findingsDo: aBlock [
	| reasons |
	[ [ reasons := GtsV2IndexChecks check: event frame content bytes: event payload decoded
			head: head range: range runs: self expectedTypeRuns packs: packs graphs: graphs ]
		on: GtsV2FramingError do: [ :error | GtsV2EnvelopeError signal: 'wrong_length' ] ]
		on: GtsV2EnvelopeError do: [ :error |
			aBlock value: 'malformed_value (', error messageText, ')' value: position value: nil.
			^ self ].
	graphs isComparable ifFalse: [
		(uncomparedDirectories includes: 'graph directory') ifFalse: [ uncomparedDirectories add: 'graph directory' ] ].
	packs isComparable ifFalse: [
		(uncomparedDirectories includes: 'pack-member directory') ifFalse: [
			uncomparedDirectories add: 'pack-member directory' ] ].
	incompletePrefix ifTrue: [
		reasons := reasons reject: [ :reason | #('index_head' 'index_root' 'index_directory') includes: reason ] ].
	reasons do: [ :reason | aBlock value: 'index_claim_failed (', reason, ')' value: position value: nil ]
]

{ #category : #validating }
GtsV2CommitChecks >> check: content link: link findingsDo: aBlock [
	| fields expected countMatches root rootMatches |
	fields := content fields: 2.
	expected := range leafCount - 1.
	countMatches := fields next uint = expected.
	root := fields next.
	rootMatches := incompletePrefix not and: [ (root bytesStartingAt: (root byteBodyOfSize: 32)) = range root ].
	countMatches ifFalse: [ aBlock value: 'commit_count_mismatch' value: position value: expected ].
	rootMatches ifFalse: [ aBlock value: 'commit_root_mismatch' value: position value: nil ].
	(countMatches and: [ rootMatches and: [ #(linked healed) includes: link ] ]) ifFalse: [ ^ false ].
	committedFrames := range leafCount.
	tailFrames := 0.
	tailPosition := nil.
	^ true
]
