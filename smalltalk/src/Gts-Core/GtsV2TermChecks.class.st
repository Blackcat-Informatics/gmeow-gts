"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2TermChecks,
	#superclass : #Object,
	#instVars : [ 'runs', 'nextId' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2TermChecks >> initialize [
	super initialize.
	runs := OrderedCollection new.
	nextId := 0
]

{ #category : #accessing }
GtsV2TermChecks >> nextId [
	^ nextId
]

{ #category : #observing }
GtsV2TermChecks >> reserve: content [
	content reservedCount = 0 ifTrue: [ ^ self ].
	content runs do: [ :run |
		nextId := nextId + run second.
		(runs isNotEmpty and: [ runs last second = run first ])
			ifTrue: [ runs last at: 1 put: nextId ]
			ifFalse: [ runs add: { nextId. run first } ] ]
]

{ #category : #validation }
GtsV2TermChecks >> findingsOf: payload from: base [
	| findings datatypeIndex start |
	findings := Set new.
	payload validateChargingText: [ :length | ] textDo: [ :block :relativeBase :index :octets |
		| datatype reason |
		index = 1 ifTrue: [ datatypeIndex := 0 ].
		datatype := nil.
		(block first = 2 and: [ (block second fourth at: index) = 1 ]) ifTrue: [
			datatypeIndex := datatypeIndex + 1.
			datatype := (block second at: 5) at: datatypeIndex ].
		reason := self checkText: octets datatype: datatype at: base + relativeBase + index - 1.
		reason ifNotNil: [ findings add: reason ] ].
	start := base.
	payload blocks do: [ :block |
		block first = 4 ifTrue: [ self checkTriples: block second from: start findings: findings ].
		start := start + block second first size ].
	^ findings
]

{ #category : #private }
GtsV2TermChecks >> checkText: octets datatype: datatype at: id [
	(GtsCborText isValidUtf8: octets from: 1 to: octets size + 1) ifFalse: [ ^ 24 ].
	datatype ifNil: [ ^ nil ].
	datatype >= id ifTrue: [ ^ 22 ].
	^ (self id: datatype hasAnyKindOf: #(1)) ifFalse: [ 23 ]
]

{ #category : #private }
GtsV2TermChecks >> checkTriples: columns from: base findings: findings [
	columns first withIndexDo: [ :subject :index |
		| id predicate object |
		id := base + index - 1.
		predicate := columns second at: index.
		object := columns third at: index.
		(subject >= id or: [ predicate >= id or: [ object >= id ] ])
			ifTrue: [ findings add: 22 ]
			ifFalse: [
				((self id: subject hasAnyKindOf: #(1 3)) and: [
					self id: predicate hasAnyKindOf: #(1) ]) ifFalse: [ findings add: 23 ] ] ]
]

{ #category : #private }
GtsV2TermChecks >> id: id hasAnyKindOf: allowed [
	| first last middle kind |
	first := 1.
	last := runs size.
	[ first < last ] whileTrue: [
		middle := (first + last) // 2.
		id < (runs at: middle) first
			ifTrue: [ last := middle ]
			ifFalse: [ first := middle + 1 ] ].
	kind := (runs at: first) second.
	^ kind > 7 or: [ allowed includes: kind ]
]
