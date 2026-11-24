"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ParityChecks,
	#superclass : #Object,
	#instVars : [ 'bytes', 'stream', 'header', 'registry', 'seenStarts', 'seenShort', 'foldStopped', 'pendingOpenerParity' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2ParityChecks class >> on: aByteArray [
	^ self new initializeOn: aByteArray
]

{ #category : #initialization }
GtsV2ParityChecks >> initializeOn: aByteArray [
	bytes := aByteArray.
	^ self
]

{ #category : #observing }
GtsV2ParityChecks >> observe: event predecessor: predecessor [
	event startsSegment ifTrue: [
		self finishOpenerLink: false.
		header := event effectiveIdentifier ifNotNil: [ :digest | digest copy ].
		registry := event opener ifNotNil: [ :opener | opener registry ].
		stream := GtsV2ParityStream on: bytes.
		seenStarts := Set new.
		seenShort := false.
		foldStopped := false ].
	header ifNil: [ ^ self ].
	(event kind = #frame and: [ event frame isTransparent not ]) ifTrue: [
		self finishOpenerLink: (#(linked healed) includes: event link) ].
	(registry > 1 and: [ event frame notNil and: [ event frame hasUnknownCriticalType ] ])
		ifTrue: [ foldStopped := true ].
	event frameType = 16 ifTrue: [ self check: event predecessor: predecessor ].
	stream observe: event
]

{ #category : #testing }
GtsV2ParityChecks >> awaitsOpenerLink [
	^ pendingOpenerParity notNil
]

{ #category : #observing }
GtsV2ParityChecks >> finishOpenerLink: passes [
	| event unexplained |
	pendingOpenerParity ifNil: [ ^ self ].
	event := pendingOpenerParity first.
	unexplained := pendingOpenerParity third + (passes ifTrue: [ pendingOpenerParity fourth ] ifFalse: [ 0 ]).
	self recordUnexplained: unexplained of: pendingOpenerParity second for: event.
	pendingOpenerParity := nil
]

{ #category : #validation }
GtsV2ParityChecks >> check: event predecessor: predecessor [
	| content geometry first end size anchor table repeated payload shardTable |
	event link = #broken ifTrue: [ ^ event markParityDamaged ].
	content := self contentOf: event frame.
	content ifNil: [ ^ event markParityDamaged ].
	geometry := content first.
	(geometry > 1 and: [ registry > 1 and: [ event problem isNil ] ]) ifTrue: [
		(foldStopped not and: [ event payload state = #verified ])
			ifTrue: [ event parityFinding: 'unknown_frame_type' ].
		^ self ].
	first := content second.
	end := content third.
	size := end - first.
	anchor := content fourth.
	table := content fifth.
	repeated := seenShort or: [ seenStarts includes: first ].
	seenShort ifFalse: [ seenStarts add: first ].
	(size between: 1 and: 4194303) ifTrue: [
		seenShort := true.
		seenStarts removeAll ].
	(geometry ~= 1 or: [ first \\ 4194304 ~= 0 or: [ (size between: 1 and: 4194304) not or: [
		repeated or: [ event problem notNil or: [ event link ~= #linked or: [ anchor ~= predecessor ] ] ] ] ] ])
		ifTrue: [ ^ event markParityDamaged ].
	event payload state = #unchecked ifTrue: [ ^ self ].
	event payload state = #damaged ifTrue: [ event markParityDamaged ].
	payload := event frame payload plainStoredBytesUpTo: 280651.
	shardTable := payload ifNotNil: [ self checkedTable: table payload: payload spanSize: size ].
	shardTable ifNil: [ ^ event markParityDamaged ].
	(stream placesSpanFrom: first to: end) ifFalse: [ ^ event parityFinding: 'parity_stale' ].
	self checkShardsFor: event table: shardTable payload: payload from: first to: end
]

{ #category : #private }
GtsV2ParityChecks >> contentOf: frame [
	[
		| fields |
		fields := frame content fields: 5.
		^ { fields next uint. fields next uint. fields next uint.
			self readDigest: fields next. self readDigest: fields next }
	] on: GtsV2EnvelopeError do: [ :error | ^ nil ]
]

{ #category : #private }
GtsV2ParityChecks >> readDigest: field [
	^ field bytesStartingAt: (field byteBodyOfSize: 32)
]

{ #category : #validation }
GtsV2ParityChecks >> checkedTable: table payload: bytes spanSize: size [
	| shardSize dataCount parityCount walker limit fields data parity computed dataDigests parityDigests body |
	shardSize := 64.
	[ shardSize < 4096 and: [ shardSize * 64 < size ] ] whileTrue: [ shardSize := shardSize * 2 ].
	dataCount := (size + shardSize - 1) // shardSize.
	parityCount := ((dataCount + 63) // 64) * 4.
	walker := GtsV2CborWalker on: bytes.
	limit := bytes size + 1.
	[
		(walker itemEndAt: 1 limit: limit) = limit ifFalse: [ ^ nil ].
		fields := (GtsV2Field walker: walker from: 1 to: limit) fields: 3.
		data := fields next.
		parity := fields next.
		dataDigests := self readDigestList: data count: dataCount.
		parityDigests := self readDigestList: parity count: parityCount.
		body := fields next byteBodyOfSize: parityCount * shardSize.
	] on: GtsV2EnvelopeError, GtsV2FramingError do: [ :error | ^ nil ].
	computed := GtsBlake3 deriveWith: 'gts2 parity table' prefix: #[131 88 32], header
		bytes: bytes from: data start to: parity end.
	^ computed = table ifTrue: [ { shardSize. dataDigests. parityDigests. body } ]
]

{ #category : #validation }
GtsV2ParityChecks >> readDigestList: list count: count [
	| fields |
	fields := list fields: count.
	^ (1 to: count) collect: [ :index |
		| digest |
		digest := fields next.
		digest bytesStartingAt: (digest byteBodyOfSize: 16) ]
]

{ #category : #validation }
GtsV2ParityChecks >> checkShardsFor: event table: table payload: payload from: first to: end [
	| shardSize dataCounts dataFailed unexplained parityFailed offset findings |
	shardSize := table first.
	dataCounts := stream compareFrom: first to: end shardSize: shardSize digests: table second.
	dataFailed := dataCounts first.
	unexplained := dataCounts second.
	offset := table fourth.
	parityFailed := 0.
	table third do: [ :digest |
		| computed |
		computed := GtsBlake3 deriveWith: 'gts2 shard' prefix: #[] bytes: payload from: offset to: offset + shardSize.
		(computed first: 16) = digest ifFalse: [ parityFailed := parityFailed + 1 ].
		offset := offset + shardSize ].
	findings := OrderedCollection new.
	dataFailed > 0 ifTrue: [ findings add: { 'parity_mismatch'. 'data'. dataFailed } ].
	parityFailed > 0 ifTrue: [ findings add: { 'parity_mismatch'. 'parity'. parityFailed } ].
	event parityShardFindings: findings.
	dataCounts third > 0
		ifTrue: [ pendingOpenerParity := { event. dataFailed. unexplained. dataCounts third } ]
		ifFalse: [ self recordUnexplained: unexplained of: dataFailed for: event ]
]

{ #category : #validation }
GtsV2ParityChecks >> recordUnexplained: count of: dataFailed for: event [
	count > 0 ifTrue: [
		count = dataFailed
			ifTrue: [ event parityFinding: 'parity_stale' ]
			ifFalse: [ event parityShardFindings add: { 'parity_unexplained'. nil. count } ] ]
]
