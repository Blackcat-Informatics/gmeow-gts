Class {
	#name : #GtsCborReaderTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsCborReaderTest >> readMinimalVectorBytes [
	^ '/workspace/vectors/01-minimal.gts' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #tests }
GtsCborReaderTest >> testDecodePrimitiveValues [
	| map |
	self assert: (GtsCborReader decode: #[0]) equals: 0.
	self assert: (GtsCborReader decode: #[24 24]) equals: 24.
	self assert: (GtsCborReader decode: #[32]) equals: -1.
	self assert: (GtsCborReader decode: #[67 1 2 255]) equals: #[1 2 255].
	self assert: (GtsCborReader decode: #[99 103 116 115]) equals: 'gts'.
	self assert: (GtsCborReader decode: #[131 1 2 3]) equals: #(1 2 3).
	map := GtsCborReader decode: #[162 97 97 1 97 98 2].
	self assert: (map at: 'a') equals: 1.
	self assert: (map at: 'b') equals: 2
]

{ #category : #tests }
GtsCborReaderTest >> testDecodeSelfDescribedMinimalVector [
	| frames header items |
	items := GtsCborReader itemsFrom: self readMinimalVectorBytes.
	self assert: items size equals: 3.
	self assert: items first class equals: GtsCborTag.
	self assert: items first tag equals: 55799.
	header := items first value.
	self assert: (header at: 'gts') equals: 'GTS1'.
	self assert: (header at: 'prof') equals: 'dist'.
	self assert: ((header at: 'id') size) equals: 32.
	frames := items allButFirst.
	self assert: ((frames first at: 't') = 'terms').
	self assert: ((frames second at: 't') = 'quads')
]

{ #category : #tests }
GtsCborReaderTest >> testRejectsTruncatedByteString [
	self should: [ GtsCborReader decode: #[66 1] ] raise: Error
]
