"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2TermChecks,
	#superclass : #Object,
	#instVars : [ 'runs', 'nextId', 'availability' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2TermChecks >> initialize [
	super initialize.
	runs := OrderedCollection new.
	availability := OrderedCollection new.
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
	| findings invalidText id |
	findings := Set new.
	invalidText := Set new.
	payload validateChargingText: [ :length | ] textDo: [ :block :relativeBase :index :octets |
		(GtsCborText isValidUtf8: octets from: 1 to: octets size + 1)
			ifFalse: [ invalidText add: base + relativeBase + index - 1 ] ].
	id := base.
	payload blocks do: [ :block |
		| kind columns datatypeOffset |
		kind := block first.
		columns := block second.
		datatypeOffset := 0.
		1 to: columns first size do: [ :index |
			| reason sources status |
			reason := (invalidText includes: id) ifTrue: [ 24 ].
			sources := #().
			(kind = 2 and: [ (columns fourth at: index) = 1 ]) ifTrue: [
				datatypeOffset := datatypeOffset + 1.
				sources := { (columns at: 5) at: datatypeOffset }.
				reason ifNil: [ reason := self problemInDatatype: sources first at: id ] ].
			kind = 4 ifTrue: [
				sources := columns collect: [ :column | column at: index ].
				reason := self problemInTriple: sources at: id ].
			reason ifNotNil: [ findings add: reason ].
			status := reason
				ifNil: [ self availabilityOf: sources pending: (#(5 7) includes: kind) ]
				ifNotNil: [ #unreadable ].
			id := id + 1.
			self appendAvailability: status through: id ] ].
	^ findings
]

{ #category : #private }
GtsV2TermChecks >> problemInDatatype: datatype at: id [
	datatype >= id ifTrue: [ ^ 22 ].
	^ (self id: datatype hasAnyKindOf: #(1)) ifFalse: [ 23 ]
]

{ #category : #private }
GtsV2TermChecks >> problemInTriple: sources at: id [
	(sources anySatisfy: [ :source | source >= id ]) ifTrue: [ ^ 22 ].
	^ ((self id: sources first hasAnyKindOf: #(1 3)) and: [
		self id: sources second hasAnyKindOf: #(1) ]) ifFalse: [ 23 ]
]

{ #category : #private }
GtsV2TermChecks >> id: id hasAnyKindOf: allowed [
	| kind |
	kind := self entryFor: id in: runs.
	^ kind > 7 or: [ allowed includes: kind ]
]

{ #category : #private }
GtsV2TermChecks >> entryFor: id in: intervals [
	| first last middle |
	first := 1.
	last := intervals size.
	[ first < last ] whileTrue: [
		middle := (first + last) // 2.
		id < (intervals at: middle) first
			ifTrue: [ last := middle ]
			ifFalse: [ first := middle + 1 ] ].
	^ (intervals at: first) second
]

{ #category : #observing }
GtsV2TermChecks >> markReservedUnreadable [
	(availability isEmpty and: [ nextId = 0 ]) ifTrue: [ ^ self ].
	(availability isNotEmpty and: [ availability last first = nextId ]) ifTrue: [ ^ self ].
	self appendAvailability: #unreadable through: nextId
]

{ #category : #private }
GtsV2TermChecks >> appendAvailability: state through: end [
	(availability isNotEmpty and: [ availability last second = state ])
		ifTrue: [ availability last at: 1 put: end ]
		ifFalse: [ availability add: { end. state } ]
]

{ #category : #private }
GtsV2TermChecks >> availabilityOf: sources pending: pending [
	| uncertain |
	uncertain := pending.
	sources do: [ :source |
		| state |
		state := self entryFor: source in: availability.
		state = #unreadable ifTrue: [ ^ #unreadable ].
		state = #pending ifTrue: [ uncertain := true ] ].
	^ uncertain ifTrue: [ #pending ] ifFalse: [ #readable ]
]

{ #category : #validation }
GtsV2TermChecks >> row: ids kinds: kinds hiddenBefore: hiddenBefore inDoubt: inDoubt [
	| state present |
	present := ids reject: [ :id | id isNil ].
	(present anySatisfy: [ :id | id >= nextId ]) ifTrue: [ ^ 22 ].
	(inDoubt or: [ present anySatisfy: [ :id | id < hiddenBefore ] ]) ifTrue: [ ^ 25 ].
	state := self availabilityOf: present pending: false.
	state = #unreadable ifTrue: [ ^ 25 ].
	state = #pending ifTrue: [ ^ nil ].
	ids withIndexDo: [ :id :index |
		| allowed |
		allowed := kinds at: index.
		(id notNil and: [ allowed isNotEmpty and: [ (self id: id hasAnyKindOf: allowed) not ] ])
			ifTrue: [ ^ 23 ] ].
	^ nil
]
