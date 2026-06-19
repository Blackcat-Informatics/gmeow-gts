Class {
	#name : #GtsReaderTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsReaderTest >> readMinimalVectorBytes [
	^ '/workspace/vectors/01-minimal.gts' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #private }
GtsReaderTest >> readZstdVectorBytes [
	^ '/workspace/vectors/02-zstd-frame.gts' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #tests }
GtsReaderTest >> testMinimalVectorFoldsToExpectedGraph [
	| graph |
	graph := GtsReader read: self readMinimalVectorBytes.
	self assert: graph diagnostics isEmpty.
	self assert: graph terms size equals: 3.
	self assert: graph quads size equals: 1.
	self assert: graph profiles equals: #('dist').
	self
		assert: (GtsHex encode: graph segmentHeads first)
		equals: 'ec5a15cbe3b79c333712d64ed83a70e69a2d1be8c1316835727e5d5219823cd9'
]

{ #category : #tests }
GtsReaderTest >> testMinimalVectorProjectsToNQuads [
	| graph |
	graph := GtsReader read: self readMinimalVectorBytes.
	self
		assert: (GtsNQuads toNQuads: graph)
		equals: '<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> "Cat"@en .', String lf
]

{ #category : #tests }
GtsReaderTest >> testZstdVectorFoldsToExpectedGraph [
	| graph |
	graph := GtsReader read: self readZstdVectorBytes.
	self assert: graph diagnostics isEmpty.
	self assert: graph terms size equals: 3.
	self assert: graph quads size equals: 1.
	self assert: graph profiles equals: #('generic').
	self
		assert: (GtsHex encode: graph segmentHeads first)
		equals: '4bdbb88d5c994034359346c2c66fe46bb0f9c3670365086cdf3a8282c6bd5c9f'.
	self
		assert: (GtsNQuads toNQuads: graph)
		equals: '<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> "Cat"@en .', String lf
]
