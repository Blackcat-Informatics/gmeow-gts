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
	| ciphertextBytes additionalDataBytes keyBytes message nonceBytes result |
	self ensureInitialized.
	ciphertextBytes := ciphertext asByteArray.
	ciphertextBytes size >= self aes256GcmMacBytes
		ifFalse: [ self error: 'AES-256-GCM ciphertext is shorter than its tag' ].
	additionalDataBytes := additionalData asByteArray.
	nonceBytes := self requireBytes: nonce asByteArray named: 'AES-256-GCM nonce' size: self aes256GcmNonceBytes.
	keyBytes := self requireBytes: key asByteArray named: 'AES-256-GCM key' size: self aes256GcmKeyBytes.
	message := ByteArray new: ciphertextBytes size - self aes256GcmMacBytes.
	result := GtsSodiumLibrary uniqueInstance
		aes256GcmDecrypt: ciphertextBytes
		length: ciphertextBytes size
		additionalData: additionalDataBytes
		additionalDataLength: additionalDataBytes size
		nonce: nonceBytes
		key: keyBytes
		output: message.
	self requireSuccess: result.
	^ message
]

{ #category : #encryption }
GtsSodium class >> aes256GcmEncrypt: message additionalData: additionalData nonce: nonce key: key [
	| messageBytes additionalDataBytes ciphertext keyBytes nonceBytes result |
	self ensureInitialized.
	messageBytes := message asByteArray.
	additionalDataBytes := additionalData asByteArray.
	nonceBytes := self requireBytes: nonce asByteArray named: 'AES-256-GCM nonce' size: self aes256GcmNonceBytes.
	keyBytes := self requireBytes: key asByteArray named: 'AES-256-GCM key' size: self aes256GcmKeyBytes.
	ciphertext := ByteArray new: messageBytes size + self aes256GcmMacBytes.
	result := GtsSodiumLibrary uniqueInstance
		aes256GcmEncrypt: messageBytes
		length: messageBytes size
		additionalData: additionalDataBytes
		additionalDataLength: additionalDataBytes size
		nonce: nonceBytes
		key: keyBytes
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
GtsSodium class >> requireBytes: bytes named: label size: expectedSize [
	bytes size = expectedSize
		ifFalse: [ self error: label, ' must be ', expectedSize asString, ' bytes' ].
	^ bytes
]

{ #category : #private }
GtsSodium class >> requireSuccess: result [
	result = 0 ifFalse: [ self error: 'libsodium operation failed' ]
]

{ #category : #signing }
GtsSodium class >> seedKeyPairFrom: seed [
	| publicKey secretKey seedBytes result |
	self ensureInitialized.
	seedBytes := self requireBytes: seed asByteArray named: 'Ed25519 seed' size: self ed25519SeedBytes.
	publicKey := ByteArray new: self ed25519PublicKeyBytes.
	secretKey := ByteArray new: self ed25519SecretKeyBytes.
	result := GtsSodiumLibrary uniqueInstance
		seedKeyPairFrom: seedBytes
		publicKey: publicKey
		secretKey: secretKey.
	self requireSuccess: result.
	^ Dictionary newFromPairs: {
		'publicKey'. publicKey.
		'secretKey'. secretKey }
]

{ #category : #signing }
GtsSodium class >> sign: message secretKey: secretKey [
	| messageBytes secretKeyBytes signature result |
	self ensureInitialized.
	messageBytes := message asByteArray.
	secretKeyBytes := self requireBytes: secretKey asByteArray named: 'Ed25519 secret key' size: self ed25519SecretKeyBytes.
	signature := ByteArray new: self ed25519SignatureBytes.
	result := GtsSodiumLibrary uniqueInstance
		ed25519Sign: messageBytes
		length: messageBytes size
		secretKey: secretKeyBytes
		output: signature.
	self requireSuccess: result.
	^ signature
]

{ #category : #signing }
GtsSodium class >> verify: signature message: message publicKey: publicKey [
	| messageBytes publicKeyBytes signatureBytes |
	self ensureInitialized.
	messageBytes := message asByteArray.
	signatureBytes := self requireBytes: signature asByteArray named: 'Ed25519 signature' size: self ed25519SignatureBytes.
	publicKeyBytes := self requireBytes: publicKey asByteArray named: 'Ed25519 public key' size: self ed25519PublicKeyBytes.
	^ (GtsSodiumLibrary uniqueInstance
		ed25519Verify: signatureBytes
		message: messageBytes
		length: messageBytes size
		publicKey: publicKeyBytes) = 0
]
