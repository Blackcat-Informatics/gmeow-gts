Class {
	#name : #GtsTerm,
	#superclass : #Object,
	#instVars : [
		'kind',
		'value',
		'language',
		'direction',
		'datatype',
		'reifier',
		'triple'
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
GtsTerm class >> tripleComponents: spo [
	"A SELF-DESCRIBING quoted-triple term stating its OWN (s p o) (wire 'tt', §7.3)."
	^ self tripleComponents: spo reifier: nil
]

{ #category : #'instance creation' }
GtsTerm class >> tripleComponents: spo reifier: reifierId [
	"A quoted-triple term carrying its own components and, optionally, the LEGACY
	reifier indirection (wire 'rf'). 'tt' is authoritative whenever it is present."
	^ self new
		kind: #triple;
		triple: spo;
		reifier: reifierId;
		yourself
]

{ #category : #'instance creation' }
GtsTerm class >> tripleReifier: reifierId [
	"A LEGACY 'tt'-less quoted-triple term that resolves through a reifier id (§7.3)."
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
	((textDirection isNil) or: [ #('ltr' 'rtl') includes: textDirection ])
		ifFalse: [ self error: 'invalid literal direction' ].
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
GtsTerm >> triple [
	"The quoted triple's OWN (s p o) term ids (wire 'tt'), or nil (§7.3)."
	^ triple
]

{ #category : #accessing }
GtsTerm >> triple: spo [
	"TOTAL by construction: a triple term states exactly three components, so anything
	else is simply not a stated triple. A malformed 'tt' is a reader DIAGNOSTIC plus a
	dropped 'tt' (§7.3), never a raised error, so this setter degrades rather than
	signals."
	triple := (spo notNil
		and: [ spo isCollection and: [ spo isString not and: [ spo size = 3 ] ] ])
		ifTrue: [ spo asArray ]
		ifFalse: [ nil ]
]

{ #category : #accessing }
GtsTerm >> value [
	^ value
]

{ #category : #accessing }
GtsTerm >> value: termValue [
	value := termValue
]
