"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2WireTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #tests }
GtsV2WireTest >> testTypedTermRecordsReserveIdsIndependentlyOfFold [
	#(
		'terms-id-base-mismatch' 'terms-id-base-ill-typed-reserves' 'terms-id-base-after-opaque'
		'terms-record-field-order' 'terms-record-field-order-newer-version'
		'terms-runs-order' 'terms-runs-empty' 'terms-run-kind-unregistered' 'terms-run-count-zero'
		'terms-runs-adjacent-same-kind' 'terms-restart-group-zero'
		'broken-link-content-value-not-judged' 'broken-link-newer-value-not-interpreted'
	) do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P3/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry | self assertTypedTermRecordsOf: entry ] ]
]

{ #category : #tests }
GtsV2WireTest >> testTypedTermReservationAtUintBoundary [
	| report fixture file frames |
	fixture := STONJSON fromString: '/workspace/vectors/v2/P3/terms-id-above-uint.json' asFileReference contents.
	file := (GtsCborReader decode: (GtsHex decode: ((fixture at: 'cases') first at: 'read_result'))) seventh.
	frames := file second first at: 17.
	report := (GtsV2Wire inspect: (self p3Vector: 'terms-id-above-uint.bin')) typedWireFindings.
	"The first frame's payload fails the block count after its runs reserve their IDs."
	2 to: 3 do: [ :index |
		| frame |
		frame := frames at: index.
		self assert: (report frameFindingsAt: frame second) equals: (frame at: 11) asArray ]
]

{ #category : #private }
GtsV2WireTest >> assertTypedTermRecordsOf: entry [
	| name input file report termFinding |
	name := entry at: 'input_file'.
	input := self p3Vector: name.
	file := (GtsCborReader decode: (GtsHex decode: (entry at: 'read_result'))) seventh.
	report := (GtsV2Wire inspect: input) typedWireFindings.
	termFinding := [ :finding | #(6 7) includes: finding first ].
	file second do: [ :segment |
		(segment at: 17) do: [ :frame |
			frame fourth = #(2 1) ifTrue: [
				| expected actual fold |
				expected := ((frame at: 11) select: termFinding) asArray.
				actual := (report frameFindingsAt: frame second) select: termFinding.
				self assert: actual = expected description: name, ' at ', frame second printString,
					' ', actual printString, ' expected ', expected printString.
				fold := frame ninth.
				self assert: (report wireFoldStateAt: frame second) = (fold first = 1 ifFalse: [ fold ])
					description: name ] ] ]
]

{ #category : #private }
GtsV2WireTest >> p3Vector: name [
	^ ('/workspace/vectors/v2/P3/', name) asFileReference binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #tests }
GtsV2WireTest >> testTypedRowOrderAndGrammarFindingsMatchP3 [
	#(
		'rows-order-canonical-ok' 'rows-order-canonical-two-runs'
		'rows-order-canonical-violated' 'rows-order-canonical-across-frames'
		'rows-canonical-follows-opaque' 'rows-record-field-order'
		'rows-column-length' 'rows-graph-runs-sum' 'rows-graph-runs-adjacent-equal'
		'rows-graph-run-zero' 'annotations-count-zero'
	) do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P3/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry | self assertTypedRowFindingsOf: entry ] ]
]

{ #category : #private }
GtsV2WireTest >> assertTypedRowFindingsOf: entry [
	| name input file report contentFinding |
	name := entry at: 'input_file'.
	input := ('/workspace/vectors/v2/P3/', name) asFileReference binaryReadStreamDo: [ :stream | stream upToEnd ].
	file := (GtsCborReader decode: (GtsHex decode: (entry at: 'read_result'))) seventh.
	report := (GtsV2Wire inspect: input) typedWireFindings.
	contentFinding := [ :finding | #(7 21) includes: finding first ].
	file second do: [ :segment |
		(segment at: 17) do: [ :frame |
			| type expected actual |
			type := frame fourth.
			(type first = 2 and: [ type second between: 2 and: 5 ]) ifTrue: [
				expected := ((frame at: 11) select: contentFinding) asArray.
				actual := (report frameFindingsAt: frame second) select: contentFinding.
				self assert: actual = expected description: name, ' at ', frame second printString,
					' ', actual printString, ' expected ', expected printString.
				frame ninth first = 1 ifTrue: [
					self assert: (report wireFoldStateAt: frame second) isNil description: name ] ] ] ]
]

{ #category : #tests }
GtsV2WireTest >> testWireFoldStatesMatchPublishedRecords [
	#(
		'P7/read__fold-precedence-each'
		'P7/left-out-zero' 'P7/left-out-unreadable-segment' 'P7/left-out-unknown-type' 'P7/left-out-stopped'
		'P7/left-out-region' 'P7/left-out-refused' 'P7/left-out-torn-end' 'P7/left-out-torn-mid-file'
		'P1/parity-codec-newer' 'P1/parity-later-version-payload-damaged' 'P1/parity-later-version-after-stop'
	) do: [ :name |
		| root fixture |
		root := '/workspace/vectors/v2/', (name copyUpTo: $/), '/'.
		fixture := STONJSON fromString: ('/workspace/vectors/v2/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry |
			((entry at: 'capacity') at: 'access') = 'whole' ifTrue: [ self assertWireFoldOf: entry root: root ] ] ]
]

{ #category : #private }
GtsV2WireTest >> assertWireFoldOf: entry root: root [
	| fileName input capacity budget report file |
	fileName := entry at: 'input_file'.
	input := (root, fileName) asFileReference binaryReadStreamDo: [ :stream | stream upToEnd ].
	capacity := entry at: 'capacity'.
	budget := (GtsV2PayloadBudget withReadLimit: (capacity at: 'decoded_bytes_per_read'))
		itemLimit: (capacity at: 'items_per_read');
		storedLimit: (capacity at: 'stored_bytes_per_frame') frameLimit: (capacity at: 'decoded_bytes_per_frame').
	report := (GtsV2Wire inspect: input budget: budget) typedWireFindings.
	file := (GtsCborReader decode: (GtsHex decode: (entry at: 'read_result'))) seventh.
	self assert: report wireLeftOut = file seventh description: fileName, ' left out ', report wireLeftOut printString,
		' expected ', file seventh printString.
	file second do: [ :segment |
		(segment at: 17) do: [ :frame |
			| actual expected |
			actual := report wireFoldStateAt: frame second.
			expected := frame at: 9.
			expected first = 1 ifTrue: [ expected := nil ].
			self assert: actual = expected description: fileName, ' fold state at ', frame second printString, ' ',
				actual printString, ' expected ', expected printString ] ]
]

{ #category : #tests }
GtsV2WireTest >> testTypedFileEndAndSegmentRangesMatchPublishedRecords [
	#(
		'P7/file-end-no-commit' 'P7/file-end-at-valid-commit' 'P7/file-end-at-authenticated-commit'
		'P7/file-end-opaque-after-commit' 'P7/file-end-region-after-commit'
		'P7/file-end-torn' 'P7/file-end-undelimitable' 'P7/file-end-not-read'
		'P7/end-locator-own-grammar' 'P7/end-frame-no-effective-identifier'
		'P1/resync-after-torn-swallowed-failing' 'P1/resync-after-torn-staying'
		'P1/resync-after-torn-header' 'P1/resync-after-undelimitable'
		'P1/resync-first-item-torn' 'P1/resync-past-displaced-join'
	) do: [ :name |
		| root fixture |
		root := '/workspace/vectors/v2/', (name copyUpTo: $/), '/'.
		fixture := STONJSON fromString: ('/workspace/vectors/v2/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry |
			((entry at: 'capacity') at: 'access') = 'whole' ifTrue: [ self assertTypedFileEndOf: entry root: root ] ] ]
]

{ #category : #private }
GtsV2WireTest >> assertTypedFileEndOf: entry root: root [
	| fileName input budget read inputs report file capacity |
	fileName := entry at: 'input_file'.
	input := (root, fileName) asFileReference binaryReadStreamDo: [ :stream | stream upToEnd ].
	capacity := entry at: 'capacity'.
	budget := (GtsV2PayloadBudget withReadLimit: (capacity at: 'decoded_bytes_per_read'))
		itemLimit: (capacity at: 'items_per_read').
	read := GtsCborReader decode: (GtsHex decode: (entry at: 'read_result')).
	inputs := read sixth.
	report := (GtsV2Wire inspect: input budget: budget)
		typedWireFindingsHeld: inputs second trusted: inputs first.
	file := read seventh.
	self assert: report fileEnd = file sixth
		description: fileName, ' end ', report fileEnd printString, ' expected ', file sixth printString.
	file second do: [ :segment |
		| extent expected |
		extent := report segmentRangeAt: segment first.
		expected := segment copyFrom: 2 to: 3.
		self assert: extent = expected description: fileName, ' range ', extent printString,
			' expected ', expected printString ]
]

{ #category : #tests }
GtsV2WireTest >> testTypedOpenersMatchP1Records [
	#(
		'header-profile-zero' 'header-profile-unknown' 'lineage-begins-links-unavailable'
		'resync-past-absent' 'resync-past-matches' 'resync-past-differs'
		'resync-past-not-checked-count' 'resync-past-not-checked-cut' 'resync-past-displaced-join'
		'resync-past-unit-equal' 'resync-profile-after-torn-signature' 'resync-field-head-not-shortest'
		'marks-none' 'marks-unknown-tag' 'marks-code-zero' 'marks-range-after-item' 'marks-range-empty'
		'marks-future-finding-code' 'marks-outside-grammar' 'marks-order-at-one-byte'
		'marks-filler-on-scope' 'marks-observed-on-scope' 'marks-registry-version-not-raised'
		'marks-hostile-displaced'
	) do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P1/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry |
			((entry at: 'capacity') at: 'access') = 'whole' ifTrue: [ self assertTypedOpenerOf: entry ] ] ]
]

{ #category : #private }
GtsV2WireTest >> assertTypedOpenerOf: entry [
	| fileName input capacity budget report file |
	fileName := entry at: 'input_file'.
	input := ('/workspace/vectors/v2/P1/', fileName) asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ].
	capacity := entry at: 'capacity'.
	budget := (GtsV2PayloadBudget withReadLimit: (capacity at: 'decoded_bytes_per_read'))
		itemLimit: (capacity at: 'items_per_read').
	report := (GtsV2Wire inspect: input budget: budget) typedWireFindings.
	file := (GtsCborReader decode: (GtsHex decode: (entry at: 'read_result'))) seventh.
	file second do: [ :segment |
		| actual expected ordinal |
		ordinal := segment first.
		actual := report openerFieldsAt: ordinal.
		expected := segment copyFrom: 4 to: 8.
		self assert: actual = expected description: fileName, ' ', actual printString, ' expected ', expected printString.
		actual := (report segmentFindingsAt: ordinal) select: [ :finding | finding first = 48 ].
		expected := ((segment at: 18) select: [ :finding | finding first = 48 ]) asArray.
		self assert: actual = expected description: fileName ]
]

{ #category : #tests }
GtsV2WireTest >> testTypedResyncPastChargesSharedPrefixOnce [
	| header first second input limit report |
	header := #[135 1 2 1 1 129 1 129 1 129 1].
	first := header, (self resyncWithPastOf: header).
	second := first, (self resyncWithPastOf: first).
	input := second, (self resyncWithPastOf: second).
	limit := second size.
	report := (GtsV2Wire inspect: input budget: (GtsV2PayloadBudget withReadLimit: limit)) typedWireFindings.
	1 to: 3 do: [ :ordinal | self assert: (report openerFieldsAt: ordinal) first equals: #(2 1 false 2 0) ].
	report := (GtsV2Wire inspect: input budget: (GtsV2PayloadBudget withReadLimit: limit - 1)) typedWireFindings.
	1 to: 2 do: [ :ordinal | self assert: (report openerFieldsAt: ordinal) first equals: #(2 1 false 2 0) ].
	self assert: (report openerFieldsAt: 3) first equals: #(2 1 false 4 0)
]

{ #category : #private }
GtsV2WireTest >> resyncWithPastOf: prefix [
	| marker body identifier |
	marker := #[252 243 235 50 162 240 38 125 209 106 83 240 155 255 135 146].
	body := GtsCborWriter encode: { 5. 2. marker. 1. 1. #(1). #(1). prefix size.
		{ 2. GtsBlake3 deriveWith: 'gts2 unit' bytes: prefix }. #() }.
	identifier := GtsBlake3 deriveWith: 'gts2 header' bytes: body.
	body at: 1 put: 139.
	^ body, (GtsCborWriter encode: identifier)
]

{ #category : #tests }
GtsV2WireTest >> testTypedFileResultForInputWithoutSegments [
	#(
		'P7/read__empty'
		'P7/read__not-this-format'
		'P7/read__generation-three'
		'P1/file-first-undelimitable'
	) do: [ :name |
		| root fixture |
		root := '/workspace/vectors/v2/' asFileReference / (name copyUpTo: $/).
		fixture := STONJSON fromString: (('/workspace/vectors/v2/', name, '.json') asFileReference contents).
		(fixture at: 'cases') do: [ :entry |
			| input expected report |
			input := (root / (entry at: 'input_file')) binaryReadStreamDo: [ :stream | stream upToEnd ].
			expected := (GtsCborReader decode: (GtsHex decode: (entry at: 'read_result'))) seventh.
			report := (GtsV2Wire inspect: input) typedWireFindings.
			self assert: (GtsCborWriter encode: report fileResultWithoutSegments)
				equals: (GtsCborWriter encode: expected) ] ]
]

{ #category : #tests }
GtsV2WireTest >> testTypedItemCountFindingUsesUnreadRangeAndNextCount [
	| bytes report |
	bytes := self vector: 'append-empty-file.after'.
	report := (GtsV2Wire inspect: bytes budget: (GtsV2PayloadBudget new itemLimit: 0)) typedWireFindings.
	self assert: report fileResultWithoutSegments equals:
		{ 2. #(). { { 20. #(2 87). #(1). #(1). #(2 0). { 2. bytes size }. #(2 1) } }. #(1). #(1 0). 7. 0 }
]

{ #category : #tests }
GtsV2WireTest >> testTypedRefusalNamesTheItemThatContainsTheCandidate [
	| input report |
	input := self vector: 'resync-limit-torn-signature-at-new-kind.v1'.
	report := (GtsV2Wire inspect: input budget: (GtsV2PayloadBudget withReadLimit: 4095)) typedWireFindings.
	self assert: (report segmentFindingsAt: 0) last
		equals: #(20 (2 86) (1) (2 1) (2 3379) (2 4096) (2 4096))
]

{ #category : #tests }
GtsV2WireTest >> testTypedWireIntegrityFindingsMatchP1Records [
	#(
		'lineage-ordinal-mismatch' 'lineage-matches' 'lineage-head-mismatch' 'lineage-first-present'
		'lineage-begins-links-unavailable' 'links-unavailable-lineage-not-compared'
		'lineage-previous-unreadable'
		'link-healed' 'link-heal-opaque-between' 'frame-unknown-critical' 'frame-unknown-noncritical'
		'frame-unknown-transparent' 'frame-unknown-transparent-not-bool'
		'identifier-ill-typed-identity-well-typed' 'identifier-ill-typed-identity-ill-typed'
		'payload-digest' 'payload-length' 'payload-unknown-codec' 'payload-total-order'
		'chain-break-withheld-codec-constraint' 'locator-not-index-item' 'item-kind-newer'
		'stored-matches' 'stored-mismatch' 'stored-check-short' 'frame-storage-tag-newer' 'parity-codec-newer'
		'parity-later-version-payload-damaged' 'parity-later-version-after-stop'
	) do: [ :name |
		| fixture |
		fixture := STONJSON fromString: ('/workspace/vectors/v2/P1/', name, '.json') asFileReference contents.
		(fixture at: 'cases') do: [ :entry | self assertTypedWireIntegrityOf: entry ] ]
]

{ #category : #private }
GtsV2WireTest >> assertTypedWireIntegrityOf: entry [
	| name input capacity budget report file wireFinding |
	name := entry at: 'input_file'.
	input := ('/workspace/vectors/v2/P1/', name) asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ].
	capacity := entry at: 'capacity'.
	budget := (GtsV2PayloadBudget withReadLimit: (capacity at: 'decoded_bytes_per_read'))
		itemLimit: (capacity at: 'items_per_read').
	report := (GtsV2Wire inspect: input budget: budget) typedWireFindings.
	file := (GtsCborReader decode: (GtsHex decode: (entry at: 'read_result'))) seventh.
	wireFinding := [ :finding |
		(#(4 5 6 7 11 12 13 14 20 30 42 43) includes: finding first) or: [
			finding first = 18 and: [ #(19 23) includes: finding second second ] ] ].
	file second do: [ :segment |
		| actual expected ordinal |
		ordinal := segment first.
		self assert: (report anchoredFramesAt: ordinal) = (segment at: 11) description: name.
		self assert: (report transparentFramesAt: ordinal) = (segment at: 14) asArray description: name.
		expected := (segment at: 17) collect: [ :frame | frame copyFrom: 1 to: 7 ].
		self assert: (report frameWireFieldsAt: ordinal) = expected asArray description: name.
		actual := (report segmentFindingsAt: segment first) select: wireFinding.
		expected := ((segment at: 18) select: wireFinding) asArray.
		self assert: actual = expected description: name, ' segment ', segment first printString,
			' ', actual printString, ' expected ', expected printString.
		(segment at: 17) do: [ :frame |
			actual := (report frameFindingsAt: frame second) select: wireFinding.
			expected := ((frame at: 11) select: wireFinding) asArray.
			self assert: actual = expected description: name, ' frame ', frame first printString,
				' ', actual printString, ' expected ', expected printString ] ]
]

{ #category : #private }
GtsV2WireTest >> vector: name [
	^ ('/workspace/vectors/v2/P1/', name, '.bin') asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #private }
GtsV2WireTest >> events: bytes [
	^ self events: bytes budget: GtsV2PayloadBudget new
]

{ #category : #private }
GtsV2WireTest >> events: bytes budget: budget [
	| events |
	events := OrderedCollection new.
	(GtsV2Wire inspect: bytes budget: budget) eventsDo: [ :event | events add: event ].
	^ events
]

{ #category : #tests }
GtsV2WireTest >> testDispatchUsesTheFirstCompleteItemAndFirstVersionKey [
	| wire |
	self assert: (GtsV2Wire inspect: (self vector: 'gen-first-generation-tagged')) format equals: #legacy.
	self assert: (GtsV2Wire inspect: (self vector: 'gen-first-generation-v-twice')) format equals: #unknown.
	self assert: (GtsV2Wire inspect: (self vector: 'gen-first-generation-two')) format equals: #unsupported.
	self assert: (GtsV2Wire inspect: (self vector: 'gen-record-one')) format equals: #unknown.
	self assert: (GtsV2Wire inspect: (self vector: 'gen-record-two')) format equals: #v2.
	wire := GtsV2Wire inspect: #[130 1 27 255 255 255 255 255 255 255 255 28].
	self assert: wire format equals: #unsupported.
	self assert: wire generation equals: 18446744073709551615.
	self assert: (GtsV2Wire inspect: #[191 127 96 97 118 255 1 255]) format equals: #legacy
]

{ #category : #tests }
GtsV2WireTest >> testWalkRetainsZeroBasedRangesBeforeTornTail [
	| events |
	events := self events: (self vector: 'torn-after-frames').
	self assert: (events collect: [ :event | event kind ]) asArray equals: #(header frame frame frame stop).
	self assert: (events at: 4) offset equals: 239.
	self assert: (events at: 4) end equals: 353.
	self assert: events last offset equals: 353.
	self assert: events last problem equals: 'torn_tail'
]

{ #category : #tests }
GtsV2WireTest >> testDispatchRecoversFromForeignTornAndCompleteFirstItems [
	#(
		('resync-first-byte-foreign.whole' 2 'undelimitable_input')
		('resync-first-item-torn.whole' 10 'torn_tail')
		('resync-first-item-fails.whole' 5 'torn_tail')
	) do: [ :entry |
		| wire events |
		wire := GtsV2Wire inspect: (self vector: entry first).
		self assert: wire format equals: #v2.
		self assert: wire generation equals: 2.
		events := OrderedCollection new.
		wire eventsDo: [ :event | events add: event ].
		self assert: events first kind equals: #framingGap.
		self assert: events first problem equals: entry third.
		self assert: events first offset equals: 0.
		self assert: events first end equals: entry second.
		self assert: events second scanned.
		self assert: events second offset equals: entry second.
		self assert: ((events allButFirst: 2) allSatisfy: [ :event | event link = #linked ]) ]
]

{ #category : #tests }
GtsV2WireTest >> testDispatchSearchStartsOnlyInsideTheFailedFirstItem [
	| bytes wire events |
	bytes := self vector: 'resync-first-byte-foreign.whole'.
	bytes at: 1 put: 0.
	wire := GtsV2Wire inspect: bytes.
	self assert: wire format equals: #unknown.
	self assert: wire generation isNil.

	events := self events: (self vector: 'resync-first-item.whole').
	self assert: events first kind equals: #resync.
	self assert: events first offset equals: 0.
	self deny: events first scanned
]

{ #category : #tests }
GtsV2WireTest >> testInitialResyncSharesTheDecodedBudgetAndEnumerationCanRepeat [
	| wire snapshots |
	wire := GtsV2Wire
		inspect: (self vector: 'resync-charge-kept-first-item-lost')
		budget: (GtsV2PayloadBudget withReadLimit: 4626).
	self assert: wire format equals: #v2.
	snapshots := (1 to: 2) collect: [ :pass |
		| events refusal |
		events := OrderedCollection new.
		wire eventsDo: [ :event | events add: event ].
		self assert: (events count: [ :event | event scanned ]) equals: 1.
		refusal := events detect: [ :event | event kind = #resyncRefused ].
		self assert: refusal offset equals: 900.
		self assert: refusal problem equals: 'total_decoded'.
		self assert: refusal payload count equals: 8692.
		events collect: [ :event | { event kind. event offset. event link } ] ].
	self assert: snapshots first equals: snapshots second
]

{ #category : #tests }
GtsV2WireTest >> testInitialResyncRefusalKeepsTheDispatchedOutcome [
	| bytes wire refusal events |
	bytes := self vector: 'resync-first-byte-foreign.whole'.
	wire := GtsV2Wire inspect: bytes budget: (GtsV2PayloadBudget withReadLimit: 4095).
	self assert: wire format equals: #framingError.
	self assert: wire stopReason equals: 'undelimitable_input'.
	refusal := wire initialResyncRefusal.
	self assert: refusal offset equals: 2.
	self assert: refusal payload count equals: 4096.

	events := self
		events: (self vector: 'resync-refused-in-first-opener-once')
		budget: (GtsV2PayloadBudget withReadLimit: 600).
	self assert: events first kind equals: #header.
	self assert: events first problem isNotNil.
	self assert: (events count: [ :event | event kind = #resyncRefused ]) equals: 1.
	self assert: events second offset equals: 700
]

{ #category : #tests }
GtsV2WireTest >> testFirstScanCountsItemsDelimitedThroughItsDecisionPoint [
	| events bytes |
	bytes := self vector: 'item-limit-region-candidate-untested'.
	events := self events: bytes budget: ((GtsV2PayloadBudget withReadLimit: 4095) itemLimit: 5).
	self deny: (events anySatisfy: [ :event | event scanned or: [ event kind = #resyncRefused ] ]).
	self assert: events first kind equals: #header.
	self assert: events first problem isNotNil.
	self assert: events last kind equals: #stop.
	self assert: events last offset equals: 271.
	self assert: events last problem equals: 'undelimitable_input'
]

{ #category : #tests }
GtsV2WireTest >> testMidFileClassificationScansOnePrecedingItemForRegistryOne [
	| events kept recovered higher |
	events := self events: (self vector: 'resync-limit-torn-signature-at-new-kind.v1').
	kept := events second.
	recovered := events third.
	self assert: kept frameType equals: 14.
	self assert: kept link equals: #linked.
	self assert: kept end equals: 3537.
	self assert: recovered scanned.
	self assert: recovered offset equals: 3379.
	self assert: events last link equals: #linked.
	self deny: (events anySatisfy: [ :event | event kind = #framingGap ]).

	higher := self events: (self vector: 'resync-limit-torn-signature-at-new-kind.v2').
	self deny: (higher anySatisfy: [ :event | event scanned ]).
	self assert: higher third problem equals: 'unknown_item_kind'
]

{ #category : #tests }
GtsV2WireTest >> testMidFileOpenerAndClosingGrammarFailuresCanContainAResync [
	| bytes originalPrefix tail |
	bytes := self vector: 'resync-after-undelimitable.whole'.
	originalPrefix := bytes first: 114.
	tail := bytes copyFrom: 211 to: bytes size.
	{ #[131 1 2 88 65]. #[130 3 88 65]. #[130 4 88 65] } do: [ :badItemHead |
		| padding events gap opener |
		padding := 210 - originalPrefix size - badItemHead size - 2.
		events := self events: originalPrefix, (ByteArray with: 88 with: padding),
			(ByteArray new: padding), badItemHead, tail.
		gap := events detect: [ :event | event kind = #framingGap ].
		opener := events detect: [ :event | event scanned ].
		self assert: gap problem equals: 'torn_tail'.
		self assert: gap offset equals: 210 - badItemHead size.
		self assert: gap end equals: 210.
		self assert: opener offset equals: 210.
		self assert: events last link equals: #linked ]
]

{ #category : #tests }
GtsV2WireTest >> testOverlappingCompleteItemScanDoesNotRetestAnOccurrence [
	| events refusals |
	events := self
		events: (self vector: 'resync-occurrence-tested-once')
		budget: (GtsV2PayloadBudget withReadLimit: 4626).
	refusals := events select: [ :event | event kind = #resyncRefused ].
	self assert: refusals size equals: 1.
	self assert: refusals first offset equals: 900.
	self assert: refusals first payload count equals: 8192.
	self deny: (events anySatisfy: [ :event | event scanned ])
]

{ #category : #tests }
GtsV2WireTest >> testRecoveredOpenerCanRemoveAHealingWitness [
	| events frame refusal |
	events := self events: (self vector: 'resync-heal-witness-removed.ample-whole').
	frame := events detect: [ :event | event offset = 114 ].
	self assert: frame link equals: #broken.
	self assert: (events detect: [ :event | event scanned ]) offset equals: 316.
	self deny: (events anySatisfy: [ :event | event kind = #frame and: [ event offset = 231 ] ]).

	events := self
		events: (self vector: 'resync-heal-witness-removed.low-whole')
		budget: (GtsV2PayloadBudget withReadLimit: 4114).
	frame := events detect: [ :event | event offset = 114 ].
	refusal := events detect: [ :event | event kind = #resyncRefused ].
	self assert: frame link equals: #healed.
	self assert: refusal offset equals: 316.
	self deny: (events anySatisfy: [ :event | event scanned ])
]

{ #category : #tests }
GtsV2WireTest >> testFailedFrameScansOnlyAfterTheImmediateWitnessIsDecided [
	| events |
	events := self events: (self vector: 'resync-healed-not-scanned.whole').
	self assert: events third link equals: #healed.
	self deny: (events anySatisfy: [ :event | event scanned ]).

	events := self events: (self vector: 'resync-heal-next-item-only.i-whole').
	self assert: (events detect: [ :event | event scanned ]) offset equals: 199.

	events := self
		events: (self vector: 'resync-heal-at-item-limit.whole')
		budget: (GtsV2PayloadBudget new itemLimit: 3).
	self assert: events third link equals: #broken.
	self assert: events last problem equals: 'items_per_read'.
	self deny: (events anySatisfy: [ :event | event scanned ])
]

{ #category : #tests }
GtsV2WireTest >> testFailedFrameChargesItsPayloadBeforeTestingTheCandidate [
	| bytes events frame |
	bytes := self vector: 'resync-total-refused-no-occurrence-tested'.
	events := self events: bytes.
	self assert: (events detect: [ :event | event scanned ]) offset equals: 700.

	events := self events: bytes budget: (GtsV2PayloadBudget withReadLimit: 600).
	frame := events detect: [ :event | event kind = #frame ].
	self assert: frame payload reason equals: 'total_decoded'.
	self assert: frame payload count equals: 698.
	self deny: (events anySatisfy: [ :event | event scanned or: [ event kind = #resyncRefused ] ])
]

{ #category : #tests }
GtsV2WireTest >> testTypeSpecificContentValueDoesNotTriggerAFramingScan [
	| events |
	events := self events: (self vector: 'framing-test-content-value').
	self assert: events last frameType equals: 9.
	self assert: events last link equals: #linked.
	self deny: (events anySatisfy: [ :event | event scanned ])
]

{ #category : #tests }
GtsV2WireTest >> testResyncResumesAfterAnUndelimitableOrIncompleteItem [
	#(
		('resync-after-undelimitable.whole' 210 'undelimitable_input')
		('resync-after-torn-staying.whole' 130 'torn_tail')
	) do: [ :entry |
		| events opener gap following |
		events := self events: (self vector: entry first).
		opener := events detect: [ :event | event scanned ].
		gap := events detect: [ :event | event kind = #framingGap ].
		self assert: opener kind equals: #resync.
		self assert: opener offset equals: entry second.
		self assert: gap problem equals: entry third.
		self assert: gap end equals: opener offset.
		following := events allButFirst: (events indexOf: opener).
		self deny: following isEmpty.
		self assert: (following allSatisfy: [ :event | event link = #linked ]) ]
]

{ #category : #tests }
GtsV2WireTest >> testResyncChecksPlaceBeforeChargingAndShapeAfterCharging [
	| bytes events refusal |
	events := self
		events: (self vector: 'resync-offset-false.whole')
		budget: (GtsV2PayloadBudget withReadLimit: 4095).
	self deny: (events anySatisfy: [ :event | event scanned or: [ event kind = #resyncRefused ] ]).
	self assert: events last kind equals: #stop.

	bytes := self vector: 'resync-candidate-marks-unordered.whole'.
	events := self events: bytes.
	self deny: (events anySatisfy: [ :event | event scanned ]).
	self assert: events last kind equals: #stop.

	events := self events: bytes budget: (GtsV2PayloadBudget withReadLimit: 4095).
	refusal := events detect: [ :event | event kind = #resyncRefused ].
	self assert: refusal offset equals: 210.
	self assert: refusal end - refusal offset equals: 4096.
	self assert: refusal problem equals: 'total_decoded'.
	self assert: refusal payload count >= 4096.
	self assert: events last kind equals: #stop
]

{ #category : #tests }
GtsV2WireTest >> testResyncRequiresTheStoredIdentifierToMatchWhenScanned [
	| bytes opener events |
	bytes := self vector: 'resync-after-undelimitable.whole'.
	opener := (self events: bytes) detect: [ :event | event scanned ].
	bytes at: opener end put: ((bytes at: opener end) bitXor: 1).
	events := self events: bytes.
	self deny: (events anySatisfy: [ :event | event scanned ]).
	self assert: events last problem equals: 'undelimitable_input'
]

{ #category : #tests }
GtsV2WireTest >> testResyncCountsAnUndelimitableRegionAgainstTheItemLimit [
	| events bytes opener |
	events := self
		events: (self vector: 'resync-after-undelimitable.whole')
		budget: ((GtsV2PayloadBudget withReadLimit: 4095) itemLimit: 4).
	self deny: (events anySatisfy: [ :event | event scanned or: [ event kind = #resyncRefused ] ]).
	self assert: events last problem equals: 'undelimitable_input'.

	bytes := self vector: 'resync-after-torn-staying.whole'.
	events := self
		events: bytes
		budget: (GtsV2PayloadBudget new itemLimit: 3).
	opener := events detect: [ :event | event scanned ].
	self assert: opener offset equals: 130.
	self assert: events last problem equals: 'items_per_read'
]

{ #category : #tests }
GtsV2WireTest >> testRecoveredUndelimitableRegionFollowsClosingOffsets [
	| bytes prefix offsets events |
	bytes := self vector: 'resync-after-undelimitable.whole'.
	prefix := (bytes first: 114), #[88 85], (ByteArray new: 85),
		#[130 4 131 11 24 103 24 87].
	offsets := (self events: prefix) last.
	self assert: offsets kind equals: #offsets.
	self assert: offsets problem isNil.
	self assert: offsets closingClaimFailure isNil.

	events := self events: prefix, (bytes copyFrom: 210 to: bytes size).
	self assert: (events anySatisfy: [ :event | event scanned ]).
	offsets := events detect: [ :event | event kind = #offsets ].
	self assert: offsets problem equals: 'unordered_or_repeated'
]

{ #category : #tests }
GtsV2WireTest >> testMalformedPrefixKeepsItemsUncheckedUntilNextOpener [
	| events opener |
	opener := self vector: 'gen-record-two'.
	events := self events: (#[130 1 2 0], opener, #[0]).
	self assert: (events collect: [ :event | event kind ]) asArray equals: #(header unchecked header opaque).
	self assert: events first problem equals: 'wrong_length'.
	self assert: events last problem equals: 'wrong_type'.
	self assert: (self events: opener, #[129 0]) last problem equals: 'unregistered_value'.
	self assert: (self events: (self vector: 'header-long-form-first-item')) first problem equals: 'not_deterministic'.
	self assert: (self events: (self vector: 'frame-test-array-long-form')) last problem equals: 'not_deterministic'
]

{ #category : #tests }
GtsV2WireTest >> testWalkDistinguishesTornPrefixesFromImpossibleHeads [
	| opener entries wire |
	opener := self vector: 'gen-record-two'.
	entries := {
		#[248] -> 'torn_tail'.
		#[248 0] -> 'undelimitable_input'.
		#[191 1 255] -> 'undelimitable_input'.
		#[95 122] -> 'undelimitable_input'.
		#[155 255 255 255 255 255 255 255 255] -> 'torn_tail' }.
	entries do: [ :entry |
		| last |
		last := (self events: opener, entry key) last.
		self assert: last kind equals: #stop.
		self assert: last offset equals: opener size.
		self assert: last problem equals: entry value ].
	wire := GtsV2Wire inspect: (self vector: 'file-first-incomplete').
	self assert: wire format equals: #framingError.
	self assert: wire stopReason equals: 'torn_tail'
]

{ #category : #tests }
GtsV2WireTest >> testWalksNestedAndChunkedItemsWithoutRecursiveDecoding [
	| opener nested events |
	opener := self vector: 'gen-record-two'.
	nested := (ByteArray new: 4096 withAll: 129), #[0].
	events := self events: (opener, nested, #[95 65 1 255], opener).
	self assert: (events collect: [ :event | event kind ]) asArray equals: #(header opaque opaque header).
	self assert: events last offset equals: opener size + nested size + 4
]

{ #category : #tests }
GtsV2WireTest >> testResumableItemDelimitsAcrossEverySplit [
	| walker cursor fork |
	walker := GtsV2CborWalker on: #[159 191 97 107 95 88 3 1 2 3 255 255 25 0 1 255 42].
	cursor := walker itemAt: 1.
	1 to: 16 do: [ :limit |
		self assert: (cursor endUpTo: limit) isNil.
		limit = 8 ifTrue: [ fork := cursor copy ] ].
	self assert: (cursor endUpTo: 17) equals: 17.
	self assert: (fork endUpTo: 12) isNil.
	self assert: (fork endUpTo: 18) equals: 17.

	cursor := (GtsV2CborWalker on: #[191 0 255]) itemAt: 1.
	self assert: (cursor endUpTo: 3) isNil.
	self should: [ cursor endUpTo: 4 ] raise: GtsV2FramingError
]

{ #category : #tests }
GtsV2WireTest >> testRecoveryLookaheadRetainsANestedItemAcrossDecisionBoundaries [
	| opener failures nested wire |
	opener := #[130 1 2].
	failures := ByteArray new: 64.
	nested := (ByteArray new: 8192 withAll: 129), #[0].
	wire := GtsV2Wire
		inspect: opener, failures, nested
		budget: (GtsV2PayloadBudget new itemLimit: 66).
	2 timesRepeat: [
		| events |
		events := OrderedCollection new.
		wire eventsDo: [ :event | events add: event ].
		self assert: events size equals: 66.
		self assert: events last kind equals: #unchecked.
		self assert: events last offset equals: opener size + failures size.
		self assert: events last end equals: opener size + failures size + nested size ]
]

{ #category : #tests }
GtsV2WireTest >> testOpenerRegistryAndResyncGrammar [
	| events |
	events := self events: (self vector: 'header-hash-newer').
	self assert: events first opener registry equals: 2.
	self assert: events first opener hashAlgorithm equals: 9.
	self assert: events last kind equals: #unchecked.
	self assert: (self events: (self vector: 'header-hash-unregistered')) first problem
		equals: 'unregistered_value'.
	#(
		('marks-order-at-one-byte.head-before-order' 'not_deterministic')
		('marks-order-at-one-byte.65th-and-below' 'out_of_range')
		('marks-order-at-one-byte.order-before-deeper-head' 'unordered_or_repeated')
		('marks-order-at-one-byte.text-first-below' 'wrong_type')
		('resync-too-long.walked' 'out_of_range')
		('resync-too-long.exactly-4096' nil)
	) do: [ :entry |
		| resync |
		resync := (self events: (self vector: entry first)) detect: [ :event | event kind = #resync ].
		self assert: resync problem = entry second description: entry first ]
]

{ #category : #tests }
GtsV2WireTest >> testResyncLimitChecksTheStartOfNestedFields [
	| prefix |
	prefix := GtsHex decode: '8b050250fcf3eb32a2f0267dd16a53f09bff879202098101820282'.
	#((4061 'wrong_type') (4062 'out_of_range')) do: [ :entry |
		| size digest bytes |
		size := entry first.
		digest := (ByteArray with: 89 with: size // 256 with: size \\ 256), (ByteArray new: size).
		bytes := prefix, digest, #[1 0 130 2 0 128 64].
		self assert: (self events: bytes) first problem equals: entry second ]
]

{ #category : #tests }
GtsV2WireTest >> testFrameEnvelopeReportsFirstProblemInWireOrder [
	| parity |
	#(
		('frame-flags-order' 'forbidden_combination')
		('envelope-field-before-content-value' 'wrong_type')
		('frame-compressed-empty' 'forbidden_combination')
		('stored-check-short' 'wrong_length')
		('frame-unknown-content-map' 'wrong_type')
		('frame-unknown-content-bad-utf8' 'wrong_type')
		('frame-signatures-unordered' 'unordered_or_repeated')
		('frame-storage-tag-newer' nil)
	) do: [ :entry |
		| frame |
		frame := (self events: (self vector: entry first)) last.
		self assert: frame kind equals: #frame.
		self assert: frame problem = entry second description: entry first ].
	parity := (self events: (self vector: 'parity-signature')) detect: [ :event | event frameType = 16 ].
	self assert: parity problem equals: 'forbidden_combination'
]

{ #category : #tests }
GtsV2WireTest >> testLinkUsesOnlyTheNextItemAndKeepsTransparentFramesOutOfTheChain [
	#(
		('link-healed-effective' (healed linked))
		('link-healed-before-transparent' (healed linked))
		('link-heal-opaque-between' (linked broken broken linked))
		('link-transparent-damaged' (linked linked linked broken linked))
		('resync-identifier-field-differs' (linked linked linked linked))
	) do: [ :entry |
		| links |
		links := self linksIn: (self vector: entry first).
		self assert: links = entry second description: entry first ]
]

{ #category : #tests }
GtsV2WireTest >> testMissingStoredIdentifierFallsBackOnlyToAWellTypedIdentity [
	self assert: (self linksIn: (self vector: 'identifier-ill-typed-identity-well-typed'))
		equals: #(broken linked linked linked).
	self assert: (self linksIn: (self vector: 'identifier-ill-typed-identity-ill-typed'))
		equals: #(broken broken linked linked)
]

{ #category : #tests }
GtsV2WireTest >> testLinkUsesBytesOfIndefiniteIdentifierDespiteEnvelopeFinding [
	| bytes firstFrame body modified frames |
	bytes := self vector: 'link-ok'.
	firstFrame := (self events: bytes) second.
	body := firstFrame end - 32.
	modified := (bytes copyFrom: 1 to: body - 3), #[95 80],
		(bytes copyFrom: body to: body + 15), #[80],
		(bytes copyFrom: body + 16 to: body + 31), #[255],
		(bytes copyFrom: firstFrame end to: bytes size).
	frames := self events: modified.
	self assert: frames second problem equals: 'not_deterministic'.
	self assert: (self linksIn: modified) equals: #(linked linked linked linked)
]

{ #category : #tests }
GtsV2WireTest >> testEventsCannotAlterThePredecessorForLaterFrames [
	| links wire |
	links := OrderedCollection new.
	wire := GtsV2Wire inspect: (self vector: 'link-ok').
	wire eventsDo: [ :event |
		event link ifNotNil: [ :link | links add: link ].
		event effectiveIdentifier ifNotNil: [ :identifier |
			identifier at: 1 put: ((identifier at: 1) bitXor: 255) ] ].
	self assert: links asArray equals: #(linked linked linked linked)
]

{ #category : #private }
GtsV2WireTest >> linksIn: bytes [
	| frames |
	frames := (self events: bytes) select: [ :event | event kind = #frame ].
	^ (frames collect: [ :event | event link ]) asArray
]

{ #category : #tests }
GtsV2WireTest >> testStoredCheckAndPayloadFindingAreIndependent [
	#(
		('stored-mismatch' false damaged 'decode_failed')
		('zstd-ok' true verified nil)
		('zstd-header-short' true damaged 'codec_constraint')
		('zstd-two-frames' true damaged 'decode_failed')
		('zstd-reserved-block' true damaged 'decode_failed')
		('zstd-output-short' true damaged 'decode_failed')
		('payload-length' nil damaged 'decoded_length')
		('payload-digest' nil damaged 'payload_digest')
		('payload-unknown-codec' true unchecked 'unknown_codec')
	) do: [ :entry |
		| frame |
		frame := (self events: (self vector: entry first)) detect: [ :event | event kind = #frame ].
		self assert: frame storedMatches = entry second description: entry first.
		self assert: frame payload state = entry third description: entry first.
		self assert: frame payload reason = entry fourth description: entry first ].
	(self events: (self vector: 'payload-codec-constraint-twice')) do: [ :event |
		event kind = #frame ifTrue: [
			self assert: event problem equals: 'forbidden_combination'.
			self assert: event payload state equals: #damaged.
			self assert: event payload reason isNil ] ]
]

{ #category : #tests }
GtsV2WireTest >> testStoredCheckHashesTheBytesOfAnIndefinitePayload [
	| bytes frame walker fields stored chunked first changed |
	bytes := self vector: 'zstd-ok'.
	frame := (self events: bytes) second.
	walker := GtsV2CborWalker on: bytes.
	fields := (GtsV2Field walker: walker from: frame offset + 1 to: frame end + 1) fields: 11.
	8 timesRepeat: [ fields next ].
	stored := fields next.
	first := stored byteBodyOfSize: nil.
	chunked := ByteArray streamContents: [ :stream |
		stream nextPut: 95.
		first to: stored end - 1 do: [ :index |
			stream nextPut: 65.
			stream nextPut: (bytes at: index) ].
		stream nextPut: 255 ].
	changed := (bytes copyFrom: 1 to: stored start - 1), chunked,
		(bytes copyFrom: stored end to: bytes size).
	frame := (self events: changed) second.
	self assert: frame problem equals: 'not_deterministic'.
	self assert: frame storedMatches equals: true.
	self assert: frame payload state equals: #damaged.
	self assert: frame payload reason isNil
]

{ #category : #tests }
GtsV2WireTest >> testTotalDecodedRefusalRemainsAcrossSegments [
	| compressed events frames |
	compressed := self vector: 'zstd-ok'.
	events := self events: compressed, compressed, (self vector: 'link-ok')
		budget: (GtsV2PayloadBudget withReadLimit: 599).
	frames := events select: [ :event | event kind = #frame ].
	self assert: frames first payload state equals: #verified.
	self assert: frames second storedMatches equals: true.
	self assert: frames second payload reason equals: 'total_decoded'.
	self assert: frames second payload count equals: 600.
	self assert: frames third payload reason equals: 'total_decoded'.
	self assert: frames third payload count equals: 300
]

{ #category : #tests }
GtsV2WireTest >> testClosingClaimsUseObservedItemsAndTheLastTypedIndex [
	#(
		('offsets-counts-opaque' offsets nil nil)
		('offsets-count-short' offsets nil 'index_offsets')
		('offsets-count-long' offsets nil 'index_offsets')
		('offsets-length-differs' offsets nil 'index_offsets')
		('locator-last-index.first' locator nil 'index_locator')
		('locator-last-index.second' locator nil nil)
		('locator-index-type-long-form' locator nil 'index_locator')
		('locator-short-form' locator 'not_deterministic' nil)
	) do: [ :entry |
		| event |
		event := (self events: (self vector: entry first)) last.
		self assert: event kind = entry second description: entry first.
		self assert: event problem = entry third description: entry first.
		self assert: event closingClaimFailure = entry fourth description: entry first ]
]

{ #category : #tests }
GtsV2WireTest >> testClosingPlacementComesBeforeClaimsAndResetsAtAnOpener [
	| header offsets locator events |
	header := self vector: 'gen-record-two'.
	offsets := #[130 4 129 11].
	locator := #[130 3 27 0 0 0 0 0 0 0 1].
	events := self events: header, offsets, locator, locator, header, offsets.
	self assert: (events collect: [ :event | event kind ]) asArray
		equals: #(header offsets locator locator header offsets).
	self assert: (events collect: [ :event | event problem ]) asArray
		equals: #(nil 'unordered_or_repeated' 'unordered_or_repeated' 'unordered_or_repeated' nil nil).
	self assert: (events allSatisfy: [ :event | event closingClaimFailure isNil ]).
	events := self events: header, offsets, locator, #[27].
	self assert: (events at: 2) closingClaimFailure isNil.
	self assert: (events at: 3) closingClaimFailure equals: 'index_locator'.
	self assert: events last problem equals: 'torn_tail'
]

{ #category : #tests }
GtsV2WireTest >> testMalformedLengthAfterMismatchStillReportsGrammarError [
	| event |
	event := (self events: (self vector: 'gen-record-two'), #[130 4 130 0 96]) last.
	self assert: event closingClaimFailure isNil.
	self assert: event problem equals: 'wrong_type'
]
