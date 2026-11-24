"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2RdfRows,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #decoding }
GtsV2RdfRows class >> readType: type content: content bytes: bytes into: fold [
	| columns graphs subjects predicates objects |
	columns := self decodeType: type content: content bytes: bytes.
	graphs := columns first.
	type = 5 ifTrue: [ graphs do: [ :name | fold node: name ]. ^ self ].
	subjects := columns second.
	predicates := columns third.
	objects := columns fourth.
	type = 4 ifTrue: [ self checkAnnotations: columns identity: [ :id | id ] ].
	1 to: subjects size do: [ :index |
		| subject object terms |
		subject := fold node: (subjects at: index).
		object := fold resolve: (objects at: index) kinds: (type = 3 ifTrue: [ #(triple) ]).
		terms := type = 3
			ifTrue: [ { subject. object } ]
			ifFalse: [ { subject. (fold resolve: (predicates at: index) kinds: #(iri)). object } ].
		fold addLine: type terms: terms graph: (graphs at: index) ]
]

{ #category : #decoding }
GtsV2RdfRows class >> decodeType: type content: content bytes: bytes [
	| head count body graphs graphRuns subjects predicates predicateRuns objects counts order |
	head := content fields: (type = 5 ifTrue: [ 1 ] ifFalse: [ 2 ]).
	type ~= 5 ifTrue: [
		order := head next uint.
		(order between: 1 and: 2) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ] ].
	count := head next uint.
	count = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	body := GtsV2PayloadCursor on: bytes.
	body requireArray: (#(4 3 5 1) at: type - 1).
	type = 5 ifTrue: [
		graphs := self deltas: body minimum: 0.
		body requireEnd.
		GtsV2RdfColumns requireCount: count in: graphs.
		^ { graphs } ].
	graphRuns := self runs: body read: [
		| name |
		body readOptionDo: [ name := body readHead: 0 ].
		name ].
	type = 2
		ifTrue: [
			predicateRuns := self runs: body read: [ body readHead: 0 ].
			objects := self deltas: body minimum: 0.
			subjects := self deltas: body minimum: 0 ]
		ifFalse: [
			subjects := self deltas: body minimum: 0.
			type = 4 ifTrue: [ predicates := self deltas: body minimum: 0 ].
			objects := self deltas: body minimum: 0.
			type = 4 ifTrue: [ counts := self deltas: body minimum: 1 ] ].
	body requireEnd.
	self requireCount: count inRuns: graphRuns.
	type = 2
		ifTrue: [
			GtsV2RdfColumns requireCount: count in: objects.
			GtsV2RdfColumns requireCount: count in: subjects.
			self requireCount: count inRuns: predicateRuns.
			predicates := self expand: predicateRuns count: count ]
		ifFalse: [
			GtsV2RdfColumns requireCount: count in: subjects.
			type = 4 ifTrue: [ GtsV2RdfColumns requireCount: count in: predicates ].
			GtsV2RdfColumns requireCount: count in: objects.
			type = 4 ifTrue: [ GtsV2RdfColumns requireCount: count in: counts ] ].
	graphs := self expand: graphRuns count: count.
	^ { graphs. subjects. predicates. objects }
]

{ #category : #validation }
GtsV2RdfRows class >> checkAnnotations: columns identity: key [
	| seen |
	seen := Set new.
	1 to: columns first size do: [ :index |
		| row |
		row := columns collect: [ :column | (column at: index) ifNotNil: [ :id | key value: id ] ].
		(seen includes: row) ifTrue: [ GtsV2EnvelopeError signal: 'unordered_or_repeated' ].
		seen add: row ]
]

{ #category : #private }
GtsV2RdfRows class >> runs: cursor read: aBlock [
	| result |
	result := OrderedCollection new.
	(cursor readHead: 4) timesRepeat: [
		| key length |
		cursor requireArray: 2.
		key := aBlock value.
		length := cursor readHead: 0.
		length = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		(result isNotEmpty and: [ key = result last first ]) ifTrue: [
			GtsV2EnvelopeError signal: 'not_deterministic' ].
		result add: { key. length } ].
	^ result
]

{ #category : #private }
GtsV2RdfRows class >> deltas: cursor minimum: minimum [
	| values current |
	values := OrderedCollection new.
	current := 0.
	(cursor readHead: 4) timesRepeat: [
		current := current + cursor readInteger.
		(current between: minimum and: 18446744073709551615) ifFalse: [
			GtsV2EnvelopeError signal: 'out_of_range' ].
		values add: current ].
	^ values
]

{ #category : #private }
GtsV2RdfRows class >> requireCount: count inRuns: runs [
	(runs inject: 0 into: [ :sum :run | sum + run second ]) = count ifFalse: [
		GtsV2EnvelopeError signal: 'inconsistent_counts' ]
]

{ #category : #private }
GtsV2RdfRows class >> expand: runs count: count [
	| result index |
	result := Array new: count.
	index := 1.
	runs do: [ :run |
		run second timesRepeat: [
			result at: index put: run first.
			index := index + 1 ] ].
	^ result
]
