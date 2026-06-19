Class {
	#name : #GtsNQuads,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #private }
GtsNQuads class >> objectStringFor: term [
	term kind = #literal ifTrue: [ ^ self stringForLiteral: term ].
	^ self resourceStringFor: term
]

{ #category : #private }
GtsNQuads class >> resourceStringFor: term [
	term kind = #blankNode ifTrue: [ ^ '_:', term value ].
	^ '<', term value, '>'
]

{ #category : #private }
GtsNQuads class >> stringForLiteral: term [
	| out |
	out := '"', term value, '"'.
	term language ifNotNil: [ out := out, '@', term language ].
	term datatype ifNotNil: [ out := out, '^^', (self resourceStringFor: term datatype) ].
	^ out
]

{ #category : #projecting }
GtsNQuads class >> toNQuads: graph [
	| lines |
	lines := graph quads collect: [ :row |
		| object predicate subject |
		subject := graph termAtId: (row at: 1).
		predicate := graph termAtId: (row at: 2).
		object := graph termAtId: (row at: 3).
		(self resourceStringFor: subject), ' ',
			(self resourceStringFor: predicate), ' ',
			(self objectStringFor: object), ' .' ].
	^ String streamContents: [ :stream |
		lines asSortedCollection do: [ :line |
			stream
				nextPutAll: line;
				lf ] ]
]
