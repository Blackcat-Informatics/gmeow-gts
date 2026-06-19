Class {
	#name : #GtsCborWriter,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #encoding }
GtsCborWriter class >> encode: anObject [
	^ ByteArray streamContents: [ :stream | self write: anObject on: stream ]
]

{ #category : #private }
GtsCborWriter class >> encodedByteKey: left isLessThan: right [
	1 to: (left size min: right size) do: [ :index |
		(left at: index) = (right at: index)
			ifFalse: [ ^ (left at: index) < (right at: index) ] ].
	^ left size < right size
]

{ #category : #private }
GtsCborWriter class >> write: anObject on: stream [
	anObject isInteger ifTrue: [ ^ self writeInteger: anObject on: stream ].
	(anObject isKindOf: ByteArray) ifTrue: [ ^ self writeByteString: anObject on: stream ].
	anObject isSymbol ifTrue: [ ^ self writeTextString: anObject asString on: stream ].
	anObject isString ifTrue: [ ^ self writeTextString: anObject on: stream ].
	(anObject isKindOf: Association) ifTrue: [
		^ self writeMap: (Dictionary with: anObject) on: stream ].
	anObject isDictionary ifTrue: [ ^ self writeMap: anObject on: stream ].
	((anObject isArray) or: [ anObject isKindOf: OrderedCollection ])
		ifTrue: [ ^ self writeArray: anObject on: stream ].
	self error: 'unsupported CBOR value: ', anObject class name
]

{ #category : #private }
GtsCborWriter class >> writeArray: aCollection on: stream [
	self writeMajor: 4 argument: aCollection size on: stream.
	aCollection do: [ :each | self write: each on: stream ]
]

{ #category : #private }
GtsCborWriter class >> writeBigEndian: value width: byteCount on: stream [
	1 to: byteCount do: [ :index |
		stream nextPut: ((value bitShift: ((byteCount - index) * -8)) bitAnd: 16rFF) ]
]

{ #category : #private }
GtsCborWriter class >> writeByteString: aByteArray on: stream [
	self writeMajor: 2 argument: aByteArray size on: stream.
	stream nextPutAll: aByteArray
]

{ #category : #private }
GtsCborWriter class >> writeInteger: value on: stream [
	value >= 0
		ifTrue: [ ^ self writeMajor: 0 argument: value on: stream ].
	self writeMajor: 1 argument: value negated - 1 on: stream
]

{ #category : #private }
GtsCborWriter class >> writeMajor: major argument: value on: stream [
	value < 24 ifTrue: [ ^ stream nextPut: (major * 32) + value ].
	value <= 16rFF ifTrue: [
		stream nextPut: (major * 32) + 24.
		^ self writeBigEndian: value width: 1 on: stream ].
	value <= 16rFFFF ifTrue: [
		stream nextPut: (major * 32) + 25.
		^ self writeBigEndian: value width: 2 on: stream ].
	value <= 16rFFFFFFFF ifTrue: [
		stream nextPut: (major * 32) + 26.
		^ self writeBigEndian: value width: 4 on: stream ].
	value <= 16rFFFFFFFFFFFFFFFF ifTrue: [
		stream nextPut: (major * 32) + 27.
		^ self writeBigEndian: value width: 8 on: stream ].
	self error: 'CBOR integer too large'
]

{ #category : #private }
GtsCborWriter class >> writeMap: aDictionary on: stream [
	| encodedPairs |
	self writeMajor: 5 argument: aDictionary size on: stream.
	encodedPairs := aDictionary associations collect: [ :association |
		{ (self encode: association key). association value } ].
	encodedPairs := encodedPairs asSortedCollection: [ :left :right |
		self encodedByteKey: left first isLessThan: right first ].
	encodedPairs do: [ :pair |
		stream nextPutAll: pair first.
		self write: pair second on: stream ]
]

{ #category : #private }
GtsCborWriter class >> writeTextString: aString on: stream [
	| bytes |
	bytes := aString utf8Encoded.
	self writeMajor: 3 argument: bytes size on: stream.
	stream nextPutAll: bytes
]
