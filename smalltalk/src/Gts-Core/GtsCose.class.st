Class {
	#name : #GtsCose,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #encrypt0 }
GtsCose class >> decrypt0: coseBytes keys: keyMap [
	| aad ciphertext iv key kid parsed protected |
	parsed := self parseEncrypt0: coseBytes.
	kid := parsed at: 'kid'.
	key := keyMap at: kid ifAbsent: [ GtsCryptoError signal: 'no content key for recipient' ].
	protected := parsed at: 'protected'.
	iv := parsed at: 'iv'.
	ciphertext := parsed at: 'ciphertext'.
	aad := GtsCborWriter encode: { 'Encrypt0'. protected. ByteArray new }.
	^ GtsSodium
		aes256GcmDecrypt: ciphertext
		additionalData: aad
		nonce: iv
		key: key
]

{ #category : #encrypt0 }
GtsCose class >> encrypt0: plaintext kid: kid key: key iv: iv [
	| aad ciphertext protected |
	protected := self encrypt0Protected.
	aad := GtsCborWriter encode: { 'Encrypt0'. protected. ByteArray new }.
	ciphertext := GtsSodium
		aes256GcmEncrypt: plaintext
		additionalData: aad
		nonce: iv
		key: key.
	^ GtsCborWriter encode: (GtsCborTag tag: 16 value: {
		protected.
		Dictionary newFromPairs: {
			4. kid utf8Encoded.
			5. iv }.
		ciphertext })
]

{ #category : #private }
GtsCose class >> encrypt0Protected [
	^ GtsCborWriter encode: (Dictionary newFromPairs: { 1. 3 })
]

{ #category : #encrypt0 }
GtsCose class >> parseEncrypt0: coseBytes [
	| arr kid iv tag unprotected |
	tag := GtsCborReader decode: coseBytes.
	((tag isKindOf: GtsCborTag) and: [ tag tag = 16 ])
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Encrypt0' ].
	arr := tag value.
	(arr isArray and: [ arr size = 3 ])
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Encrypt0' ].
	unprotected := arr at: 2.
	unprotected isDictionary ifFalse: [ GtsCryptoError signal: 'malformed COSE_Encrypt0' ].
	kid := unprotected at: 4 ifAbsent: [ GtsCryptoError signal: 'malformed COSE_Encrypt0' ].
	iv := unprotected at: 5 ifAbsent: [ GtsCryptoError signal: 'malformed COSE_Encrypt0' ].
	^ Dictionary newFromPairs: {
		'kid'. kid utf8Decoded.
		'protected'. (arr at: 1).
		'iv'. iv.
		'ciphertext'. (arr at: 3) }
]

{ #category : #sign1 }
GtsCose class >> parseSign1: coseBytes [
	| arr kid tag unprotected |
	tag := GtsCborReader decode: coseBytes.
	((tag isKindOf: GtsCborTag) and: [ tag tag = 18 ])
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	arr := tag value.
	(arr isArray and: [ arr size = 4 ])
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	(arr at: 3) isNil ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	unprotected := arr at: 2.
	unprotected isDictionary ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	kid := unprotected at: 4 ifAbsent: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	^ Dictionary newFromPairs: {
		'kid'. kid utf8Decoded.
		'protected'. (arr at: 1).
		'signature'. (arr at: 4) }
]

{ #category : #private }
GtsCose class >> sign1Protected [
	^ GtsCborWriter encode: (Dictionary newFromPairs: { 1. -8 })
]

{ #category : #sign1 }
GtsCose class >> signFrameId: frameId secretKey: secretKey kid: kid [
	| protected signature |
	protected := self sign1Protected.
	signature := GtsSodium
		sign: (self sigStructureProtected: protected frameId: frameId)
		secretKey: secretKey.
	^ GtsCborWriter encode: (GtsCborTag tag: 18 value: {
		protected.
		Dictionary newFromPairs: { 4. kid utf8Encoded }.
		nil.
		signature })
]

{ #category : #private }
GtsCose class >> sigStructureProtected: protected frameId: frameId [
	^ GtsCborWriter encode: { 'Signature1'. protected. ByteArray new. frameId }
]

{ #category : #sign1 }
GtsCose class >> verifySign1: coseBytes frameId: frameId publicKey: publicKey [
	| parsed |
	parsed := self parseSign1: coseBytes.
	^ GtsSodium
		verify: (parsed at: 'signature')
		message: (self sigStructureProtected: (parsed at: 'protected') frameId: frameId)
		publicKey: publicKey
]
