"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2Wire,
	#superclass : #Object,
	#instVars : [ 'bytes', 'walker', 'format', 'generation', 'stopReason',
		'initialBudget', 'firstOpener', 'initialGap', 'initialItemCount', 'initialResyncRefusal', 'firstScanEnd',
		'initialLookahead', 'initialHeldItems' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2Wire class >> inspect: aByteArray [
	^ self inspect: aByteArray budget: GtsV2PayloadBudget new
]

{ #category : #'instance creation' }
GtsV2Wire class >> inspect: aByteArray budget: budget [
	^ self new initializeOn: aByteArray budget: budget
]

{ #category : #initialization }
GtsV2Wire >> initializeOn: aByteArray budget: budget [
	| firstEnd top kind version |
	bytes := aByteArray.
	bytes isEmpty ifTrue: [ format := #empty. ^ self ].
	initialBudget := budget copy.
	initialItemCount := 0.
	firstScanEnd := 1.
	initialHeldItems := Dictionary new.
	walker := GtsV2CborWalker on: bytes.
	initialBudget itemLimit = 0 ifTrue: [
		format := #framingError.
		stopReason := 'items_per_read'.
		^ self ].
	[ firstEnd := walker itemEndAt: 1 limit: bytes size + 1 ]
		on: GtsV2FramingError
		do: [ :error |
			format := #framingError.
			stopReason := error messageText.
			stopReason = 'undelimitable_input' ifTrue: [ initialItemCount := 1 ].
			self recoverFirstBefore: bytes size + 1.
			^ self ].
	initialItemCount := 1.
	top := walker headAt: 1 limit: firstEnd.
	kind := self recordKind: top limit: firstEnd.
	(kind isNotNil and: [ #(1 5) includes: kind argument ]) ifTrue: [
		version := self unsignedHeadAt: kind end limit: firstEnd.
		format := #unknown.
		version ifNotNil: [
			generation := version argument.
			generation = 2 ifTrue: [
				format := #v2.
				firstOpener := self classifyAt: 1 limit: firstEnd registry: nil ].
			generation > 2 ifTrue: [ format := #unsupported ] ].
		(format = #unknown or: [ firstOpener notNil and: [ firstOpener problem notNil ] ])
			ifTrue: [ self recoverFirstBefore: firstEnd ].
		^ self ].
	generation := self legacyGeneration: top limit: firstEnd.
	format := generation
		ifNil: [ #unknown ]
		ifNotNil: [ generation = 1 ifTrue: [ #legacy ] ifFalse: [ #unsupported ] ].
	format = #unknown ifTrue: [ self recoverFirstBefore: firstEnd ].
	^ self
]

{ #category : #dispatching }
GtsV2Wire >> recoverFirstBefore: limit [
	| recovery |
	initialLookahead := self
		scanDecisionFrom: limit
		lookahead: { limit. initialItemCount }
		held: initialHeldItems
		itemLimit: initialBudget itemLimit.
	initialLookahead second >= initialBudget itemLimit ifTrue: [ ^ self ].
	recovery := GtsV2ResyncScan on: walker from: 1 to: limit budget: initialBudget.
	recovery ifNil: [ firstScanEnd := limit. ^ self ].
	recovery kind = #resyncRefused ifTrue: [ initialResyncRefusal := recovery. ^ self ].
	recovery offset > 0 ifTrue: [ initialGap := self framingGapFrom: 1 to: recovery offset + 1 ].
	firstOpener := recovery.
	initialItemCount := initialLookahead second.
	initialLookahead := nil.
	initialHeldItems removeAll.
	format := #v2.
	generation := 2.
	stopReason := nil
]

{ #category : #dispatching }
GtsV2Wire >> scanDecisionFrom: start lookahead: frontier held: held itemLimit: itemLimit [
	"Resume framing-only lookahead at its next byte and keep the item count there."
	| count offset end decisionEnd limit finish |
	offset := frontier first.
	count := frontier second.
	end := bytes size + 1.
	decisionEnd := (start + 4096) min: end.
	[ offset < decisionEnd and: [ count < itemLimit ] ] whileTrue: [
		limit := offset = start ifTrue: [ end ] ifFalse: [ decisionEnd ].
		finish := self heldEndAt: offset limit: limit held: held.
		finish isString ifTrue: [ ^ { end. count + 1 } ].
		finish ifNil: [ ^ { (limit = end ifTrue: [ end ] ifFalse: [ offset ]). count } ].
		finish > limit ifTrue: [ ^ { offset. count } ].
		offset := finish.
		count := count + 1 ].
	^ { offset. count }
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

{ #category : #accessing }
GtsV2Wire >> initialResyncRefusal [
	^ initialResyncRefusal
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
	self eventsDo: aBlock decodedWhen: [ :event | false ]
]

{ #category : #enumerating }
GtsV2Wire >> eventsDo: aBlock retainDecoded: retain [
	self eventsDo: aBlock decodedWhen: [ :event |
		retain and: [ event frameType isNotNil and: [ event frameType between: 1 and: 13 ] ] ]
]

{ #category : #enumerating }
GtsV2Wire >> eventsDo: aBlock decodedWhen: predicate [
	self eventsDo: aBlock decodedWhen: predicate unprotectedTailsDo: [ :offset :length | ]
]

{ #category : #enumerating }
GtsV2Wire >> eventsDo: aBlock decodedWhen: predicate unprotectedTailsDo: report [
	| plan |
	plan := GtsV2ParityRepair on: bytes keepingCopy: false.
	self eventsDo: aBlock decodedWhen: predicate unprotectedTailsDo: report repairing: plan.
	plan finish.
	^ plan
]

{ #category : #enumerating }
GtsV2Wire >> eventsDo: aBlock decodedWhen: predicate unprotectedTailsDo: report repairing: plan [
	| pending pendingOffsets predecessor closing parity deliver flush budget refusalAfter pendingRefusal heldForParity emit finishSegment releaseHeld detached lastOrdinary |
	format = #v2 ifFalse: [ ^ self ].
	budget := initialBudget copy.
	closing := GtsV2ClosingChecks on: walker.
	parity := GtsV2ParityChecks on: bytes repairing: plan.
	detached := false.
	releaseHeld := [ heldForParity ifNotNil: [ heldForParity do: aBlock. heldForParity := nil ] ].
	finishSegment := [
		parity finishOpenerLink: false.
		releaseHeld value.
		parity unprotectedTailDo: report.
		plan finishSegmentWith: budget ].
	emit := [ :event |
		parity awaitsOpenerLink
			ifTrue: [
				heldForParity ifNil: [ heldForParity := OrderedCollection new ].
				heldForParity add: event ]
			ifFalse: [
				releaseHeld value.
				aBlock value: event ] ].
	deliver := [ :event :next :afterLocator |
		| effective |
		closing observe: event following: next afterLocator: afterLocator.
		event startsSegment ifTrue: [
			detached := false.
			lastOrdinary := event effectiveIdentifier ifNotNil: [ :digest | digest copy ] ].
		event resumptionAnchor ifNotNil: [ :anchor |
			detached := anchor ~= lastOrdinary.
			predecessor := anchor copy ].
		effective := self finalize: event following: next predecessor: predecessor.
		(event frame notNil and: [ event frame isTransparent not ]) ifTrue: [
			detached ifTrue: [ event markDetached ].
			event effectiveIdentifier ifNotNil: [ :identifier |
				lastOrdinary := identifier copy.
				detached := false ] ].
		parity observe: event predecessor: predecessor.
		plan observe: event.
		effective == predecessor ifFalse: [ predecessor := effective ifNotNil: [ effective copy ] ].
		emit value: event.
		event == refusalAfter ifTrue: [
			emit value: pendingRefusal.
			refusalAfter := nil.
			pendingRefusal := nil ] ].
	flush := [
		pendingOffsets ifNotNil: [ deliver value: pendingOffsets value: pending value: nil ].
		pending ifNotNil: [ deliver value: pending value: nil value: nil ].
		pendingOffsets := nil.
		pending := nil ].
	self scanEventsDo: [ :next |
		pendingOffsets ifNotNil: [
			deliver value: pendingOffsets value: pending value: next.
			pendingOffsets := nil ].
		pending ifNotNil: [
			(pending kind = #offsets and: [ next kind = #locator ])
				ifTrue: [ pendingOffsets := pending ]
				ifFalse: [ deliver value: pending value: next value: nil ] ].
		next startsSegment
			ifTrue: [
				finishSegment value.
				deliver value: next value: nil value: nil.
				pending := nil ]
			ifFalse: [ pending := next ] ]
		limitItems: budget itemLimit
		prepareFrame: [ :event :witness |
			| effective |
			effective := (pending ifNotNil: [
				pending resumptionAnchor ifNil: [ pending effectiveIdentifier ] ]) ifNil: [ predecessor ].
			self prepare: event predecessor: effective witness: witness.
			event checkPayloadWith: budget retainDecoded: (predicate value: event) ]
		onFramingLoss: [ :offset :previous :finish :scannedThrough :incomplete |
			| start |
			start := (previous notNil and: [ previous link ~= #healed ])
				ifTrue: [ previous offset + 2 ]
				ifFalse: [ offset ].
			start := start max: scannedThrough.
			(incomplete and: [ parity hasReadableOpener ])
				ifTrue: [
					GtsV2ParityCandidate scan: walker resyncFrom: start parityFrom: offset
						checks: parity previous: previous budget: budget ]
				ifFalse: [ GtsV2ResyncScan on: walker from: start to: finish budget: budget ] ]
		onRefusal: [ :retained :refusal |
			refusalAfter := retained.
			pendingRefusal := refusal ].
	flush value.
	finishSegment value
]

{ #category : #repairing }
GtsV2Wire >> parityRepair [
	| plan |
	format = #v2 ifFalse: [ GtsV2FramingError signal: 'repair requires GTS v2 input' ].
	plan := GtsV2ParityRepair on: bytes.
	self eventsDo: [ :event | ] decodedWhen: [ :event | false ]
		unprotectedTailsDo: [ :offset :length | ] repairing: plan.
	plan finish.
	^ plan
]

{ #category : #enumerating }
GtsV2Wire >> scanEventsDo: aBlock limitItems: itemLimit prepareFrame: prepare onFramingLoss: recover onRefusal: report [
	| offset limit count registry finish event previous scannedThrough frontier held refusal |
	initialGap ifNotNil: [ aBlock value: initialGap copy ].
	previous := firstOpener copy.
	aBlock value: previous.
	initialResyncRefusal ifNotNil: [ aBlock value: initialResyncRefusal copy ].
	offset := firstOpener end + 1.
	registry := previous effectiveIdentifier ifNotNil: [ previous opener registry ].
	limit := bytes size + 1.
	count := initialItemCount + (previous scanned ifTrue: [ 1 ] ifFalse: [ 0 ]).
	scannedThrough := firstScanEnd.
	frontier := initialLookahead.
	held := Dictionary new: initialHeldItems size.
	initialHeldItems keysAndValuesDo: [ :start :state | held at: start put: state copy ].
	[ offset < limit ] whileTrue: [
		count >= itemLimit ifTrue: [
			aBlock value: (self stopAt: offset reason: 'items_per_read').
			^ self ].
		event := nil.
		refusal := nil.
		[ finish := self endAt: offset limit: limit held: held ]
			on: GtsV2FramingError
			do: [ :error |
				(error messageText = 'undelimitable_input' and: [ count + 1 = itemLimit ]) ifTrue: [
					aBlock value: (self stopAt: offset reason: error messageText).
					^ self ].
				event := recover valueWithArguments: { offset. previous. limit. scannedThrough. count + 1 < itemLimit }.
				(event isNil or: [ #(resyncRefused parityRefused) includes: event kind ]) ifTrue: [
					event ifNotNil: [ aBlock value: event ].
					aBlock value: (self stopAt: offset reason: error messageText).
					^ self ].
				event offset > (offset - 1) ifTrue: [
					aBlock value: (event resumptionAnchor
						ifNil: [ self framingGapFrom: offset to: event offset + 1 ]
						ifNotNil: [
							(GtsV2WireEvent kind: #framingGap from: offset - 1
								to: event offset problem: 'undelimitable_input')
								markParityGap;
								yourself ]) ].
				(error messageText = 'undelimitable_input' or: [ event resumptionAnchor notNil ])
					ifTrue: [ count := count + 1 ].
				finish := event end + 1 ].
		event ifNil: [
			event := self classifyAt: offset limit: finish registry: registry.
			event frame ifNotNil: [
				prepare value: event value: [
					count + 1 < itemLimit ifTrue: [
						self frameWitnessAt: finish registry: registry held: held ] ] ].
			event failsFraming ifTrue: [
				| recovery |
				(frontier isNil or: [ frontier first <= finish ]) ifTrue: [ frontier := { finish. count + 1 } ].
				frontier := self scanDecisionFrom: finish lookahead: frontier held: held itemLimit: itemLimit.
				frontier second < itemLimit ifTrue: [
					recovery := recover valueWithArguments: { offset. previous. finish. scannedThrough. false }.
					(recovery notNil and: [ recovery scanned ])
						ifTrue: [
							recovery offset > event offset ifTrue: [
								aBlock value: (self framingGapFrom: offset to: recovery offset + 1) ].
							event := recovery.
							finish := event end + 1.
							count := frontier second ]
						ifFalse: [
							scannedThrough := finish.
							refusal := recovery ] ] ] ].
		event scanned ifTrue: [
			"Keep the count charged for held items. Rewalk their bytes from the recovered opener."
			scannedThrough := event offset + 2.
			frontier := nil.
			held removeAll ].
		event resumptionAnchor ifNotNil: [
			frontier := nil.
			held removeAll ].
		event startsSegment ifTrue: [
			registry := event effectiveIdentifier ifNotNil: [ event opener registry ] ].
		aBlock value: event.
		refusal ifNotNil: [ report value: event value: refusal ].
		previous := event.
		offset := finish.
		count := count + 1 ]
]

{ #category : #enumerating }
GtsV2Wire >> frameWitnessAt: offset registry: registry held: held [
	| limit finish array code item |
	limit := bytes size + 1.
	offset >= limit ifTrue: [ ^ nil ].
	finish := self heldEndAt: offset limit: limit held: held.
	(finish isNil or: [ finish isString ]) ifTrue: [ ^ nil ].
	array := walker headAt: offset limit: finish.
	code := self recordKind: array limit: finish.
	(array isShortest and: [ array argument = 11 and: [
		code notNil and: [ code isShortest and: [ code argument = 2 ] ] ] ]) ifFalse: [ ^ nil ].
	item := GtsV2Field walker: walker from: offset to: finish.
	^ GtsV2FrameEnvelope read: item registry: registry
]

{ #category : #validation }
GtsV2Wire >> prepare: event predecessor: predecessor witness: peek [
	self check: event witness: peek predecessor: predecessor.
	event framingFailure: (event problem notNil or: [ event link = #broken ])
]

{ #category : #validation }
GtsV2Wire >> finalize: event following: next predecessor: predecessor [
	"A scanned opener can remove the item that tentatively witnessed this frame."
	event link = #healed ifTrue: [
		(next isNil or: [ next frame isNil or: [ next offset ~= event end ] ])
			ifTrue: [ ^ self check: event witness: [ nil ] predecessor: predecessor ] ].
	^ event effectiveIdentifier ifNil: [ event startsSegment ifFalse: [ predecessor ] ]
]

{ #category : #enumerating }
GtsV2Wire >> endAt: offset limit: limit held: held [
	| finish |
	held at: offset ifAbsent: [ ^ walker itemEndAt: offset limit: limit ].
	finish := self heldEndAt: offset limit: limit held: held.
	held removeKey: offset.
	finish isString ifTrue: [ GtsV2FramingError signal: finish ].
	^ finish ifNil: [ GtsV2FramingError signal: 'torn_tail' ]
]

{ #category : #enumerating }
GtsV2Wire >> heldEndAt: offset limit: limit held: held [
	| state finish |
	state := held at: offset ifAbsentPut: [ walker itemAt: offset ].
	(state isInteger or: [ state isString ]) ifTrue: [ ^ state ].
	finish := [ state endUpTo: limit ]
		on: GtsV2FramingError
		do: [ :error | error messageText ].
	finish ifNotNil: [ held at: offset put: finish ].
	^ finish
]

{ #category : #classifying }
GtsV2Wire >> framingGapFrom: start to: limit [
	[ walker itemEndAt: start limit: limit ]
		on: GtsV2FramingError
		do: [ :error |
			^ GtsV2WireEvent kind: #framingGap from: start - 1 to: limit - 1 problem: error messageText ].
	GtsV2FramingError signal: 'resync gap begins at a complete item'
]

{ #category : #validation }
GtsV2Wire >> check: event witness: peek predecessor: predecessor [
	| frame computed stored state effective witness |
	frame := event frame.
	computed := frame identifierOn: bytes predecessor: predecessor.
	stored := frame storedIdentifier.
	state := #broken.
	(computed isNotNil and: [ stored isNotNil ]) ifTrue: [
		computed = stored
			ifTrue: [ state := #linked ]
			ifFalse: [
				(frame isTransparent not and: [ (witness := peek value) isNotNil and: [
					witness storedIdentifier isNotNil and: [
						(witness identifierOn: bytes predecessor: computed) = witness storedIdentifier ] ] ])
					ifTrue: [ state := #healed ] ] ].
	effective := frame isTransparent ifFalse: [
		state = #healed ifTrue: [ computed ] ifFalse: [ stored ifNil: [ computed ] ] ].
	event link: state effectiveIdentifier: effective.
	^ effective ifNil: [ predecessor ]
]

{ #category : #classifying }
GtsV2Wire >> classifyAt: offset limit: limit registry: registry [
	| array code kind problem field opener frame framingFailure |
	array := walker headAt: offset limit: limit.
	code := self recordKind: array limit: limit.
	kind := #opaque.
	code ifNotNil: [
		kind := #(header frame locator offsets resync) at: code argument ifAbsent: [ #opaque ] ].
	field := GtsV2Field walker: walker from: offset to: limit.
	(kind = #header or: [ kind = #resync ])
		ifTrue: [
			[ opener := GtsV2Opener read: field resync: kind = #resync ]
				on: GtsV2EnvelopeError
				do: [ :error | problem := error messageText ].
			framingFailure := opener isNil ]
		ifFalse: [
			problem := self itemProblemFor: array code: code kind: kind registry: (registry ifNil: [ 2 ]).
			framingFailure := problem notNil and: [ problem ~= 'unknown_item_kind' ].
			registry
				ifNil: [ kind := #unchecked. problem := nil ]
				ifNotNil: [
					problem ifNotNil: [ kind := #opaque ].
					(#(offsets locator) includes: kind) ifTrue: [
						framingFailure := GtsV2ClosingChecks framingFailsFor: kind item: field ].
					kind = #frame ifTrue: [
						frame := GtsV2FrameEnvelope read: field registry: registry.
						problem := frame problem ] ] ].
	^ (GtsV2WireEvent kind: kind from: offset - 1 to: limit - 1 problem: problem)
		opener: opener frame: frame;
		framingFailure: framingFailure
]

{ #category : #classifying }
GtsV2Wire >> itemProblemFor: array code: code kind: kind registry: registry [
	code ifNil: [ ^ 'wrong_type' ].
	(array isShortest and: [ code isShortest ]) ifFalse: [ ^ 'not_deterministic' ].
	(kind = #frame and: [ array argument ~= 11 ]) ifTrue: [ ^ 'wrong_length' ].
	code argument = 0 ifTrue: [ ^ 'unregistered_value' ].
	kind = #opaque ifTrue: [
		^ registry = 1 ifTrue: [ 'unregistered_value' ] ifFalse: [ 'unknown_item_kind' ] ].
	^ nil
]

{ #category : #classifying }
GtsV2Wire >> stopAt: offset reason: reason [
	^ GtsV2WireEvent kind: #stop from: offset - 1 to: offset - 1 problem: reason
]
