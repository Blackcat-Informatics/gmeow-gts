"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2PackClaims,
	#superclass : #Object,
	#instVars : [ 'entries', 'sorted', 'comparable' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2PackClaims >> initialize [
	super initialize.
	entries := OrderedCollection new.
	comparable := true
]

{ #category : #recording }
GtsV2PackClaims >> observe: event position: position report: report [
	| bytes pack |
	bytes := event payload decoded.
	(event problem isNotNil or: [ (#(linked healed) includes: event link) not or: [ bytes isNil ] ])
		ifTrue: [ ^ self markUncompared ].
	[ [ pack := GtsV2Pack read: event frame content bytes: bytes ]
		on: GtsV2FramingError do: [ :error | GtsV2EnvelopeError signal: 'wrong_length' ] ]
		on: GtsV2EnvelopeError do: [ :error |
			self markUncompared.
			report value: #malformed_value value: error messageText.
			^ self ].
	pack membersDo: [ :type :content :start :size :index |
		type <= 9 ifTrue: [
			| digest |
			digest := GtsV2RdfColumns body: content list next size: 32.
			(GtsBlake3 deriveWith: 'gts2 unit' prefix: #[] bytes: bytes from: start to: start + size) = digest
				ifFalse: [ report value: #damaged_frame value: 'payload_digest' ].
			self add: digest position: position index: index ] ]
]

{ #category : #private }
GtsV2PackClaims >> markUncompared [
	comparable := false.
	entries := nil.
	sorted := nil
]

{ #category : #private }
GtsV2PackClaims >> add: digest position: position index: index [
	entries ifNil: [ ^ self ].
	sorted := nil.
	"Each canonical entry takes at least 37 bytes in the decoded index."
	entries size = ((1 bitShift: 26) // 37) ifTrue: [ entries := nil. ^ self ].
	entries add: { digest. position. index }
]

{ #category : #testing }
GtsV2PackClaims >> isComparable [
	^ comparable
]

{ #category : #accessing }
GtsV2PackClaims >> expectedEntries: count [
	(entries isNil or: [ entries size ~= count ]) ifTrue: [ ^ nil ].
	sorted ifNil: [
		sorted := entries asArray sort: [ :left :right | self entry: left precedes: right ] ].
	^ sorted
]

{ #category : #private }
GtsV2PackClaims >> entry: left precedes: right [
	| a b |
	a := left first.
	b := right first.
	1 to: 32 do: [ :index |
		(a at: index) = (b at: index) ifFalse: [ ^ (a at: index) < (b at: index) ] ].
	left second = right second ifFalse: [ ^ left second < right second ].
	^ left third <= right third
]
