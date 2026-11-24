"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2RdfSuppression,
	#superclass : #Object,
	#instVars : [ 'holders', 'frames', 'targets', 'occurrences' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2RdfSuppression >> initialize [
	super initialize.
	holders := Dictionary new.
	frames := OrderedCollection new.
	targets := (1 to: 8) collect: [ :kind | Set new ]
]

{ #category : #recording }
GtsV2RdfSuppression >> record: identifier segment: blanks [
	| pair |
	"Only the first two distinct segments can be the first holder other than the entry's segment."
	pair := holders at: identifier ifAbsentPut: [ { blanks. nil } ].
	(pair first ~~ blanks and: [ pair second isNil ]) ifTrue: [ pair at: 2 put: blanks ]
]

{ #category : #decoding }
GtsV2RdfSuppression >> read: content bytes: bytes dictionary: dictionary segment: blanks [
	| count entries |
	count := (content fields: 1) next uint.
	entries := GtsV2RdfColumns read: (GtsV2RdfColumns item: bytes) with: [ :item | self entry: item ].
	GtsV2RdfColumns requireCount: count in: entries.
	entries do: [ :entry | entry second do: [ :leaf | self checkLeaf: leaf ] ].
	frames add: { entries. dictionary. dictionary size. blanks }
]

{ #category : #private }
GtsV2RdfSuppression >> entry: item [
	| fields kind target slots arity leaves |
	fields := item fields: 2.
	kind := fields next uint.
	(kind between: 1 and: 8) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	target := fields next.
	(#(1 3 4) includes: kind) ifTrue: [ ^ { kind. { self leaf: target } } ].
	kind = 5 ifTrue: [
		self checkQualifiedName: target.
		^ { kind. #() } ].
	kind = 8 ifTrue: [
		slots := target fields: 2.
		leaves := { self leaf: slots next }.
		self checkReferent: slots next.
		^ { kind. leaves } ].
	arity := kind = 6 ifTrue: [ 2 ] ifFalse: [ 3 ].
	slots := target fields: arity + 1.
	leaves := OrderedCollection new.
	arity timesRepeat: [ leaves add: (self leaf: slots next) ].
	target := slots next.
	target optionUpTo: target end do: [ :name | leaves add: (self leaf: name) ].
	^ { kind. leaves }
]

{ #category : #private }
GtsV2RdfSuppression >> checkQualifiedName: field [
	| fields |
	fields := field fields: 2.
	fields next uint = 1 ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	fields next byteBodyOfSize: 32
]

{ #category : #private }
GtsV2RdfSuppression >> checkReferent: field [
	| fields kind body |
	fields := field fields: 2.
	kind := fields next uint.
	(kind between: 1 and: 3) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	body := fields next.
	kind = 1 ifTrue: [ self checkQualifiedName: body ] ifFalse: [ body byteBodyOfSize: 32 ]
]

{ #category : #private }
GtsV2RdfSuppression >> leaf: item [
	| cursor pending program |
	"A postfix traversal keeps both decoding and resolution independent of the VM call stack."
	cursor := item cursor.
	pending := OrderedCollection with: 1.
	program := OrderedCollection new.
	[ pending isNotEmpty ] whileTrue: [
		| remaining kind |
		remaining := pending removeLast - 1.
		pending add: remaining.
		cursor openArray: 2.
		kind := cursor next uint.
		(kind between: 1 and: 3) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
		kind caseOf: {
			[ 1 ] -> [ program add: { 1. cursor next uint } ].
			[ 2 ] -> [
				cursor openArray: 2.
				program add: { 2. (GtsV2RdfColumns body: cursor next size: 32).
					(GtsV2RdfColumns body: cursor next size: nil) } ].
			[ 3 ] -> [
				cursor openArray: 3.
				pending add: 3 ] }.
		[ pending isNotEmpty and: [ pending last = 0 ] ] whileTrue: [
			pending removeLast.
			pending isNotEmpty ifTrue: [ program add: #(3) ] ] ].
	^ program
]

{ #category : #private }
GtsV2RdfSuppression >> checkLeaf: program [
	| crossSegment |
	crossSegment := OrderedCollection new.
	program do: [ :instruction |
		instruction first = 3
			ifTrue: [
				| object predicate subject |
				object := crossSegment removeLast.
				predicate := crossSegment removeLast.
				subject := crossSegment removeLast.
				(subject or: [ predicate or: [ object ] ]) ifFalse: [
					GtsV2EnvelopeError signal: 'forbidden_combination' ].
				crossSegment add: true ]
			ifFalse: [ crossSegment add: instruction first = 2 ] ]
]

{ #category : #resolving }
GtsV2RdfSuppression >> resolveWith: fold graph: graph [
	frames do: [ :frame |
		frame first do: [ :entry | self resolveEntry: entry frame: frame fold: fold ] ].
	(targets first isEmpty) ifTrue: [ ^ self ].
	occurrences := Array new: graph termCount.
	0 to: graph termCount - 1 do: [ :identifier |
		| term |
		term := graph termAtId: identifier.
		occurrences at: identifier + 1 put: ((targets first includes: identifier) or: [
			term kind = #triple and: [ term triple anySatisfy: [ :child | occurrences at: child + 1 ] ] ]) ]
]

{ #category : #private }
GtsV2RdfSuppression >> resolveEntry: entry frame: frame fold: fold [
	| kind terms |
	kind := entry first.
	(#(5 8) includes: kind) ifTrue: [ ^ self ].
	terms := entry second collect: [ :leaf |
		(self resolveLeaf: leaf frame: frame fold: fold) ifNil: [ ^ self ] ].
	(targets at: kind) add: ((#(1 3 4) includes: kind) ifTrue: [ terms first ] ifFalse: [ terms asArray ])
]

{ #category : #private }
GtsV2RdfSuppression >> resolveLeaf: program frame: frame fold: fold [
	| stack |
	stack := OrderedCollection new.
	program do: [ :instruction |
		instruction first caseOf: {
			[ 1 ] -> [
				instruction second >= frame third ifTrue: [ ^ nil ].
				stack add: (frame second at: instruction second + 1) ].
			[ 2 ] -> [
				stack add: ((self blank: instruction third frame: instruction second excluding: frame fourth)
					ifNil: [ ^ nil ]) ].
			[ 3 ] -> [
				| object predicate subject |
				object := stack removeLast.
				predicate := stack removeLast.
				subject := stack removeLast.
				((#(iri blankNode) includes: (fold kindOf: subject)) and: [ (fold kindOf: predicate) = #iri ])
					ifFalse: [ ^ nil ].
				stack add: (fold internTriple: { subject. predicate. object }) ] } ].
	^ stack last
]

{ #category : #private }
GtsV2RdfSuppression >> blank: label frame: digest excluding: segment [
	| pair holder |
	pair := holders at: digest ifAbsent: [ ^ nil ].
	holder := pair first == segment ifTrue: [ pair second ] ifFalse: [ pair first ].
	^ holder ifNotNil: [ holder at: label ifAbsent: [ nil ] ]
]

{ #category : #testing }
GtsV2RdfSuppression >> hides: type terms: terms [
	| graphArity exact |
	(occurrences isNotNil and: [ terms anySatisfy: [ :identifier | occurrences at: identifier + 1 ] ])
		ifTrue: [ ^ true ].
	graphArity := type = 3 ifTrue: [ 3 ] ifFalse: [ 4 ].
	(terms size = graphArity and: [ (targets at: 4) includes: terms last ]) ifTrue: [ ^ true ].
	(type ~= 2 and: [ (targets at: 3) includes: terms first ]) ifTrue: [ ^ true ].
	exact := #(2 6 7) at: type - 1.
	^ (targets at: exact) includes: terms
]
