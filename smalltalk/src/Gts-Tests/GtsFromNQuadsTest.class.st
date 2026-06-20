Class {
	#name : #GtsFromNQuadsTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsFromNQuadsTest >> sortedLines: text [
	^ ((text lines collect: [ :line | line trimBoth ])
		select: [ :line | line isEmpty not ]) asSortedCollection asArray
]

{ #category : #private }
GtsFromNQuadsTest >> roundTrip: nquads [
	^ GtsNQuads toNQuads: (GtsReader read: (GtsFromNQuads fromString: nquads))
]

{ #category : #tests }
GtsFromNQuadsTest >> testFromNQuadsInvertsFoldOutputForCorpusVector [
	| graph nquads roundTripped |
	graph := GtsReader read: ('/workspace/vectors/11-datatype-defaulting.gts' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]).
	nquads := GtsNQuads toNQuads: graph.
	roundTripped := self roundTrip: nquads.
	self assert: (self sortedLines: roundTripped) equals: (self sortedLines: nquads)
]

{ #category : #tests }
GtsFromNQuadsTest >> testFromNQuadsPreservesLanguageAndDatatypeLiterals [
	| nquads roundTripped xsdInteger |
	xsdInteger := 'http://www.w3.org/2001/XMLSchema#integer'.
	nquads := '<https://ex/s> <https://ex/label> "Cat"@en .', String lf,
		'<https://ex/s> <https://ex/n> "42"^^<', xsdInteger, '> .', String lf,
		'_:b0 <https://ex/p> <https://ex/s> .', String lf.
	roundTripped := self roundTrip: nquads.
	self assert: (self sortedLines: roundTripped) equals: (self sortedLines: nquads)
]

{ #category : #tests }
GtsFromNQuadsTest >> testFromNQuadsQuotedTripleReifierRoundTrip [
	| expected nquads rdfReifies roundTripped |
	rdfReifies := 'http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies'.
	nquads := '<https://ex/r1> <', rdfReifies, '> <<( _:b0 <https://ex/p> _:b1)>> .', String lf,
		'<https://ex/r2> <', rdfReifies, '> <<( <https://ex/s> <https://ex/p> "Cat"@en)>> .', String lf.
	expected := '<https://ex/r1> <', rdfReifies, '> <<( _:b0 <https://ex/p> _:b1 )>> .', String lf,
		'<https://ex/r2> <', rdfReifies, '> <<( <https://ex/s> <https://ex/p> "Cat"@en )>> .', String lf.
	roundTripped := self roundTrip: nquads.
	self assert: (self sortedLines: roundTripped) equals: (self sortedLines: expected)
]

{ #category : #tests }
GtsFromNQuadsTest >> testFromNQuadsRejectsMalformedStatements [
	self
		should: [ GtsFromNQuads fromString: '<https://ex/s> <https://ex/p> .', String lf ]
		raise: Error.
	self
		should: [ GtsFromNQuads fromString: '<https://ex/s> <https://ex/p> _: .', String lf ]
		raise: Error.
	self
		should: [ GtsFromNQuads fromString: '<https://ex/s> <https://ex/p> "Cat"@ .', String lf ]
		raise: Error
]
