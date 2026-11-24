"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ParityRepair,
	#superclass : #Object,
	#instVars : [ 'input', 'working', 'output', 'items', 'spans', 'findings', 'stop', 'refusal' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2ParityRepair class >> on: bytes [
	^ self new initializeOn: bytes
]

{ #category : #initialization }
GtsV2ParityRepair >> initializeOn: bytes [
	input := bytes.
	items := OrderedCollection new.
	spans := OrderedCollection new.
	findings := OrderedCollection new.
	^ self
]

{ #category : #accessing }
GtsV2ParityRepair >> repairedBytes [
	^ output
]

{ #category : #accessing }
GtsV2ParityRepair >> findings [
	^ findings asArray
]

{ #category : #accessing }
GtsV2ParityRepair >> stop [
	^ stop
]

{ #category : #accessing }
GtsV2ParityRepair >> refusal [
	^ refusal
]

{ #category : #private }
GtsV2ParityRepair >> finish [
	working := nil.
	input := nil
]

{ #category : #observing }
GtsV2ParityRepair >> observe: event [
	event kind = #stop ifTrue: [ stop := event. ^ self ].
	(#(parityRefused resyncRefused) includes: event kind) ifTrue: [ refusal := event. ^ self ].
	event startsSegment ifTrue: [ items add: event. ^ self ].
	items isEmpty ifTrue: [ ^ self ].
	((#(frame opaque offsets locator unchecked) includes: event kind) or: [
		event kind = #framingGap and: [ event problem = 'undelimitable_input' ] ])
		ifTrue: [ items add: event ]
]

{ #category : #observing }
GtsV2ParityRepair >> addSpan: span [
	spans add: span
]

{ #category : #repairing }
GtsV2ParityRepair >> finishSegmentWith: budget [
	spans ifNotEmpty: [ self repairSegmentWith: budget ].
	items removeAll.
	spans removeAll
]

{ #category : #private }
GtsV2ParityRepair >> repairSegmentWith: budget [
	| limit owners dependents |
	limit := items last end.
	owners := self stageSpans.
	dependents := IdentityDictionary new.
	self checkRunsWith: budget before: limit owners: owners dependents: dependents.
	self propagateFailuresThrough: dependents.
	spans do: [ :span |
		span failure
			ifNil: [
				span shardCount > 0 ifTrue: [
					output ifNil: [ output := input copy ].
					span patchesDo: [ :patch | self apply: patch to: output ].
					findings add: { 'repaired'. span offset. span shardCount } ] ]
			ifNotNil: [ :reason | findings add: { 'repair_failed'. span offset. reason } ].
		span patchesDo: [ :patch |
			"Two segments can share bytes when a resync starts inside a retained item."
			working replaceFrom: patch first to: patch first + patch second size - 1
				with: input startingAt: patch first ] ]
]

{ #category : #private }
GtsV2ParityRepair >> stageSpans [
	| owners |
	owners := Dictionary new.
	spans do: [ :span |
		span patchesDo: [ :patch |
			working ifNil: [ working := input copy ].
			self apply: patch to: working ].
		span dataPatchesDo: [ :patch |
			| first last |
			first := self itemContaining: patch first - 1.
			last := self itemContaining: patch first + patch second size - 2.
			first to: last do: [ :index |
				(owners at: index ifAbsentPut: [ IdentitySet new ]) add: span ] ] ].
	^ owners
]

{ #category : #private }
GtsV2ParityRepair >> itemContaining: offset [
	| first last |
	first := 1.
	last := items size.
	[ first < last ] whileTrue: [
		| middle |
		middle := (first + last) // 2.
		offset < (items at: middle) end
			ifTrue: [ last := middle ]
			ifFalse: [ first := middle + 1 ] ].
	^ first
]

{ #category : #private }
GtsV2ParityRepair >> apply: patch to: bytes [
	bytes replaceFrom: patch first to: patch first + patch second size - 1
		with: patch second startingAt: 1
]

{ #category : #private }
GtsV2ParityRepair >> checkRunsWith: budget before: limit owners: owners dependents: dependents [
	| opener predecessor registry predecessorRun failedRun index walker |
	owners ifEmpty: [ ^ self ].
	opener := items first.
	predecessor := opener effectiveIdentifier.
	registry := opener opener registry.
	walker := GtsV2CborWalker on: working.
	index := 1.
	failedRun := false.
	[ index <= items size and: [ (items at: index) offset < limit ] ] whileTrue: [
		| event |
		event := items at: index.
		(owners includesKey: index)
			ifTrue: [
				| run last members |
				run := self walkFromItem: index to: limit on: walker
					predecessor: predecessor opener: opener registry: registry budget: budget.
				last := run first.
				members := IdentitySet new.
				index to: last do: [ :part |
					(owners at: part ifAbsent: [ #() ]) do: [ :span | members add: span ] ].
				members do: [ :span |
					(dependents at: span ifAbsentPut: [ IdentitySet new ]) add: members.
					(run second not or: [ failedRun ]) ifTrue: [ span failure: 'verify_failed' ] ].
				predecessorRun ifNotNil: [ (dependents at: predecessorRun) addAll: members ].
				run second
					ifTrue: [
						run third ifNotNil: [ :identifier |
							predecessor := identifier.
							predecessorRun := members.
							dependents at: members put: members copy.
							failedRun := false ] ]
					ifFalse: [ failedRun := true ].
				index := last + 1 ]
			ifFalse: [
				(event frame notNil and: [ event effectiveIdentifier notNil ]) ifTrue: [
					predecessor := event effectiveIdentifier.
					predecessorRun := nil.
					failedRun := false ].
				index := index + 1 ] ]
]

{ #category : #private }
GtsV2ParityRepair >> walkFromItem: index to: limit on: walker predecessor: predecessor opener: opener registry: registry budget: budget [
	| position current produced last |
	position := (items at: index) offset.
	current := predecessor.
	last := index.
	[
		| item end frame |
		item := walker itemAt: position + 1.
		[ end := item endUpTo: limit + 1 ] on: GtsV2FramingError do: [ :error | end := nil ].
		end ifNil: [ ^ { self itemContaining: item examinedEnd - 2. false. nil } ].
		end := end - 1.
		[ (items at: last) end < end ] whileTrue: [ last := last + 1 ].
		position = opener offset
			ifTrue: [
				(end = opener end and: [
					(position + 1 to: end) allSatisfy: [ :byte | (working at: byte) = (input at: byte) ] ])
					ifFalse: [ ^ { last. false. nil } ] ]
			ifFalse: [
				frame := self checkedFrameFrom: position to: end on: walker
					predecessor: current registry: registry budget: budget.
				frame ifNil: [ ^ { last. false. nil } ].
				frame isTransparent ifFalse: [
					current := frame storedIdentifier.
					produced := current ] ].
		position := end.
		position = limit or: [ position = (items at: last) end and: [
			(items at: last + 1) offset = position ] ]
	] whileFalse.
	^ { last. true. produced }
]

{ #category : #private }
GtsV2ParityRepair >> checkedFrameFrom: start to: end on: walker predecessor: predecessor registry: registry budget: budget [
	| frame matches state |
	(end - start >= 2 and: [ (working at: start + 1) = 139 and: [ (working at: start + 2) = 2 ] ])
		ifFalse: [ ^ nil ].
	frame := GtsV2FrameEnvelope read: (GtsV2Field walker: walker from: start + 1 to: end + 1)
		registry: registry.
	(frame problem isNil and: [ frame storedIdentifier = (frame identifierOn: working predecessor: predecessor) ])
		ifFalse: [ ^ nil ].
	matches := frame payload storedMatches.
	matches = false ifTrue: [ ^ nil ].
	state := (frame payload checkWith: budget retainDecoded: false) state.
	^ (state = #verified or: [ matches = true and: [ state = #unchecked ] ]) ifTrue: [ frame ]
]

{ #category : #private }
GtsV2ParityRepair >> propagateFailuresThrough: dependents [
	| pending seen |
	pending := (spans select: [ :span | span failure notNil ]) asOrderedCollection.
	seen := IdentitySet new.
	[ pending isNotEmpty ] whileTrue: [
		| node |
		node := pending removeLast.
		(seen includes: node) ifFalse: [
			seen add: node.
			(node isKindOf: GtsV2ParityRepairSpan) ifTrue: [
				node failure ifNil: [ node failure: 'verify_failed' ] ].
			pending addAll: (dependents at: node ifAbsent: [ #() ]).
			(node isKindOf: IdentitySet) ifTrue: [ pending addAll: node ] ] ]
]
