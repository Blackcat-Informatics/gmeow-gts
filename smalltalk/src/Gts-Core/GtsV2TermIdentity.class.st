"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2TermIdentity,
	#superclass : #Object,
	#instVars : [ 'dictionary', 'keys', 'textBytes' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2TermIdentity >> initialize [
	super initialize.
	dictionary := OrderedCollection new.
	keys := Dictionary new.
	textBytes := 0
]

{ #category : #decoding }
GtsV2TermIdentity >> observe: event [
	self isComplete ifFalse: [ ^ self ].
	(event problem isNotNil or: [ (#(linked healed) includes: event link) not or: [
		event payload decoded isNil ] ]) ifTrue: [ ^ self abandon ].
	[ GtsV2RdfTerms new read: event frame content bytes: event payload decoded fold: self values: nil ]
		on: GtsV2FramingError, GtsV2EnvelopeError do: [ :error | self abandon ]
]

{ #category : #private }
GtsV2TermIdentity >> abandon [
	dictionary := nil.
	keys := nil
]

{ #category : #testing }
GtsV2TermIdentity >> isComplete [
	^ dictionary isNotNil
]

{ #category : #accessing }
GtsV2TermIdentity >> dictionarySize [
	^ dictionary size
]

{ #category : #accessing }
GtsV2TermIdentity >> identityOf: id [
	^ dictionary at: id + 1 ifAbsent: [ id negated - 1 ]
]

{ #category : #validation }
GtsV2TermIdentity >> chargeText: length [
	textBytes := textBytes + length.
	textBytes > 268435456 ifTrue: [ GtsV2EnvelopeError signal: 'formed term capacity' ]
]

{ #category : #private }
GtsV2TermIdentity >> remember: key [
	dictionary add: (keys at: key ifAbsentPut: [ keys size ])
]

{ #category : #terms }
GtsV2TermIdentity >> addIri: text [
	self remember: { #iri. text }
]

{ #category : #terms }
GtsV2TermIdentity >> addBlank: label [
	self remember: { #blank. label }
]

{ #category : #terms }
GtsV2TermIdentity >> addLiteral: textOrReference datatype: local [
	self remember: { #literal. textOrReference. (self identityOf: local) }
]

{ #category : #terms }
GtsV2TermIdentity >> addLiteral: textOrReference language: language direction: direction [
	self remember: { #language. textOrReference. language. direction }
]

{ #category : #terms }
GtsV2TermIdentity >> addLiteral: textOrReference datatypeIri: iri [
	self remember: { #value. textOrReference. iri }
]

{ #category : #terms }
GtsV2TermIdentity >> addTripleSubject: subject predicate: predicate object: object [
	self remember: { #triple. (self identityOf: subject). (self identityOf: predicate). (self identityOf: object) }
]

{ #category : #terms }
GtsV2TermIdentity >> resolveReference: digest length: length octets: octets values: store [
	^ { digest. length }
]
