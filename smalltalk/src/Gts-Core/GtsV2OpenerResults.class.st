"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2OpenerResults,
	#superclass : #Object,
	#instVars : [ 'bytes', 'limit', 'pastCount', 'exhausted', 'checksByStart', 'records' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2OpenerResults class >> on: input limit: readLimit [
	^ self new initializeOn: input limit: readLimit
]

{ #category : #initialization }
GtsV2OpenerResults >> initializeOn: input limit: readLimit [
	bytes := input.
	limit := readLimit.
	pastCount := 0.
	exhausted := false.
	checksByStart := Dictionary new.
	records := OrderedCollection new.
	^ self
]

{ #category : #accessing }
GtsV2OpenerResults >> at: ordinal [
	"The opener, header identifier, registry version, hash algorithm, and profile fields of P7."
	^ records at: ordinal + 1
]

{ #category : #observing }
GtsV2OpenerResults >> observe: event [
	| opener record |
	opener := event opener.
	record := event kind = #header
		ifTrue: [ #(1) ]
		ifFalse: [
			{ 2. event scanned ifTrue: [ 2 ] ifFalse: [ 1 ].
				event statedOpenerOffset ~= event offset. 1.
				opener ifNil: [ 0 ] ifNotNil: [ opener marksIgnored ] } ].
	records add: { record. self option: event effectiveIdentifier.
		self option: (opener ifNotNil: [ opener registry ]).
		self option: (opener ifNotNil: [ opener hashAlgorithm ]).
		self option: (opener ifNotNil: [ opener profile ]) }.
	(event kind = #resync and: [ opener notNil and: [ opener past notNil ] ]) ifTrue: [
		record at: 4 put: 4.
		self schedulePastOf: opener at: event offset record: record ]
]

{ #category : #private }
GtsV2OpenerResults >> option: value [
	^ value ifNil: [ #(1) ] ifNotNil: [ { 2. value } ]
]

{ #category : #private }
GtsV2OpenerResults >> schedulePastOf: opener at: offset record: record [
	| start checks chargedEnd charge |
	(exhausted or: [ opener identifier isNil or: [ offset < opener statedOffset ] ]) ifTrue: [ ^ self ].
	start := offset - opener statedOffset.
	checks := checksByStart at: start ifAbsentPut: [ OrderedCollection new ].
	chargedEnd := checks ifEmpty: [ start ] ifNotEmpty: [ checks last first ].
	charge := offset - chargedEnd.
	pastCount + charge > limit ifTrue: [
		exhausted := true.
		^ self ].
	pastCount := pastCount + charge.
	checks add: { offset. opener past. record }
]

{ #category : #observing }
GtsV2OpenerResults >> finish [
	"Check ranges with the same start in one pass over their common bytes."
	checksByStart keysAndValuesDo: [ :start :checks |
		checks ifNotEmpty: [
			GtsBlake3 deriveWith: 'gts2 unit' snapshottingWith: [ :update :snapshot |
				| first |
				first := start + 1.
				checks do: [ :entry |
					| finish state |
					finish := entry first + 1.
					update value: bytes value: first value: finish.
					state := snapshot value = entry second ifTrue: [ 2 ] ifFalse: [ 3 ].
					entry third at: 4 put: state.
					first := finish ] ] ] ].
	checksByStart removeAll.
	bytes := nil
]
