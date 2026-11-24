"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2Pack,
	#superclass : #Object,
	#instVars : [ 'cursor', 'members', 'bodyStart' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2Pack class >> read: content bytes: bytes [
	^ self new read: content bytes: bytes
]

{ #category : #decoding }
GtsV2Pack >> read: content bytes: bytes [
	| header count unitLength declaredCount bodyLength actualLength actualUnitLength offset |
	header := content fields: 2.
	count := header next uint.
	unitLength := header next uint.
	count = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	cursor := GtsV2PayloadCursor on: bytes.
	cursor requireArray: 2.
	declaredCount := cursor readHead: 4.
	members := OrderedCollection new.
	declaredCount timesRepeat: [ members add: self readMember ].
	bodyLength := cursor readHead: 2.
	bodyStart := cursor offset.
	cursor skip: bodyLength.
	cursor requireEnd.
	count = declaredCount ifFalse: [ GtsV2EnvelopeError signal: 'inconsistent_counts' ].
	actualLength := 0.
	actualUnitLength := 0.
	members do: [ :member |
		actualLength := actualLength + member third.
		member first <= 9 ifTrue: [ actualUnitLength := actualUnitLength + member third ] ].
	(actualLength = bodyLength and: [ actualUnitLength = unitLength ]) ifFalse: [
		GtsV2EnvelopeError signal: 'inconsistent_counts' ].
	offset := bodyStart.
	members do: [ :member |
		self check: member at: offset.
		offset := offset + member third ].
	^ self
]

{ #category : #private }
GtsV2Pack >> readMember [
	| arity kind start content |
	arity := cursor readHead: 4.
	arity = 0 ifTrue: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	kind := cursor readHead: 0.
	(kind between: 1 and: 4) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	arity = 3 ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	start := cursor offset.
	cursor requireArray: (#(1 2 4 3) at: kind).
	cursor readDigest.
	kind >= 3 ifTrue: [ cursor readHead: 0 ].
	(#(2 3) includes: kind) ifTrue: [ cursor readOptionDo: [ cursor readText ] ].
	kind >= 3 ifTrue: [ cursor readHead: 0 ].
	content := cursor fieldFrom: start.
	^ { kind + 7. content. cursor readHead: 0 }
]

{ #category : #private }
GtsV2Pack >> check: member at: offset [
	| type end fields total count body declaredCount sum zero |
	type := member first.
	type = 8 ifTrue: [
		member third = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ] ].
	type <= 9 ifTrue: [ ^ self ].
	fields := member second list.
	fields next.
	total := fields next uint.
	type = 10 ifTrue: [ fields next ].
	count := fields next uint.
	count < (type = 10 ifTrue: [ 2 ] ifFalse: [ 1 ]) ifTrue: [
		GtsV2EnvelopeError signal: 'out_of_range' ].
	end := offset + member third.
	body := cursor sliceFrom: offset to: end.
	declaredCount := body readHead: 4.
	sum := 0.
	zero := false.
	declaredCount timesRepeat: [
		| length |
		body requireArray: 2.
		body readDigest.
		length := body readHead: 0.
		sum := sum + length.
		length = 0 ifTrue: [ zero := true ] ].
	body requireEnd.
	count = declaredCount ifFalse: [ GtsV2EnvelopeError signal: 'inconsistent_counts' ].
	(zero or: [ sum > 18446744073709551615 ]) ifTrue: [
		GtsV2EnvelopeError signal: 'out_of_range' ].
	sum = total ifFalse: [ GtsV2EnvelopeError signal: 'inconsistent_counts' ]
]

{ #category : #enumerating }
GtsV2Pack >> membersDo: block [
	| offset |
	offset := bodyStart.
	members withIndexDo: [ :member :index |
		block valueWithArguments: { member first. member second. offset. member third. index - 1 }.
		offset := offset + member third ]
]
