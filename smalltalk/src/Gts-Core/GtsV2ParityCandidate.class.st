"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ParityCandidate,
	#superclass : #Object,
	#instVars : [ 'walker', 'offset', 'limit' ],
	#category : #'Gts-Core'
}

{ #category : #scanning }
GtsV2ParityCandidate class >> scan: walker resyncFrom: first parityFrom: damaged checks: checks previous: previous budget: budget [
	| bytes parity resync |
	budget decodedExhausted ifTrue: [ ^ nil ].
	bytes := walker bytes.
	parity := #[139 2 16 244 245].
	resync := GtsV2ResyncScan pattern.
	(first min: damaged) to: bytes size do: [ :start |
		(bytes at: start) = 139 ifTrue: [
			| result |
			(start >= first and: [ self prefix: resync at: start in: bytes ]) ifTrue: [
				result := GtsV2ResyncScan candidateOn: walker at: start budget: budget ].
			(start >= damaged and: [ self prefix: parity at: start in: bytes ]) ifTrue: [
				result := self new check: walker at: start checks: checks previous: previous budget: budget ].
			result ifNotNil: [ ^ result ] ] ].
	^ nil
]

{ #category : #private }
GtsV2ParityCandidate class >> prefix: pattern at: start in: bytes [
	start + pattern size - 1 > bytes size ifTrue: [ ^ false ].
	1 to: pattern size do: [ :index |
		(bytes at: start + index - 1) = (pattern at: index) ifFalse: [ ^ false ] ].
	^ true
]

{ #category : #private }
GtsV2ParityCandidate >> check: aWalker at: start checks: checks previous: previous budget: budget [
	| first end span anchor table length field frame payload |
	walker := aWalker.
	offset := start + 5.
	limit := walker bytes size + 1.
	[
		self expect: 133.
		self expect: 1.
		first := (self head: 0) argument.
		first \\ 4194304 = 0 ifFalse: [ ^ nil ].
		end := (self head: 0) argument.
		span := end - first.
		(span between: 1 and: 4194304) ifFalse: [ ^ nil ].
		anchor := (self bytesOfSize: 32) encodedBytes.
		table := (self bytesOfSize: 32) encodedBytes.
		(checks allowsCandidateFrom: first after: previous) ifFalse: [ ^ nil ].
		length := (self head: 0) argument.
		length = (self payloadLengthFor: (checks layoutFor: span)) ifFalse: [ ^ nil ].
		self bytesOfSize: 32.
		self expect: 129.
		self expect: 1.
		field := self bytesOfSize: length.
		self bytesOfSize: 32.
		self expect: 128
	] on: GtsV2FramingError, GtsV2EnvelopeError do: [ :error | ^ nil ].
	frame := GtsV2FrameEnvelope read: (GtsV2Field walker: walker from: start to: offset) registry: 1.
	((frame identifierOn: walker bytes predecessor: anchor) = frame storedIdentifier) ifFalse: [ ^ nil ].
	payload := frame payload checkWith: budget retainDecoded: false.
	payload reason = 'total_decoded' ifTrue: [
		^ (GtsV2WireEvent kind: #parityRefused from: start - 1 to: offset - 1 problem: payload reason)
			resyncRefusal: payload ].
	payload state = #verified ifFalse: [ ^ nil ].
	(checks checkedTable: table payload: field encodedBytes spanSize: span) ifNil: [ ^ nil ].
	^ (GtsV2WireEvent kind: #frame from: start - 1 to: offset - 1 problem: nil)
		opener: nil frame: frame;
		resumeFrom: anchor checkedPayload: payload;
		yourself
]

{ #category : #private }
GtsV2ParityCandidate >> expect: byte [
	(offset >= limit or: [ (walker bytes at: offset) ~= byte ]) ifTrue: [
		GtsV2EnvelopeError signal: 'wrong_type' ].
	offset := offset + 1
]

{ #category : #private }
GtsV2ParityCandidate >> head: major [
	| head |
	head := (GtsV2Field walker: walker from: offset to: limit) typedHead: major.
	offset := head end.
	^ head
]

{ #category : #private }
GtsV2ParityCandidate >> bytesOfSize: size [
	| head start |
	head := self head: 2.
	start := offset.
	(head argument = size and: [ start + size <= limit ]) ifFalse: [
		GtsV2EnvelopeError signal: 'wrong_length' ].
	offset := start + size.
	^ GtsV2Field walker: walker from: start to: offset
]

{ #category : #private }
GtsV2ParityCandidate >> payloadLengthFor: layout [
	| dataCount parityCount parityBytes |
	dataCount := layout second.
	parityCount := layout third.
	parityBytes := parityCount * layout first.
	^ 1 + (self headSizeFor: dataCount) + (dataCount * 17)
		+ (self headSizeFor: parityCount) + (parityCount * 17)
		+ (self headSizeFor: parityBytes) + parityBytes
]

{ #category : #private }
GtsV2ParityCandidate >> headSizeFor: value [
	value < 24 ifTrue: [ ^ 1 ].
	value <= 255 ifTrue: [ ^ 2 ].
	value <= 65535 ifTrue: [ ^ 3 ].
	^ 5
]
