Class {
	#name : #GtsVerifyTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsVerifyTest >> jsonVectorAt: path [
	^ STONJSON fromString: (path asFileReference readStreamDo: [ :stream | stream contents ])
]

{ #category : #tests }
GtsVerifyTest >> testSignedVectorSignaturesVerifyWithKey [
	| graph key validCount vector |
	vector := self jsonVectorAt: '/workspace/vectors/signed/basic.json'.
	graph := GtsReader read: (GtsHex decode: (vector at: 'gts')).
	key := Dictionary newFromPairs: { (vector at: 'kid'). (GtsHex decode: (vector at: 'pub')) }.
	validCount := 0.
	graph signatures do: [ :signature |
		| status |
		status := GtsCLI verifySignature: signature keys: key.
		self assert: status equals: 'valid'.
		validCount := validCount + 1 ].
	self assert: validCount equals: 2
]

{ #category : #tests }
GtsVerifyTest >> testSignedVectorWrongKeyIsInvalid [
	| graph key vector |
	vector := self jsonVectorAt: '/workspace/vectors/signed/basic.json'.
	graph := GtsReader read: (GtsHex decode: (vector at: 'gts')).
	key := Dictionary newFromPairs: {
		(vector at: 'kid').
		(ByteArray new: GtsSodium ed25519PublicKeyBytes withAll: 0) }.
	graph signatures do: [ :signature |
		self assert: (GtsCLI verifySignature: signature keys: key) equals: 'invalid' ]
]

{ #category : #tests }
GtsVerifyTest >> testSignatureAlgorithmDispatchVectors [
	"GTS-SPEC section 9.2: an unsupported algorithm is unverified, a malformed
	COSE_Sign1 is invalid, and EdDSA with a short signature is invalid only once
	a key resolves."

	| vectors |
	vectors := '/workspace/vectors/signed' asFileReference children select: [ :each |
		(each basename beginsWith: 'sigalg-') and: [ each basename endsWith: '.json' ] ].
	self assert: vectors size equals: 68.
	vectors do: [ :file |
		| keyed vector unkeyed |
		vector := self jsonVectorAt: file fullName.
		keyed := Dictionary newFromPairs: { (vector at: 'kid'). (GtsHex decode: (vector at: 'pub')) }.
		unkeyed := Dictionary new.
		self
			assert: (GtsReader read: (GtsHex decode: (vector at: 'gts'))) signatures size
			equals: 2.
		(GtsReader read: (GtsHex decode: (vector at: 'gts'))) signatures do: [ :signature |
			self
				assert: (GtsCLI verifySignature: signature keys: keyed)
				equals: (vector at: 'status_with_key').
			self
				assert: (GtsCLI verifySignature: signature keys: unkeyed)
				equals: (vector at: 'status_without_key').
			(vector includesKey: 'reported_kid') ifTrue: [
				self assert: (signature at: 'kid') equals: (vector at: 'reported_kid') ] ] ]
]

{ #category : #tests }
GtsVerifyTest >> testV2CommitChecksUseTheObservedPrefix [
	#(
		('commit-valid' 3 1 5 nil nil)
		('commit-count-mismatch' 0 4 1 'commit_count_mismatch' 2)
		('commit-root-mismatch' 0 4 1 'commit_root_mismatch' nil)
		('commit-payload-nonempty' 0 2 1 nil nil)
		('range-transparent.with-parity' 3 0 nil nil nil)
		('range-transparent.no-parity' 3 0 nil nil nil)
		('range-healed-leaf' 3 0 nil nil nil)
	) do: [ :entry |
		| result checks findings |
		result := self checkV2: (self v2Vector: entry first).
		checks := result first.
		findings := result second.
		self assert: checks committedFrames = entry second description: entry first.
		self assert: checks tailFrames = entry third description: entry first.
		self assert: checks tailPosition = entry fourth description: entry first.
		self assert: findings asArray = ((entry at: 5)
			ifNil: [ #() ]
			ifNotNil: [ :reason | { { reason. 4. entry at: 6 } } ]) description: entry first ]
]

{ #category : #tests }
GtsVerifyTest >> testV2CommitFindingsSurvivePayloadDamageAndFramingStop [
	| bytes wire frame walker fields payload modified result |
	bytes := self v2Vector: 'commit-root-mismatch'.
	wire := GtsV2Wire inspect: bytes.
	wire eventsDo: [ :event | event frameType = 14 ifTrue: [ frame := event ] ].
	walker := GtsV2CborWalker on: bytes.
	fields := (GtsV2Field walker: walker from: frame offset + 1 to: frame end + 1) fields: 11.
	8 timesRepeat: [ fields next ].
	payload := fields next.
	modified := (bytes copyFrom: 1 to: payload start - 1), #[65 0],
		(bytes copyFrom: payload end to: bytes size), #[27].
	result := self checkV2: modified.
	self assert: result second asArray equals: #(('commit_root_mismatch' 4 nil)).
	self assert: result third last kind equals: #stop.
	frame := result third detect: [ :event | event frameType = 14 ].
	self assert: frame payload reason equals: 'decoded_length'.
	result := self checkV2: (self v2Vector: 'commit-valid'), #[27].
	self assert: result third last kind equals: #stop.
	self assert: result first committedFrames equals: 3
]

{ #category : #tests }
GtsVerifyTest >> testV2IndexPrefixClaims [
	#(
		('index-intact' nil)
		('index-nonterminal-claims' nil)
		('index-each-claim.head' 'index_head')
		('index-each-claim.range_root' 'index_root')
		('index-each-claim.type_runs' 'index_directory')
		('index-each-claim.graph_directory' 'index_directory')
		('index-each-claim.pack_members' 'index_members')
		('index-covers-frame-of-no-type.long-form-15-typed' 'index_directory')
		('index-covers-frame-of-no-type.long-form-15-omitted' 'index_directory')
	) do: [ :entry |
		| findings expected |
		findings := (self checkV2: (self v2Vector: entry first)) second collect: [ :finding | finding first ].
		expected := entry second ifNil: [ #() ] ifNotNil: [ :reason | { 'index_claim_failed (', reason, ')' } ].
		self assert: findings asArray = expected description: entry first ]
]

{ #category : #tests }
GtsVerifyTest >> testV2IndexGraphRangesCrossFramesOfTheSameType [
	| expected |
	expected := #(((1) ((2 0 0 1))) ((2 4) ((2 0 1 2) (5 1 0 1)))).
	{ expected. #(((1) ((2 0 0 1))) ((2 4) ((2 0 1 1) (5 1 0 1) (2 2 0 1)))) }
		withIndexDo: [ :claim :case |
			| writer range head index findings |
			writer := GtsV2SegmentWriter new.
			writer appendType: 2 content: #(2 2)
				payload: (GtsCborWriter encode: #((((1) 1) ((2 4) 1)) ((1 2)) (2 0) (0 0))).
			writer writeType: 5 content: #(1) payload: (GtsCborWriter encode: #((4))).
			writer appendType: 2 content: #(2 1)
				payload: (GtsCborWriter encode: #((((2 4) 1)) ((1 1)) (2) (0))).
			range := GtsV2Range new.
			(GtsV2Wire inspect: writer bytes) eventsDo: [ :event |
				head := event effectiveIdentifier.
				range add: head ].
			index := GtsCborWriter encode: { #((2 1) (5 1) (2 1)). claim. #(0 0 0 ()). range root. #(1). #() }.
			writer writeType: 15 content: { 3. head } payload: index.
			findings := (self checkV2: writer bytes) second asArray.
			self assert: findings equals:
				(case = 1 ifTrue: [ #() ] ifFalse: [ #(('index_claim_failed (index_directory)' 4 nil)) ]) ]
]

{ #category : #tests }
GtsVerifyTest >> testV2IndexCoverageChecksRowGrammar [
	#(
		('annotations-triple-components-written-alike.components-written-alike' 'unordered_or_repeated')
		('annotations-triple-components-written-alike.components-differ' nil)
		('rows-order-type-before-count' 'out_of_range')
	) do: [ :test |
		| bytes findings expected |
		bytes := ('/workspace/vectors/v2/P3/', test first, '.bin') asFileReference
			binaryReadStreamDo: [ :stream | stream upToEnd ].
		findings := (self checkV2: bytes) second collect: [ :finding | finding first ].
		expected := test second ifNil: [ #() ] ifNotNil: [ :reason | { 'malformed_value (', reason, ')' } ].
		self assert: findings asArray equals: expected ]
]

{ #category : #tests }
GtsVerifyTest >> testV2IndexComparesPackMemberIdentityAndOrder [
	| units first second value directory packed expected |
	units := #(#[1] #[2]) collect: [ :bytes | { (GtsBlake3 deriveWith: 'gts2 unit' bytes: bytes). bytes } ].
	units sort: [ :left :right | (GtsCose compareBytes: left first with: right first) < 0 ].
	first := units first.
	second := units second.
	value := GtsCborWriter encode: { { first first. 1 } }.
	directory := { { 4. { first first. 1. 1 }. value size }.
		{ 2. { second first. #(1) }. 1 }.
		{ 1. { first first }. 1 } }.
	packed := GtsCborWriter encode: { directory. value, second second, first second }.
	expected := { { first first. 1. 2 }. { second first. 1. 1 } }.
	{ expected. { { first first. 0. 2 }. expected second }.
		{ { first first. 1. 0 }. expected second }.
		{ expected first. { first first. 1. 1 } }. expected reversed } withIndexDo: [ :claim :case |
		| writer range head index findings |
		writer := GtsV2SegmentWriter new.
		writer appendType: 8 content: { first first } payload: first second.
		writer writeType: 13 content: #(3 2) payload: packed.
		range := GtsV2Range new.
		(GtsV2Wire inspect: writer bytes) eventsDo: [ :event |
			head := event effectiveIdentifier.
			range add: head ].
		index := GtsCborWriter encode: { #((8 1) (13 1)). #(). #(0 0 0 ()). range root. #(1). claim }.
		writer writeType: 15 content: { 2. head } payload: index.
		findings := (self checkV2: writer bytes) second asArray.
		self assert: findings equals: (case = 1 ifTrue: [ #() ] ifFalse: [ #(('index_claim_failed (index_members)' 3 nil)) ]) ]
]

{ #category : #tests }
GtsVerifyTest >> testV2IndexChecksDamagedMembersAndSkipsOpaquePacks [
	| digest expected |
	digest := ByteArray new: 32.
	expected := GtsCborWriter encode: { { 1. { digest }. 1 } }.
	{ { expected. 'damaged_frame (payload_digest)'. true }.
		{ #[128]. 'malformed_value (inconsistent_counts)'. false } } do: [ :test |
		| writer bytes range head index result findings |
		writer := GtsV2SegmentWriter new.
		bytes := #[130], test first, #[65 1].
		writer writeType: 13 content: #(1 1) payload: bytes.
		range := GtsV2Range new.
		(GtsV2Wire inspect: writer bytes) eventsDo: [ :event |
			head := event effectiveIdentifier.
			range add: head ].
		index := GtsCborWriter encode: { #((13 1)). #(). #(0 0 0 ()). range root. #(1). #() }.
		writer writeType: 15 content: { 1. head } payload: index.
		result := self checkV2: writer bytes.
		findings := result second asArray.
		self assert: findings equals: (test third
			ifTrue: [ { { test second. 1. nil }. { 'index_claim_failed (index_members)'. 2. nil } } ]
			ifFalse: [ { { test second. 1. nil } } ]).
		self assert: result first uncomparedDirectories asArray equals:
			(test third ifTrue: [ #() ] ifFalse: [ #('pack-member directory') ]) ]
]

{ #category : #tests }
GtsVerifyTest >> testV2MalformedIndexDoesNotReportPartialClaims [
	| shortDigest |
	shortDigest := GtsCborWriter encode: { #(). #(). #(0 0 0 ()). (ByteArray new: 31). #(1). #() }.
	{ { #[]. 'wrong_length' }. { shortDigest. 'wrong_length' }. { #[134 101]. 'wrong_type' } } do: [ :test |
		| writer findings expected |
		writer := GtsV2SegmentWriter new.
		writer writeType: 15 content: { 2. (ByteArray new: 32) } payload: test first.
		findings := (self checkV2: writer bytes) second.
		expected := 'malformed_value (', test second, ')'.
		self assert: findings asArray equals: { { expected. 1. nil } } ]
]

{ #category : #private }
GtsVerifyTest >> v2Vector: name [
	^ ('/workspace/vectors/v2/P5/', name, '.bin') asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #private }
GtsVerifyTest >> checkV2: bytes [
	| checks findings events |
	findings := OrderedCollection new.
	events := OrderedCollection new.
	(GtsV2Wire inspect: bytes) eventsDo: [ :event |
		events add: event.
		event startsSegment
			ifTrue: [ checks := GtsV2CommitChecks openedBy: event effectiveIdentifier ]
			ifFalse: [
				event kind = #stop ifFalse: [
					checks observe: event findingsDo: [ :reason :position :expected |
						findings add: { reason. position. expected } ] ] ] ]
		budget: GtsV2PayloadBudget new
		decodedWhen: [ :event | #(1 2 3 4 5 13 15) includes: event frameType ].
	^ { checks. findings. events }
]
