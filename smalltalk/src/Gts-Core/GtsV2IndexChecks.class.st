"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2IndexChecks,
	#superclass : #Object,
	#instVars : [ 'cursor' ],
	#category : #'Gts-Core'
}

{ #category : #validation }
GtsV2IndexChecks class >> check: content bytes: bytes head: head range: range runs: expectedRuns packs: packs graphs: graphs [
	^ self new check: content bytes: bytes head: head range: range runs: expectedRuns packs: packs graphs: graphs
]

{ #category : #validation }
GtsV2IndexChecks >> check: content bytes: bytes head: head range: range runs: expectedRuns packs: packs graphs: graphs [
	| header frameCount claimedHead typeRunsMatch graphDirectoryMatches rootStart membersMatch failures |
	header := content fields: 2.
	frameCount := header next uint.
	claimedHead := GtsV2RdfColumns body: header next size: 32.
	cursor := GtsV2PayloadCursor on: bytes.
	cursor requireArray: 6.
	typeRunsMatch := self matchesRuns: expectedRuns.
	graphDirectoryMatches := self matchesGraphDirectory: graphs.
	self checkStatistics.
	rootStart := cursor readDigest.
	cursor readOptionDo: [ cursor readDigest ].
	membersMatch := self matchesMembers: packs.
	cursor requireEnd.
	failures := OrderedCollection new.
	(frameCount = (range leafCount - 1) and: [ claimedHead = head ])
		ifFalse: [ failures add: 'index_head' ].
	(bytes copyFrom: rootStart to: rootStart + 31) = range root
		ifFalse: [ failures add: 'index_root' ].
	(typeRunsMatch not or: [ graphs isComparable and: [ graphDirectoryMatches not ] ])
		ifTrue: [ failures add: 'index_directory' ].
	(packs isComparable and: [ membersMatch not ]) ifTrue: [ failures add: 'index_members' ].
	^ failures
]

{ #category : #private }
GtsV2IndexChecks >> matchesRuns: expected [
	| count first prefix length tail bytes |
	count := cursor readHead: 4.
	first := cursor offset.
	count timesRepeat: [
		cursor requireArray: 2.
		self readFrameType.
		cursor readHead: 0 ].
	expected ifNil: [ ^ false ].
	prefix := expected first.
	length := expected second.
	tail := expected third.
	cursor offset - first = (length + tail size) ifFalse: [ ^ false ].
	bytes := cursor bytes.
	1 to: length do: [ :index |
		(bytes at: first + index - 1) = (prefix at: index) ifFalse: [ ^ false ] ].
	1 to: tail size do: [ :index |
		(bytes at: first + length + index - 1) = (tail at: index) ifFalse: [ ^ false ] ].
	^ true
]

{ #category : #private }
GtsV2IndexChecks >> matchesGraphDirectory: graphs [
	| count expected matches |
	count := cursor readHead: 4.
	expected := graphs expectedNames: count.
	matches := expected isNotNil.
	1 to: count do: [ :index |
		| graph ranges rangeCount |
		cursor requireArray: 2.
		graph := -1.
		cursor readOptionDo: [ graph := cursor readHead: 0 ].
		rangeCount := cursor readHead: 4.
		(matches and: [ (expected at: index) = graph ])
			ifTrue: [
				ranges := graphs rangesOf: graph.
				matches := ranges size = rangeCount ]
			ifFalse: [ matches := false ].
		1 to: rangeCount do: [ :rangeIndex |
			| type position row length |
			cursor requireArray: 4.
			type := self readFrameType.
			position := cursor readHead: 0.
			row := cursor readHead: 0.
			length := cursor readHead: 0.
			matches ifTrue: [ matches := (ranges at: rangeIndex) = { type. position. row. length } ] ] ].
	^ matches
]

{ #category : #private }
GtsV2IndexChecks >> checkStatistics [
	cursor requireArray: 4.
	3 timesRepeat: [ cursor readHead: 0 ].
	(cursor readHead: 4) timesRepeat: [
		cursor requireArray: 2.
		cursor readHead: 0.
		cursor readHead: 0 ]
]

{ #category : #private }
GtsV2IndexChecks >> matchesMembers: packs [
	| count expected matches bytes |
	count := cursor readHead: 4.
	expected := packs expectedEntries: count.
	matches := expected isNotNil.
	bytes := cursor bytes.
	1 to: count do: [ :index |
		| start position member entry |
		cursor requireArray: 3.
		start := cursor readDigest.
		position := cursor readHead: 0.
		member := cursor readHead: 0.
		matches ifTrue: [
			entry := expected at: index.
			matches := entry second = position and: [
				entry third = member and: [ (bytes copyFrom: start to: start + 31) = entry first ] ] ] ].
	^ matches
]

{ #category : #private }
GtsV2IndexChecks >> readFrameType [
	| type |
	type := cursor readHead: 0.
	(type between: 1 and: 16) ifFalse: [ GtsV2EnvelopeError signal: 'unregistered_value' ].
	^ type
]
