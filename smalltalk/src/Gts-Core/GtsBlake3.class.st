Class {
	#name : #GtsBlake3,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #accessing }
GtsBlake3 class >> digestSize [
	^ GtsBlake3Library uniqueInstance outLength
]

{ #category : #hashing }
GtsBlake3 class >> digestString: aByteArray [
	^ 'blake3:', (self hexDigest: aByteArray)
]

{ #category : #hashing }
GtsBlake3 class >> hash: aByteArray [
	| inputBytes digest |
	inputBytes := aByteArray asByteArray.
	digest := ByteArray new: self digestSize.
	GtsBlake3Library uniqueInstance
		hashInput: inputBytes
		length: inputBytes size
		output: digest.
	^ digest
]

{ #category : #hashing }
GtsBlake3 class >> deriveWith: context prefix: prefix bytes: input from: start to: finish [
	| digest contextBytes |
	(start >= 1 and: [ start <= finish and: [ finish <= (input size + 1) ] ])
		ifFalse: [ GtsCryptoError signal: 'invalid BLAKE3 input range' ].
	contextBytes := context asByteArray.
	digest := ByteArray new: self digestSize.
	GtsBlake3Library uniqueInstance
		deriveWith: contextBytes
		contextLength: contextBytes size
		prefix: prefix
		prefixLength: prefix size
		input: input
		start: start - 1
		length: finish - start
		output: digest.
	^ digest
]

{ #category : #hashing }
GtsBlake3 class >> hexDigest: aByteArray [
	^ GtsHex encode: (self hash: aByteArray)
]
