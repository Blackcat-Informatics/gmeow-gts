"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ParityStream,
	#superclass : #Object,
	#instVars : [ 'bytes', 'ranges', 'size', 'lastStart', 'openerRange', 'tailStart', 'tailOffset' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2ParityStream class >> on: aByteArray [
	^ self new initializeOn: aByteArray
]

{ #category : #initialization }
GtsV2ParityStream >> initializeOn: aByteArray [
	bytes := aByteArray.
	ranges := OrderedCollection new.
	size := 0.
	lastStart := 0.
	tailStart := 0.
	^ self
]

{ #category : #testing }
GtsV2ParityStream >> placesSpanFrom: first to: end [
	^ end <= size and: [ lastStart < end and: [ end - first = 4194304 or: [ end = size ] ] ]
]

{ #category : #observing }
GtsV2ParityStream >> observe: event [
	event startsSegment ifTrue: [
		self append: event passes: false.
		openerRange := ranges last.
		self protectThrough: size.
		^ self ].
	event kind = #frame ifTrue: [
		event frame isTransparent ifTrue: [ ^ self ].
		openerRange ifNotNil: [
			openerRange at: 4 put: (#(linked healed) includes: event link).
			openerRange := nil ].
		^ self append: event passes: (self framePasses: event) ].
	(event kind = #opaque or: [ event kind = #framingGap and: [ event problem = 'undelimitable_input' ] ])
		ifTrue: [ self append: event passes: false ]
]

{ #category : #testing }
GtsV2ParityStream >> framePasses: event [
	^ event problem isNil and: [ event link = #linked and: [
		event storedMatches
			ifNil: [ event payload state = #verified ]
			ifNotNil: [ :matches | matches ] ] ]
]

{ #category : #private }
GtsV2ParityStream >> append: event passes: passes [
	| previous inputStart cutoff |
	"Ranges hold the S start, S end, input start, and whether their items pass."
	lastStart := size.
	size := size + event end - event offset.
	inputStart := event offset + 1.
	(tailOffset isNil and: [ tailStart < size ]) ifTrue: [
		tailOffset := inputStart + tailStart - lastStart - 1 ].
	previous := ranges isEmpty ifFalse: [ ranges last ].
	(previous notNil and: [ previous ~~ openerRange and: [ previous fourth = passes and: [
		previous third + previous second - previous first = inputStart ] ] ])
		ifTrue: [ previous at: 2 put: size ]
		ifFalse: [ ranges add: { lastStart. size. inputStart. passes } ].
	cutoff := lastStart - (lastStart \\ 4194304).
	[ ranges first second <= cutoff ] whileTrue: [ ranges removeFirst ]
]

{ #category : #observing }
GtsV2ParityStream >> protectThrough: position [
	position <= tailStart ifTrue: [ ^ self ].
	self protectThrough: position at: (self inputOffsetAt: position)
]

{ #category : #observing }
GtsV2ParityStream >> protectThrough: position at: inputOffset [
	position <= tailStart ifTrue: [ ^ self ].
	tailStart := position.
	tailOffset := inputOffset
]

{ #category : #private }
GtsV2ParityStream >> inputOffsetAt: position [
	^ position < size ifTrue: [
		| range |
		range := ranges at: (self rangeIndexAt: position).
		range third + position - range first - 1 ]
]

{ #category : #enumerating }
GtsV2ParityStream >> unprotectedTailDo: report [
	tailOffset ifNotNil: [ report value: tailOffset value: size - tailStart ]
]

{ #category : #validation }
GtsV2ParityStream >> compareFrom: first to: end shardSize: shardSize digests: digests [
	| failed unexplained pendingOpener index |
	failed := OrderedCollection new.
	unexplained := 0.
	pendingOpener := 0.
	index := 0.
	self shardsFrom: first to: end size: shardSize do: [ :data :passes :shardEnd |
		index := index + 1.
		((GtsBlake3 deriveWith: 'gts2 shard' bytes: data) first: 16) = (digests at: index) ifFalse: [
			failed add: index.
			passes ifTrue: [ unexplained := unexplained + 1 ].
			(openerRange notNil and: [ shardEnd <= openerRange second ])
				ifTrue: [ pendingOpener := pendingOpener + 1 ] ] ].
	^ { failed. unexplained. pendingOpener }
]

{ #category : #enumerating }
GtsV2ParityStream >> shardsFrom: first to: end size: shardSize do: consume [
	| rangeIndex data position |
	rangeIndex := self rangeIndexAt: first.
	data := ByteArray new: shardSize.
	position := first.
	[ position < end ] whileTrue: [
		| shardStart shardEnd passes |
		shardStart := position.
		shardEnd := (position + shardSize) min: end.
		passes := true.
		[ position < shardEnd ] whileTrue: [
			| range finish input |
			range := ranges at: rangeIndex.
			finish := shardEnd min: range second.
			input := range third + position - range first.
			data replaceFrom: position - shardStart + 1 to: finish - shardStart with: bytes startingAt: input.
			passes := passes and: [ range fourth ].
			position := finish.
			position = range second ifTrue: [ rangeIndex := rangeIndex + 1 ] ].
		shardEnd - shardStart + 1 to: shardSize do: [ :index | data at: index put: 0 ].
		consume value: data value: passes value: shardEnd ]
]

{ #category : #private }
GtsV2ParityStream >> rangeIndexAt: position [
	| low high mid |
	low := 1.
	high := ranges size.
	[ low < high ] whileTrue: [
		mid := (low + high) // 2.
		(ranges at: mid) second <= position
			ifTrue: [ low := mid + 1 ]
			ifFalse: [ high := mid ] ].
	^ low
]
