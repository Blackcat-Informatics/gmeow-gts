Class {
	#name : #GtsOpenPGPTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsOpenPGPTest >> jsonVectorAt: path [
	^ STONJSON fromString: (path asFileReference readStreamDo: [ :stream | stream contents ])
]

{ #category : #tests }
GtsOpenPGPTest >> testExtractKeyTextMatchesFrozenVector [
	| graph gtsBytes vector |
	vector := self jsonVectorAt: '/workspace/vectors/openpgp/extract-key.json'.
	gtsBytes := GtsHex decode: (vector at: 'gts').
	graph := GtsReader read: gtsBytes.
	self
		assert: (GtsOpenPGP extractKeyTextFromGraph: graph)
		equals: (vector at: 'stdout')
]

{ #category : #tests }
GtsOpenPGPTest >> testParseFrozenTransportKey [
	| key vector |
	vector := self jsonVectorAt: '/workspace/vectors/openpgp/test-key.json'.
	key := GtsOpenPGP parseTransportKey: (vector at: 'armored').
	self assert: (GtsHex encode: (key at: 'rawPublic')) equals: (vector at: 'raw_pub').
	self assert: (key at: 'fingerprint') equals: (vector at: 'fingerprint').
	self
		assert: (GtsOpenPGP formatFingerprint: (key at: 'fingerprint'))
		equals: (vector at: 'fingerprint_grouped').
	self
		assert: (GtsOpenPGP emojihash: (key at: 'rawPublic') length: 11)
		equals: (vector at: 'emojihash')
]

{ #category : #tests }
GtsOpenPGPTest >> testMalformedTransportKeySignalsCryptoError [
	self should: [ GtsOpenPGP parseTransportKey: '' ] raise: GtsCryptoError
]
