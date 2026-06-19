Class {
	#name : #GtsMinimalVectorTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsMinimalVectorTest >> readMinimalVectorBytes [
	^ '/workspace/vectors/01-minimal.gts' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #tests }
GtsMinimalVectorTest >> testMinimalVectorMatchesCommittedCorpusBytes [
	| expected generated |
	expected := self readMinimalVectorBytes.
	generated := GtsMinimalVectorFixture bytes.
	self assert: generated size equals: expected size.
	self assert: (GtsHex encode: generated) equals: (GtsHex encode: expected)
]
