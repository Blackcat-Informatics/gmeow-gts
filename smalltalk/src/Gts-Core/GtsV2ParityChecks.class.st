"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ParityChecks,
	#superclass : #Object,
	#instVars : [ 'bytes', 'stream', 'header', 'registry', 'seenStarts', 'seenShort', 'foldStopped', 'pendingOpenerParity', 'parityJudged', 'codec', 'repairing' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2ParityChecks class >> on: aByteArray repairing: plan [
	^ self new initializeOn: aByteArray repairing: plan
]

{ #category : #initialization }
GtsV2ParityChecks >> initializeOn: aByteArray repairing: plan [
	bytes := aByteArray.
	repairing := plan.
	^ self
]

{ #category : #observing }
GtsV2ParityChecks >> observe: event predecessor: predecessor [
	event startsSegment ifTrue: [
		header := event effectiveIdentifier ifNotNil: [ :digest | digest copy ].
		registry := event opener ifNotNil: [ :opener | opener registry ].
		stream := GtsV2ParityStream on: bytes.
		seenStarts := Set new.
		seenShort := false.
		parityJudged := false.
		foldStopped := false ].
	header ifNil: [ ^ self ].
	(registry > 1 and: [ event frame notNil and: [ event frame hasUnknownCriticalType ] ])
		ifTrue: [ foldStopped := true ].
	event frameType = 16 ifTrue: [ self check: event predecessor: predecessor ].
	stream observe: event.
	pendingOpenerParity ifNotNil: [
		(pendingOpenerParity at: 6) ifNil: [
			pendingOpenerParity at: 6 put: (stream inputOffsetAt: (pendingOpenerParity at: 5)) ].
		(event kind = #frame and: [ event frame isTransparent not ]) ifTrue: [
			self finishOpenerLink: (#(linked healed) includes: event link) ] ]
]

{ #category : #testing }
GtsV2ParityChecks >> awaitsOpenerLink [
	^ pendingOpenerParity notNil
]

{ #category : #observing }
GtsV2ParityChecks >> finishOpenerLink: passes [
	| event unexplained proposal |
	pendingOpenerParity ifNil: [ ^ self ].
	event := pendingOpenerParity first.
	unexplained := pendingOpenerParity third + (passes ifTrue: [ pendingOpenerParity fourth ] ifFalse: [ 0 ]).
	self recordUnexplained: unexplained of: pendingOpenerParity second for: event.
	event parityFinding = 'parity_stale' ifFalse: [
		stream protectThrough: (pendingOpenerParity at: 5) at: (pendingOpenerParity at: 6).
		self recordReconstructionFailure: (pendingOpenerParity at: 7) for: event.
		proposal := pendingOpenerParity at: 8.
		proposal ifNotNil: [
			proposal finishOpenerLink: passes.
			repairing addSpan: proposal ] ].
	pendingOpenerParity := nil
]

{ #category : #enumerating }
GtsV2ParityChecks >> unprotectedTailDo: report [
	parityJudged = true ifTrue: [ stream unprotectedTailDo: report ]
]

{ #category : #validation }
GtsV2ParityChecks >> check: event predecessor: predecessor [
	| content geometry first end size anchor table repeated payload shardTable |
	content := event link ~= #broken ifTrue: [ self contentOf: event frame ].
	(content notNil and: [ content first > 1 and: [ registry > 1 and: [ event problem isNil ] ] ]) ifTrue: [
		(foldStopped not and: [ event payload state = #verified ])
			ifTrue: [ event parityFinding: 'unknown_frame_type' ].
		^ self ].
	parityJudged := true.
	content ifNil: [ ^ event markParityDamaged ].
	geometry := content first.
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
	| shardSize dataCounts dataFailed unexplained parityFailed offset findings failure proposal capture |
	shardSize := table first.
	dataCounts := stream compareFrom: first to: end shardSize: shardSize digests: table second.
	dataFailed := dataCounts first size.
	unexplained := dataCounts second size.
	offset := table fourth.
	parityFailed := OrderedCollection new.
	table third withIndexDo: [ :digest :index |
		| computed |
		computed := GtsBlake3 deriveWith: 'gts2 shard' prefix: #[] bytes: payload from: offset to: offset + shardSize.
		(computed first: 16) = digest ifFalse: [ parityFailed add: index ].
		offset := offset + shardSize ].
	findings := OrderedCollection new.
	dataFailed > 0 ifTrue: [ findings add: { 'parity_mismatch'. 'data'. dataFailed } ].
	parityFailed isNotEmpty ifTrue: [ findings add: { 'parity_mismatch'. 'parity'. parityFailed size } ].
	event parityShardFindings: findings.
	(dataFailed > unexplained or: [ dataFailed = 0 and: [ parityFailed isNotEmpty ] ]) ifTrue: [
		repairing ifNotNil: [
			proposal := GtsV2ParityRepairSpan at: event offset.
			capture := self repairCaptureFor: proposal frame: event frame table: table
				from: first to: end mismatches: dataCounts ].
		failure := self reconstructionFailureFor: table payload: payload from: first to: end
			erasures: { dataCounts first. parityFailed } capture: capture.
		proposal ifNotNil: [ proposal failure: failure ] ].
	dataCounts third isNotEmpty
		ifTrue: [ pendingOpenerParity := { event. dataFailed. unexplained. dataCounts third size. end. nil. failure. proposal } ]
		ifFalse: [
			self recordUnexplained: unexplained of: dataFailed for: event.
			self protectThrough: end for: event.
			self recordReconstructionFailure: failure for: event.
			proposal ifNotNil: [ repairing addSpan: proposal ] ]
]

{ #category : #private }
GtsV2ParityChecks >> repairCaptureFor: proposal frame: frame table: table from: first to: end mismatches: dataCounts [
	| shardSize parityStart |
	shardSize := table first.
	parityStart := frame payload storedBodyStart + table fourth - 1.
	^ [ :kind :slot :shard |
		kind = #data
			ifTrue: [
				(dataCounts second includes: slot) ifFalse: [
					| start finish |
					start := first + ((slot - 1) * shardSize).
					finish := (start + shardSize) min: end.
					proposal addData: (stream patchesFor: shard from: start to: finish)
						conditional: (dataCounts third includes: slot) ] ]
			ifFalse: [ proposal addParity: shard at: parityStart + ((slot - 1) * shardSize) ] ]
]

{ #category : #validation }
GtsV2ParityChecks >> recordReconstructionFailure: reason for: event [
	reason ifNotNil: [ event parityShardFindings add: { 'repair_failed'. reason. nil } ]
]

{ #category : #validation }
GtsV2ParityChecks >> reconstructionFailureFor: table payload: payload from: first to: end erasures: erasures capture: capture [
	| dataMissing parityMissing dataDigests depth counts data shardSize valid |
	dataMissing := erasures first asSet.
	parityMissing := erasures second asSet.
	dataDigests := table second.
	depth := table third size // 4.
	counts := Array new: depth withAll: 0.
	dataMissing do: [ :column |
		| stripe |
		stripe := ((column - 1) \\ depth) + 1.
		counts at: stripe put: (counts at: stripe) + 1 ].
	parityMissing do: [ :row |
		| stripe |
		stripe := ((row - 1) // 4) + 1.
		counts at: stripe put: (counts at: stripe) + 1 ].
	(counts anySatisfy: [ :count | count > 4 ]) ifTrue: [ ^ 'too_many_erasures' ].
	shardSize := table first.
	data := Array new: dataDigests size.
	self streamShardsFrom: first to: end size: shardSize into: data ignoring: dataMissing.
	codec ifNil: [ codec := GtsV2ParityCodec new ].
	valid := true.
	counts withIndexDo: [ :count :stripe |
		count > 0 ifTrue: [
			(self checksStripe: stripe depth: depth table: table payload: payload data: data
				missingParity: parityMissing capture: capture)
				ifFalse: [ valid := false ] ] ].
	^ valid ifFalse: [ 'verify_failed' ]
]

{ #category : #private }
GtsV2ParityChecks >> streamShardsFrom: first to: end size: shardSize into: data ignoring: erased [
	| index |
	index := 0.
	stream shardsFrom: first to: end size: shardSize do: [ :shard :passes :shardEnd |
		index := index + 1.
		(erased includes: index) ifFalse: [ data at: index put: shard copy ] ]
]

{ #category : #validation }
GtsV2ParityChecks >> checksStripe: stripe depth: depth table: table payload: payload data: data missingParity: parityMissing capture: capture [
	| columns rows shardSize knownData knownParity solution dataMatches parityMatches |
	columns := stripe to: data size by: depth.
	rows := ((stripe - 1) * 4 + 1) to: stripe * 4.
	shardSize := table first.
	knownData := columns collect: [ :column | data at: column ].
	knownParity := rows collect: [ :row |
		(parityMissing includes: row) ifFalse: [
			| start |
			start := table fourth + ((row - 1) * shardSize).
			payload copyFrom: start to: start + shardSize - 1 ] ].
	solution := codec solveData: knownData parity: knownParity shardSize: shardSize.
	dataMatches := self checksSolved: solution first original: knownData slots: columns digests: table second
		kind: #data capture: capture.
	parityMatches := self checksSolved: solution second original: knownParity slots: rows digests: table third
		kind: #parity capture: capture.
	^ dataMatches and: [ parityMatches ]
]

{ #category : #validation }
GtsV2ParityChecks >> checksSolved: solved original: known slots: slots digests: digests kind: kind capture: capture [
	| valid |
	valid := true.
	known withIndexDo: [ :shard :index |
		shard ifNil: [
			| actual restored slot |
			restored := solved at: index.
			slot := slots at: index.
			actual := (GtsBlake3 deriveWith: 'gts2 shard' bytes: restored) first: 16.
			actual = (digests at: slot) ifFalse: [ valid := false ].
			capture ifNotNil: [ capture value: kind value: slot value: restored ] ] ].
	^ valid
]

{ #category : #private }
GtsV2ParityChecks >> protectThrough: end for: event [
	event parityFinding = 'parity_stale' ifFalse: [ stream protectThrough: end ]
]

{ #category : #validation }
GtsV2ParityChecks >> recordUnexplained: count of: dataFailed for: event [
	count > 0 ifTrue: [
		count = dataFailed
			ifTrue: [ event parityFinding: 'parity_stale' ]
			ifFalse: [ event parityShardFindings add: { 'parity_unexplained'. nil. count } ] ]
]
