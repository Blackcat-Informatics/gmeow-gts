Class {
	#name : #GtsWriterTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsWriterTest >> readMinimalVectorBytes [
	^ '/workspace/vectors/01-minimal.gts' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #tests }
GtsWriterTest >> testWriterReproducesMinimalVector [
	| bytes writer |
	writer := GtsWriter forProfile: 'dist'.
	writer addTerms: {
		GtsTerm iri: 'https://example.org/Cat'.
		GtsTerm iri: 'http://www.w3.org/2000/01/rdf-schema#label'.
		GtsTerm literal: 'Cat' language: 'en' datatype: nil }.
	writer addQuads: { #(0 1 2) }.
	bytes := writer bytes.
	self assert: (GtsHex encode: bytes) equals: (GtsHex encode: self readMinimalVectorBytes)
]
