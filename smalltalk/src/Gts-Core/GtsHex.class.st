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
