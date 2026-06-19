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
	| inputBytes inputSize digestSize |
	inputBytes := aByteArray asByteArray.
	inputSize := inputBytes size.
	digestSize := self digestSize.
	^ ExternalAddress
		allocate: (inputSize max: 1)
		allocate: digestSize
		during: [ :inputAddress :outputAddress |
			| digest |
			1 to: inputSize do: [ :index |
				inputAddress unsignedByteAt: index put: (inputBytes at: index) ].
			GtsBlake3Library uniqueInstance
				hashInput: inputAddress
				length: inputSize
				output: outputAddress.
			digest := ByteArray new: digestSize.
			1 to: digestSize do: [ :index |
				digest at: index put: (outputAddress unsignedByteAt: index) ].
			digest ]
]

{ #category : #hashing }
GtsBlake3 class >> hexDigest: aByteArray [
	^ GtsHex encode: (self hash: aByteArray)
]
