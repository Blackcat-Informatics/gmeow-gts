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
		('index-covers-frame-of-no-type.long-form-15-typed' 'index_directory')
		('index-covers-frame-of-no-type.long-form-15-omitted' 'index_directory')
	) do: [ :entry |
		| findings expected |
		findings := (self checkV2: (self v2Vector: entry first)) second collect: [ :finding | finding first ].
		expected := entry second ifNil: [ #() ] ifNotNil: [ :reason | { 'index_claim_failed (', reason, ')' } ].
		self assert: findings asArray = expected description: entry first ]
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
	held := ((decoded at: 6) second select: [ :entry | entry second = 1 ])
		collect: [ :entry | (GtsHex encode: entry first) -> entry third ].
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
		('key-trusted-not-held' 'unverified' 'key_not_held')
		('key-length-short' 'invalid' 'value_length')
		('subject-healed-frame' 'valid' nil)
		('cov-broken-signed-frame' 'valid' nil)
	) do: [ :test | self assertV2SignatureVector: test first results: { test allButFirst } ].
	self assertV2SignatureVector: 'sig-list-unordered' results: #()
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
		decodedWhen: [ :event | event frameType = 15 ].
	^ { checks. findings. events }
]
