"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2RdfTypedValue,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #decoding }
GtsV2RdfTypedValue class >> read: item into: fold [
	| fields kind value text datatype |
	fields := item list.
	fields hasNext ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	kind := fields next uint.
	(kind between: 1 and: 5) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	fields := item fields: (kind = 4 ifTrue: [ 3 ] ifFalse: [ 2 ]).
	fields next.
	value := fields next.
	kind = 5 ifTrue: [ ^ self readOctets: value into: fold ].
	text := kind caseOf: {
		[ 1 ] -> [ value flag printString ].
		[ 2 ] -> [ value uint printString ].
		[ 3 ] -> [ value signedInteger printString ].
		[ 4 ] -> [ self timeAt: value signedInteger fraction: fields next ] }.
	fold chargeText: text size.
	datatype := 'http://www.w3.org/2001/XMLSchema#', (#('boolean' 'nonNegativeInteger' 'integer' 'dateTime') at: kind).
	fold addLiteral: text datatypeIri: datatype
]

{ #category : #private }
GtsV2RdfTypedValue class >> timeAt: seconds fraction: field [
	| time nanos |
	(seconds between: -62135596800 and: 253402300799)
		ifFalse: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	nanos := field uint.
	nanos >= 1000000000 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	time := (DateAndTime fromUnixTime: seconds) asUTC.
	^ String streamContents: [ :stream |
		time printYMDOn: stream.
		stream nextPut: $T.
		time printHMSOn: stream.
		nanos > 0 ifTrue: [
			| fraction |
			fraction := (1000000000 + nanos) printString allButFirst trimRight: [ :character | character = $0 ].
			stream nextPut: $.; nextPutAll: fraction ].
		stream nextPut: $Z ]
]

{ #category : #private }
GtsV2RdfTypedValue class >> readOctets: value into: fold [
	| start length text |
	start := value byteBodyOfSize: nil.
	length := value end - start.
	fold chargeText: length * 2.
	text := self hexOf: { value bytesStartingAt: start } length: length.
	fold addLiteral: text datatypeIri: self hexBinary
]

{ #category : #accessing }
GtsV2RdfTypedValue class >> hexBinary [
	^ 'http://www.w3.org/2001/XMLSchema#hexBinary'
]

{ #category : #encoding }
GtsV2RdfTypedValue class >> hexOf: parts length: length [
	| digits text index |
	digits := '0123456789ABCDEF'.
	text := String new: length * 2.
	index := 1.
	parts do: [ :part | part do: [ :octet |
		text at: index put: (digits at: (octet bitShift: -4) + 1).
		text at: index + 1 put: (digits at: (octet bitAnd: 15) + 1).
		index := index + 2 ] ].
	^ text
]
