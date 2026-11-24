"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ParityTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsV2ParityTest >> bytesFor: name [
	^ ('/workspace/vectors/v2/P5/', name) asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #tests }
GtsV2ParityTest >> testTypedDurabilityFindingsMatchTheP5Records [
	#('recover-no-signature' 'recover-length' 'recover-verify-failed'
		'parity-unexplained' 'range-transparent' 'recovery-parity-in-resync-segment') do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P5/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry | self assertTypedDurabilityOf: entry ] ]
]

{ #category : #tests }
GtsV2ParityTest >> testTypedResumptionRefusalBelongsToTheSegment [
	| report |
	report := (GtsV2Wire inspect: (self bytesFor: 'recover-length.with-parity.bin')
		budget: (GtsV2PayloadBudget withReadLimit: 465)) typedWireFindings.
	self assert: report fileFindings equals: #((8 (1) (1) (1) (2 11) (2 1092) (1))).
	self assert: (report segmentFindingsAt: 0) equals: #((20 (2 86) (1) (2 1) (2 481) (2 622) (2 466))).
	self assert: (report frameFindingsAt: 481) isEmpty
]

{ #category : #private }
GtsV2ParityTest >> assertTypedDurabilityOf: entry [
	| report file input |
	input := entry at: 'input_file'.
	report := (GtsV2Wire inspect: (self bytesFor: input)) typedWireFindings.
	file := (GtsCborReader decode: (GtsHex decode: (entry at: 'read_result'))) seventh.
	self assert: (report fileFindings select: [ :finding | #(8 9) includes: finding first ])
		equals: (file third select: [ :finding | #(8 9) includes: finding first ]) asArray.
	file second do: [ :segment |
		self assert: ((report segmentFindingsAt: segment first) select: [ :finding | finding first = 36 ])
			equals: ((segment at: 18) select: [ :finding | finding first = 36 ]) asArray.
		(segment at: 17) do: [ :frame |
			| expected |
			expected := ((frame at: 11) select: [ :finding | finding first between: 30 and: 35 ]) asArray.
			self assert: ((report frameFindingsAt: frame second) select: [ :finding | finding first between: 30 and: 35 ])
				= expected description: input ] ]
]

{ #category : #tests }
GtsV2ParityTest >> testVerifiedParityRepairMatchesPublishedCopies [
	#('recover-no-signature' 'parity-damaged-shard' 'recover-verify-failed') do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P5/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry |
			| input repaired oracle expected |
			input := entry at: 'input_file'.
			repaired := (GtsV2Wire inspect: (self bytesFor: input)) parityRepair.
			oracle := entry at: 'repaired_copy_file' ifAbsent: [
				input = 'parity-damaged-shard.bin' ifTrue: [ 'parity-base.bin' ] ].
			self assert: repaired repairedBytes = (oracle ifNotNil: [ self bytesFor: oracle ]) description: input.
			expected := self repairFindingsOf: entry.
			self assert: repaired findings = expected
				description: input, ' ', repaired findings printString, ' expected ', expected printString ] ]
]

{ #category : #tests }
GtsV2ParityTest >> testRepairFailurePropagatesAcrossAnItemThatSpansTwoParityRanges [
	| manifest entry decoded fileSize input repair |
	manifest := STONJSON fromString: '/workspace/vectors/v2/P5/recover-straddle-propagate.json' asFileReference contents.
	entry := (manifest at: 'cases') first.
	decoded := GtsCborReader decode: (GtsHex decode: (entry at: 'read_result')).
	fileSize := (decoded at: 7) second last at: 3.
	input := GtsZstd decompressSingleFrame: (self bytesFor: (entry at: 'input_file'), '.zst') expectedSize: fileSize.
	self assert: input notNil.
	repair := (GtsV2Wire inspect: input) parityRepair.
	self assert: repair repairedBytes isNil.
	self assert: repair findings equals: (self repairFindingsOf: entry)
]

{ #category : #tests }
GtsV2ParityTest >> testRepairRecheckChargesTheBudgetBeforeTheNextSegment [
	| input bytes wire plan chunks |
	input := self bytesFor: 'recover-no-signature.signature-zero.bin'.
	bytes := input, input.
	wire := GtsV2Wire inspect: bytes budget: (GtsV2PayloadBudget withReadLimit: 25000).
	chunks := OrderedCollection new.
	plan := wire eventsDo: [ :event | event frameType = 8 ifTrue: [ chunks add: event payload ] ]
		decodedWhen: [ :event | false ] unprotectedTailsDo: [ :offset :length | ].
	self assert: chunks size equals: 2.
	self assert: chunks last reason equals: 'total_decoded'.
	self assert: plan repairedBytes isNil.
	self assert: plan findings equals: #(('repaired' 9456 4))
]

{ #category : #tests }
GtsV2ParityTest >> testRepairResumesAtParityAfterFramingLoss [
	| recoveredResync |
	#('recover-length' 'recover-zeroed-range' 'recovery-parity-in-resync-segment') do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P5/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry |
			| input repaired expected |
			input := entry at: 'input_file'.
			repaired := (GtsV2Wire inspect: (self bytesFor: input)) parityRepair.
			expected := self repairFindingsOf: entry.
			self assert: repaired findings = expected description: input.
			expected ifNotEmpty: [
				| parity |
				self assert: repaired stop isNil description: input.
				parity := (self parityIn: repaired repairedBytes) last.
				self assert: parity parityShardFindings isEmpty description: input.
				name = 'recovery-parity-in-resync-segment' ifTrue: [
					recoveredResync
						ifNil: [ recoveredResync := repaired repairedBytes ]
						ifNotNil: [ self assert: repaired repairedBytes equals: recoveredResync ] ] ] ] ]
]

{ #category : #tests }
GtsV2ParityTest >> testParityResumptionChargesTheCandidateBeforeItChecksPayload [
	| input refused admitted |
	input := self bytesFor: 'recover-length.with-parity.bin'.
	refused := (GtsV2Wire inspect: input budget: (GtsV2PayloadBudget withReadLimit: 465)) parityRepair.
	self assert: refused repairedBytes isNil.
	self assert: refused stop problem equals: 'undelimitable_input'.
	self assert: refused refusal offset equals: 481.
	self assert: refused refusal end equals: 1103.
	self assert: refused refusal payload reason equals: 'total_decoded'.
	self assert: refused refusal payload count equals: 466.
	admitted := (GtsV2Wire inspect: input budget: (GtsV2PayloadBudget withReadLimit: 466)) parityRepair.
	self assert: admitted refusal isNil.
	self assert: admitted stop isNil.
	self assert: admitted findings equals: #(('repair_failed' 481 'verify_failed'))
]

{ #category : #tests }
GtsV2ParityTest >> testOffsetsCountTheDamagedRegionAsOneItemAfterParityResumption [
	| input offsets plan observed |
	input := (self bytesFor: 'recover-length.with-parity.bin'),
		#[130 4 131 11 25 1 214 25 2 110].
	plan := GtsV2ParityRepair on: input.
	observed := OrderedCollection new.
	(GtsV2Wire inspect: input) eventsDo: [ :event | observed add: event ]
		decodedWhen: [ :event | false ] unprotectedTailsDo: [ :offset :length | ] repairing: plan.
	self assert: (observed collect: [ :event | event kind ]) asArray equals: #(header framingGap frame offsets).
	offsets := observed last.
	self assert: offsets problem isNil.
	self assert: offsets closingClaimFailure isNil
]

{ #category : #tests }
GtsV2ParityTest >> testParityResumptionRequiresRoomForTheDamagedRegionAndTheCandidate [
	| input limited allowed |
	input := self bytesFor: 'recovery-parity-in-resync-segment.incomplete-item.bin'.
	limited := (GtsV2Wire inspect: input budget: (GtsV2PayloadBudget new itemLimit: 5)) parityRepair.
	self assert: limited stop problem equals: 'torn_tail'.
	self assert: limited repairedBytes isNil.
	allowed := (GtsV2Wire inspect: input budget: (GtsV2PayloadBudget new itemLimit: 6)) parityRepair.
	self assert: allowed stop isNil.
	self assert: allowed findings equals: #(('repaired' 586 1))
]

{ #category : #tests }
GtsV2ParityTest >> testWholeReadReportsRecoveryAndKeepsDetachedRowsOutOfTheFold [
	| damaged suffix sound source frame events report rows repair checks failures |
	damaged := self bytesFor: 'recover-length.with-parity.bin'.
	frame := (self parityIn: damaged) first.
	source := '<https://e/s> <https://e/p> "detached" .', String lf.
	suffix := self rdfFramesFor: source startingAt: frame resumptionAnchor.
	rows := '<https://e/s> <https://e/p> "separate segment" .', String lf.
	sound := GtsFromNQuads v2FromString: rows.
	events := OrderedCollection new.
	report := (GtsV2Wire inspect: damaged, suffix, sound)
		eventsDo: [ :event | events add: event ]
		decodedWhen: [ :event | false ] unprotectedTailsDo: [ :offset :length | ].
	self assert: report repairedBytes isNil.
	self assert: report findings equals: #(('repaired' 481 1)).
	self assert: (events select: [ :event | event isParityGap ]) size equals: 1.
	self assert: ((events select: [ :event | event detached ]) collect: [ :event | event frameType ]) asArray
		equals: #(1).
	checks := GtsV2CommitChecks openedBy: events first effectiveIdentifier.
	failures := OrderedCollection new.
	(events allButFirst first: 5) do: [ :event |
		event isParityGap
			ifTrue: [ checks observeDamagedRegion ]
			ifFalse: [
				event detached ifTrue: [ checks observeDetachedFrame ].
				checks observe: event findingsDo: [ :reason :position :expected | failures add: { reason. position } ] ] ].
	self assert: failures asArray equals: #(('commit_root_mismatch' 5)).
	self assert: checks committedFrames equals: 0.
	self assert: (GtsNQuads toNQuads: (GtsV2RdfFold read: (GtsV2Wire inspect: damaged, suffix, sound))) equals: rows.
	repair := (GtsV2Wire inspect: damaged, suffix, sound) parityRepair.
	self assert: repair findings equals: report findings
]

{ #category : #tests }
GtsV2ParityTest >> testRepairKeepsItsOpenerIdentifierWhenTheVisitorEditsTheEvent [
	| report |
	report := (GtsV2Wire inspect: (self bytesFor: 'recover-length.with-parity.bin'))
		eventsDo: [ :event |
			event startsSegment ifTrue: [ event effectiveIdentifier at: 1 put: ((event effectiveIdentifier first) bitXor: 1) ] ]
		decodedWhen: [ :event | false ] unprotectedTailsDo: [ :offset :length | ].
	self assert: report findings equals: #(('repaired' 481 1))
]

{ #category : #private }
GtsV2ParityTest >> rdfFramesFor: source startingAt: predecessor [
	| current |
	current := predecessor.
	^ ByteArray streamContents: [ :stream |
		(GtsV2Wire inspect: (GtsFromNQuads v2FromString: source)) eventsDo: [ :event |
			(#(1 2 14) includes: event frameType) ifTrue: [
				| payload content identity encoded |
				payload := event frame payload plainStoredBytesUpTo: 4096.
				content := GtsCborReader decode: event frame content encodedBytes.
				identity := { event frameType. event frameType < 14. false. content. payload size.
					GtsBlake3 deriveWith: 'gts2 payload' bytes: payload }.
				encoded := (GtsCborWriter encode: identity) allButFirst.
				current := GtsBlake3 deriveWith: 'gts2 frame' prefix: #[135 88 32], current
					bytes: encoded from: 1 to: encoded size + 1.
				stream nextPutAll: (GtsCborWriter encode: { 2 }, identity, { #(1). payload. current. #() }) ] ] ]
]

{ #category : #private }
GtsV2ParityTest >> repairFindingsOf: entry [
	| decoded expected |
	decoded := GtsCborReader decode: (GtsHex decode: (entry at: 'read_result')).
	expected := OrderedCollection new.
	(decoded at: 7) second do: [ :segment |
		(segment at: 17) do: [ :frame |
			(frame at: 11) do: [ :finding |
				finding first = 34 ifTrue: [
					expected add: { 'repair_failed'. frame second.
						(finding second second = 42 ifTrue: [ 'too_many_erasures' ] ifFalse: [ 'verify_failed' ]) } ].
				finding first = 35 ifTrue: [
					expected add: { 'repaired'. frame second. (finding at: 7) second } ] ] ] ].
	^ expected asArray
]

{ #category : #private }
GtsV2ParityTest >> parityIn: bytes [
	| events |
	events := OrderedCollection new.
	(GtsV2Wire inspect: bytes) eventsDo: [ :event |
		event frameType = 16 ifTrue: [ events add: event ] ].
	^ events
]

{ #category : #tests }
GtsV2ParityTest >> testParityFrameAndTableFindingsMatchP5Vectors [
	#(
		'parity-base' 'parity-damaged-each' 'parity-above-capacity'
		'parity-above-capacity-and-damaged' 'parity-above-capacity-wrong-table'
		'parity-broken-copy-no-first' 'parity-condition-six-first-by-case'
		'parity-later-version' 'parity-span-262143' 'parity-span-262144'
		'parity-span-4096' 'parity-span-65536' 'parity-span-short'
		'parity-wrong-transparent'
	) do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P5/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry | self assertParityOf: entry ] ]
]

{ #category : #private }
GtsV2ParityTest >> assertParityOf: entry [
	| input decoded expected actual |
	input := entry at: 'input_file'.
	decoded := GtsCborReader decode: (GtsHex decode: (entry at: 'read_result')).
	expected := OrderedCollection new.
	(decoded at: 7) second do: [ :segment |
		(segment at: 17) do: [ :frame |
			(frame at: 4) = #(2 16) ifTrue: [
				| findings |
				findings := (frame at: 11) collect: [ :finding | finding first ].
				expected add: { frame second. (findings includes: 30). (findings includes: 5).
					(findings includes: 20) } ] ] ].
	actual := (self parityIn: (self bytesFor: input)) collect: [ :event |
		{ event offset. event isDamagedParity. (event parityFinding = 'unknown_frame_type').
			(event isDamagedParity not and: [ event payload state = #unchecked ]) } ].
	self assert: actual = expected description: input, ': ', actual printString, ' expected ', expected printString
]

{ #category : #tests }
GtsV2ParityTest >> testShortParitySpanMustEndAtTheLastStreamItem [
	| bytes frame modified result |
	bytes := self bytesFor: 'parity-base.bin'.
	frame := (self parityIn: bytes) first.
	self assert: frame parityFinding isNil.
	modified := (bytes first: frame offset), #[0], (bytes copyFrom: frame offset + 1 to: bytes size).
	result := (self parityIn: modified) first.
	self deny: result isDamagedParity.
	self assert: result parityFinding equals: 'parity_stale'
]

{ #category : #tests }
GtsV2ParityTest >> testDamagedTransparentParityDoesNotStopTheFold [
	| bytes source ordinary parity |
	bytes := self bytesFor: 'parity-broken-copy-no-first.bin'.
	parity := bytes allButFirst: (self parityIn: bytes) first offset.
	source := '<https://e/s> <https://e/p> "v" .', String lf.
	ordinary := GtsFromNQuads v2FromString: source.
	self assert: (self parityIn: ordinary, parity) first isDamagedParity.
	self assert: (GtsNQuads toNQuads: (GtsV2RdfFold read: (GtsV2Wire inspect: ordinary, parity))) equals: source
]

{ #category : #tests }
GtsV2ParityTest >> testLaterGeometryAfterUnknownCriticalTypeAddsNoParityFinding [
	| bytes parity |
	bytes := '/workspace/vectors/v2/P1/parity-later-version-after-stop.bin' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ].
	parity := (self parityIn: bytes) first.
	self deny: parity isDamagedParity.
	self assert: parity parityFinding isNil
]

{ #category : #tests }
GtsV2ParityTest >> testParityShardFindingsMatchP5Vectors [
	#('parity-base' 'parity-damaged-shard' 'parity-unexplained'
		'parity-unexplained-two-stripes' 'recover-no-signature') do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P5/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry | self assertShardFindingsOf: entry ] ]
]

{ #category : #private }
GtsV2ParityTest >> assertShardFindingsOf: entry [
	| input decoded expected actual recoveredOffsets |
	input := entry at: 'input_file'.
	decoded := GtsCborReader decode: (GtsHex decode: (entry at: 'read_result')).
	expected := OrderedCollection new.
	recoveredOffsets := Set new.
	(decoded at: 7) second do: [ :segment |
		(segment at: 17) do: [ :frame |
			(frame at: 4) = #(2 16) ifTrue: [
				| findings |
				findings := frame at: 11.
				(findings anySatisfy: [ :finding | finding first = 35 ])
					ifTrue: [ recoveredOffsets add: frame second ].
				expected add: { frame second.
					((findings select: [ :finding | #(31 32) includes: finding first ])
						collect: [ :finding | self shardFindingFrom: finding ]) asArray.
					(findings anySatisfy: [ :finding | finding first = 33 ]) } ] ] ].
	actual := (self parityIn: (self bytesFor: input)) collect: [ :event |
		(recoveredOffsets includes: event offset) ifTrue: [
			self deny: (event parityShardFindings anySatisfy: [ :finding | finding first = 'repair_failed' ]) ].
		{ event offset. (event parityShardFindings reject: [ :finding | finding first = 'repair_failed' ]) asArray.
			(event parityFinding = 'parity_stale') } ].
	self assert: actual = expected description: input, ': ', actual printString, ' expected ', expected printString
]

{ #category : #private }
GtsV2ParityTest >> shardFindingFrom: finding [
	| count |
	count := (finding at: 7) second.
	finding first = 32 ifTrue: [ ^ { 'parity_unexplained'. nil. count } ].
	^ { 'parity_mismatch'. (finding second second = 40 ifTrue: [ 'data' ] ifFalse: [ 'parity' ]). count }
]

{ #category : #tests }
GtsV2ParityTest >> testChangedSignatureBytesMakeParityStaleWithoutFailingTheFrame [
	| bytes signatures end parity |
	bytes := self bytesFor: 'recover-no-signature.signature-zero.repaired.bin'.
	(GtsV2Wire inspect: bytes) eventsDo: [ :event |
		event frame ifNotNil: [ :frame |
			frame signatureRecords ifNotNil: [ :field |
				field list hasNext ifTrue: [ signatures := field ] ] ] ].
	end := signatures end - 1.
	bytes at: end put: ((bytes at: end) bitXor: 1).
	parity := (self parityIn: bytes) first.
	self deny: parity isDamagedParity.
	self assert: parity parityFinding equals: 'parity_stale'.
	self assert: parity parityShardFindings asArray equals: #(('parity_mismatch' 'data' 1))
]

{ #category : #tests }
GtsV2ParityTest >> testOpenerShardExplanationWaitsForTheFirstOrdinaryLink [
	| writer header identifier parity prefix events |
	writer := GtsV2SegmentWriter new.
	header := writer bytes.
	identifier := GtsBlake3 deriveWith: 'gts2 header' bytes: header.
	parity := self mismatchingParityForHeader: header identifier: identifier.
	prefix := header, parity.
	self assert: (self parityIn: prefix) first parityFinding isNil.
	writer commit.
	events := OrderedCollection new.
	(GtsV2Wire inspect: prefix, (writer bytes allButFirst: header size))
		eventsDo: [ :event | events add: event ].
	self assert: (events collect: [ :event | event frameType ]) asArray equals: #(nil 16 14).
	self assert: events second parityFinding equals: 'parity_stale'.
	self assert: events second parityShardFindings asArray equals: #(('parity_mismatch' 'data' 1))
]

{ #category : #private }
GtsV2ParityTest >> mismatchingParityForHeader: header identifier: identifier [
	| data parity digests table payload |
	data := { ByteArray new: 16 }.
	parity := ByteArray new: 256.
	digests := Array new: 4 withAll: ((GtsBlake3 deriveWith: 'gts2 shard' bytes: (ByteArray new: 64)) first: 16).
	table := GtsBlake3 deriveWith: 'gts2 parity table' bytes: (GtsCborWriter encode: { identifier. data. digests }).
	payload := GtsCborWriter encode: { data. digests. parity }.
	^ self encodeParity: { 1. 0. header size. identifier. table } payload: payload
]

{ #category : #private }
GtsV2ParityTest >> encodeParity: content payload: payload [
	| identity encoded frameId |
	identity := { 16. false. true. content. payload size. (GtsBlake3 deriveWith: 'gts2 payload' bytes: payload) }.
	encoded := (GtsCborWriter encode: identity) allButFirst.
	frameId := GtsBlake3 deriveWith: 'gts2 frame' prefix: #[135 88 32], content fourth
		bytes: encoded from: 1 to: encoded size + 1.
	^ GtsCborWriter encode: { 2 }, identity, { #(1). payload. frameId. #() }
]

{ #category : #tests }
GtsV2ParityTest >> testUnprotectedTailsMatchP5Vectors [
	#(
		('range-transparent' false)
		('tail-none' false)
		('parity-above-capacity' false)
		('parity-condition-six-first-by-case' false)
		('parity-append-keeps-frames' true)
	) do: [ :test |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P5/', test first, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry | self assertTailsOf: entry compressed: test second ] ]
]

{ #category : #private }
GtsV2ParityTest >> assertTailsOf: entry compressed: compressed [
	| input decoded segments expected bytes actual |
	input := entry at: 'input_file'.
	decoded := GtsCborReader decode: (GtsHex decode: (entry at: 'read_result')).
	segments := (decoded at: 7) second.
	expected := OrderedCollection new.
	segments do: [ :segment |
		(segment at: 18) do: [ :finding |
			finding first = 36 ifTrue: [
				expected add: { (finding at: 5) second. (finding at: 6) second } ] ] ].
	bytes := compressed
		ifFalse: [ self bytesFor: input ]
		ifTrue: [ GtsZstd decompressSingleFrame: (self bytesFor: input, '.zst') expectedSize: segments first third ].
	self assert: bytes notNil description: input.
	actual := self tailsOf: bytes.
	self assert: actual = expected description: input, ': ', actual printString, ' expected ', expected printString
]

{ #category : #private }
GtsV2ParityTest >> tailsOf: bytes [
	| tails |
	tails := OrderedCollection new.
	(GtsV2Wire inspect: bytes)
		eventsDo: [ :event | ]
		decodedWhen: [ :event | false ]
		unprotectedTailsDo: [ :offset :length | tails add: { offset. length } ].
	^ tails
]

{ #category : #tests }
GtsV2ParityTest >> testEachSegmentReportsItsOwnTailAtAnAbsoluteOffset [
	| segment firstTail secondTail order |
	segment := self bytesFor: 'range-transparent.with-parity.bin'.
	firstTail := (self tailsOf: segment) first.
	secondTail := { firstTail first + segment size. firstTail second }.
	order := OrderedCollection new.
	(GtsV2Wire inspect: segment, segment)
		eventsDo: [ :event | event startsSegment ifTrue: [ order add: event offset ] ]
		decodedWhen: [ :event | false ]
		unprotectedTailsDo: [ :offset :length | order add: { offset. length } ].
	self assert: order asArray equals: { 0. firstTail. segment size. secondTail }
]

{ #category : #tests }
GtsV2ParityTest >> testSolveDataAndParityErasuresFromThePublishedStripe [
	| bytes parityEvent payload fields parityBytes data parity damagedData damagedParity solved |
	bytes := self bytesFor: 'parity-base.bin'.
	parityEvent := (self parityIn: bytes) first.
	payload := parityEvent frame payload plainStoredBytesUpTo: 280651.
	fields := (GtsV2Field walker: (GtsV2CborWalker on: payload) from: 1 to: payload size + 1) fields: 3.
	fields next.
	fields next.
	parityBytes := fields next.
	parityBytes := parityBytes bytesStartingAt: (parityBytes byteBodyOfSize: 256).
	parity := (1 to: 256 by: 64) collect: [ :start | parityBytes copyFrom: start to: start + 63 ].
	data := (1 to: parityEvent offset by: 64) collect: [ :start |
		| shard finish |
		shard := ByteArray new: 64.
		finish := (start + 63) min: parityEvent offset.
		shard replaceFrom: 1 to: finish - start + 1 with: bytes startingAt: start.
		shard ].
	damagedData := data copy.
	damagedData at: 1 put: nil.
	damagedData at: data size put: nil.
	damagedParity := parity copy.
	damagedParity at: 1 put: nil.
	damagedParity at: 3 put: nil.
	solved := GtsV2ParityCodec new solveData: damagedData parity: damagedParity shardSize: 64.
	self assert: solved first equals: data.
	self assert: solved second equals: parity
]

{ #category : #tests }
GtsV2ParityTest >> testReconstructingEveryParityRowMatchesThePublishedMatrix [
	| fixture rows data solved |
	fixture := STONJSON fromString: '/workspace/vectors/v2/P5/parity-matrix.json' asFileReference contents.
	rows := ((((fixture at: 'cases') first at: 'result') at: 'rows') collect: [ :row | row asByteArray ]) asArray.
	data := (1 to: 64) collect: [ :index |
		(ByteArray new: 64) at: index put: 1; yourself ].
	solved := GtsV2ParityCodec new solveData: data parity: (Array new: 4) shardSize: 64.
	self assert: solved second equals: rows
]

{ #category : #tests }
GtsV2ParityTest >> testRepairFailureWhenOneStripeHasFiveErasures [
	| bytes dataStart parityFrame parityStart field fields finding |
	bytes := self bytesFor: 'parity-base.bin'.
	parityFrame := (self parityIn: bytes) first.
	dataStart := (self storedFieldIn: bytes frameAt: 11) byteBodyOfSize: nil.
	field := self storedFieldIn: bytes frameAt: parityFrame offset.
	fields := (GtsV2Field walker: (GtsV2CborWalker on: bytes)
		from: (field byteBodyOfSize: nil) to: field end) fields: 3.
	fields next.
	fields next.
	parityStart := fields next byteBodyOfSize: 256.
	bytes at: dataStart put: ((bytes at: dataStart) bitXor: 1).
	parityStart to: parityStart + 192 by: 64 do: [ :offset |
		bytes at: offset put: ((bytes at: offset) bitXor: 1) ].
	parityFrame := (self parityIn: bytes) first.
	finding := parityFrame parityShardFindings.
	self assert: finding asArray equals: #(
		('parity_mismatch' 'data' 1)
		('parity_mismatch' 'parity' 4)).
	self assert: (GtsV2Wire inspect: bytes) parityRepair findings equals:
		{ { 'repair_failed'. parityFrame offset. 'too_many_erasures' } }.
	bytes at: parityStart put: ((bytes at: parityStart) bitXor: 1).
	self assert: (self parityIn: bytes) first parityShardFindings asArray equals: #(
		('parity_mismatch' 'data' 1)
		('parity_mismatch' 'parity' 3))
]

{ #category : #tests }
GtsV2ParityTest >> testReconstructionRejectsSolvedBytesWithTheWrongDigest [
	| bytes parity payload content header dataStart column modified result |
	bytes := self bytesFor: 'parity-base.bin'.
	parity := (self parityIn: bytes) first.
	payload := GtsCborReader decode: (parity frame payload plainStoredBytesUpTo: 280651).
	content := GtsCborReader decode: parity frame content encodedBytes.
	header := GtsBlake3 deriveWith: 'gts2 header' bytes: (bytes first: 11).
	dataStart := (self storedFieldIn: bytes frameAt: 11) byteBodyOfSize: nil.
	column := ((dataStart - 1) // 64) + 1.
	payload first at: column put: (ByteArray new: 16).
	content at: 5 put: (GtsBlake3 deriveWith: 'gts2 parity table'
		bytes: (GtsCborWriter encode: { header. payload first. payload second })).
	bytes at: dataStart put: ((bytes at: dataStart) bitXor: 1).
	modified := (bytes first: parity offset), (self encodeParity: content payload: (GtsCborWriter encode: payload)).
	result := (self parityIn: modified) first.
	self deny: result isDamagedParity.
	self assert: result parityShardFindings asArray equals: #(('parity_mismatch' 'data' 1)).
	self assert: (GtsV2Wire inspect: modified) parityRepair findings equals:
		{ { 'repair_failed'. parity offset. 'verify_failed' } }
]

{ #category : #private }
GtsV2ParityTest >> storedFieldIn: bytes frameAt: offset [
	| walker fields start end |
	walker := GtsV2CborWalker on: bytes.
	start := offset + 1.
	end := walker itemEndAt: start limit: bytes size + 1.
	fields := (GtsV2Field walker: walker from: start to: end) fields: 11.
	8 timesRepeat: [ fields next ].
	^ fields next
]
