"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ClosingChecks,
	#superclass : #Object,
	#instVars : [ 'walker', 'segmentStart', 'indexOffset', 'offsetsSeen', 'locatorSeen', 'damagedRegions' ],
	#category : #'Gts-Core'
}

{ #category : #testing }
GtsV2ClosingChecks class >> framingFailsFor: kind item: item [
	[
		| content entries |
		content := self contentOf: item.
		kind = #locator
			ifTrue: [ content locatorDistance ]
			ifFalse: [
				entries := content list.
				[ entries hasNext ] whileTrue: [ entries next uint ] ]
	] on: GtsV2EnvelopeError do: [ :error | ^ true ].
	^ false
]

{ #category : #private }
GtsV2ClosingChecks class >> contentOf: item [
	| fields |
	fields := item fields: 2.
	fields next.
	^ fields next
]

{ #category : #'instance creation' }
GtsV2ClosingChecks class >> on: walker [
	^ self new initializeOn: walker
]

{ #category : #initialization }
GtsV2ClosingChecks >> initializeOn: aWalker [
	walker := aWalker.
	^ self
]

{ #category : #validation }
GtsV2ClosingChecks >> observe: event following: next afterLocator: afterLocator [
	event startsSegment ifTrue: [
		segmentStart := event offset + 1.
		indexOffset := nil.
		offsetsSeen := false.
		locatorSeen := false.
		damagedRegions := Dictionary new.
		^ self ].
	(segmentStart notNil and: [ event kind = #framingGap and: [ event problem = 'undelimitable_input' ] ]) ifTrue: [
		damagedRegions at: event offset + 1 put: event end + 1 ].
	event frameType = 15 ifTrue: [ indexOffset := event offset ].
	event kind = #offsets ifTrue: [
		^ self checkOffsets: event following: next afterLocator: afterLocator ].
	event kind = #locator ifTrue: [ self checkLocator: event following: next ]
]

{ #category : #validation }
GtsV2ClosingChecks >> checkOffsets: event following: next afterLocator: afterLocator [
	| misplaced |
	misplaced := offsetsSeen or: [ locatorSeen or: [
		(self endsSegment: next) not and: [
			next kind ~= #locator or: [ (self endsSegment: afterLocator) not ] ] ] ].
	offsetsSeen := true.
	misplaced ifTrue: [ ^ event closingProblem: 'unordered_or_repeated' ].
	self check: event reason: 'index_offsets' if: [ :field |
		self lengths: field matchPrefixEndingAt: event offset + 1 ]
]

{ #category : #validation }
GtsV2ClosingChecks >> checkLocator: event following: next [
	| misplaced |
	misplaced := locatorSeen or: [ (self endsSegment: next) not ].
	locatorSeen := true.
	misplaced ifTrue: [ ^ event closingProblem: 'unordered_or_repeated' ].
	self check: event reason: 'index_locator' if: [ :field |
		| distance |
		distance := field locatorDistance.
		indexOffset isNotNil and: [ distance = (event offset - indexOffset) ] ]
]

{ #category : #private }
GtsV2ClosingChecks >> endsSegment: event [
	^ event isNil or: [ event startsSegment or: [
		(#(stop resyncRefused parityRefused) includes: event kind) or: [
			event kind = #framingGap and: [ event problem = 'torn_tail' ] ] ] ]
]

{ #category : #private }
GtsV2ClosingChecks >> check: event reason: reason if: aBlock [
	[
		| item |
		item := GtsV2Field walker: walker from: event offset + 1 to: event end + 1.
		(aBlock value: (self class contentOf: item)) ifFalse: [ event closingClaimFailure: reason ]
	] on: GtsV2EnvelopeError do: [ :error | event closingProblem: error messageText ]
]

{ #category : #private }
GtsV2ClosingChecks >> lengths: field matchPrefixEndingAt: limit [
	| lengths observed matches |
	lengths := field list.
	observed := segmentStart.
	matches := true.
	[ lengths hasNext ] whileTrue: [
		| claimed |
		claimed := lengths next uint.
		observed = limit
			ifTrue: [ matches := false ]
			ifFalse: [
				| finish |
				finish := damagedRegions at: observed ifAbsent: [ walker itemEndAt: observed limit: limit ].
				claimed = (finish - observed) ifFalse: [ matches := false ].
				observed := finish ] ].
	^ matches and: [ observed = limit ]
]
