Class {
	#name : #GtsNQuads,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #private }
GtsNQuads class >> escape: aString [
	^ String streamContents: [ :stream |
		aString do: [ :character |
			character = $\ ifTrue: [ stream nextPutAll: '\\' ] ifFalse: [
			character = $" ifTrue: [ stream nextPutAll: '\"' ] ifFalse: [
			character = Character lf ifTrue: [ stream nextPutAll: '\n' ] ifFalse: [
			character = Character cr ifTrue: [ stream nextPutAll: '\r' ] ifFalse: [
			character = Character tab ifTrue: [ stream nextPutAll: '\t' ] ifFalse: [
			character asciiValue < 32
				ifTrue: [ stream nextPutAll: '\u', (self fourHexDigits: character asciiValue) ]
				ifFalse: [ stream nextPut: character ] ] ] ] ] ] ] ]
]

{ #category : #private }
GtsNQuads class >> fourHexDigits: value [
	| digits |
	digits := '0123456789ABCDEF'.
	^ String streamContents: [ :stream |
		#(12 8 4 0) do: [ :shift |
			stream nextPut: (digits at: (((value bitShift: shift negated) bitAnd: 16rF) + 1)) ] ]
]

{ #category : #private }
GtsNQuads class >> renderQuotedTriple: spo in: graph [
	^ self renderQuotedTriple: spo in: graph open: #()
]

{ #category : #private }
GtsNQuads class >> renderQuotedTriple: spo in: graph open: open [
	^ '<<( ',
		(self renderTermId: (spo at: 1) in: graph open: open), ' ',
		(self renderTermId: (spo at: 2) in: graph open: open), ' ',
		(self renderTermId: (spo at: 3) in: graph open: open),
		' )>>'
]

{ #category : #private }
GtsNQuads class >> renderTermId: termId in: graph [
	^ self renderTermId: termId in: graph open: #()
]

{ #category : #private }
GtsNQuads class >> renderTermId: termId in: graph open: open [
	"`open` carries the triple terms whose rendering is still in progress. A `reifies`
	row may name the very term that resolves through it (§7.3), so a projection MUST NOT
	recurse blindly: a self-reaching term degrades to the same blank node an unbound
	triple term already produces."
	| spo term |
	((termId isInteger not) or: [ termId < 0 or: [ termId >= graph terms size ] ])
		ifTrue: [ ^ '_:out_of_range_', termId asString ].
	(open includes: termId) ifTrue: [ ^ '_:unbound_triple_', termId asString ].
	term := graph termAtId: termId.
	term kind = #iri ifTrue: [ ^ '<', term value, '>' ].
	term kind = #blankNode ifTrue: [
		(term value notNil and: [ term value isEmpty not ])
			ifTrue: [ ^ '_:', term value ].
		^ '_:b', termId asString ].
	term kind = #literal ifTrue: [
		| literal |
		literal := '"', (self escape: term value), '"'.
		(term language notNil and: [ term language isEmpty not ])
			ifTrue: [
				(term direction notNil and: [ term direction isEmpty not ])
					ifTrue: [ ^ literal, '@', term language, '--', term direction ].
				^ literal, '@', term language ].
		term datatype ifNotNil: [
			^ literal, '^^', (self renderTermId: term datatype in: graph open: open) ].
		^ literal ].
	term kind = #triple ifTrue: [
		"Quoted triple (RDF 1.2 triple term): its own 'tt' is authoritative; a legacy
		'tt'-less term still resolves through its reifier (§7.3). A term that states no
		triple at all degrades to a syntactically valid blank node."
		spo := graph tripleForTermId: termId.
		spo ifNotNil: [
			^ self renderQuotedTriple: spo in: graph open: (open copyWith: termId) ].
		^ '_:unbound_triple_', termId asString ].
	^ ''
]

{ #category : #projecting }
GtsNQuads class >> toNQuads: graph [
	| lines |
	lines := OrderedCollection new.
	graph quads do: [ :row |
		| line |
		line := (self renderTermId: (row at: 1) in: graph), ' ',
			(self renderTermId: (row at: 2) in: graph), ' ',
			(self renderTermId: (row at: 3) in: graph).
		row size >= 4
			ifTrue: [ line := line, ' ', (self renderTermId: (row at: 4) in: graph) ].
		lines add: line, ' .' ].
	graph reifiers do: [ :entry |
		| line |
		line := (self renderTermId: (entry at: 'rid') in: graph),
			' <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ',
			(self renderQuotedTriple: (entry at: 'spo') in: graph).
		(entry at: 'graph' ifAbsent: [ nil ]) ifNotNil: [ :graphId |
			line := line, ' ', (self renderTermId: graphId in: graph) ].
		lines add: line, ' .' ].
	graph annotations do: [ :row |
		| line |
		line := (self renderTermId: (row at: 1) in: graph), ' ',
			(self renderTermId: (row at: 2) in: graph), ' ',
			(self renderTermId: (row at: 3) in: graph).
		row size >= 4
			ifTrue: [ line := line, ' ', (self renderTermId: (row at: 4) in: graph) ].
		lines add: line, ' .' ].
	^ String streamContents: [ :stream |
		lines asSortedCollection do: [ :line |
			stream
				nextPutAll: line;
				lf ] ]
]
