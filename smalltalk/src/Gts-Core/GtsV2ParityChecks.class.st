"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ParityChecks,
	#superclass : #Object,
	#instVars : [ 'header', 'registry', 'streamEnd', 'lastStart', 'seenStarts', 'seenShort', 'foldStopped' ],
	#category : #'Gts-Core'
}

{ #category : #observing }
GtsV2ParityChecks >> observe: event predecessor: predecessor [
	event startsSegment ifTrue: [
		header := event effectiveIdentifier ifNotNil: [ :digest | digest copy ].
		registry := event opener ifNotNil: [ :opener | opener registry ].
		streamEnd := 0.
		seenStarts := Set new.
		seenShort := false.
		foldStopped := false ].
	header ifNil: [ ^ self ].
	(registry > 1 and: [ event frame notNil and: [ event frame hasUnknownCriticalType ] ])
		ifTrue: [ foldStopped := true ].
	event frameType = 16 ifTrue: [ self check: event predecessor: predecessor ].
	(self includesInStream: event) ifTrue: [
		lastStart := streamEnd.
		streamEnd := streamEnd + event end - event offset ]
]

{ #category : #testing }
GtsV2ParityChecks >> includesInStream: event [
	event kind = #frame ifTrue: [ ^ event frame isTransparent not ].
	event kind = #framingGap ifTrue: [ ^ event problem = 'undelimitable_input' ].
	^ event startsSegment or: [ event kind = #opaque ]
]

{ #category : #validation }
GtsV2ParityChecks >> check: event predecessor: predecessor [
	| content geometry first end size anchor table repeated payload |
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
	(payload isNil or: [ (self holdsTable: table payload: payload spanSize: size) not ])
		ifTrue: [ ^ event markParityDamaged ].
	(end <= streamEnd and: [ lastStart < end and: [ size = 4194304 or: [ end = streamEnd ] ] ])
		ifFalse: [ event parityFinding: 'parity_stale' ]
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
GtsV2ParityChecks >> holdsTable: table payload: bytes spanSize: size [
	| shardSize dataCount parityCount walker limit fields data parity computed |
	shardSize := 64.
	[ shardSize < 4096 and: [ shardSize * 64 < size ] ] whileTrue: [ shardSize := shardSize * 2 ].
	dataCount := (size + shardSize - 1) // shardSize.
	parityCount := ((dataCount + 63) // 64) * 4.
	walker := GtsV2CborWalker on: bytes.
	limit := bytes size + 1.
	[
		(walker itemEndAt: 1 limit: limit) = limit ifFalse: [ ^ false ].
		fields := (GtsV2Field walker: walker from: 1 to: limit) fields: 3.
		data := fields next.
		parity := fields next.
		self checkDigestList: data count: dataCount.
		self checkDigestList: parity count: parityCount.
		fields next byteBodyOfSize: parityCount * shardSize.
	] on: GtsV2EnvelopeError, GtsV2FramingError do: [ :error | ^ false ].
	computed := GtsBlake3 deriveWith: 'gts2 parity table' prefix: #[131 88 32], header
		bytes: bytes from: data start to: parity end.
	^ computed = table
]

{ #category : #validation }
GtsV2ParityChecks >> checkDigestList: list count: count [
	| fields |
	fields := list fields: count.
	count timesRepeat: [ fields next byteBodyOfSize: 16 ]
]
