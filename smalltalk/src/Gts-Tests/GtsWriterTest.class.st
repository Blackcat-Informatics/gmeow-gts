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
GtsWriterTest >> testWriterEmitsSelfDescribingTripleTerms [
	| graph writer |
	writer := GtsWriter forProfile: 'generic'.
	writer addTerms: {
		GtsTerm iri: 'https://ex/s'.
		GtsTerm iri: 'https://ex/p'.
		GtsTerm iri: 'https://ex/o'.
		GtsTerm tripleComponents: #(0 1 2) }.
	writer addQuads: { #(3 1 2) }.
	graph := GtsReader read: writer bytes.
	self assert: graph diagnostics isEmpty.
	self assert: graph reifiers isEmpty.
	self assert: (graph tripleForTermId: 3) equals: #(0 1 2).
	self
		assert: (GtsNQuads toNQuads: graph)
		equals: '<<( <https://ex/s> <https://ex/p> <https://ex/o> )>> <https://ex/p> <https://ex/o> .', String lf
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
