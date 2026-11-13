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

{ #category : #private }
GtsCose class >> isPlainHeaderValue: anObject [
	"True if anObject holds only integers, byte strings, text strings, arrays,
	maps, nil, true and false at any depth: no tag or other value."

	anObject isNil ifTrue: [ ^ true ].
	(anObject == true or: [ anObject == false ]) ifTrue: [ ^ true ].
	anObject isInteger ifTrue: [ ^ true ].
	(anObject isKindOf: ByteArray) ifTrue: [ ^ true ].
	anObject isString ifTrue: [ ^ true ].
	anObject isArray ifTrue: [ ^ anObject allSatisfy: [ :each | self isPlainHeaderValue: each ] ].
	anObject isDictionary ifTrue: [
		^ anObject associations allSatisfy: [ :each |
			(self isPlainHeaderValue: each key) and: [ self isPlainHeaderValue: each value ] ] ].
	^ false
]

{ #category : #private }
GtsCose class >> declaredAlgorithm: protected [
	"The alg (label 1) of a protected header (RFC 9052 section 3.1).

	Header rule (identical in every engine): the protected header must be
	deterministically encoded CBOR (RFC 8949 section 4.2, GTS-SPEC section 4)
	holding exactly one map, containing only integers, byte strings, text strings,
	arrays, maps, null, true and false (no float, tag or other simple value at any
	depth), and its integer key 1 must map to an integer or text string.

	Here: the reader signals on indefinite lengths and floats; the bytes must equal
	the writer's encoding of what they decode to (non-shortest integers, unsorted
	keys and duplicate keys, which a Dictionary collapses, all differ); and
	isPlainHeaderValue: rejects tags and other values. Signals GtsCryptoError if
	the header is malformed or declares no algorithm."

	| algorithm header |
	(protected isKindOf: ByteArray)
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	header := [ | item reader |
		reader := GtsCborReader on: protected.
		item := reader readItem.
		reader atEnd ifFalse: [ Error signal: 'trailing CBOR bytes' ].
		(self isPlainHeaderValue: item)
			ifFalse: [ Error signal: 'protected header holds a tag or other value' ].
		(GtsCborWriter encode: item) = protected
			ifFalse: [ Error signal: 'protected header is not deterministically encoded' ].
		item ]
		on: Error
		do: [ :error | GtsCryptoError signal: 'malformed COSE_Sign1' ].
	header isDictionary ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	algorithm := header at: 1 ifAbsent: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	(algorithm isInteger or: [ algorithm isString ])
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	^ algorithm
]

{ #category : #sign1 }
GtsCose class >> eddsaAlgorithm [
	"The COSE algorithm identifier for EdDSA, the only signing algorithm this engine verifies."

	^ -8
]

{ #category : #sign1 }
GtsCose class >> parseSign1: coseBytes [
	"Answer kid, alg, protected and signature. alg is the declared COSE algorithm
	identifier (an integer or a string). Signals GtsCryptoError if malformed."

	| arr kid tag unprotected |
	tag := GtsCborReader decode: coseBytes.
	((tag isKindOf: GtsCborTag) and: [ tag tag = 18 ])
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	arr := tag value.
	(arr isArray and: [ arr size = 4 ])
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	(arr at: 3) isNil ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	((arr at: 4) isKindOf: ByteArray) ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	unprotected := arr at: 2.
	unprotected isDictionary ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	kid := unprotected at: 4 ifAbsent: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	^ Dictionary newFromPairs: {
		'kid'. kid utf8Decoded.
		'alg'. (self declaredAlgorithm: (arr at: 1)).
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
	"Answer whether the EdDSA signature is valid. A COSE_Sign1 that declares any
	other algorithm is never valid here; GtsCLI verifySignature:keys: reports it
	as unverified (GTS-SPEC section 9.2)."

	| parsed |
	parsed := self parseSign1: coseBytes.
	(parsed at: 'alg') = self eddsaAlgorithm ifFalse: [ ^ false ].
	^ GtsSodium
		verify: (parsed at: 'signature')
		message: (self sigStructureProtected: (parsed at: 'protected') frameId: frameId)
		publicKey: publicKey
]
