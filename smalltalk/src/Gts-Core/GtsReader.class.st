Class {
	#name : #GtsReader,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #folding }
GtsReader class >> catalogFromHeader: header [
	^ header at: 'cat' ifAbsent: [ Dictionary new ]
]

{ #category : #codecs }
GtsReader class >> decodeTransform: transformId data: encodedData catalog: catalog [
	| codec codecName |
	codec := catalog at: transformId ifAbsent: [ self error: 'unknown codec' ].
	codecName := codec at: 'name' ifAbsent: [ self error: 'codec has no name' ].
	codecName = 'identity' ifTrue: [ ^ encodedData ].
	((codecName = 'zstd') or: [ codecName = 'zstd-rsyncable' ])
		ifTrue: [ ^ GtsZstd decompress: encodedData ].
	self error: 'unsupported codec'
]

{ #category : #folding }
GtsReader class >> foldFrame: frame index: frameIndex into: graph catalog: catalog [
	| frameType payload |
	frameType := frame at: 't' ifAbsent: [ '' ].
	payload := self payloadForFrame: frame catalog: catalog.
	frameType = 'terms' ifTrue: [ ^ self foldTerms: payload into: graph ].
	frameType = 'quads' ifTrue: [ ^ self foldQuads: payload into: graph ].
	graph addDiagnostic: (GtsDiagnostic
		code: 'UnknownFrameType'
		detail: 'unsupported frame type'
		frameIndex: frameIndex)
]

{ #category : #folding }
GtsReader class >> foldQuads: rows into: graph [
	rows do: [ :row | graph addQuad: row ]
]

{ #category : #folding }
GtsReader class >> foldTerm: termMap into: graph [
	| kind |
	kind := termMap at: 'k'.
	kind = 0 ifTrue: [ ^ graph addTerm: (GtsTerm iri: (termMap at: 'v')) ].
	kind = 1 ifTrue: [
		^ graph addTerm: (GtsTerm
			literal: (termMap at: 'v')
			language: (termMap at: 'l' ifAbsent: [ nil ])
			datatype: (termMap at: 'dt' ifAbsent: [ nil ])) ].
	kind = 2 ifTrue: [ ^ graph addTerm: (GtsTerm blankNode: (termMap at: 'v')) ].
	self error: 'unsupported term kind'
]

{ #category : #folding }
GtsReader class >> foldTerms: termRows into: graph [
	termRows do: [ :termMap | self foldTerm: termMap into: graph ]
]

{ #category : #codecs }
GtsReader class >> payloadForFrame: frame catalog: catalog [
	| data transforms |
	data := frame at: 'd' ifAbsent: [ nil ].
	transforms := frame at: 'x' ifAbsent: [ ^ data ].
	transforms reverseDo: [ :transformId |
		data := self decodeTransform: transformId data: data catalog: catalog ].
	^ GtsCborReader decode: data
]

{ #category : #reading }
GtsReader class >> read: aByteArray [
	| catalog expectedPrev graph header items lastHead |
	graph := GtsGraph new.
	items := GtsCborReader itemsFrom: aByteArray.
	items isEmpty ifTrue: [
		graph addDiagnostic: (GtsDiagnostic code: 'DamagedFrame' detail: 'empty input' frameIndex: 0).
		^ graph ].
	header := GtsWire unwrapHeader: items first.
	catalog := self catalogFromHeader: header.
	((header at: 'id' ifAbsent: [ nil ]) = (GtsWire headerIdFor: header))
		ifFalse: [
			graph addDiagnostic: (GtsDiagnostic code: 'DamagedFrame' detail: 'header self-hash mismatch' frameIndex: 0) ].
	graph addProfile: (header at: 'prof' ifAbsent: [ 'generic' ]).
	expectedPrev := header at: 'id'.
	lastHead := expectedPrev.
	2 to: items size do: [ :itemIndex |
		| computed frame frameIndex storedId |
		frame := items at: itemIndex.
		frameIndex := itemIndex - 1.
		storedId := frame at: 'id' ifAbsent: [ nil ].
		computed := GtsWire contentIdForFrame: frame.
		storedId = computed
			ifFalse: [
				graph addDiagnostic: (GtsDiagnostic code: 'DamagedFrame' detail: 'frame self-hash mismatch' frameIndex: frameIndex) ].
		(frame at: 'prev' ifAbsent: [ nil ]) = expectedPrev
			ifFalse: [
				graph addDiagnostic: (GtsDiagnostic code: 'DamagedFrame' detail: 'prev chain mismatch' frameIndex: frameIndex) ].
		[ self foldFrame: frame index: frameIndex into: graph catalog: catalog ]
			on: Error
			do: [ :error |
				graph addDiagnostic: (GtsDiagnostic
					code: 'DamagedFrame'
					detail: error messageText
					frameIndex: frameIndex) ].
		expectedPrev := storedId.
		lastHead := storedId ].
	graph addSegmentHead: lastHead.
	^ graph
]
