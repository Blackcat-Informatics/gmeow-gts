"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2Wire,
	#superclass : #Object,
	#instVars : [ 'bytes', 'walker', 'firstEnd', 'format', 'generation', 'stopReason' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2Wire class >> inspect: aByteArray [
	^ self new initializeOn: aByteArray
]

{ #category : #initialization }
GtsV2Wire >> initializeOn: aByteArray [
	| top kind version |
	bytes := aByteArray.
	bytes isEmpty ifTrue: [ format := #empty. ^ self ].
	walker := GtsV2CborWalker on: bytes.
	[ firstEnd := walker itemEndAt: 1 limit: bytes size + 1 ]
		on: GtsV2FramingError
		do: [ :error |
			format := #framingError.
			stopReason := error messageText.
			^ self ].
	top := walker headAt: 1 limit: firstEnd.
	kind := self recordKind: top limit: firstEnd.
	(kind isNotNil and: [ #(1 5) includes: kind argument ]) ifTrue: [
		version := self unsignedHeadAt: kind end limit: firstEnd.
		format := #unknown.
		version ifNotNil: [
			generation := version argument.
			generation = 2 ifTrue: [ format := #v2 ].
			generation > 2 ifTrue: [ format := #unsupported ] ].
		^ self ].
	generation := self legacyGeneration: top limit: firstEnd.
	format := generation
		ifNil: [ #unknown ]
		ifNotNil: [ generation = 1 ifTrue: [ #legacy ] ifFalse: [ #unsupported ] ].
	^ self
]

{ #category : #accessing }
GtsV2Wire >> format [
	^ format
]

{ #category : #accessing }
GtsV2Wire >> generation [
	^ generation
]

{ #category : #accessing }
GtsV2Wire >> stopReason [
	^ stopReason
]

{ #category : #dispatching }
GtsV2Wire >> recordKind: head limit: limit [
	(head major = 4 and: [ head argument ~= 0 ]) ifFalse: [ ^ nil ].
	^ self unsignedHeadAt: head end limit: limit
]

{ #category : #dispatching }
GtsV2Wire >> unsignedHeadAt: start limit: limit [
	| head |
	(start = limit or: [ (bytes at: start) // 32 ~= 0 ]) ifTrue: [ ^ nil ].
	head := walker headAt: start limit: limit.
	^ head
]

{ #category : #dispatching }
GtsV2Wire >> legacyGeneration: head limit: limit [
	| map bodyEnd offset keyEnd value |
	map := head.
	(map major = 6 and: [ map argument = 55799 ]) ifTrue: [
		map := walker headAt: map end limit: limit ].
	map major = 5 ifFalse: [ ^ nil ].
	bodyEnd := map argument ifNil: [ limit - 1 ] ifNotNil: [ limit ].
	offset := map end.
	[ offset < bodyEnd ] whileTrue: [
		keyEnd := walker itemEndAt: offset limit: bodyEnd.
		(self isVersionKeyAt: offset limit: keyEnd) ifTrue: [
			value := self unsignedHeadAt: keyEnd limit: bodyEnd.
			^ value ifNotNil: [ value argument ] ].
		offset := walker itemEndAt: keyEnd limit: bodyEnd ].
	^ nil
]

{ #category : #dispatching }
GtsV2Wire >> isVersionKeyAt: start limit: limit [
	| head found offset chunk size |
	head := walker headAt: start limit: limit.
	head major = 3 ifFalse: [ ^ false ].
	head argument ifNotNil: [
		^ head argument = 1 and: [ (bytes at: head end) = 118 ] ].
	found := false.
	offset := head end.
	[ offset < (limit - 1) ] whileTrue: [
		chunk := walker headAt: offset limit: limit.
		size := chunk argument.
		size > 1 ifTrue: [ ^ false ].
		size = 1 ifTrue: [
			(found or: [ (bytes at: chunk end) ~= 118 ]) ifTrue: [ ^ false ].
			found := true ].
		offset := chunk end + size ].
	^ found
]

{ #category : #enumerating }
GtsV2Wire >> eventsDo: aBlock [
	| offset limit count prefixValid finish event |
	format = #v2 ifFalse: [ ^ self ].
	offset := 1.
	limit := bytes size + 1.
	count := 0.
	prefixValid := false.
	[ offset < limit ] whileTrue: [
		count = 67108864 ifTrue: [
			aBlock value: (self stopAt: offset reason: 'items_per_read').
			^ self ].
		[ finish := offset = 1
			ifTrue: [ firstEnd ]
			ifFalse: [ walker itemEndAt: offset limit: limit ] ]
			on: GtsV2FramingError
			do: [ :error |
				aBlock value: (self stopAt: offset reason: error messageText).
				^ self ].
		event := self classifyAt: offset limit: finish prefixValid: prefixValid.
		event startsSegment ifTrue: [ prefixValid := event problem isNil ].
		aBlock value: event.
		offset := finish.
		count := count + 1 ]
]

{ #category : #classifying }
GtsV2Wire >> classifyAt: offset limit: limit prefixValid: prefixValid [
	| array code kind problem |
	array := walker headAt: offset limit: limit.
	code := self recordKind: array limit: limit.
	kind := #opaque.
	code ifNotNil: [
		kind := #(header frame locator offsets resync) at: code argument ifAbsent: [ #opaque ] ].
	(kind = #header or: [ kind = #resync ])
		ifTrue: [ problem := self openerProblemFor: array code: code kind: kind limit: limit ]
		ifFalse: [
			prefixValid
				ifTrue: [ problem := self itemProblemFor: array code: code kind: kind.
					problem ifNotNil: [ kind := #opaque ] ]
				ifFalse: [ kind := #unchecked ] ].
	^ GtsV2WireEvent kind: kind from: offset - 1 to: limit - 1 problem: problem
]

{ #category : #classifying }
GtsV2Wire >> openerProblemFor: array code: code kind: kind limit: limit [
	| version |
	array isShortest ifFalse: [ ^ 'not_deterministic' ].
	array argument = (kind = #header ifTrue: [ 7 ] ifFalse: [ 11 ]) ifFalse: [ ^ 'wrong_length' ].
	code isShortest ifFalse: [ ^ 'not_deterministic' ].
	version := walker headAt: code end limit: limit.
	(version major <= 4 and: [ version isShortest not ]) ifTrue: [ ^ 'not_deterministic' ].
	version major = 0 ifFalse: [ ^ 'wrong_type' ].
	version argument = 2 ifFalse: [ ^ 'out_of_range' ].
	^ nil
]

{ #category : #classifying }
GtsV2Wire >> itemProblemFor: array code: code kind: kind [
	code ifNil: [ ^ 'wrong_type' ].
	(array isShortest and: [ code isShortest ]) ifFalse: [ ^ 'not_deterministic' ].
	(kind = #frame and: [ array argument ~= 11 ]) ifTrue: [ ^ 'wrong_length' ].
	code argument = 0 ifTrue: [ ^ 'unregistered_value' ].
	kind = #opaque ifTrue: [ ^ 'unknown item kind ', code argument printString ].
	^ nil
]

{ #category : #classifying }
GtsV2Wire >> stopAt: offset reason: reason [
	^ GtsV2WireEvent kind: #stop from: offset - 1 to: offset - 1 problem: reason
]
