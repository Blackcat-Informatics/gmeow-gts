Class {
	#name : #GtsTerm,
	#superclass : #Object,
	#instVars : [
		'kind',
		'value',
		'language',
		'datatype'
	],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsTerm class >> blankNode: label [
	^ self new
		kind: #blankNode;
		value: label;
		yourself
]

{ #category : #'instance creation' }
GtsTerm class >> iri: iriString [
	^ self new
		kind: #iri;
		value: iriString;
		yourself
]

{ #category : #'instance creation' }
GtsTerm class >> literal: literalValue language: languageTag datatype: datatypeId [
	^ self new
		kind: #literal;
		value: literalValue;
		language: languageTag;
		datatype: datatypeId;
		yourself
]

{ #category : #accessing }
GtsTerm >> datatype [
	^ datatype
]

{ #category : #accessing }
GtsTerm >> datatype: datatypeId [
	datatype := datatypeId
]

{ #category : #accessing }
GtsTerm >> kind [
	^ kind
]

{ #category : #accessing }
GtsTerm >> kind: aSymbol [
	kind := aSymbol
]

{ #category : #accessing }
GtsTerm >> language [
	^ language
]

{ #category : #accessing }
GtsTerm >> language: languageTag [
	language := languageTag
]

{ #category : #accessing }
GtsTerm >> value [
	^ value
]

{ #category : #accessing }
GtsTerm >> value: termValue [
	value := termValue
]
