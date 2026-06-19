Class {
	#name : #GtsSodium,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #accessing }
GtsSodium class >> aes256GcmKeyBytes [
	self ensureInitialized.
	^ GtsSodiumLibrary uniqueInstance aes256GcmKeyBytes
]

{ #category : #accessing }
GtsSodium class >> aes256GcmMacBytes [
	self ensureInitialized.
	^ GtsSodiumLibrary uniqueInstance aes256GcmMacBytes
]

{ #category : #accessing }
GtsSodium class >> aes256GcmNonceBytes [
	self ensureInitialized.
	^ GtsSodiumLibrary uniqueInstance aes256GcmNonceBytes
]

{ #category : #accessing }
GtsSodium class >> aes256GcmIsAvailable [
	self ensureInitialized.
	^ GtsSodiumLibrary uniqueInstance aes256GcmIsAvailable = 1
]

{ #category : #encryption }
GtsSodium class >> aes256GcmDecrypt: ciphertext additionalData: additionalData nonce: nonce key: key [
	| ciphertextBytes additionalDataBytes message result |
	self ensureInitialized.
	ciphertextBytes := ciphertext asByteArray.
	ciphertextBytes size >= self aes256GcmMacBytes
		ifFalse: [ self error: 'AES-256-GCM ciphertext is shorter than its tag' ].
	additionalDataBytes := additionalData asByteArray.
	message := ByteArray new: ciphertextBytes size - self aes256GcmMacBytes.
	result := GtsSodiumLibrary uniqueInstance
		aes256GcmDecrypt: ciphertextBytes
		length: ciphertextBytes size
		additionalData: additionalDataBytes
		additionalDataLength: additionalDataBytes size
		nonce: nonce asByteArray
		key: key asByteArray
		output: message.
	self requireSuccess: result.
	^ message
]

{ #category : #encryption }
GtsSodium class >> aes256GcmEncrypt: message additionalData: additionalData nonce: nonce key: key [
	| messageBytes additionalDataBytes ciphertext result |
	self ensureInitialized.
	messageBytes := message asByteArray.
	additionalDataBytes := additionalData asByteArray.
	ciphertext := ByteArray new: messageBytes size + self aes256GcmMacBytes.
	result := GtsSodiumLibrary uniqueInstance
		aes256GcmEncrypt: messageBytes
		length: messageBytes size
		additionalData: additionalDataBytes
		additionalDataLength: additionalDataBytes size
		nonce: nonce asByteArray
		key: key asByteArray
		output: ciphertext.
	self requireSuccess: result.
	^ ciphertext
]

{ #category : #accessing }
GtsSodium class >> ed25519PublicKeyBytes [
	self ensureInitialized.
	^ GtsSodiumLibrary uniqueInstance ed25519PublicKeyBytes
]

{ #category : #accessing }
GtsSodium class >> ed25519SecretKeyBytes [
	self ensureInitialized.
	^ GtsSodiumLibrary uniqueInstance ed25519SecretKeyBytes
]

{ #category : #accessing }
GtsSodium class >> ed25519SeedBytes [
	self ensureInitialized.
	^ GtsSodiumLibrary uniqueInstance ed25519SeedBytes
]

{ #category : #accessing }
GtsSodium class >> ed25519SignatureBytes [
	self ensureInitialized.
	^ GtsSodiumLibrary uniqueInstance ed25519SignatureBytes
]

{ #category : #initialization }
GtsSodium class >> ensureInitialized [
	| result |
	result := GtsSodiumLibrary uniqueInstance initializeSodium.
	result < 0 ifTrue: [ self error: 'libsodium initialization failed' ]
]

{ #category : #private }
GtsSodium class >> requireSuccess: result [
	result = 0 ifFalse: [ self error: 'libsodium operation failed' ]
]

{ #category : #signing }
GtsSodium class >> seedKeyPairFrom: seed [
	| publicKey secretKey result |
	self ensureInitialized.
	seed size = self ed25519SeedBytes
		ifFalse: [ self error: 'Ed25519 seed must be 32 bytes' ].
	publicKey := ByteArray new: self ed25519PublicKeyBytes.
	secretKey := ByteArray new: self ed25519SecretKeyBytes.
	result := GtsSodiumLibrary uniqueInstance
		seedKeyPairFrom: seed asByteArray
		publicKey: publicKey
		secretKey: secretKey.
	self requireSuccess: result.
	^ Dictionary newFromPairs: {
		'publicKey'. publicKey.
		'secretKey'. secretKey }
]

{ #category : #signing }
GtsSodium class >> sign: message secretKey: secretKey [
	| messageBytes signature result |
	self ensureInitialized.
	messageBytes := message asByteArray.
	signature := ByteArray new: self ed25519SignatureBytes.
	result := GtsSodiumLibrary uniqueInstance
		ed25519Sign: messageBytes
		length: messageBytes size
		secretKey: secretKey asByteArray
		output: signature.
	self requireSuccess: result.
	^ signature
]

{ #category : #signing }
GtsSodium class >> verify: signature message: message publicKey: publicKey [
	| messageBytes |
	self ensureInitialized.
	messageBytes := message asByteArray.
	^ (GtsSodiumLibrary uniqueInstance
		ed25519Verify: signature asByteArray
		message: messageBytes
		length: messageBytes size
		publicKey: publicKey asByteArray) = 0
]
