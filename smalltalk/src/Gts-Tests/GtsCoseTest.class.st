Class {
	#name : #GtsCoseTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsCoseTest >> jsonVectorAt: path [
	^ STONJSON fromString: (path asFileReference readStreamDo: [ :stream | stream contents ])
]

{ #category : #tests }
GtsCoseTest >> testEncrypt0MatchesFrozenVector [
	| ciphertext key plaintext restored vector |
	vector := self jsonVectorAt: '/workspace/vectors/encrypt0/basic.json'.
	ciphertext := GtsCose
		encrypt0: (GtsHex decode: (vector at: 'plaintext'))
		kid: (vector at: 'kid')
		key: (GtsHex decode: (vector at: 'key'))
		iv: (GtsHex decode: (vector at: 'iv')).
	self assert: (GtsHex encode: ciphertext) equals: (vector at: 'cose').
	key := Dictionary newFromPairs: { (vector at: 'kid'). (GtsHex decode: (vector at: 'key')) }.
	plaintext := GtsHex decode: (vector at: 'plaintext').
	restored := GtsCose decrypt0: ciphertext keys: key.
	self assert: restored equals: plaintext
]

{ #category : #tests }
GtsCoseTest >> testSign1MatchesFrozenVector [
	| cose frameId keyPair vector |
	vector := self jsonVectorAt: '/workspace/vectors/cose/sign1-basic.json'.
	frameId := GtsHex decode: (vector at: 'frame_id').
	keyPair := GtsSodium seedKeyPairFrom: (GtsHex decode: (vector at: 'seed')).
	cose := GtsCose
		signFrameId: frameId
		secretKey: (keyPair at: 'secretKey')
		kid: (vector at: 'kid').
	self assert: (GtsHex encode: cose) equals: (vector at: 'cose').
	self
		assert: (GtsCose verifySign1: cose frameId: frameId publicKey: (GtsHex decode: (vector at: 'pub')))
]
