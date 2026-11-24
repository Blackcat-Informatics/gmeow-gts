"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2IndexChecks,
	#superclass : #Object,
	#instVars : [ 'walker', 'offset', 'limit' ],
	#category : #'Gts-Core'
}

{ #category : #validation }
GtsV2IndexChecks class >> check: content bytes: bytes head: head range: range runs: expectedRuns [
	^ self new check: content bytes: bytes head: head range: range runs: expectedRuns
]

{ #category : #validation }
GtsV2IndexChecks >> check: content bytes: bytes head: head range: range runs: expectedRuns [
	| header frameCount claimedHead directoryMatches rootStart failures |
	header := content fields: 2.
	frameCount := header next uint.
	claimedHead := GtsV2RdfColumns body: header next size: 32.
	walker := GtsV2CborWalker on: bytes.
	offset := 1.
	limit := bytes size + 1.
	self requireArray: 6.
	directoryMatches := self matchesRuns: expectedRuns.
	self checkGraphDirectory.
	self checkStatistics.
	rootStart := self readDigest.
	self readOptionDo: [ self readDigest ].
	(self readHead: 4) timesRepeat: [
		self requireArray: 3.
		self readDigest.
		self readHead: 0.
		self readHead: 0 ].
	offset = limit ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	failures := OrderedCollection new.
	(frameCount = (range leafCount - 1) and: [ claimedHead = head ])
		ifFalse: [ failures add: 'index_head' ].
	(bytes copyFrom: rootStart to: rootStart + 31) = range root
		ifFalse: [ failures add: 'index_root' ].
	directoryMatches ifFalse: [ failures add: 'index_directory' ].
	^ failures
]

{ #category : #private }
GtsV2IndexChecks >> matchesRuns: expected [
	| count first prefix length tail bytes |
	count := self readHead: 4.
	first := offset.
	count timesRepeat: [
		self requireArray: 2.
		self readFrameType.
		self readHead: 0 ].
	expected ifNil: [ ^ false ].
	prefix := expected first.
	length := expected second.
	tail := expected third.
	offset - first = (length + tail size) ifFalse: [ ^ false ].
	bytes := walker bytes.
	1 to: length do: [ :index |
		(bytes at: first + index - 1) = (prefix at: index) ifFalse: [ ^ false ] ].
	1 to: tail size do: [ :index |
		(bytes at: first + length + index - 1) = (tail at: index) ifFalse: [ ^ false ] ].
	^ true
]

{ #category : #private }
GtsV2IndexChecks >> checkGraphDirectory [
	(self readHead: 4) timesRepeat: [
		self requireArray: 2.
		self readOptionDo: [ self readHead: 0 ].
		(self readHead: 4) timesRepeat: [
			self requireArray: 4.
			self readFrameType.
			3 timesRepeat: [ self readHead: 0 ] ] ]
]

{ #category : #private }
GtsV2IndexChecks >> checkStatistics [
	self requireArray: 4.
	3 timesRepeat: [ self readHead: 0 ].
	(self readHead: 4) timesRepeat: [
		self requireArray: 2.
		self readHead: 0.
		self readHead: 0 ]
]

{ #category : #private }
GtsV2IndexChecks >> readHead: major [
	| head |
	(offset < limit and: [ (walker bytes at: offset) = 255 ]) ifTrue: [
		GtsV2EnvelopeError signal: 'wrong_length' ].
	head := (GtsV2Field walker: walker from: offset to: limit) typedHead: major.
	offset := head end.
	^ head argument
]

{ #category : #private }
GtsV2IndexChecks >> requireArray: arity [
	(self readHead: 4) = arity ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ]
]

{ #category : #private }
GtsV2IndexChecks >> readFrameType [
	((self readHead: 0) between: 1 and: 16) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ]
]

{ #category : #private }
GtsV2IndexChecks >> readDigest [
	| start |
	(self readHead: 2) = 32 ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	start := offset.
	offset := offset + 32.
	offset > limit ifTrue: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	^ start
]

{ #category : #private }
GtsV2IndexChecks >> readOptionDo: block [
	| count tag |
	count := self readHead: 4.
	count = 0 ifTrue: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	tag := self readHead: 0.
	(tag between: 1 and: 2) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	tag = count ifFalse: [ GtsV2EnvelopeError signal: 'wrong_length' ].
	tag = 2 ifTrue: [ block value ]
]
