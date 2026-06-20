Class {
	#name : #GtsHex,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #converting }
GtsHex class >> encode: aByteArray [
	| digits |
	digits := '0123456789abcdef'.
	^ String streamContents: [ :stream |
		aByteArray do: [ :byte |
			stream
				nextPut: (digits at: (byte // 16) + 1);
				nextPut: (digits at: (byte \\ 16) + 1) ] ]
]

{ #category : #converting }
GtsHex class >> decode: aString [
	| bytes |
	aString size even ifFalse: [ self error: 'hex string length must be even' ].
	bytes := ByteArray new: aString size // 2.
	1 to: bytes size do: [ :index |
		bytes
			at: index
			put: ((self nibbleFor: (aString at: (index * 2) - 1)) * 16
				+ (self nibbleFor: (aString at: index * 2))) ].
	^ bytes
]

{ #category : #private }
GtsHex class >> nibbleFor: character [
	| code |
	code := character asciiValue.
	(code between: $0 asciiValue and: $9 asciiValue)
		ifTrue: [ ^ code - $0 asciiValue ].
	(code between: $a asciiValue and: $f asciiValue)
		ifTrue: [ ^ code - $a asciiValue + 10 ].
	(code between: $A asciiValue and: $F asciiValue)
		ifTrue: [ ^ code - $A asciiValue + 10 ].
	self error: 'invalid hex character'
]
