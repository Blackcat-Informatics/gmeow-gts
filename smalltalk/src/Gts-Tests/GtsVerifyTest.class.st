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
						findings add: { reason. position. expected } ] ] ] ].
	^ { checks. findings. events }
]
