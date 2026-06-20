Class {
	#name : #GtsCompactTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsCompactTest >> vectorBytes: name [
	^ ('/workspace/vectors/', name) asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #tests }
GtsCompactTest >> testCompactReproducesFrozenStreamableVector [
	| compacted expected |
	compacted := GtsCompact
		compactStreamable: (self vectorBytes: '25-streamable-source.gts')
		timestamp: '2026-01-01T00:00:00Z'
		sealOriginal: false.
	expected := self vectorBytes: '25b-streamable-compacted.gts'.
	self assert: (GtsHex encode: compacted) equals: (GtsHex encode: expected)
]

{ #category : #tests }
GtsCompactTest >> testCompactRefusesEvidenceWithoutSeal [
	| writer |
	writer := GtsWriter forProfile: 'evidence'.
	writer addTerms: {
		GtsTerm iri: 'https://example.org/Cat'.
		GtsTerm iri: 'http://www.w3.org/2000/01/rdf-schema#label'.
		GtsTerm literal: 'Cat' language: 'en' datatype: nil }.
	writer addQuads: { #(0 1 2) }.
	self
		should: [
			GtsCompact
				compactStreamable: writer bytes
				timestamp: '2026-01-01T00:00:00Z'
				sealOriginal: false ]
		raise: Error
]
