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
GtsBlake3 class >> deriveWith: context bytes: input [
	^ self deriveWith: context prefix: #[] bytes: input from: 1 to: input size + 1
]

{ #category : #hashing }
GtsBlake3 class >> deriveWith: context prefix: prefix bytes: input from: start to: finish [
	| digest contextBytes |
	self checkRangeOf: input from: start to: finish.
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
GtsBlake3 class >> deriveWith: context updatingWith: aBlock [
	| library contextBytes state digest |
	library := GtsBlake3Library uniqueInstance.
	contextBytes := context asByteArray.
	digest := ByteArray new: self digestSize.
	state := library deriveStateFor: contextBytes length: contextBytes size.
	(state isNil or: [ state isNull ]) ifTrue: [ GtsCryptoError signal: 'cannot allocate BLAKE3 state' ].
	[
		aBlock value: [ :input :start :finish |
			state ifNil: [ GtsCryptoError signal: 'BLAKE3 state has been released' ].
			self checkRangeOf: input from: start to: finish.
			library update: state input: input start: start - 1 length: finish - start ].
		library finish: state output: digest
	] ensure: [
		| released |
		released := state.
		state := nil.
		library free: released ].
	^ digest
]

{ #category : #private }
GtsBlake3 class >> checkRangeOf: input from: start to: finish [
	(start >= 1 and: [ start <= finish and: [ finish <= (input size + 1) ] ])
		ifFalse: [ GtsCryptoError signal: 'invalid BLAKE3 input range' ]
]

{ #category : #hashing }
GtsBlake3 class >> hexDigest: aByteArray [
	^ GtsHex encode: (self hash: aByteArray)
]
