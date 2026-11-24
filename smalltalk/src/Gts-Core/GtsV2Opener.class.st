"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2Opener,
	#superclass : #Object,
	#instVars : [ 'registry', 'hashAlgorithm', 'identifier', 'digestLength', 'statedOffset', 'end', 'lineage' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2Opener class >> read: item resync: isResync [
	^ self new read: item resync: isResync
]

{ #category : #'instance creation' }
GtsV2Opener class >> candidate: item charging: charge [
	^ self new readCandidate: item charging: charge
]

{ #category : #copying }
GtsV2Opener >> postCopy [
	super postCopy.
	identifier ifNotNil: [ identifier := identifier copy ]
]

{ #category : #accessing }
GtsV2Opener >> end [
	^ end
]

{ #category : #accessing }
GtsV2Opener >> registry [
	^ registry
]

{ #category : #accessing }
GtsV2Opener >> hashAlgorithm [
	^ hashAlgorithm
]

{ #category : #accessing }
GtsV2Opener >> identifier [
	^ identifier
]

{ #category : #accessing }
GtsV2Opener >> lineage [
	^ lineage ifNotNil: [ { lineage first ifNotNil: [ :digest | digest copy ]. lineage second } ]
]

{ #category : #parsing }
GtsV2Opener >> read: item resync: isResync [
	| bound fields |
	bound := isResync ifTrue: [ item end min: item start + 4096 ] ifFalse: [ item end ].
	fields := item fields: (isResync ifTrue: [ 11 ] ifFalse: [ 7 ]) bound: bound.
	self readPrefix: fields resync: isResync bound: bound candidate: false.
	isResync
		ifTrue: [ self readResyncSuffix: fields item: item bound: bound ]
		ifFalse: [
			hashAlgorithm = 1 ifTrue: [
				identifier := item deriveWith: 'gts2 header' prefix: #[] from: item start to: item end ] ].
	^ self
]

{ #category : #parsing }
GtsV2Opener >> readCandidate: item charging: charge [
	| prefix fields stored |
	prefix := item cappedAt: item start + 104.
	fields := prefix fields: 11.
	self readPrefix: fields resync: true bound: prefix end candidate: true.
	statedOffset = (item start - 1) ifFalse: [ ^ nil ].
	charge value ifFalse: [ ^ nil ].
	stored := self readResyncSuffix: (item cursorFrom: fields offset) item: item bound: item end.
	^ identifier = stored ifTrue: [ self ]
]

{ #category : #parsing }
GtsV2Opener >> readPrefix: fields resync: isResync bound: bound candidate: isCandidate [
	fields next uint.
	fields next uint = 2 ifFalse: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	isResync ifTrue: [ self checkMarker: fields next ].
	registry := fields next uint.
	registry = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	hashAlgorithm := fields next uint.
	(hashAlgorithm = 0 or: [ (isCandidate or: [ registry = 1 ]) and: [ hashAlgorithm ~= 1 ] ])
		ifTrue: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	digestLength := hashAlgorithm = 1 ifTrue: [ 32 ] ifFalse: [ nil ].
	fields next optionUpTo: bound do: [ :profile |
		profile uint = 0 ifTrue: [ GtsV2EnvelopeError signal: 'unregistered_value' ] ].
	isResync ifFalse: [
		fields next optionUpTo: bound do: [ :claim |
			| code |
			code := claim uint.
			(code = 0 or: [ registry = 1 and: [ code ~= 1 ] ])
				ifTrue: [ GtsV2EnvelopeError signal: 'unregistered_value' ] ] ].
	fields next optionUpTo: bound do: [ :encodedLineage |
		| pair previous body ordinal |
		pair := encodedLineage fields: 2 bound: bound.
		previous := pair next.
		body := previous byteBodyOfSize: digestLength.
		ordinal := pair next uint.
		ordinal = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		lineage := { digestLength ifNotNil: [ previous bytesStartingAt: body ]. ordinal } ].
	isResync ifTrue: [ statedOffset := fields next uint ]
]

{ #category : #parsing }
GtsV2Opener >> readResyncSuffix: fields item: item bound: bound [
	| bodyEnd stored first |
	fields next optionUpTo: bound do: [ :past | past byteBodyOfSize: digestLength ].
	self checkMarks: fields next bound: bound.
	bodyEnd := fields offset.
	stored := fields next.
	first := stored byteBodyOfSize: digestLength.
	end := fields offset.
	end > bound ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	hashAlgorithm = 1 ifTrue: [
		identifier := item
			deriveWith: 'gts2 header'
			prefix: #[138]
			from: item start + 1
			to: bodyEnd.
		^ stored bytesStartingAt: first ].
	^ nil
]

{ #category : #validation }
GtsV2Opener >> checkMarker: field [
	| expected start |
	expected := #[252 243 235 50 162 240 38 125 209 106 83 240 155 255 135 146].
	start := (field byteBodyOfSize: nil) - field start.
	field size - start = expected size ifFalse: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	1 to: expected size do: [ :index |
		(field octetAt: start + index - 1) = (expected at: index)
			ifFalse: [ GtsV2EnvelopeError signal: 'out_of_range' ] ]
]

{ #category : #validation }
GtsV2Opener >> checkMarks: field bound: bound [
	| marks count previous mark |
	marks := field list.
	count := 0.
	[ marks hasNext ] whileTrue: [
		mark := marks next.
		mark start >= bound ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		mark checkMarkHead.
		count = 64 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		(previous isNotNil and: [ (previous compare: mark) >= 0 ])
			ifTrue: [ GtsV2EnvelopeError signal: 'unordered_or_repeated' ].
		mark checkGrammarUpTo: bound.
		previous := mark.
		count := count + 1 ]
]
