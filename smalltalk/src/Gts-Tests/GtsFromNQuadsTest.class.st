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
GtsFromNQuadsTest >> testV2FoldPreservesImportedRowsAndScopesBlanksPerSegment [
	| source bytes graph |
	source := '<https://e/s> <https://e/p> <<( <https://e/a> <https://e/p> "v"@en--rtl )>> <https://e/g> .', String lf.
	bytes := GtsFromNQuads v2FromString: source.
	self assert: (GtsNQuads toNQuads: (GtsV2RdfFold read: (GtsV2Wire inspect: bytes))) equals: source.
	bytes := GtsFromNQuads v2FromString: '_:a <https://e/p> "v" .'.
	graph := GtsV2RdfFold read: (GtsV2Wire inspect: bytes, bytes).
	self assert: graph quads size equals: 2.
	self deny: graph quads first first = graph quads second first
]

{ #category : #tests }
GtsFromNQuadsTest >> testV2FoldRebuildsAValueFromAnEarlierSegment [
	| writer value digest dictionary rows wire graph valueSegment |
	writer := GtsV2SegmentWriter new.
	value := 'word' utf8Encoded.
	digest := GtsBlake3 deriveWith: 'gts2 unit' bytes: value.
	writer appendType: 8 content: { digest } payload: value.
	writer appendType: 11 content: { digest. 4. 1 } payload: (GtsCborWriter encode: { { digest. 4 } }).
	writer commit.
	valueSegment := writer bytes.
	writer := GtsV2SegmentWriter new.
	dictionary := GtsCborWriter encode: {
		{ { 1. #(0 0 0). { 5. 5. GtsGraph xsdString size }.
			('urn:surn:p', GtsGraph xsdString) utf8Encoded }.
		{ 5. { digest }. #(4). #(1). #(2). #() } }. #() }.
	writer appendType: 1 content: #(0 ((1 3) (5 1)) 2 1) payload: dictionary.
	rows := GtsCborWriter encode: #((((1) 1)) ((1 1)) (3) (0)).
	writer appendType: 2 content: #(2 1) payload: rows.
	writer commit.
	wire := GtsV2Wire inspect: valueSegment, writer bytes.
	graph := GtsV2RdfFold read: wire.
	self assert: (GtsNQuads toNQuads: graph) equals: '<urn:s> <urn:p> "word" .', String lf.
	self should: [ GtsV2RdfFold read: (GtsV2Wire inspect: writer bytes) ] raise: GtsArchiveError
]

{ #category : #tests }
GtsFromNQuadsTest >> testV2FoldProjectsCanonicalTypedValues [
	| writer value first second firstDigest secondDigest digest prefix hexIri dictionary rows graph expected |
	writer := GtsV2SegmentWriter new.
	first := #[255].
	second := #[0 65].
	value := first, second.
	firstDigest := GtsBlake3 deriveWith: 'gts2 unit' bytes: first.
	secondDigest := GtsBlake3 deriveWith: 'gts2 unit' bytes: second.
	digest := GtsBlake3 deriveWith: 'gts2 unit' bytes: value.
	writer appendType: 8 content: { firstDigest } payload: first.
	writer appendType: 8 content: { secondDigest } payload: second.
	writer appendType: 11 content: { digest. 3. 2 }
		payload: (GtsCborWriter encode: { { firstDigest. 1 }. { secondDigest. 2 } }).
	prefix := 'http://www.w3.org/2001/XMLSchema#'.
	hexIri := prefix, 'hexBinary'.
	dictionary := GtsCborWriter encode: {
		{ { 1. #(0 0 0). { 5. 5. hexIri size }. ('urn:surn:p', hexIri) utf8Encoded }.
		{ 6. { #(1 false). #(2 18446744073709551615). #(3 -18446744073709551616).
			#(4 -62135596800 1002000). #(4 253402300799 0). { 5. value }. { 5. #[] } } }.
		{ 7. { digest }. #(3) }.
		{ 2. #(0). #(6). 'FF0041' utf8Encoded. #(1). #(2). #() } }. #() }.
	writer appendType: 1 content: #(0 ((1 3) (6 7) (7 1) (2 1)) 2 1) payload: dictionary.
	rows := GtsCborWriter encode: #((((1) 9)) ((1 9)) (3 1 1 1 1 1 1 1 1) (0 0 0 0 0 0 0 0 0)).
	writer appendType: 2 content: #(2 9) payload: rows.
	writer commit.
	graph := GtsV2RdfFold read: (GtsV2Wire inspect: writer bytes).
	expected := #(
		('false' 'boolean')
		('18446744073709551615' 'nonNegativeInteger')
		('-18446744073709551616' 'integer')
		('0001-01-01T00:00:00.001002Z' 'dateTime')
		('9999-12-31T23:59:59Z' 'dateTime')
		('FF0041' 'hexBinary')
		('' 'hexBinary')) collect: [ :pair |
			'<urn:s> <urn:p> "', pair first, '"^^<', prefix, pair second, '> .' ].
	self assert: (self sortedLines: (GtsNQuads toNQuads: graph)) equals: expected sorted
]

{ #category : #tests }
GtsFromNQuadsTest >> testV2FoldRejectsOutOfRangeTypedTimestamps [
	#(#(4 -62135596801 0) #(4 253402300800 0) #(4 0 1000000000)) do: [ :value |
		| writer dictionary |
		writer := GtsV2SegmentWriter new.
		dictionary := GtsCborWriter encode: { { { 6. { value } } }. #() }.
		writer appendType: 1 content: #(0 ((6 1)) 2 1) payload: dictionary.
		writer commit.
		self
			should: [ GtsV2RdfFold read: (GtsV2Wire inspect: writer bytes) ]
			raise: GtsArchiveError
			withExceptionDo: [ :error | self assert: (error messageText endsWith: ': out_of_range') ] ]
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
