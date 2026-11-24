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
GtsVerifyTest >> v2SignatureResults: case [
	| decoded held verifier bytes header results |
	decoded := GtsCborReader decode: (GtsHex decode: (case at: 'read_result')).
	held := (decoded at: 6) second collect: [ :entry |
		{ (GtsHex encode: entry first). entry second. entry third } ].
	verifier := GtsV2SignatureChecks held: held reversed, held.
	bytes := ('/workspace/vectors/v2/P6/', (case at: 'input_file')) asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ].
	results := OrderedCollection new.
	(GtsV2Wire inspect: bytes) eventsDo: [ :event |
		event startsSegment
			ifTrue: [ header := event effectiveIdentifier ]
			ifFalse: [
				header ifNotNil: [
					verifier event: event header: header resultsDo: [ :algorithm :kid :status :reason |
						results add: { status. reason } ] ] ] ].
	^ results asArray
]

{ #category : #private }
GtsVerifyTest >> assertV2SignatureVector: name results: expected [
	| json |
	json := self jsonVectorAt: '/workspace/vectors/v2/P6/', name, '.json'.
	(json at: 'cases') do: [ :case |
		self assert: (self v2SignatureResults: case) = expected description: (case at: 'input_file') ]
]

{ #category : #tests }
GtsVerifyTest >> testV2Ed25519SignaturePolicy [
	#(
		('ed25519-honest' nil)
		('ed25519-mixed-order-valid' nil)
		('ed25519-mixed-order-cofactored-only' 'equation_failed')
		('ed25519-key-non-canonical-sign' 'equation_failed')
		('ed25519-key-non-canonical-y' 'equation_failed')
		('ed25519-key-off-curve' 'equation_failed')
		('ed25519-order-scalar-key' 'non_canonical_scalar')
		('ed25519-order-key-nonce' 'small_order_key')
		('ed25519-length-63' 'value_length')
		('ed25519-length-65' 'value_length')
	) do: [ :test |
		self assertV2SignatureVector: test first
			results: { { (test second ifNil: [ 'valid' ] ifNotNil: [ 'invalid' ]). test second } } ].
	1 to: 8 do: [ :index |
		self assertV2SignatureVector: 'ed25519-key-small-order-', index printString
			results: #(('invalid' 'small_order_key')).
		self assertV2SignatureVector: 'ed25519-nonce-small-order-', index printString
			results: #(('invalid' 'small_order_nonce')) ]
]

{ #category : #tests }
GtsVerifyTest >> testV2SignatureKeysAndEffectiveSubjects [
	#(
		('key-held-order' 'valid' nil)
		('key-mismatch-two-entries' 'valid' nil)
		('key-mismatch-bytes' 'invalid' 'key_mismatch')
		('key-mismatch-algorithm' 'invalid' 'key_mismatch')
		('key-trusted-not-held' 'unverified' 'key_not_held')
		('key-length-short' 'invalid' 'value_length')
		('subject-healed-frame' 'valid' nil)
		('cov-broken-signed-frame' 'valid' nil)
	) do: [ :test | self assertV2SignatureVector: test first results: { test allButFirst } ].
	self assertV2SignatureVector: 'sig-list-unordered' results: #()
]

{ #category : #tests }
GtsVerifyTest >> testV2CompositeSignaturePolicy [
	| hedged |
	self assertV2SignatureVector: 'composite-honest' results: #(('valid' nil)).
	#(
		('composite-length-3372' 'value_length')
		('composite-length-3374' 'value_length')
		('composite-key-1983' 'value_length')
		('composite-key-1985' 'value_length')
		('composite-classical-nonce' 'composite_classical_failed')
		('composite-classical-scalar' 'composite_classical_failed')
		('composite-classical-small-order' 'composite_classical_failed')
		('composite-both' 'composite_classical_failed')
		('composite-context-nonempty' 'composite_classical_failed')
		('composite-prehash-wrong' 'composite_classical_failed')
		('composite-label-wrong' 'composite_classical_failed')
		('composite-post-quantum' 'composite_post_quantum_failed')
		('composite-stripped-half' 'composite_post_quantum_failed')
		('composite-mldsa-context' 'composite_post_quantum_failed')
		('composite-hint-count-above-omega' 'composite_post_quantum_failed')
		('composite-hint-count-decreasing' 'composite_post_quantum_failed')
		('composite-hint-indices-unordered' 'composite_post_quantum_failed')
		('composite-hint-padding-nonzero' 'composite_post_quantum_failed')
	) do: [ :test |
		self assertV2SignatureVector: test first results: { { 'invalid'. test second } } ].
	hedged := self jsonVectorAt: '/workspace/vectors/v2/P6/composite-hedged.json'.
	(hedged at: 'cases') do: [ :case |
		| count |
		count := ((case at: 'input_file') endsWith: '.both.bin') ifTrue: [ 2 ] ifFalse: [ 1 ].
		self assert: (self v2SignatureResults: case) equals: (Array new: count withAll: #('valid' nil)) ]
]

{ #category : #private }
GtsVerifyTest >> assertAuthenticationOf: case [
	| decoded inputs held verifier bytes checks authentication header actual signatures expected |
	decoded := GtsCborReader decode: (GtsHex decode: (case at: 'read_result')).
	inputs := decoded at: 6.
	held := inputs second collect: [ :entry | { (GtsHex encode: entry first). entry second. entry third } ].
	verifier := GtsV2SignatureChecks held: held.
	bytes := ('/workspace/vectors/v2/P6/', (case at: 'input_file')) asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ].
	actual := OrderedCollection new.
	(GtsV2Wire inspect: bytes) eventsDo: [ :event |
		event startsSegment
			ifTrue: [
				authentication ifNotNil: [ actual add: { authentication summary. signatures asArray } ].
				header := event effectiveIdentifier.
				checks := GtsV2CommitChecks openedBy: header.
				authentication := GtsV2Authentication trusted: inputs first.
				signatures := OrderedCollection new ]
			ifFalse: [
				event kind ~= #stop ifTrue: [
					authentication observe: event validCommit:
						(checks observe: event findingsDo: [ :finding :position :detail | ]).
					verifier event: event header: header resultsDo: [ :algorithm :kid :status :reason |
						| coverage |
						coverage := authentication coverageFor: status keyId: kid.
						signatures add: { (coverage at: 'coverage_first'). (coverage at: 'coverage_end').
							(coverage at: 'authenticated') } ] ] ] ].
	actual add: { authentication summary. signatures asArray }.
	expected := (decoded at: 7) second collect: [ :segment |
		| expectedSignatures |
		expectedSignatures := OrderedCollection new.
		(segment at: 17) do: [ :frame |
			(frame at: 8) do: [ :signature | expectedSignatures add: (signature copyFrom: 5 to: 7) ] ].
		{ (Dictionary newFromPairs: { 'authenticity'. (segment at: 10).
			'authenticated_frames'. (segment at: 13) }). expectedSignatures asArray } ].
	self assert: actual asArray = expected description: (case at: 'input_file')
]

{ #category : #tests }
GtsVerifyTest >> testV2SignatureCoverageAndAuthentication [
	#('cov-broken-signed-frame' 'cov-break-before' 'cov-break-after' 'cov-healed'
		'cov-transparent-between' 'cov-not-valid' 'auth-commit-valid' 'auth-commit-bad-count'
		'auth-commit-bad-root' 'auth-commit-malformed' 'auth-untrusted' 'auth-non-commit'
		'auth-tail' 'auth-empty-segment-index' 'auth-canonical-end' 'auth-two-commits-two-signers'
		'auth-broken-middle' 'auth-transparent-never' 'auth-payload-damaged' 'key-trusted-not-held')
		do: [ :name |
			| vector |
			vector := self jsonVectorAt: '/workspace/vectors/v2/P6/', name, '.json'.
			(vector at: 'cases') do: [ :case | self assertAuthenticationOf: case ] ]
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
		decodedWhen: [ :event | #(1 2 3 4 5 13 15) includes: event frameType ].
	^ { checks. findings. events }
]
