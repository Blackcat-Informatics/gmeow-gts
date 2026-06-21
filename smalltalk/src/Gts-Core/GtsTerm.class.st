Class {
	#name : #GtsTerm,
	#superclass : #Object,
	#instVars : [
		'kind',
		'value',
		'language',
		'direction',
		'datatype',
		'reifier'
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
	^ self
		literal: literalValue
		language: languageTag
		direction: nil
		datatype: datatypeId
]

{ #category : #'instance creation' }
GtsTerm class >> literal: literalValue language: languageTag direction: textDirection datatype: datatypeId [
	^ self new
		kind: #literal;
		value: literalValue;
		language: languageTag;
		direction: textDirection;
		datatype: datatypeId;
		yourself
]

{ #category : #'instance creation' }
GtsTerm class >> tripleReifier: reifierId [
	^ self new
		kind: #triple;
		reifier: reifierId;
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
GtsTerm >> direction [
	^ direction
]

{ #category : #accessing }
GtsTerm >> direction: textDirection [
	direction := textDirection
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
GtsTerm >> reifier [
	^ reifier
]

{ #category : #accessing }
GtsTerm >> reifier: reifierId [
	reifier := reifierId
]

{ #category : #accessing }
GtsTerm >> value [
	^ value
]

{ #category : #accessing }
GtsTerm >> value: termValue [
	value := termValue
]
