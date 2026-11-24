"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2Field,
	#superclass : #Object,
	#instVars : [ 'walker', 'start', 'end' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2Field class >> walker: aWalker from: first to: finish [
	^ self new initializeWalker: aWalker from: first to: finish
]

{ #category : #initialization }
GtsV2Field >> initializeWalker: aWalker from: first to: finish [
	walker := aWalker.
	start := first.
	end := finish.
	^ self
]

{ #category : #accessing }
GtsV2Field >> start [
	^ start
]

{ #category : #accessing }
GtsV2Field >> end [
	^ end
]

{ #category : #accessing }
GtsV2Field >> size [
	^ end - start
]

{ #category : #parsing }
GtsV2Field >> typedHead: major [
	| head |
	head := walker headAt: start limit: end.
	self checkHead: head.
	head major = major ifFalse: [ GtsV2EnvelopeError signal: 'wrong_type' ].
	^ head
]

{ #category : #validation }
GtsV2Field >> checkHead: head [
	(head major <= 4 and: [ head isShortest not ])
		ifTrue: [ GtsV2EnvelopeError signal: 'not_deterministic' ]
]

{ #category : #parsing }
GtsV2Field >> uint [
	^ (self typedHead: 0) argument
]

{ #category : #parsing }
GtsV2Field >> flag [
	self typedHead: 7.
	(walker bytes at: start) = 244 ifTrue: [ ^ false ].
	(walker bytes at: start) = 245 ifTrue: [ ^ true ].
	^ GtsV2EnvelopeError signal: 'wrong_type'
]

{ #category : #parsing }
GtsV2Field >> byteBodyOfSize: expected [
	| head |
	head := self typedHead: 2.
	(expected isNotNil and: [ head argument ~= expected ])
		ifTrue: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	^ head end
]

{ #category : #parsing }
GtsV2Field >> list [
	| head |
	head := self typedHead: 4.
	^ GtsV2Fields walker: walker from: head end to: end bound: end
]

{ #category : #parsing }
GtsV2Field >> fields: count [
	^ self fields: count bound: end
]

{ #category : #parsing }
GtsV2Field >> fields: count bound: bound [
	| head |
	head := self typedHead: 4.
	head argument = count ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	^ GtsV2Fields walker: walker from: head end to: end bound: bound
]

{ #category : #parsing }
GtsV2Field >> optionUpTo: bound do: aBlock [
	| head fields tag |
	head := self typedHead: 4.
	head argument = 0 ifTrue: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	fields := GtsV2Fields walker: walker from: head end to: end bound: bound.
	tag := fields next uint.
	(tag between: 1 and: 2) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	tag = head argument ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	tag = 2 ifTrue: [ aBlock value: fields next ]
]

{ #category : #validation }
GtsV2Field >> checkGrammarUpTo: bound [
	| offset head major bytes |
	offset := start.
	bytes := walker bytes.
	[ offset < end ] whileTrue: [
		offset >= bound ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		head := walker headAt: offset limit: end.
		self checkHead: head.
		major := head major.
		((major between: 5 and: 6) or: [
			major = 7 and: [ ((bytes at: offset) between: 244 and: 245) not ] ])
			ifTrue: [ GtsV2EnvelopeError signal: 'wrong_type' ].
		offset := head end.
		(major between: 2 and: 3) ifTrue: [
			offset := offset + head argument.
			major = 3 ifTrue: [
				(GtsCborText isValidUtf8: bytes from: head end to: offset)
					ifFalse: [ GtsV2EnvelopeError signal: 'wrong_type' ] ] ] ]
]

{ #category : #validation }
GtsV2Field >> checkMarkHead [
	| head first |
	head := self typedHead: 4.
	head argument = 0 ifTrue: [ GtsV2EnvelopeError signal: 'wrong_type' ].
	first := walker headAt: head end limit: end.
	first major = 0 ifFalse: [ GtsV2EnvelopeError signal: 'wrong_type' ]
]

{ #category : #comparing }
GtsV2Field >> compare: other [
	| left right |
	0 to: (self size min: other size) - 1 do: [ :index |
		left := walker bytes at: start + index.
		right := other octetAt: index.
		left ~= right ifTrue: [ ^ left - right ] ].
	^ self size - other size
]

{ #category : #accessing }
GtsV2Field >> octetAt: index [
	^ walker bytes at: start + index
]
