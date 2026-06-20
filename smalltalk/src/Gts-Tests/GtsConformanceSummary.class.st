Class {
	#name : #GtsConformanceSummary,
	#superclass : #Object,
	#category : #'Gts-Tests'
}

{ #category : #summaries }
GtsConformanceSummary class >> actualForVectorNamed: vectorName [
	| expected graph mode |
	expected := self expectedForVectorNamed: vectorName.
	mode := expected at: 'mode'.
	graph := GtsReader
		read: (self bytesForVectorNamed: vectorName)
		allowSegments: mode ~= 'pre-segment'.
	^ self summarize: graph mode: mode
]

{ #category : #private }
GtsConformanceSummary class >> blobSummaryFor: graph [
	| summary |
	summary := Dictionary new.
	graph blobs associationsDo: [ :association |
		| digest meta mediaType |
		digest := association key.
		meta := graph blobMeta at: digest ifAbsent: [ Dictionary new ].
		mediaType := meta isDictionary
			ifTrue: [ meta at: 'mt' ifAbsent: [ '' ] ]
			ifFalse: [ '' ].
		summary at: digest put: (Dictionary newFromPairs: {
			'size'. association value size.
			'mt'. mediaType }) ].
	^ summary
]

{ #category : #reading }
GtsConformanceSummary class >> bytesForVectorNamed: vectorName [
	^ ('/workspace/vectors/', vectorName, '.gts') asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #comparing }
GtsConformanceSummary class >> divergences [
	| out |
	out := OrderedCollection new.
	self topLevelVectorNames do: [ :vectorName |
		| actual expected |
		expected := self expectedForVectorNamed: vectorName.
		actual := self actualForVectorNamed: vectorName.
		actual = expected ifFalse: [
			out add: (Dictionary newFromPairs: {
				'name'. vectorName.
				'actual'. actual.
				'expected'. expected }) ] ].
	^ out asArray
]

{ #category : #reading }
GtsConformanceSummary class >> expectedForVectorNamed: vectorName [
	| json |
	json := ('/workspace/vectors/', vectorName, '.expected.json') asFileReference
		readStreamDo: [ :stream | stream contents ].
	^ STONJSON fromString: json
]

{ #category : #private }
GtsConformanceSummary class >> nquadLinesFor: graph [
	| text trimmed |
	text := GtsNQuads toNQuads: graph.
	trimmed := text trimBoth.
	trimmed isEmpty ifTrue: [ ^ #() ].
	^ (trimmed substrings: String lf) asSortedCollection asArray
]

{ #category : #private }
GtsConformanceSummary class >> segmentHeadHexFor: graph [
	^ graph segmentHeads collect: [ :head | GtsHex encode: head ]
]

{ #category : #summaries }
GtsConformanceSummary class >> summarize: graph mode: mode [
	^ Dictionary newFromPairs: {
		'mode'. mode.
		'diagnostics'. (graph diagnostics collect: [ :diagnostic | diagnostic code ]).
		'terms'. graph terms size.
		'quads'. graph quads size.
		'segments'. graph segmentHeads size.
		'segment_heads'. (self segmentHeadHexFor: graph).
		'profiles'. graph profiles.
		'streamable'. (self streamableSummaryFor: graph).
		'opaque_reasons'. graph opaqueReasons asSortedCollection asArray.
		'suppressions'. graph suppressions size.
		'blobs'. (self blobSummaryFor: graph).
		'nquads'. (self nquadLinesFor: graph) }
]

{ #category : #private }
GtsConformanceSummary class >> streamableSummaryFor: graph [
	^ graph streamable collect: [ :layout |
		Dictionary newFromPairs: {
			'claimed'. (layout at: 'claimed').
			'covered'. (layout at: 'covered').
			'tail'. (layout at: 'tail') } ]
]

{ #category : #reading }
GtsConformanceSummary class >> topLevelVectorNames [
	| names |
	names := OrderedCollection new.
	'/workspace/vectors' asFileReference children do: [ :entry |
		entry extension = 'gts' ifTrue: [ names add: entry basenameWithoutExtension ] ].
	^ names asSortedCollection asArray
]
