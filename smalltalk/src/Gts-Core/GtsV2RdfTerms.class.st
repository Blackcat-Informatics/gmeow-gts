"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2RdfTerms,
	#superclass : #Object,
	#instVars : [ 'fold', 'values', 'languages', 'usedLanguages', 'restart', 'priorKind' ],
	#category : #'Gts-Core'
}

{ #category : #decoding }
GtsV2RdfTerms >> read: content bytes: bytes fold: aFold values: aStore [
	| head base runs order body blocks |
	fold := aFold.
	values := aStore.
	head := content fields: 4.
	base := head next uint.
	base = fold dictionarySize ifFalse: [ GtsV2EnvelopeError signal: 'inconsistent_counts' ].
	runs := GtsV2RdfColumns read: head next with: [ :run | self readRun: run ].
	runs isEmpty ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	order := head next uint.
	(order between: 1 and: 2) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	restart := head next uint.
	restart = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	body := (GtsV2RdfColumns item: bytes) fields: 2.
	blocks := GtsV2RdfColumns read: body next with: [ :block | block ].
	GtsV2RdfColumns requireCount: runs size in: blocks.
	languages := GtsV2RdfColumns read: body next with: [ :language | GtsV2RdfColumns text: language ].
	usedLanguages := 0.
	languages asSet size = languages size ifFalse: [ GtsV2EnvelopeError signal: 'not_deterministic' ].
	runs withIndexDo: [ :run :index | self readBlock: (blocks at: index) kind: run first count: run second ].
	usedLanguages = languages size ifFalse: [ GtsV2EnvelopeError signal: 'not_deterministic' ]
]

{ #category : #private }
GtsV2RdfTerms >> readRun: field [
	| fields kind count |
	fields := field fields: 2.
	kind := fields next uint.
	(kind between: 1 and: 7) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	kind = priorKind ifTrue: [ GtsV2EnvelopeError signal: 'not_deterministic' ].
	priorKind := kind.
	count := fields next uint.
	count = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	^ { kind. count }
]

{ #category : #private }
GtsV2RdfTerms >> readBlock: field kind: kind count: count [
	| fields labelColumn |
	fields := field fields: (#(4 7 2 4 6 2 3) at: kind).
	fields next uint = kind ifFalse: [ GtsV2EnvelopeError signal: 'forbidden_combination' ].
	kind <= 2 ifTrue: [ ^ self readFrontCoded: fields kind: kind count: count ].
	kind = 4 ifTrue: [ ^ self readTriples: fields count: count ].
	(kind = 5 or: [ kind = 7 ]) ifTrue: [
		^ self readReferences: fields count: count octets: kind = 7 ].
	kind = 6 ifTrue: [ ^ self readTypedValues: fields next count: count ].
	labelColumn := GtsV2RdfColumns read: fields next with: [ :item | GtsV2RdfColumns body: item size: nil ].
	GtsV2RdfColumns requireCount: count in: labelColumn.
	labelColumn do: [ :label | fold addBlank: label ]
]

{ #category : #private }
GtsV2RdfTerms >> readFrontCoded: fields kind: kind count: count [
	| prefixes lengths suffixes cursor previous literal |
	prefixes := GtsV2RdfColumns uints: fields next.
	lengths := GtsV2RdfColumns uints: fields next.
	GtsV2RdfColumns requireCount: count in: prefixes.
	GtsV2RdfColumns requireCount: count in: lengths.
	suffixes := GtsV2RdfColumns body: fields next size: nil.
	literal := kind = 2 ifTrue: [ self literalsFrom: fields count: count ].
	cursor := 1.
	previous := #[].
	1 to: count do: [ :index |
		| prefix length end isRestart text |
		prefix := prefixes at: index.
		length := lengths at: index.
		end := cursor + length - 1.
		(prefix > previous size or: [ end > suffixes size ])
			ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
		isRestart := (index - 1) \\ restart = 0.
		(isRestart and: [ prefix ~= 0 ]) ifTrue: [ GtsV2EnvelopeError signal: 'forbidden_combination' ].
		(isRestart not and: [ prefix < previous size and: [ length > 0 and: [
			(previous at: prefix + 1) = (suffixes at: cursor) ] ] ])
			ifTrue: [ GtsV2EnvelopeError signal: 'not_deterministic' ].
		fold chargeText: prefix + length.
		previous := (previous copyFrom: 1 to: prefix), (suffixes copyFrom: cursor to: end).
		cursor := end + 1.
		text := GtsV2RdfColumns utf8: previous.
		kind = 1 ifTrue: [ fold addIri: text ] ifFalse: [ literal value: index value: text ] ].
	cursor = (suffixes size + 1) ifFalse: [ GtsV2EnvelopeError signal: 'inconsistent_counts' ]
]

{ #category : #private }
GtsV2RdfTerms >> readTriples: fields count: count [
	| subjects predicates objects |
	subjects := GtsV2RdfColumns uints: fields next.
	predicates := GtsV2RdfColumns uints: fields next.
	objects := GtsV2RdfColumns uints: fields next.
	{ subjects. predicates. objects } do: [ :column | GtsV2RdfColumns requireCount: count in: column ].
	1 to: count do: [ :index |
		fold addTripleSubject: (subjects at: index) predicate: (predicates at: index) object: (objects at: index) ]
]

{ #category : #private }
GtsV2RdfTerms >> readTypedValues: field count: count [
	| entries |
	entries := GtsV2RdfColumns read: field with: [ :item | item ].
	GtsV2RdfColumns requireCount: count in: entries.
	entries do: [ :item | GtsV2RdfTypedValue read: item into: fold ]
]

{ #category : #private }
GtsV2RdfTerms >> readReferences: fields count: count octets: octets [
	| digests lengths literal |
	digests := GtsV2RdfColumns read: fields next with: [ :digest | GtsV2RdfColumns body: digest size: 32 ].
	lengths := GtsV2RdfColumns uints: fields next.
	GtsV2RdfColumns requireCount: count in: digests.
	GtsV2RdfColumns requireCount: count in: lengths.
	octets ifFalse: [ literal := self literalsFrom: fields count: count ].
	1 to: count do: [ :index |
		| digest length |
		digest := digests at: index.
		length := lengths at: index.
		octets
			ifTrue: [
				fold chargeText: length * 2.
				fold addLiteral: (values octetsFor: digest length: length) datatypeIri: GtsV2RdfTypedValue hexBinary ]
			ifFalse: [
				fold chargeText: length.
				literal value: index value: (values textFor: digest length: length) ] ]
]

{ #category : #private }
GtsV2RdfTerms >> literalsFrom: fields count: count [
	| forms datatypes tags typedCount datatypeIndex tagIndex |
	forms := GtsV2RdfColumns uints: fields next.
	datatypes := GtsV2RdfColumns uints: fields next.
	tags := GtsV2RdfColumns uints: fields next.
	GtsV2RdfColumns requireCount: count in: forms.
	(forms allSatisfy: [ :form | form between: 1 and: 4 ])
		ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	typedCount := forms count: [ :form | form = 1 ].
	GtsV2RdfColumns requireCount: typedCount in: datatypes.
	GtsV2RdfColumns requireCount: count - typedCount in: tags.
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
				tag := self languageAt: (tags at: tagIndex).
				fold addLiteral: text language: tag direction: (#(nil nil 'ltr' 'rtl') at: form) ] ]
]

{ #category : #private }
GtsV2RdfTerms >> languageAt: index [
	index < languages size ifFalse: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	index > usedLanguages ifTrue: [ GtsV2EnvelopeError signal: 'not_deterministic' ].
	index = usedLanguages ifTrue: [ usedLanguages := usedLanguages + 1 ].
	^ languages at: index + 1
]
