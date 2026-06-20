Class {
	#name : #GtsNQuadsTokenizer,
	#superclass : #Object,
	#instVars : [
		'source',
		'position'
	],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsNQuadsTokenizer class >> on: line [
	^ self new initializeOn: line
]

{ #category : #testing }
GtsNQuadsTokenizer >> atEnd [
	self skipWhitespace.
	^ position > source size or: [ (source at: position) = $. ]
]

{ #category : #private }
GtsNQuadsTokenizer >> atomKind: kind value: value [
	^ Dictionary newFromPairs: {
		'nodeType'. 'atom'.
		'kind'. kind.
		'value'. value }
]

{ #category : #private }
GtsNQuadsTokenizer >> atomKind: kind value: value language: language datatype: datatype [
	| atom |
	atom := self atomKind: kind value: value.
	language ifNotNil: [ atom at: 'language' put: language ].
	datatype ifNotNil: [ atom at: 'datatype' put: datatype ].
	^ atom
]

{ #category : #private }
GtsNQuadsTokenizer >> bnode [
	| start |
	(self startsWith: '_:') ifFalse: [ self error: 'bad blank node in ', source ].
	position := position + 2.
	start := position.
	[ position <= source size and: [ self isBlankNodeCharacter: (source at: position) ] ]
		whileTrue: [ position := position + 1 ].
	(position > start and: [ (source at: position - 1) = $. ])
		ifTrue: [ position := position - 1 ].
	position = start ifTrue: [ self error: 'empty blank node label in ', source ].
	^ source copyFrom: start to: position - 1
]

{ #category : #private }
GtsNQuadsTokenizer >> consumeStatementTerminator [
	self skipWhitespace.
	(position <= source size and: [ (source at: position) = $. ])
		ifFalse: [ self error: 'expected statement terminator in ', source ].
	position := position + 1.
	self skipWhitespace.
	position <= source size
		ifTrue: [ self error: 'unexpected trailing tokens in ', source ]
]

{ #category : #private }
GtsNQuadsTokenizer >> escape [
	| character code raw width |
	position > source size ifTrue: [ self error: 'bad escape at end of ', source ].
	character := source at: position.
	position := position + 1.
	character = $\ ifTrue: [ ^ '\' ].
	character = $" ifTrue: [ ^ '"' ].
	character = $b ifTrue: [ ^ (String with: (Character value: 8)) ].
	character = $f ifTrue: [ ^ (String with: (Character value: 12)) ].
	character = $n ifTrue: [ ^ String lf ].
	character = $r ifTrue: [ ^ String cr ].
	character = $t ifTrue: [ ^ String tab ].
	(character = $u or: [ character = $U ]) ifTrue: [
		width := character = $u ifTrue: [ 4 ] ifFalse: [ 8 ].
		position + width - 1 > source size
			ifTrue: [ self error: 'bad unicode escape in ', source ].
		raw := source copyFrom: position to: position + width - 1.
		(raw allSatisfy: [ :each | self isHexDigitCharacter: each ])
			ifFalse: [ self error: 'bad unicode escape in ', source ].
		position := position + width.
		code := self integerFromHex: raw.
		^ String with: (Character value: code) ].
	self error: 'unsupported escape \', character asString, ' in ', source
]

{ #category : #private }
GtsNQuadsTokenizer >> expect: token [
	(self startsWith: token) ifFalse: [ self error: 'expected ', token, ' in ', source ].
	position := position + token size
]

{ #category : #private }
GtsNQuadsTokenizer >> hexValueOf: character [
	| code |
	code := character asciiValue.
	(code between: $0 asciiValue and: $9 asciiValue)
		ifTrue: [ ^ code - $0 asciiValue ].
	(code between: $a asciiValue and: $f asciiValue)
		ifTrue: [ ^ code - $a asciiValue + 10 ].
	(code between: $A asciiValue and: $F asciiValue)
		ifTrue: [ ^ code - $A asciiValue + 10 ].
	self error: 'invalid hex digit'
]

{ #category : #initialization }
GtsNQuadsTokenizer >> initializeOn: line [
	source := line.
	position := 1.
	^ self
]

{ #category : #private }
GtsNQuadsTokenizer >> integerFromHex: text [
	| value |
	value := 0.
	text do: [ :character |
		value := (value bitShift: 4) + (self hexValueOf: character) ].
	^ value
]

{ #category : #private }
GtsNQuadsTokenizer >> iri [
	| end start |
	(position <= source size and: [ (source at: position) = $< ])
		ifFalse: [ self error: 'bad IRI in ', source ].
	start := position + 1.
	end := start.
	[ end <= source size and: [ (source at: end) ~= $> ] ]
		whileTrue: [ end := end + 1 ].
	end > source size ifTrue: [ self error: 'unterminated IRI in ', source ].
	position := end + 1.
	^ source copyFrom: start to: end - 1
]

{ #category : #private }
GtsNQuadsTokenizer >> isAsciiLetterOrDigit: character [
	| code |
	code := character asciiValue.
	^ (code between: $0 asciiValue and: $9 asciiValue)
		or: [ (code between: $A asciiValue and: $Z asciiValue)
		or: [ code between: $a asciiValue and: $z asciiValue ] ]
]

{ #category : #private }
GtsNQuadsTokenizer >> isBlankNodeCharacter: character [
	^ (self isAsciiLetterOrDigit: character)
		or: [ character = $_ or: [ character = $- or: [ character = $. ] ] ]
]

{ #category : #private }
GtsNQuadsTokenizer >> isHexDigitCharacter: character [
	| code |
	code := character asciiValue.
	^ (code between: $0 asciiValue and: $9 asciiValue)
		or: [ (code between: $A asciiValue and: $F asciiValue)
		or: [ code between: $a asciiValue and: $f asciiValue ] ]
]

{ #category : #private }
GtsNQuadsTokenizer >> isLanguageCharacter: character [
	^ (self isAsciiLetterOrDigit: character) or: [ character = $- ]
]

{ #category : #private }
GtsNQuadsTokenizer >> literal [
	| character datatype language start value |
	position := position + 1.
	value := String streamContents: [ :stream |
		[ position <= source size ] whileTrue: [
			character := source at: position.
			position := position + 1.
			character = $\ ifTrue: [ stream nextPutAll: self escape ] ifFalse: [
			character = $" ifTrue: [
				language := nil.
				datatype := nil.
				(position <= source size and: [ (source at: position) = $@ ])
					ifTrue: [
						position := position + 1.
						start := position.
						[ position <= source size and: [ self isLanguageCharacter: (source at: position) ] ]
							whileTrue: [ position := position + 1 ].
						position = start ifTrue: [ self error: 'empty language tag in ', source ].
						language := source copyFrom: start to: position - 1 ]
					ifFalse: [
						(self startsWith: '^^') ifTrue: [
							position := position + 2.
							self skipWhitespace.
							datatype := self iri ] ].
				^ self
					atomKind: #literal
					value: stream contents
					language: language
					datatype: datatype ].
			stream nextPut: character ] ] ].
	self error: 'unterminated literal in ', source
]

{ #category : #parsing }
GtsNQuadsTokenizer >> node [
	self skipWhitespace.
	position > source size ifTrue: [ self error: 'unexpected end of line: ', source ].
	(self startsWith: '<<(') ifTrue: [ ^ self quotedTriple ].
	(source at: position) = $< ifTrue: [ ^ self atomKind: #iri value: self iri ].
	(source at: position) = $_ ifTrue: [ ^ self atomKind: #blankNode value: self bnode ].
	(source at: position) = $" ifTrue: [ ^ self literal ].
	self error: 'unexpected token in ', source
]

{ #category : #private }
GtsNQuadsTokenizer >> quotedTriple [
	| object predicate subject |
	position := position + 3.
	subject := self node.
	predicate := self node.
	object := self node.
	self skipWhitespace.
	self expect: ')>>'.
	^ Dictionary newFromPairs: {
		'nodeType'. 'triple'.
		's'. subject.
		'p'. predicate.
		'o'. object }
]

{ #category : #private }
GtsNQuadsTokenizer >> skipWhitespace [
	[ position <= source size
		and: [ (source at: position) = Character space
		or: [ (source at: position) = Character tab ] ] ]
		whileTrue: [ position := position + 1 ]
]

{ #category : #private }
GtsNQuadsTokenizer >> startsWith: token [
	^ (source findString: token startingAt: position) = position
]
