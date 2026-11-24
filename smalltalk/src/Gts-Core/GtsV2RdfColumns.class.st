"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2RdfColumns,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #decoding }
GtsV2RdfColumns class >> item: bytes [
	| walker limit |
	walker := GtsV2CborWalker on: bytes.
	limit := bytes size + 1.
	(walker itemEndAt: 1 limit: limit) = limit
		ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	^ GtsV2Field walker: walker from: 1 to: limit
]

{ #category : #decoding }
GtsV2RdfColumns class >> read: field with: aBlock [
	| cursor result |
	cursor := field list.
	result := OrderedCollection new.
	[ cursor hasNext ] whileTrue: [ result add: (aBlock value: cursor next) ].
	^ result asArray
]

{ #category : #decoding }
GtsV2RdfColumns class >> uints: field [
	^ self read: field with: [ :item | item uint ]
]

{ #category : #decoding }
GtsV2RdfColumns class >> body: field size: size [
	^ field bytesStartingAt: (field byteBodyOfSize: size)
]

{ #category : #decoding }
GtsV2RdfColumns class >> text: field [
	| head |
	head := field typedHead: 3.
	^ self utf8: (field bytesStartingAt: head end)
]

{ #category : #decoding }
GtsV2RdfColumns class >> utf8: bytes [
	(GtsCborText isValidUtf8: bytes from: 1 to: bytes size + 1)
		ifFalse: [ GtsV2EnvelopeError signal: 'ill_formed_term' ].
	^ bytes utf8Decoded
]

{ #category : #decoding }
GtsV2RdfColumns class >> deltas: field count: count minimum: minimum [
	| current values |
	current := 0.
	values := self read: field with: [ :item |
		current := current + item signedInteger.
		(current between: minimum and: 18446744073709551615)
			ifFalse: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		current ].
	self requireCount: count in: values.
	^ values
]

{ #category : #validation }
GtsV2RdfColumns class >> requireCount: count in: collection [
	collection size = count ifFalse: [ GtsV2EnvelopeError signal: 'inconsistent_counts' ]
]
