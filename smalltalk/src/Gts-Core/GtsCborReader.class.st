Class {
	#name : #GtsCborReader,
	#superclass : #Object,
	#instVars : [
		'bytes',
		'position'
	],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsCborReader class >> on: aByteArray [
	^ self new initializeOn: aByteArray
]

{ #category : #decoding }
GtsCborReader class >> decode: aByteArray [
	^ (self on: aByteArray) readItem
]

{ #category : #decoding }
GtsCborReader class >> itemsFrom: aByteArray [
	| reader items |
	reader := self on: aByteArray.
	items := OrderedCollection new.
	[ reader atEnd ] whileFalse: [ items add: reader readItem ].
	^ items asArray
]

{ #category : #accessing }
GtsCborReader >> atEnd [
	^ position > bytes size
]

{ #category : #initialization }
GtsCborReader >> initializeOn: aByteArray [
	bytes := aByteArray asByteArray.
	position := 1.
	^ self
]

{ #category : #private }
GtsCborReader >> nextByte [
	| byte |
	self atEnd ifTrue: [ self error: 'truncated CBOR item' ].
	byte := bytes at: position.
	position := position + 1.
	^ byte
]

{ #category : #private }
GtsCborReader >> readArgument: additional [
	additional < 24 ifTrue: [ ^ additional ].
	additional = 24 ifTrue: [ ^ self readUnsignedIntegerBytes: 1 ].
	additional = 25 ifTrue: [ ^ self readUnsignedIntegerBytes: 2 ].
	additional = 26 ifTrue: [ ^ self readUnsignedIntegerBytes: 4 ].
	additional = 27 ifTrue: [ ^ self readUnsignedIntegerBytes: 8 ].
	self error: 'indefinite-length CBOR is not supported'
]

{ #category : #decoding }
GtsCborReader >> readItem [
	| additional argument initial major |
	initial := self nextByte.
	major := initial // 32.
	additional := initial \\ 32.
	argument := self readArgument: additional.
	major = 0 ifTrue: [ ^ argument ].
	major = 1 ifTrue: [ ^ -1 - argument ].
	major = 2 ifTrue: [ ^ self readByteString: argument ].
	major = 3 ifTrue: [ ^ (self readByteString: argument) utf8Decoded ].
	major = 4 ifTrue: [ ^ self readArray: argument ].
	major = 5 ifTrue: [ ^ self readMap: argument ].
	major = 6 ifTrue: [ ^ GtsCborTag tag: argument value: self readItem ].
	major = 7 ifTrue: [ ^ self readSimple: additional argument: argument ].
	self error: 'unsupported CBOR major type'
]

{ #category : #private }
GtsCborReader >> readArray: count [
	| values |
	values := Array new: count.
	1 to: count do: [ :index | values at: index put: self readItem ].
	^ values
]

{ #category : #private }
GtsCborReader >> readByteString: count [
	| out |
	position + count - 1 > bytes size ifTrue: [ self error: 'truncated CBOR byte string' ].
	out := bytes copyFrom: position to: position + count - 1.
	position := position + count.
	^ out
]

{ #category : #private }
GtsCborReader >> readMap: count [
	| map |
	map := Dictionary new.
	1 to: count do: [ :index | map at: self readItem put: self readItem ].
	^ map
]

{ #category : #private }
GtsCborReader >> readSimple: additional argument: argument [
	additional = 20 ifTrue: [ ^ false ].
	additional = 21 ifTrue: [ ^ true ].
	additional = 22 ifTrue: [ ^ nil ].
	self error: 'unsupported CBOR simple value'
]

{ #category : #private }
GtsCborReader >> readUnsignedIntegerBytes: byteCount [
	| value |
	value := 0.
	1 to: byteCount do: [ :index | value := (value bitShift: 8) + self nextByte ].
	^ value
]

{ #category : #private }
GtsCborReader >> skipToEnd [
	position := bytes size + 1
]
