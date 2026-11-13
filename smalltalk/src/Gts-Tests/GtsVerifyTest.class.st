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
	self assert: vectors size equals: 14.
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
