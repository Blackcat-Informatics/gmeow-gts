Class {
	#name : #GtsHex,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #converting }
GtsHex class >> encode: aByteArray [
	| digits stream |
	digits := '0123456789abcdef'.
	stream := String new writeStream.
	aByteArray do: [ :byte |
		stream
			nextPut: (digits at: (byte // 16) + 1);
			nextPut: (digits at: (byte \\ 16) + 1) ].
	^ stream contents
]
