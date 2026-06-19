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
GtsBlake3 class >> hexDigest: aByteArray [
	^ GtsHex encode: (self hash: aByteArray)
]
