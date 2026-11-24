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
	^ self
		atomKind: kind
		value: value
		language: language
		direction: nil
		datatype: datatype
]

{ #category : #private }
GtsNQuadsTokenizer >> atomKind: kind value: value language: language direction: direction datatype: datatype [
	| atom |
	atom := self atomKind: kind value: value.
	language ifNotNil: [ atom at: 'language' put: language ].
	direction ifNotNil: [ atom at: 'direction' put: direction ].
	datatype ifNotNil: [ atom at: 'datatype' put: datatype ].
	^ atom
]

{ #category : #private }
GtsNQuadsTokenizer >> bnode [
	| start finish |
	(self startsWith: '_:') ifFalse: [ GtsParserError signal: 'bad blank node in ', source ].
	position := position + 2.
	start := position.
	(position <= source size and: [ self isBlankNodeStart: (source at: position) ])
		ifFalse: [ GtsParserError signal: 'invalid blank node label in ', source ].
	finish := position.
	[ position <= source size and: [ self isBlankNodeCharacter: (source at: position) ] ]
		whileTrue: [
			(source at: position) = $. ifFalse: [ finish := position ].
			position := position + 1 ].
	position := finish + 1.
	^ source copyFrom: start to: finish
]

{ #category : #private }
GtsNQuadsTokenizer >> consumeStatementTerminator [
	self skipWhitespace.
	(position <= source size and: [ (source at: position) = $. ])
		ifFalse: [ GtsParserError signal: 'expected statement terminator in ', source ].
	position := position + 1.
	self consumeCommentOrEnd
]

{ #category : #parsing }
GtsNQuadsTokenizer >> consumeVersionHint [
	| hint |
	self expect: 'VERSION'.
	self skipWhitespace.
	(position <= source size and: [ (source at: position) = $" ])
		ifFalse: [ GtsParserError signal: 'expected a VERSION string' ].
	hint := self literal.
	((hint includesKey: 'datatype') or: [ hint includesKey: 'language' ]) ifTrue: [
		GtsParserError signal: 'VERSION requires a plain string' ].
	self consumeCommentOrEnd
]

{ #category : #private }
GtsNQuadsTokenizer >> consumeCommentOrEnd [
	self skipWhitespace.
	(position <= source size and: [ (source at: position) ~= $# ])
		ifTrue: [ GtsParserError signal: 'unexpected trailing tokens in ', source ]
]

{ #category : #private }
GtsNQuadsTokenizer >> escape [
	| character code raw width |
	position > source size ifTrue: [ GtsParserError signal: 'bad escape at end of ', source ].
	character := source at: position.
	position := position + 1.
	character = $\ ifTrue: [ ^ '\' ].
	character = $" ifTrue: [ ^ '"' ].
	character = $' ifTrue: [ ^ '''' ].
	character = $b ifTrue: [ ^ (String with: (Character value: 8)) ].
	character = $f ifTrue: [ ^ (String with: (Character value: 12)) ].
	character = $n ifTrue: [ ^ String lf ].
	character = $r ifTrue: [ ^ String cr ].
	character = $t ifTrue: [ ^ String tab ].
	(character = $u or: [ character = $U ]) ifTrue: [
		width := character = $u ifTrue: [ 4 ] ifFalse: [ 8 ].
		position + width - 1 > source size
			ifTrue: [ GtsParserError signal: 'bad unicode escape in ', source ].
		raw := source copyFrom: position to: position + width - 1.
		(raw allSatisfy: [ :each | self isHexDigitCharacter: each ])
			ifFalse: [ GtsParserError signal: 'bad unicode escape in ', source ].
		position := position + width.
		code := self integerFromHex: raw.
		(code > 16r10FFFF or: [ code between: 16rD800 and: 16rDFFF ]) ifTrue: [
			GtsParserError signal: 'N-Quads escape is outside the Unicode scalar range' ].
		^ String with: (Character value: code) ].
	GtsParserError signal: 'unsupported escape \', character asString, ' in ', source
]

{ #category : #private }
GtsNQuadsTokenizer >> expect: token [
	(self startsWith: token) ifFalse: [ GtsParserError signal: 'expected ', token, ' in ', source ].
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
	GtsParserError signal: 'invalid hex digit'
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
	| character |
	(position <= source size and: [ (source at: position) = $< ])
		ifFalse: [ GtsParserError signal: 'bad IRI in ', source ].
	position := position + 1.
	String streamContents: [ :stream |
		[ position <= source size ] whileTrue: [
			character := source at: position.
			position := position + 1.
			character = $> ifTrue: [ ^ stream contents ].
			character = $\
				ifTrue: [
					(position <= source size and: [ 'uU' includes: (source at: position) ])
						ifFalse: [ GtsParserError signal: 'N-Quads IRI escapes require a Unicode value' ].
					stream nextPutAll: self escape ]
				ifFalse: [ stream nextPut: character ] ] ].
	GtsParserError signal: 'unterminated IRI in ', source
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
	| code |
	code := character asciiValue.
	^ (self isBlankNodeStart: character) or: [
		character = $- or: [ character = $. or: [ code = 16rB7 or: [
			(code between: 16r300 and: 16r36F) or: [ code between: 16r203F and: 16r2040 ] ] ] ] ]
]

{ #category : #private }
GtsNQuadsTokenizer >> isBlankNodeStart: character [
	| code |
	((self isAsciiLetterOrDigit: character) or: [ character = $_ ]) ifTrue: [ ^ true ].
	code := character asciiValue.
	^ #(
		(16rC0 16rD6) (16rD8 16rF6) (16rF8 16r2FF) (16r370 16r37D)
		(16r37F 16r1FFF) (16r200C 16r200D) (16r2070 16r218F) (16r2C00 16r2FEF)
		(16r3001 16rD7FF) (16rF900 16rFDCF) (16rFDF0 16rFFFD) (16r10000 16rEFFFF)
	) anySatisfy: [ :range | code between: range first and: range second ]
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
	| character datatype direction language start value |
	position := position + 1.
	value := String streamContents: [ :stream |
		[ position <= source size ] whileTrue: [
			character := source at: position.
			position := position + 1.
			character = $\ ifTrue: [ stream nextPutAll: self escape ] ifFalse: [
			character = $" ifTrue: [
				language := nil.
				direction := nil.
				datatype := nil.
				(position <= source size and: [ (source at: position) = $@ ])
					ifTrue: [
						position := position + 1.
						start := position.
						[ position <= source size and: [ self isLanguageCharacter: (source at: position) ] ]
							whileTrue: [ position := position + 1 ].
						position = start ifTrue: [ GtsParserError signal: 'empty language tag in ', source ].
						language := source copyFrom: start to: position - 1.
						(language size >= 5 and: [ (language copyFrom: language size - 4 to: language size) = '--ltr' ])
							ifTrue: [
								direction := 'ltr'.
								language := language size = 5
									ifTrue: [ '' ]
									ifFalse: [ language copyFrom: 1 to: language size - 5 ] ].
						(language size >= 5 and: [ (language copyFrom: language size - 4 to: language size) = '--rtl' ])
							ifTrue: [
								direction := 'rtl'.
								language := language size = 5
									ifTrue: [ '' ]
									ifFalse: [ language copyFrom: 1 to: language size - 5 ] ].
						self validateLanguage: language ]
					ifFalse: [
						(self startsWith: '^^') ifTrue: [
							position := position + 2.
							self skipWhitespace.
							datatype := self iri ] ].
				^ self
					atomKind: #literal
					value: stream contents
					language: language
					direction: direction
					datatype: datatype ].
			stream nextPut: character ] ] ].
	GtsParserError signal: 'unterminated literal in ', source
]

{ #category : #private }
GtsNQuadsTokenizer >> validateLanguage: language [
	| primaryEnd primary |
	(language isEmpty or: [ language last = $- or: [ language includesSubstring: '--' ] ])
		ifTrue: [ GtsParserError signal: 'invalid N-Quads language tag' ].
	(language allSatisfy: [ :character | self isLanguageCharacter: character ])
		ifFalse: [ GtsParserError signal: 'invalid N-Quads language tag' ].
	primaryEnd := language indexOf: $- ifAbsent: [ language size + 1 ].
	primary := language copyFrom: 1 to: primaryEnd - 1.
	(primary isNotEmpty and: [ primary allSatisfy: [ :character |
		(character between: $A and: $Z) or: [ character between: $a and: $z ] ] ])
		ifFalse: [ GtsParserError signal: 'invalid N-Quads language tag' ]
]

{ #category : #parsing }
GtsNQuadsTokenizer >> node [
	self skipWhitespace.
	position > source size ifTrue: [ GtsParserError signal: 'unexpected end of line: ', source ].
	(self startsWith: '<<(') ifTrue: [ ^ self quotedTriple ].
	(source at: position) = $< ifTrue: [ ^ self atomKind: #iri value: self iri ].
	(source at: position) = $_ ifTrue: [ ^ self atomKind: #blankNode value: self bnode ].
	(source at: position) = $" ifTrue: [ ^ self literal ].
	GtsParserError signal: 'unexpected token in ', source
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
