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
	^ '<<( ',
		(self renderTermId: (spo at: 1) in: graph), ' ',
		(self renderTermId: (spo at: 2) in: graph), ' ',
		(self renderTermId: (spo at: 3) in: graph),
		' )>>'
]

{ #category : #private }
GtsNQuads class >> renderTermId: termId in: graph [
	| reifier spo term |
	((termId isInteger not) or: [ termId < 0 or: [ termId >= graph terms size ] ])
		ifTrue: [ ^ '_:out_of_range_', termId asString ].
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
			^ literal, '^^', (self renderTermId: term datatype in: graph) ].
		^ literal ].
	term kind = #triple ifTrue: [
		reifier := term reifier.
		reifier ifNotNil: [
			spo := graph reifierFor: reifier.
			spo ifNotNil: [ ^ self renderQuotedTriple: spo in: graph ] ].
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
		lines add: (self renderTermId: (entry at: 'rid') in: graph),
			' <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> ',
			(self renderQuotedTriple: (entry at: 'spo') in: graph),
			' .' ].
	graph annotations do: [ :row |
		lines add: (self renderTermId: (row at: 1) in: graph), ' ',
			(self renderTermId: (row at: 2) in: graph), ' ',
			(self renderTermId: (row at: 3) in: graph), ' .' ].
	^ String streamContents: [ :stream |
		lines asSortedCollection do: [ :line |
			stream
				nextPutAll: line;
				lf ] ]
]
