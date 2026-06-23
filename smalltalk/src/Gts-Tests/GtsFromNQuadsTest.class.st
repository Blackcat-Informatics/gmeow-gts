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
GtsFromNQuadsTest >> testFromNQuadsPreservesDirectionalLanguageLiterals [
	| graph literal nquads |
	nquads := '<https://ex/s> <https://ex/label> "RTL"@ar--rtl .', String lf.
	graph := GtsReader read: (GtsFromNQuads fromString: nquads).
	literal := graph terms detect: [ :term | term kind = #literal ].
	self assert: literal language equals: 'ar'.
	self assert: literal direction equals: 'rtl'.
	self assert: (graph datatypeIriForTerm: literal) equals: GtsGraph rdfDirLangString.
	self assert: (self sortedLines: (GtsNQuads toNQuads: graph)) equals: (self sortedLines: nquads)
]

{ #category : #tests }
GtsFromNQuadsTest >> testFromNQuadsPreservesMultipleReifiersForSameStatement [
	| graph nquads rdfReifies |
	rdfReifies := 'http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies'.
	nquads := '<https://ex/r1> <', rdfReifies, '> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .', String lf,
		'<https://ex/r2> <', rdfReifies, '> <<( <https://ex/s> <https://ex/p> <https://ex/o> )>> .', String lf.
	graph := GtsReader read: (GtsFromNQuads fromString: nquads).
	self assert: graph reifiers size equals: 2.
	self assert: (self sortedLines: (GtsNQuads toNQuads: graph)) equals: (self sortedLines: nquads)
]

{ #category : #tests }
GtsFromNQuadsTest >> testWriterAllowsMultipleReifiersForSameStatement [
	| graph writer |
	writer := GtsWriter forProfile: 'dist'.
	writer addTerms: {
		GtsTerm iri: 'https://ex/r1'.
		GtsTerm iri: 'https://ex/r2'.
		GtsTerm iri: 'https://ex/s'.
		GtsTerm iri: 'https://ex/p'.
		GtsTerm iri: 'https://ex/o' }.
	writer addQuads: { #(2 3 4) }.
	writer addReifiers: {
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(2 3 4) }.
		Dictionary newFromPairs: { 'rid'. 1. 'spo'. #(2 3 4) } }.
	graph := GtsReader read: writer bytes.
	self assert: graph reifiers size equals: 2
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
		should: [ GtsFromNQuads fromString: '<https://ex/s> <https://ex/p> <https://ex/o>', String lf ]
		raise: Error.
	self
		should: [ GtsFromNQuads fromString: '<https://ex/s> <https://ex/p> <https://ex/o> . extra', String lf ]
		raise: Error.
	self
		should: [ GtsFromNQuads fromString: '<https://ex/s> <https://ex/p> _: .', String lf ]
		raise: Error.
	self
		should: [ GtsFromNQuads fromString: '<https://ex/s> <https://ex/p> "Cat"@ .', String lf ]
		raise: Error
]

{ #category : #tests }
GtsFromNQuadsTest >> testTokenizerSignalsParserError [
	self
		should: [ GtsFromNQuads fromString: '<https://ex/s> <https://ex/p> "Cat"@ .', String lf ]
		raise: GtsParserError
]
