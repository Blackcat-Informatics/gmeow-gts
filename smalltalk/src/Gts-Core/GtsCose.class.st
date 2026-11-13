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
GtsCose class >> maxHeaderDepth [
	"The deepest nesting a protected header may have; the header map is level 1."

	^ 8
]

{ #category : #private }
GtsCose class >> scanHead: buf at: pos [
	"Answer { major. info. argument. next } for the item head at pos, or nil for
	an indefinite length, a reserved additional info, a truncated head, or an
	argument that is not in shortest form."

	| byte major info width arg minimum |
	pos > buf size ifTrue: [ ^ nil ].
	byte := buf at: pos.
	major := byte bitShift: -5.
	info := byte bitAnd: 31.
	info < 24 ifTrue: [ ^ { major. info. info. pos + 1 } ].
	info > 27 ifTrue: [ ^ nil ].
	width := 1 bitShift: info - 24.
	buf size - pos < width ifTrue: [ ^ nil ].
	arg := 0.
	1 to: width do: [ :i | arg := (arg bitShift: 8) + (buf at: pos + i) ].
	minimum := #(24 256 65536 4294967296) at: info - 23.
	arg < minimum ifTrue: [ ^ nil ].
	^ { major. info. arg. pos + 1 + width }
]

{ #category : #private }
GtsCose class >> compareBytes: a with: b [
	"Bytewise lexicographic comparison: -1, 0 or 1."

	1 to: (a size min: b size) do: [ :i |
		(a at: i) = (b at: i) ifFalse: [ ^ (a at: i) < (b at: i) ifTrue: [ -1 ] ifFalse: [ 1 ] ] ].
	^ a size = b size
		ifTrue: [ 0 ]
		ifFalse: [ a size < b size ifTrue: [ -1 ] ifFalse: [ 1 ] ]
]

{ #category : #private }
GtsCose class >> isValidUtf8: bytes [
	"True if bytes are well-formed UTF-8 by the Unicode table of well-formed byte
	sequences: no overlong form, no encoded surrogate, nothing above U+10FFFF, and
	no stray continuation byte or truncated sequence. Explicit, so it does not
	depend on how a particular decoder treats these cases."

	| i n b need lo hi |
	i := 1.
	n := bytes size.
	[ i <= n ] whileTrue: [
		b := bytes at: i.
		b < 16r80
			ifTrue: [ i := i + 1 ]
			ifFalse: [
				need := nil.
				lo := 16r80.
				hi := 16rBF.
				(b between: 16rC2 and: 16rDF) ifTrue: [ need := 1 ].
				(b between: 16rE0 and: 16rEF) ifTrue: [ need := 2 ].
				(b between: 16rF0 and: 16rF4) ifTrue: [ need := 3 ].
				need isNil ifTrue: [ ^ false ].
				b = 16rE0 ifTrue: [ lo := 16rA0 ].
				b = 16rED ifTrue: [ hi := 16r9F ].
				b = 16rF0 ifTrue: [ lo := 16r90 ].
				b = 16rF4 ifTrue: [ hi := 16r8F ].
				i + need > n ifTrue: [ ^ false ].
				((bytes at: i + 1) between: lo and: hi) ifFalse: [ ^ false ].
				2 to: need do: [ :k |
					((bytes at: i + k) between: 16r80 and: 16rBF) ifFalse: [ ^ false ] ].
				i := i + need + 1 ] ].
	^ true
]

{ #category : #private }
GtsCose class >> scanHeaderItem: buf at: pos depth: depth isKey: isKey [
	"Check one protected-header item against the grammar in declaredAlgorithm:.
	Answer the position after the item, or 0 if it is not allowed. depth is the
	level of this item if it is a container, so recursion is bounded by
	maxHeaderDepth, and every item consumes a byte, so huge counts end at the end
	of the buffer."

	| head major info arg next at previous keyEnd key end |
	head := self scanHead: buf at: pos.
	head isNil ifTrue: [ ^ 0 ].
	major := head at: 1.
	info := head at: 2.
	arg := head at: 3.
	next := head at: 4.
	(isKey and: [ (#(0 1 3) includes: major) not ]) ifTrue: [ ^ 0 ].
	(major = 0 or: [ major = 1 ]) ifTrue: [
		"Integers lie in -2^63 .. 2^63-1."
		^ arg < (1 bitShift: 63) ifTrue: [ next ] ifFalse: [ 0 ] ].
	(major = 2 or: [ major = 3 ]) ifTrue: [
		arg > (buf size - next + 1) ifTrue: [ ^ 0 ].
		end := next + arg.
		(major = 3 and: [ (self isValidUtf8: (buf copyFrom: next to: end - 1)) not ])
			ifTrue: [ ^ 0 ].
		^ end ].
	(major = 4 or: [ major = 5 ]) ifTrue: [
		depth > self maxHeaderDepth ifTrue: [ ^ 0 ].
		at := next.
		previous := nil.
		1 to: arg do: [ :index |
			major = 4
				ifTrue: [
					at := self scanHeaderItem: buf at: at depth: depth + 1 isKey: false.
					at = 0 ifTrue: [ ^ 0 ] ]
				ifFalse: [
					keyEnd := self scanHeaderItem: buf at: at depth: depth + 1 isKey: true.
					keyEnd = 0 ifTrue: [ ^ 0 ].
					key := buf copyFrom: at to: keyEnd - 1.
					"Strictly ascending bytewise: sorted, and no duplicate key."
					(previous notNil and: [ (self compareBytes: previous with: key) >= 0 ])
						ifTrue: [ ^ 0 ].
					previous := key.
					at := self scanHeaderItem: buf at: keyEnd depth: depth + 1 isKey: false.
					at = 0 ifTrue: [ ^ 0 ] ] ].
		^ at ].
	"null, true and false only."
	major = 7 ifTrue: [ ^ (info >= 20 and: [ info <= 22 ]) ifTrue: [ next ] ifFalse: [ 0 ] ].
	^ 0
]

{ #category : #private }
GtsCose class >> declaredAlgorithm: protected [
	"The alg (label 1) of a protected header (RFC 9052 section 3.1).

	The protected header is a byte string whose content is deterministically
	encoded CBOR (definite lengths, shortest-form integers and lengths, map keys
	in bytewise order of their encodings, no duplicate keys, no trailing bytes)
	consisting of exactly one map. Every map key, at any depth, is an integer or a
	text string. Every value, at any depth, is an integer, a byte string, a text
	string, an array, a map, null, true or false. Every integer, key or value,
	lies in the range -2^63 to 2^63-1 inclusive. Text strings are valid UTF-8.
	Nesting is at most 8 levels deep, counting the header map itself as level 1.
	The map contains the integer key 1, whose value is an integer or a text
	string. A header that does not meet all of this is malformed and the
	signature is invalid.

	Enforced by scanning the raw bytes with scanHeaderItem:at:depth:isKey:, so no
	reader behaviour is involved. Signals GtsCryptoError if the header is
	malformed or declares no algorithm."

	| root at count keyEnd valueEnd value |
	((protected isKindOf: ByteArray) and: [ protected notEmpty ])
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	(protected first bitShift: -5) = 5
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	(self scanHeaderItem: protected at: 1 depth: 1 isKey: false) = (protected size + 1)
		ifFalse: [ GtsCryptoError signal: 'malformed COSE_Sign1' ].
	root := self scanHead: protected at: 1.
	count := root at: 3.
	at := root at: 4.
	1 to: count do: [ :index |
		keyEnd := self scanHeaderItem: protected at: at depth: 2 isKey: true.
		valueEnd := self scanHeaderItem: protected at: keyEnd depth: 2 isKey: false.
		(keyEnd - at = 1 and: [ (protected at: at) = 1 ]) ifTrue: [
			value := self scanHead: protected at: keyEnd.
			(value at: 1) = 0 ifTrue: [ ^ value at: 3 ].
			(value at: 1) = 1 ifTrue: [ ^ -1 - (value at: 3) ].
			(value at: 1) = 3 ifTrue: [
				^ (protected copyFrom: (value at: 4) to: valueEnd - 1) utf8Decoded ].
			GtsCryptoError signal: 'malformed COSE_Sign1' ].
		at := valueEnd ].
	^ GtsCryptoError signal: 'malformed COSE_Sign1'
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
