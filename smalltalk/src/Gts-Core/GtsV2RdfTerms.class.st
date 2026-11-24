"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2RdfTerms,
	#superclass : #Object,
	#instVars : [ 'fold', 'values', 'payload' ],
	#category : #'Gts-Core'
}

{ #category : #decoding }
GtsV2RdfTerms >> read: content bytes: bytes fold: aFold values: aStore [
	| header |
	fold := aFold.
	values := aStore.
	header := GtsV2TermsContent read: content expecting: fold dictionarySize newer: false.
	header problem ifNotNil: [ :reason | GtsV2EnvelopeError signal: reason ].
	payload := GtsV2TermsPayload read: bytes content: header newer: false.
	payload validateChargingText: [ :length | fold chargeText: length ].
	payload blocks do: [ :block | self project: block second kind: block first ]
]

{ #category : #private }
GtsV2RdfTerms >> project: columns kind: kind [
	kind <= 2 ifTrue: [ ^ self projectText: columns kind: kind ].
	kind = 3 ifTrue: [ ^ columns first do: [ :label | fold addBlank: label ] ].
	kind = 4 ifTrue: [
		^ columns first withIndexDo: [ :subject :index |
			fold addTripleSubject: subject predicate: (columns second at: index) object: (columns third at: index) ] ].
	kind = 6 ifTrue: [
		^ columns first do: [ :entry | GtsV2RdfTypedValue project: entry into: fold ] ].
	^ self projectReferences: columns octets: kind = 7
]

{ #category : #private }
GtsV2RdfTerms >> projectText: columns kind: kind [
	| literal |
	literal := kind = 2 ifTrue: [ self literalsFrom: (columns copyFrom: 4 to: 6) ].
	payload textsOf: columns do: [ :index :octets |
		| text |
		text := GtsV2RdfColumns utf8: octets.
		kind = 1 ifTrue: [ fold addIri: text ] ifFalse: [ literal value: index value: text ] ]
]

{ #category : #private }
GtsV2RdfTerms >> projectReferences: columns octets: octets [
	| literal |
	literal := octets ifFalse: [ self literalsFrom: (columns copyFrom: 3 to: 5) ].
	columns first withIndexDo: [ :digest :index |
		| value |
		value := fold resolveReference: digest length: (columns second at: index) octets: octets values: values.
		octets
			ifTrue: [ fold addLiteral: value datatypeIri: GtsV2RdfTypedValue hexBinary ]
			ifFalse: [ literal value: index value: value ] ]
]

{ #category : #private }
GtsV2RdfTerms >> literalsFrom: columns [
	| forms datatypes tags datatypeIndex tagIndex |
	forms := columns first.
	datatypes := columns second.
	tags := columns third.
	datatypeIndex := 0.
	tagIndex := 0.
	^ [ :index :text |
		| form tag |
		form := forms at: index.
		form = 1
			ifTrue: [
				datatypeIndex := datatypeIndex + 1.
				fold addLiteral: text datatype: (datatypes at: datatypeIndex) ]
			ifFalse: [
				tagIndex := tagIndex + 1.
				tag := payload languages at: (tags at: tagIndex) + 1.
				fold addLiteral: text language: tag direction: (#(nil nil 'ltr' 'rtl') at: form) ] ]
]
