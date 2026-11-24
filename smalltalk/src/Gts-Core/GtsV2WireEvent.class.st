"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2WireEvent,
	#superclass : #Object,
	#instVars : [ 'kind', 'offset', 'end', 'problem', 'opener', 'frame', 'link', 'effectiveIdentifier', 'storedMatches', 'payload', 'closingClaimFailure', 'scanned', 'framingFailure', 'parityDamaged', 'parityFinding', 'parityShardFindings', 'resumptionAnchor', 'parityGap', 'detached' ],
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

{ #category : #copying }
GtsV2WireEvent >> postCopy [
	super postCopy.
	opener ifNotNil: [
		opener := opener copy.
		effectiveIdentifier := opener identifier ]
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
GtsV2WireEvent >> closingProblem: reason [
	problem := reason
]

{ #category : #accessing }
GtsV2WireEvent >> closingClaimFailure [
	^ closingClaimFailure
]

{ #category : #accessing }
GtsV2WireEvent >> closingClaimFailure: reason [
	closingClaimFailure := reason
]

{ #category : #accessing }
GtsV2WireEvent >> opener [
	^ opener
]

{ #category : #accessing }
GtsV2WireEvent >> frameType [
	^ frame ifNotNil: [ frame frameType ]
]

{ #category : #accessing }
GtsV2WireEvent >> frame [
	^ frame
]

{ #category : #accessing }
GtsV2WireEvent >> link [
	^ link
]

{ #category : #accessing }
GtsV2WireEvent >> effectiveIdentifier [
	^ effectiveIdentifier
]

{ #category : #accessing }
GtsV2WireEvent >> storedMatches [
	^ storedMatches
]

{ #category : #accessing }
GtsV2WireEvent >> payload [
	^ payload
]

{ #category : #accessing }
GtsV2WireEvent >> scanned [
	^ scanned = true
]

{ #category : #testing }
GtsV2WireEvent >> failsFraming [
	^ framingFailure = true
]

{ #category : #initialization }
GtsV2WireEvent >> framingFailure: failed [
	framingFailure := failed
]

{ #category : #initialization }
GtsV2WireEvent >> markScanned [
	scanned := true
]

{ #category : #initialization }
GtsV2WireEvent >> resyncRefusal: result [
	payload := result
]

{ #category : #accessing }
GtsV2WireEvent >> resumptionAnchor [
	^ resumptionAnchor
]

{ #category : #testing }
GtsV2WireEvent >> isParityGap [
	^ parityGap = true
]

{ #category : #initialization }
GtsV2WireEvent >> markParityGap [
	parityGap := true
]

{ #category : #testing }
GtsV2WireEvent >> detached [
	^ detached = true
]

{ #category : #initialization }
GtsV2WireEvent >> markDetached [
	detached := true
]

{ #category : #initialization }
GtsV2WireEvent >> resumeFrom: anchor checkedPayload: result [
	resumptionAnchor := anchor.
	payload := result.
	link := #linked
]

{ #category : #validation }
GtsV2WireEvent >> checkPayloadWith: budget retainDecoded: retain [
	| raw |
	frame ifNil: [ ^ self ].
	raw := frame payload.
	storedMatches := raw storedMatches.
	payload := raw checkWith: budget retainDecoded: retain.
	payload reason = 'codec_constraint' ifTrue: [
		problem
			ifNil: [ problem := payload reason ]
			ifNotNil: [ payload := GtsV2PayloadResult state: #damaged reason: nil count: nil ] ]
]

{ #category : #testing }
GtsV2WireEvent >> isDamagedParity [
	^ parityDamaged = true
]

{ #category : #initialization }
GtsV2WireEvent >> markParityDamaged [
	parityDamaged := true
]

{ #category : #accessing }
GtsV2WireEvent >> parityFinding [
	^ parityFinding
]

{ #category : #initialization }
GtsV2WireEvent >> parityFinding: aFinding [
	parityFinding := aFinding
]

{ #category : #accessing }
GtsV2WireEvent >> parityShardFindings [
	^ parityShardFindings ifNil: [ #() ]
]

{ #category : #initialization }
GtsV2WireEvent >> parityShardFindings: findings [
	parityShardFindings := findings
]

{ #category : #initialization }
GtsV2WireEvent >> opener: anOpener frame: aFrame [
	opener := anOpener.
	frame := aFrame.
	effectiveIdentifier := opener ifNotNil: [ opener identifier ].
	^ self
]

{ #category : #initialization }
GtsV2WireEvent >> link: state effectiveIdentifier: identifier [
	link := state.
	effectiveIdentifier := identifier
]

{ #category : #testing }
GtsV2WireEvent >> startsSegment [
	^ kind = #header or: [ kind = #resync ]
]
