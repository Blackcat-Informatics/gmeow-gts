"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2Authentication,
	#superclass : #Object,
	#instVars : [ 'trusted', 'position', 'runFirst', 'holds', 'commitValid', 'frameType',
		'counted', 'linkedCount', 'runCount', 'authenticatedRunCount', 'authenticatedCount' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2Authentication class >> trusted: keys [
	^ self new initializeTrusted: keys
]

{ #category : #initialization }
GtsV2Authentication >> initializeTrusted: keys [
	trusted := keys asSet.
	position := -1.
	self beginRunAt: 0.
	counted := 0.
	linkedCount := 0.
	authenticatedCount := 0.
	^ self
]

{ #category : #private }
GtsV2Authentication >> beginRunAt: start [
	runFirst := start.
	runCount := 0.
	authenticatedRunCount := 0
]

{ #category : #observing }
GtsV2Authentication >> observeDetachedFrame [
	self beginRunAt: position + 1
]

{ #category : #observing }
GtsV2Authentication >> observe: event validCommit: valid [
	(event kind = #frame and: [ event frame isTransparent not ]) ifFalse: [ ^ self ].
	event effectiveIdentifier ifNil: [ ^ self beginRunAt: position + 1 ].
	position := position + 1.
	frameType := event frameType.
	commitValid := valid.
	holds := #(linked healed) includes: event link.
	frameType = 15 ifFalse: [
		counted := counted + 1.
		holds ifTrue: [
			linkedCount := linkedCount + 1.
			runCount := runCount + 1 ] ].
	holds ifFalse: [ self beginRunAt: position + 1 ]
]

{ #category : #authentication }
GtsV2Authentication >> coverageFor: status keyId: kid [
	| covered authenticating |
	covered := status = 'valid' and: [ holds ].
	authenticating := covered and: [ (trusted includes: kid) and: [ frameType ~= 14 or: [ commitValid ] ] ].
	authenticating ifTrue: [
		authenticatedCount := frameType = 14
			ifTrue: [ linkedCount ]
			ifFalse: [ authenticatedCount + runCount - authenticatedRunCount ].
		authenticatedRunCount := runCount ].
	^ Dictionary newFromPairs: {
		'coverage_first'. (covered ifTrue: [ runFirst ] ifFalse: [ position ]).
		'coverage_end'. (covered ifTrue: [ position + 1 ] ifFalse: [ position ]).
		'authenticated'. authenticating }
]

{ #category : #authentication }
GtsV2Authentication >> summary [
	| authenticity |
	authenticity := authenticatedCount = 0
		ifTrue: [ 3 ]
		ifFalse: [ authenticatedCount = counted ifTrue: [ 1 ] ifFalse: [ 2 ] ].
	^ Dictionary newFromPairs: { 'authenticity'. authenticity. 'authenticated_frames'. authenticatedCount }
]
