"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2WireFindings,
	#superclass : #Object,
	#instVars : [ 'fileFindings', 'segmentFindings', 'frameFindings', 'segment', 'positions', 'position', 'currentItem', 'previousItem', 'inputLength', 'itemLimit', 'fileEnd', 'leftOut', 'head', 'lineageOrdinal', 'newerRegistry', 'claimChecks', 'claimSteps', 'committedCounts', 'inputs', 'signatureChecks', 'signatureResults', 'authentication', 'segmentAuthentication', 'headerIdentifier', 'segmentFrameResults', 'frames', 'anchoredCount', 'anchorLost', 'transparentPositions', 'openers' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2WireFindings class >> on: bytes budget: budget held: held trusted: trusted [
	^ self new initializeOn: bytes budget: budget held: held trusted: trusted
]

{ #category : #initialization }
GtsV2WireFindings >> initializeOn: bytes budget: budget held: held trusted: trusted [
	inputLength := bytes size.
	itemLimit := budget ifNil: [ 0 ] ifNotNil: [ budget itemLimit ].
	openers := GtsV2OpenerResults on: bytes limit: (budget ifNil: [ 0 ] ifNotNil: [ budget readLimit ]).
	fileFindings := OrderedCollection new.
	segmentFindings := OrderedCollection new.
	frameFindings := Dictionary new.
	positions := Dictionary new.
	claimSteps := IdentityDictionary new.
	committedCounts := OrderedCollection new.
	inputs := {
		self orderedSet: (trusted collect: [ :kid | kid copy ]).
		self orderedSet: (held collect: [ :entry | { entry first copy. entry second. entry third copy } ]) }.
	signatureChecks := GtsV2SignatureChecks held: (inputs second collect: [ :entry |
		{ GtsHex encode: entry first. entry second. entry third } ]).
	signatureResults := Dictionary new.
	segmentAuthentication := OrderedCollection new.
	segmentFrameResults := OrderedCollection new.
	fileEnd := 4.
	leftOut := 0.
	^ self
]

{ #category : #accessing }
GtsV2WireFindings >> verificationInputs [
	^ inputs
]

{ #category : #accessing }
GtsV2WireFindings >> signaturesAt: offset [
	^ signatureResults at: offset ifAbsent: [ #() ]
]

{ #category : #accessing }
GtsV2WireFindings >> authenticationAt: ordinal [
	"The authenticity code and the number of authenticated counted frames."
	^ segmentAuthentication at: ordinal + 1
]

{ #category : #private }
GtsV2WireFindings >> orderedSet: values [
	| keyed |
	keyed := values asSet asArray collect: [ :value | { GtsCborWriter encode: value. value } ].
	keyed sort: [ :left :right | (GtsCose compareBytes: left first with: right first) <= 0 ].
	^ keyed collect: [ :entry | entry second ]
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
	^ self ordered: (frameFindings at: offset ifAbsent: [ #() ]) parity: (positions includesKey: offset)
]

{ #category : #accessing }
GtsV2WireFindings >> committedFramesAt: ordinal [
	^ committedCounts at: ordinal + 1
]

{ #category : #accessing }
GtsV2WireFindings >> frameWireFieldsAt: ordinal [
	"The first seven fields of each P7 frame result, in item order."
	^ (segmentFrameResults at: ordinal + 1) first
]

{ #category : #accessing }
GtsV2WireFindings >> anchoredFramesAt: ordinal [
	^ (segmentFrameResults at: ordinal + 1) second
]

{ #category : #accessing }
GtsV2WireFindings >> transparentFramesAt: ordinal [
	^ (segmentFrameResults at: ordinal + 1) third
]

{ #category : #accessing }
GtsV2WireFindings >> openerFieldsAt: ordinal [
	^ openers at: ordinal
]

{ #category : #observing }
GtsV2WireFindings >> finishOpeners [
	openers finish
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
	| validCommit |
	event startsSegment ifTrue: [
		self finishSegment.
		position := 0.
		segment := OrderedCollection new.
		segmentFindings add: segment.
		frames := OrderedCollection new.
		anchoredCount := 0.
		anchorLost := false.
		transparentPositions := OrderedCollection new.
		headerIdentifier := event effectiveIdentifier.
		claimChecks := headerIdentifier ifNotNil: [
			GtsV2CommitChecks openedBy: headerIdentifier checkingIndexes: (GtsV2CommitChecks supportsOpener: event) ].
		authentication := headerIdentifier ifNotNil: [ GtsV2Authentication trusted: inputs first ].
		self remember: event.
		^ self recordOpener: event ].
	event kind = #stop ifTrue: [ ^ self framingStoppedAt: event offset reason: event problem ].
	(#(parityRefused resyncRefused) includes: event kind) ifTrue: [ ^ self recordRefusal: event ].
	event kind = #framingGap ifTrue: [
		self recordGapAt: event offset to: event end reason: event problem.
		event problem = 'torn_tail' ifTrue: [ ^ self ] ].
	segment ifNil: [ ^ self ].
	position := position + 1.
	self remember: event.
	event frame
		ifNil: [ self recordNonFrame: event ]
		ifNotNil: [ self recordFrame: event ].
	validCommit := claimChecks ifNil: [ false ] ifNotNil: [ self checkClaimsOf: event ].
	authentication ifNotNil: [ self checkSignaturesOf: event validCommit: validCommit ]
]

{ #category : #observing }
GtsV2WireFindings >> finishSegment [
	segment ifNil: [ ^ self ].
	segmentFrameResults add: { frames asArray. anchoredCount. transparentPositions asArray }.
	committedCounts add: (claimChecks ifNil: [ 0 ] ifNotNil: [ claimChecks committedFrames ]).
	segmentAuthentication add: (authentication ifNil: [ #(3 0) ] ifNotNil: [ authentication summaryFields ]).
	claimChecks ifNotNil: [
		claimChecks tailOffset ifNotNil: [ :offset |
			(frameFindings at: offset ifAbsentPut: [ OrderedCollection new ]) add:
				(self positionalCode: 17 reason: nil position: claimChecks tailPosition count: claimChecks tailFrames) ] ].
	segment := nil
]

{ #category : #private }
GtsV2WireFindings >> checkClaimsOf: event [
	(event kind = #framingGap and: [ event problem = 'undelimitable_input' ]) ifTrue: [
		claimChecks observeDamagedRegion.
		^ false ].
	^ claimChecks observe: event classifiedFindingsDo: [ :code :reason :at :count |
		| record |
		record := code caseOf: {
			[ #commit_count_mismatch ] -> [ self positionalCode: 15 reason: nil position: at count: count ].
			[ #commit_root_mismatch ] -> [ self positionalCode: 16 reason: nil position: at count: nil ].
			[ #index_claim_failed ] -> [
				| reasonCode |
				reasonCode := #('index_head' 'index_root' 'index_offsets' 'index_digest'
					'index_members' 'index_directory' 'index_locator') indexOf: reason.
				self positionalCode: 18 reason: reasonCode + 16 position: at count: nil ].
			[ #malformed_value ] -> [
				self rangedCode: 7 reason: (self malformedReason: reason) event: event count: nil ].
			[ #damaged_frame ] -> [
				self rangedCode: 13 reason: 12 event: event count: nil ] }.
		code = #malformed_value ifTrue: [ claimSteps at: record put: 11 ].
		code = #damaged_frame ifTrue: [ claimSteps at: record put: 14 ].
		code = #index_claim_failed ifTrue: [ claimSteps at: record put: 19 ].
		(frameFindings at: event offset ifAbsentPut: [ OrderedCollection new ]) add: record ]
]

{ #category : #private }
GtsV2WireFindings >> checkSignaturesOf: event validCommit: validCommit [
	| results |
	authentication observe: event validCommit: validCommit.
	signatureChecks event: event header: headerIdentifier resultsDo: [ :algorithm :kid :status :reason |
		| statusCode reasonCode coverage |
		statusCode := #('valid' 'invalid' 'unverified') indexOf: status.
		reasonCode := reason ifNotNil: [ self signatureReason: reason ].
		coverage := authentication coverageFieldsFor: status keyId: kid.
		results ifNil: [ results := OrderedCollection new ].
		results add: { statusCode. self option: reasonCode. algorithm. kid }, coverage.
		reasonCode ifNotNil: [
			(frameFindings at: event offset ifAbsentPut: [ OrderedCollection new ]) add:
				{ statusCode = 2 ifTrue: [ 40 ] ifFalse: [ 41 ].
					{ 2. reasonCode }. { 2. { 3. algorithm } }. { 2. position }. #(1). #(1). #(1) } ] ].
	results ifNotNil: [ signatureResults at: event offset put: results asArray ]
]

{ #category : #private }
GtsV2WireFindings >> signatureReason: reason [
	| index |
	index := #('unknown_signature_algorithm' 'key_not_held' 'key_mismatch' 'value_length'
		'non_canonical_scalar' 'small_order_key' 'small_order_nonce' 'equation_failed'
		'composite_classical_failed' 'composite_post_quantum_failed') indexOf: reason.
	index = 0 ifTrue: [ GtsV2EnvelopeError signal: 'unknown signature outcome' ].
	^ index <= 2 ifTrue: [ index + 52 ] ifFalse: [ index + 54 ]
]

{ #category : #private }
GtsV2WireFindings >> recordOpener: event [
	| previousHead previousOrdinal lineage |
	openers observe: event.
	previousHead := head.
	previousOrdinal := lineageOrdinal.
	head := event effectiveIdentifier.
	lineageOrdinal := nil.
	newerRegistry := false.
	event opener ifNil: [ self recordProblem: event into: segment. ^ self ].
	newerRegistry := event opener registry > 1.
	newerRegistry ifTrue: [
		segment add: (self positionalCode: 4 reason: nil position: nil count: nil) ].
	head ifNotNil: [
		event opener profile ifNotNil: [ :profile |
			segment add: { 48. #(1). { 2. { 4. profile. #(1) } }. #(1). #(1). #(1). #(1) } ] ].
	lineage := event opener lineage.
	lineage ifNil: [
		segmentFindings size > 1 ifTrue: [
			segment add: (self positionalCode: 42 reason: nil position: nil count: nil) ].
		^ self ].
	head ifNil: [ ^ self ].
	lineageOrdinal := lineage second.
	(previousHead notNil and: [ lineage first ~= previousHead ]) ifTrue: [
		segment add: (self positionalCode: 43 reason: 55 position: nil count: nil) ].
	(segmentFindings size = 1 or: [ previousOrdinal notNil and: [ lineageOrdinal ~= (previousOrdinal + 1) ] ]) ifTrue: [
		segment add: (self positionalCode: 43 reason: 56 position: nil count: nil) ]
]

{ #category : #private }
GtsV2WireFindings >> recordNonFrame: event [
	event unknownItemKind
		ifNil: [ self recordProblem: event into: segment ]
		ifNotNil: [ :kind | segment add: (self unknown: 84 subject: { 5. kind }) ].
	event closingClaimFailure ifNotNil: [ :reason |
		segment add: (self positionalCode: 18 reason: (reason = 'index_offsets' ifTrue: [ 19 ] ifFalse: [ 23 ])
			position: position count: nil) ]
]

{ #category : #private }
GtsV2WireFindings >> recordFrame: event [
	| records |
	self recordFrameWireFields: event.
	event effectiveIdentifier ifNotNil: [ :identifier | head := identifier ].
	records := OrderedCollection new.
	event frameType = 16
		ifTrue: [ self recordParity: event into: records ]
		ifFalse: [
			self recordProblem: event into: records.
			self recordUnknownFrame: event into: records.
			event link = #healed ifTrue: [ records add: (self rangedCode: 12 reason: nil event: event count: nil) ].
			(event link = #broken and: [ event problem isNil ]) ifTrue: [
				records add: (self rangedCode: 11 reason: nil event: event count: nil) ].
			event storedMatches = false ifTrue: [
				records add: (self rangedCode: 14 reason: nil event: event count: nil) ].
			self recordPayload: event into: records ].
	records ifNotEmpty: [ frameFindings at: event offset put: records ]
]

{ #category : #private }
GtsV2WireFindings >> recordFrameWireFields: event [
	| linkCode storedCode |
	linkCode := #(linked healed broken) indexOf: event link.
	storedCode := event storedMatches ifNotNil: [ :matches | matches ifTrue: [ 1 ] ifFalse: [ 2 ] ].
	frames add: { position. event offset. event end - event offset.
		self option: event frameType. linkCode. self typedPayload: event payload. self option: storedCode }.
	event frame isTransparent ifTrue: [
		transparentPositions add: position.
		^ self ].
	event effectiveIdentifier ifNil: [ ^ self ].
	(anchorLost or: [ event detached or: [ event link = #broken ] ])
		ifTrue: [ anchorLost := true ]
		ifFalse: [ anchoredCount := anchoredCount + 1 ]
]

{ #category : #private }
GtsV2WireFindings >> typedPayload: result [
	^ result state caseOf: {
		[ #verified ] -> [ #(1) ].
		[ #damaged ] -> [ #(2) ].
		[ #unchecked ] -> [ { 3. result reason = 'unknown_codec' ifTrue: [ 1 ] ifFalse: [ 2 ] } ] }
]

{ #category : #private }
GtsV2WireFindings >> recordUnknownFrame: event into: records [
	(newerRegistry and: [ event frame hasUnknownType ]) ifFalse: [ ^ self ].
	event frame hasUnknownCriticalType
		ifTrue: [ records add: { 6. #(1). { 2. { 2. event frameType } }. { 2. position }. #(1). #(1). #(1) } ]
		ifFalse: [ records add: (self unknown: 2 subject: { 2. event frameType }) ]
]

{ #category : #private }
GtsV2WireFindings >> recordProblem: event into: records [
	| reason |
	event problem ifNil: [ ^ self ].
	reason := self malformedReason: event problem.
	reason ifNotNil: [ records add: (self rangedCode: 7 reason: reason event: event count: nil) ]
]

{ #category : #private }
GtsV2WireFindings >> malformedReason: problem [
	| index |
	index := #('not_deterministic' 'wrong_type' 'wrong_length' 'out_of_range' 'unregistered_value'
		'unordered_or_repeated' 'inconsistent_counts' 'forbidden_combination' 'codec_constraint')
		indexOf: problem.
	^ index > 0 ifTrue: [ index + 2 ]
]

{ #category : #private }
GtsV2WireFindings >> recordPayload: event into: records [
	| result reason index |
	result := event payload.
	reason := result reason.
	reason ifNil: [ ^ self ].
	reason = 'unknown_codec' ifTrue: [
		records add: (self unknown: 1 subject: { 1. event frame payload codecCode }).
		^ self ].
	index := #('payload_digest' 'decoded_length' 'decode_failed') indexOf: reason.
	index > 0 ifTrue: [
		records add: (self rangedCode: 13 reason: (#(12 13 16) at: index) event: event count: nil).
		^ self ].
	index := #('decoded_size' 'stored_size' 'total_decoded') indexOf: reason.
	index > 0 ifTrue: [
		records add: (self rangedCode: 20 reason: (#(30 31 86) at: index) event: event count: result count) ]
]

{ #category : #private }
GtsV2WireFindings >> unknown: reason subject: subject [
	^ { 5. { 2. reason }. { 2. subject }. { 2. position }. #(1). #(1). #(1) }
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
GtsV2WireFindings >> recordParity: event into: records [
	positions at: event offset put: position.
	event isDamagedParity
		ifTrue: [ records add: (self positionalCode: 30 reason: nil position: position count: nil) ]
		ifFalse: [
			self recordPayload: event into: records.
			event parityFinding = 'unknown_frame_type' ifTrue: [
				records add: (self unknown: 2 subject: #(2 16)) ] ].
	event parityShardFindings do: [ :finding |
		| mismatch reason |
		mismatch := finding first = 'parity_mismatch'.
		reason := mismatch ifTrue: [ finding second = 'data' ifTrue: [ 40 ] ifFalse: [ 41 ] ].
		records add: (self positionalCode: (mismatch ifTrue: [ 31 ] ifFalse: [ 32 ])
			reason: reason position: position count: finding third) ].
	event parityFinding = 'parity_stale' ifTrue: [
		records add: (self positionalCode: 33 reason: nil position: position count: nil) ]
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
GtsV2WireFindings >> rangedCode: code reason: reason event: event count: count [
	^ self code: code reason: reason position: position offset: event offset length: event end - event offset count: count
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
	^ self ordered: findings parity: false
]

{ #category : #private }
GtsV2WireFindings >> ordered: findings parity: parity [
	| keyed |
	keyed := findings asArray collect: [ :finding |
		| at |
		at := finding fourth.
		{ self stepOf: finding parity: parity. (at size = 1 ifTrue: [ -1 ] ifFalse: [ at second ]).
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
GtsV2WireFindings >> stepOf: finding parity: parity [
	| code |
	claimSteps at: finding ifPresent: [ :step | ^ step ].
	code := finding first.
	code <= 3 ifTrue: [ ^ 1 ].
	(#(8 9) includes: code) ifTrue: [ ^ 2 ].
	(#(4 42 43) includes: code) ifTrue: [ ^ 3 ].
	code = 7 ifTrue: [
		finding second second = 11 ifTrue: [ ^ 7 ].
		^ finding fourth second = 0 ifTrue: [ 3 ] ifFalse: [ 4 ] ].
	code = 5 ifTrue: [
		finding second second = 1 ifTrue: [ ^ 7 ].
		^ parity ifTrue: [ 11 ] ifFalse: [ 4 ] ].
	(#(6 18) includes: code) ifTrue: [ ^ 4 ].
	(#(11 12) includes: code) ifTrue: [ ^ 5 ].
	code = 14 ifTrue: [ ^ 6 ].
	code = 13 ifTrue: [ ^ 7 ].
	(code between: 15 and: 17) ifTrue: [ ^ 13 ].
	(#(40 41) includes: code) ifTrue: [ ^ 20 ].
	code = 48 ifTrue: [ ^ 21 ].
	code = 20 ifTrue: [ ^ finding second second = 87 ifTrue: [ 2 ] ifFalse: [ 7 ] ].
	code <= 33 ifTrue: [ ^ 8 ].
	code <= 35 ifTrue: [ ^ 9 ].
	^ 10
]
