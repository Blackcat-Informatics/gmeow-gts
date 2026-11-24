"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2RdfRows,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #decoding }
GtsV2RdfRows class >> readType: type content: content bytes: bytes into: fold [
	| head count body graphs graphColumn subjects predicates objects reifies order |
	body := (GtsV2RdfColumns item: bytes) fields: (#(4 3 5 1) at: type - 1).
	type = 5 ifTrue: [
		count := (content fields: 1) next uint.
		count = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		(GtsV2RdfColumns deltas: body next count: count minimum: 0) do: [ :name | fold node: name ].
		^ self ].
	head := content fields: 2.
	order := head next uint.
	(order between: 1 and: 2) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	count := head next uint.
	count = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	graphColumn := body next.
	type = 2
		ifTrue: [
			| predicateColumn |
			predicateColumn := body next.
			objects := GtsV2RdfColumns deltas: body next count: count minimum: 0.
			subjects := GtsV2RdfColumns deltas: body next count: count minimum: 0.
			predicates := self runs: predicateColumn count: count read: [ :field | field uint ] ]
		ifFalse: [
			subjects := GtsV2RdfColumns deltas: body next count: count minimum: 0.
			type = 4 ifTrue: [ predicates := GtsV2RdfColumns deltas: body next count: count minimum: 0 ].
			objects := GtsV2RdfColumns deltas: body next count: count minimum: 0.
			type = 4 ifTrue: [ GtsV2RdfColumns deltas: body next count: count minimum: 1 ] ].
	graphs := self runs: graphColumn count: count read: [ :field |
		| name |
		field optionUpTo: field end do: [ :id | name := id uint ].
		name ].
	type = 4 ifTrue: [ self checkAnnotations: subjects predicates: predicates objects: objects graphs: graphs ].
	type = 3 ifTrue: [ reifies := fold internIri: 'http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies' ].
	1 to: count do: [ :index |
		fold addSubject: (fold node: (subjects at: index))
			predicate: (reifies ifNil: [ fold resolve: (predicates at: index) kinds: #(iri) ])
			object: (fold resolve: (objects at: index) kinds: (type = 3 ifTrue: [ #(triple) ]))
			graph: (graphs at: index) ]
]

{ #category : #private }
GtsV2RdfRows class >> checkAnnotations: subjects predicates: predicates objects: objects graphs: graphs [
	| seen |
	seen := Set new.
	1 to: subjects size do: [ :index |
		| row |
		row := { (subjects at: index). (predicates at: index). (objects at: index). (graphs at: index) }.
		(seen includes: row) ifTrue: [ GtsV2EnvelopeError signal: 'unordered_or_repeated' ].
		seen add: row ]
]

{ #category : #private }
GtsV2RdfRows class >> runs: field count: count read: aBlock [
	| result cursor previous first |
	result := OrderedCollection new.
	cursor := field list.
	first := true.
	[ cursor hasNext ] whileTrue: [
		| fields key length |
		fields := cursor next fields: 2.
		key := aBlock value: fields next.
		length := fields next uint.
		length = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		length > (count - result size) ifTrue: [ GtsV2EnvelopeError signal: 'inconsistent_counts' ].
		(first not and: [ key = previous ]) ifTrue: [ GtsV2EnvelopeError signal: 'not_deterministic' ].
		length timesRepeat: [ result add: key ].
		previous := key.
		first := false ].
	GtsV2RdfColumns requireCount: count in: result.
	^ result asArray
]
