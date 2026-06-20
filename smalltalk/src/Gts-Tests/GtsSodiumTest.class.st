Class {
	#name : #GtsSodiumTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsSodiumTest >> byteArrayOfSize: size with: value [
	| bytes |
	bytes := ByteArray new: size.
	1 to: size do: [ :index | bytes at: index put: value ].
	^ bytes
]

{ #category : #tests }
GtsSodiumTest >> testAes256GcmRoundTripWhenAvailable [
	| additionalData ciphertext key nonce payload restored tampered |
	GtsSodium aes256GcmIsAvailable ifFalse: [ ^ self ].
	key := self byteArrayOfSize: GtsSodium aes256GcmKeyBytes with: 7.
	nonce := self byteArrayOfSize: GtsSodium aes256GcmNonceBytes with: 9.
	additionalData := 'gts aad' utf8Encoded.
	payload := 'Graph Transport Substrate AES-256-GCM smoke test' utf8Encoded.
	ciphertext := GtsSodium
		aes256GcmEncrypt: payload
		additionalData: additionalData
		nonce: nonce
		key: key.
	restored := GtsSodium
		aes256GcmDecrypt: ciphertext
		additionalData: additionalData
		nonce: nonce
		key: key.
	self assert: restored equals: payload.
	tampered := ciphertext copy.
	tampered at: tampered size put: ((tampered at: tampered size) bitXor: 1).
	self
		should: [
			GtsSodium
				aes256GcmDecrypt: tampered
				additionalData: additionalData
				nonce: nonce
				key: key ]
		raise: Error
]

{ #category : #tests }
GtsSodiumTest >> testAes256GcmRejectsWrongKeyAndNonceSizes [
	| key message nonce |
	GtsSodium aes256GcmIsAvailable ifFalse: [ ^ self ].
	key := self byteArrayOfSize: GtsSodium aes256GcmKeyBytes with: 7.
	nonce := self byteArrayOfSize: GtsSodium aes256GcmNonceBytes with: 9.
	message := 'bounded native buffers' utf8Encoded.
	self
		should: [
			GtsSodium
				aes256GcmEncrypt: message
				additionalData: ByteArray new
				nonce: ByteArray new
				key: key ]
		raise: Error.
	self
		should: [
			GtsSodium
				aes256GcmEncrypt: message
				additionalData: ByteArray new
				nonce: nonce
				key: ByteArray new ]
		raise: Error
]

{ #category : #tests }
GtsSodiumTest >> testEd25519SignsAndVerifies [
	| keyPair message signature tampered |
	keyPair := GtsSodium seedKeyPairFrom: (self byteArrayOfSize: GtsSodium ed25519SeedBytes with: 1).
	message := 'Graph Transport Substrate Ed25519 smoke test' utf8Encoded.
	signature := GtsSodium sign: message secretKey: (keyPair at: 'secretKey').
	self assert: signature size equals: GtsSodium ed25519SignatureBytes.
	self
		assert: (GtsSodium
			verify: signature
			message: message
			publicKey: (keyPair at: 'publicKey')).
	tampered := message copy.
	tampered at: 1 put: ((tampered at: 1) bitXor: 1).
	self deny: (GtsSodium verify: signature message: tampered publicKey: (keyPair at: 'publicKey'))
]

{ #category : #tests }
GtsSodiumTest >> testEd25519RejectsWrongBufferSizes [
	| keyPair message signature |
	keyPair := GtsSodium seedKeyPairFrom: (self byteArrayOfSize: GtsSodium ed25519SeedBytes with: 1).
	message := 'bounded Ed25519 buffers' utf8Encoded.
	signature := GtsSodium sign: message secretKey: (keyPair at: 'secretKey').
	self
		should: [ GtsSodium seedKeyPairFrom: ByteArray new ]
		raise: Error.
	self
		should: [ GtsSodium sign: message secretKey: ByteArray new ]
		raise: Error.
	self
		should: [ GtsSodium verify: ByteArray new message: message publicKey: (keyPair at: 'publicKey') ]
		raise: Error.
	self
		should: [ GtsSodium verify: signature message: message publicKey: ByteArray new ]
		raise: Error
]

{ #category : #tests }
GtsSodiumTest >> testNativeSizesAreAvailable [
	self assert: GtsSodium ed25519SeedBytes equals: 32.
	self assert: GtsSodium ed25519PublicKeyBytes equals: 32.
	self assert: GtsSodium ed25519SecretKeyBytes equals: 64.
	self assert: GtsSodium ed25519SignatureBytes equals: 64.
	self assert: GtsSodium aes256GcmKeyBytes equals: 32.
	self assert: GtsSodium aes256GcmNonceBytes equals: 12.
	self assert: GtsSodium aes256GcmMacBytes equals: 16
]
