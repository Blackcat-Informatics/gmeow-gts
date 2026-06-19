Class {
	#name : #GtsMinimalVectorFixture,
	#superclass : #Object,
	#category : #'Gts-Tests'
}

{ #category : #building }
GtsMinimalVectorFixture class >> bytes [
	| header termsFrame quadsFrame |
	header := self header.
	termsFrame := self termsFrameAfter: (header at: 'id').
	quadsFrame := self quadsFrameAfter: (termsFrame at: 'id').
	^ ByteArray streamContents: [ :stream |
		stream nextPutAll: (GtsCborWriter encodeSelfDescribed: header).
		stream nextPutAll: (GtsCborWriter encode: termsFrame).
		stream nextPutAll: (GtsCborWriter encode: quadsFrame) ]
]

{ #category : #private }
GtsMinimalVectorFixture class >> catalog [
	^ self mapFrom: {
		0 -> (self codecNamed: 'identity' classifiedAs: 'encode').
		1 -> (self codecNamed: 'gzip' classifiedAs: 'compress').
		2 -> (self codecNamed: 'zstd' classifiedAs: 'compress').
		3 -> (self codecNamed: 'zstd-rsyncable' classifiedAs: 'compress').
		7 -> (self codecNamed: 'cose-encrypt0' classifiedAs: 'encrypt') }
]

{ #category : #private }
GtsMinimalVectorFixture class >> codecNamed: codecName classifiedAs: codecClass [
	^ self mapFrom: {
		'cls' -> codecClass.
		'name' -> codecName }
]

{ #category : #private }
GtsMinimalVectorFixture class >> contentIdFor: frame [
	| preimage |
	preimage := Dictionary new.
	frame associationsDo: [ :association |
		((#('id' 'sig') includes: association key) not)
			ifTrue: [ preimage at: association key put: association value ] ].
	^ GtsBlake3 hash: (GtsCborWriter encode: preimage)
]

{ #category : #private }
GtsMinimalVectorFixture class >> frameWithId: frame [
	frame at: 'id' put: (self contentIdFor: frame).
	^ frame
]

{ #category : #private }
GtsMinimalVectorFixture class >> header [
	| header |
	header := self mapFrom: {
		'cat' -> self catalog.
		'gts' -> 'GTS1'.
		'prof' -> 'dist'.
		'v' -> 1 }.
	header at: 'id' put: (self headerIdFor: header).
	^ header
]

{ #category : #private }
GtsMinimalVectorFixture class >> headerIdFor: header [
	^ GtsBlake3 hash: (GtsCborWriter encode: header)
]

{ #category : #private }
GtsMinimalVectorFixture class >> iriTerm: value [
	^ self mapFrom: {
		'k' -> 0.
		'v' -> value }
]

{ #category : #private }
GtsMinimalVectorFixture class >> literalTerm: value language: language [
	^ self mapFrom: {
		'k' -> 1.
		'l' -> language.
		'v' -> value }
]

{ #category : #private }
GtsMinimalVectorFixture class >> mapFrom: associations [
	| dictionary |
	dictionary := Dictionary new.
	associations do: [ :association | dictionary add: association ].
	^ dictionary
]

{ #category : #private }
GtsMinimalVectorFixture class >> quadsFrameAfter: previousId [
	^ self frameWithId: (self mapFrom: {
		'd' -> { #(0 1 2) }.
		'prev' -> previousId.
		't' -> 'quads' })
]

{ #category : #private }
GtsMinimalVectorFixture class >> termsFrameAfter: previousId [
	^ self frameWithId: (self mapFrom: {
		'd' -> {
			self iriTerm: 'https://example.org/Cat'.
			self iriTerm: 'http://www.w3.org/2000/01/rdf-schema#label'.
			self literalTerm: 'Cat' language: 'en' }.
		'prev' -> previousId.
		't' -> 'terms' })
]
