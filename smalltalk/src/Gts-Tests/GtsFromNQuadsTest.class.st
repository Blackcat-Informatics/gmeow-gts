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
GtsFromNQuadsTest >> testV2ImportCommitsAndDeduplicatesSimpleLiterals [
	| line duplicate bytes checks types |
	line := '<https://e/s> <https://e/p> "value" .', String lf.
	duplicate := '<https://e/s> <https://e/p> "value"^^<http://www.w3.org/2001/XMLSchema#string> .', String lf.
	bytes := GtsFromNQuads v2FromString: line.
	self assert: (GtsFromNQuads v2FromString: line, duplicate, line) equals: bytes.
	types := OrderedCollection new.
	(GtsV2Wire inspect: bytes) eventsDo: [ :event |
		event startsSegment
			ifTrue: [ checks := GtsV2CommitChecks openedBy: event effectiveIdentifier ]
			ifFalse: [
				self assert: event link equals: #linked.
				self assert: event payload state equals: #verified.
				types add: event frameType.
				checks observe: event findingsDo: [ :reason :position :count | self fail: reason ] ] ].
	self assert: types asArray equals: #(1 2 14).
	self assert: checks committedFrames equals: 3
]

{ #category : #tests }
GtsFromNQuadsTest >> testV2ImportChecksQuotedTripleRolesAndUnicodeEscapes [
	self
		should: [ GtsFromNQuads v2FromString:
			'<https://e/s> <https://e/p> <<( "literal" <https://e/p> <https://e/o> )>> .' ]
		raise: GtsParserError.
	self
		should: [ GtsFromNQuads v2FromString: '<https://e/s> <https://e/p> "\uD800" .' ]
		raise: GtsParserError.
	self assert: (GtsFromNQuads v2FromString: '<https://e/\u0073> <https://e/p> "v" . # comment')
		equals: (GtsFromNQuads v2FromString: '<https://e/s> <https://e/p> "v" .')
]

{ #category : #tests }
GtsFromNQuadsTest >> testV2VersionHintAllowsUnicodeBlankNodesAndChecksLanguage [
	| source |
	source := '_:é <https://e/p> "text"@en-US--rtl .'.
	self assert: (GtsFromNQuads v2FromString: 'VERSION "1.2"', String lf, source)
		equals: (GtsFromNQuads v2FromString: source).
	self should: [ GtsFromNQuads v2FromString: '_:é <https://e/p> "text"@9 .' ] raise: GtsParserError.
	self should: [ GtsFromNQuads v2FromString: '_:é <https://e/p> "text"@en--extra--rtl .' ] raise: GtsParserError
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
GtsFromNQuadsTest >> testUnreifiedTripleTermRoundTripsAsItself [
	| graph nquads quoted |
	nquads := '<https://ex/s> <https://ex/p> <<( <https://ex/a> <https://ex/b> <https://ex/c> )>> .', String lf.
	graph := GtsReader read: (GtsFromNQuads fromString: nquads).
	self assert: graph diagnostics isEmpty.
	self assert: graph reifiers isEmpty.
	quoted := graph terms detect: [ :term | term kind = #triple ].
	self assert: quoted reifier isNil.
	self assert: quoted triple notNil.
	self assert: (GtsNQuads toNQuads: graph) equals: nquads
]

{ #category : #tests }
GtsFromNQuadsTest >> testFromNQuadsKeepsBothBindingsOfOneReifier [
	| graph nquads rdfReifies |
	rdfReifies := 'http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies'.
	nquads := '<https://ex/r> <', rdfReifies, '> <<( <https://ex/s> <https://ex/p> "Cat"@en )>> .', String lf,
		'<https://ex/r> <', rdfReifies, '> <<( <https://ex/s> <https://ex/p> "Chat"@fr )>> .', String lf.
	graph := GtsReader read: (GtsFromNQuads fromString: nquads).
	self assert: graph diagnostics isEmpty.
	self assert: graph reifiers size equals: 2.
	self assert: (self sortedLines: (GtsNQuads toNQuads: graph)) equals: (self sortedLines: nquads)
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
		raise: GtsParserError
]
