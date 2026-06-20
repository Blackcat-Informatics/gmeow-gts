Class {
	#name : #GtsReplication,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #private }
GtsReplication class >> aggregateDigestFor: inventory [
	| heads |
	heads := (inventory at: 'segments') collect: [ :segment | segment at: 'head' ].
	^ GtsBlake3 hash: (GtsCborWriter encode: { 'gts-segment-heads-v1'. heads asArray })
]

{ #category : #private }
GtsReplication class >> boolJson: boolean [
	^ boolean ifTrue: [ 'true' ] ifFalse: [ 'false' ]
]

{ #category : #inventory }
GtsReplication class >> collectFramesFrom: itemRecords start: startIndex end: endIndex [
	| expectedPrev frames header |
	frames := OrderedCollection new.
	header := GtsWire unwrapHeader: ((itemRecords at: startIndex + 1) at: 'item').
	expectedPrev := header at: 'id' ifAbsent: [ GtsWire headerIdFor: header ].
	startIndex + 1 to: endIndex - 1 do: [ :itemZeroIndex |
		| computed frame frameId frameType prev record storedId valid |
		record := itemRecords at: itemZeroIndex + 1.
		frame := record at: 'item'.
		frame isDictionary
			ifFalse: [
				frames add: (Dictionary newFromPairs: {
					'item_index'. itemZeroIndex.
					'frame_index'. itemZeroIndex - startIndex - 1.
					'start'. (record at: 'start').
					'end'. (record at: 'end').
					'id'. ByteArray new.
					'type'. '<non-map>'.
					'valid'. false }) ]
			ifTrue: [
				computed := GtsWire contentIdForFrame: frame.
				storedId := frame at: 'id' ifAbsent: [ nil ].
				frameId := storedId ifNil: [ computed ].
				prev := frame at: 'prev' ifAbsent: [ nil ].
				frameType := frame at: 't' ifAbsent: [ '<unknown>' ].
				valid := storedId notNil and: [ storedId = computed and: [ prev = expectedPrev ] ].
				frames add: (Dictionary newFromPairs: {
					'item_index'. itemZeroIndex.
					'frame_index'. itemZeroIndex - startIndex - 1.
					'start'. (record at: 'start').
					'end'. (record at: 'end').
					'id'. frameId.
					'type'. frameType.
					'valid'. valid }).
				expectedPrev := frameId ] ].
	^ frames asArray
]

{ #category : #inventory }
GtsReplication class >> inventoryFor: data [
	| bounds cleanEnd itemRecords segments torn |
	itemRecords := self itemRecordsFrom: data.
	torn := itemRecords at: 'torn'.
	cleanEnd := torn ifNil: [ data size ] ifNotNil: [ torn ].
	itemRecords := itemRecords at: 'items'.
	bounds := OrderedCollection new.
	1 to: itemRecords size do: [ :index |
		(GtsReader isHeaderItem: ((itemRecords at: index) at: 'item'))
			ifTrue: [ bounds add: index - 1 ] ].
	segments := OrderedCollection new.
	1 to: bounds size do: [ :index |
		| endItem graph items startItem |
		startItem := bounds at: index.
		endItem := index < bounds size
			ifTrue: [ bounds at: index + 1 ]
			ifFalse: [ itemRecords size ].
		items := (itemRecords copyFrom: startItem + 1 to: endItem)
			collect: [ :record | record at: 'item' ].
		graph := GtsReader readSegment: items indexOffset: startItem.
		segments add: (Dictionary newFromPairs: {
			'index'. index - 1.
			'item_start'. startItem.
			'item_end'. endItem.
			'start'. ((itemRecords at: startItem + 1) at: 'start').
			'end'. (endItem < itemRecords size
				ifTrue: [ (itemRecords at: endItem + 1) at: 'start' ]
				ifFalse: [ cleanEnd ]).
			'profile'. (graph profiles ifEmpty: [ 'generic' ] ifNotEmpty: [ graph profiles first ]).
			'head'. (graph segmentHeads ifEmpty: [ nil ] ifNotEmpty: [ graph segmentHeads first ]).
			'frame_count'. endItem - startItem - 1.
			'layout'. (graph streamable ifEmpty: [
				Dictionary newFromPairs: { 'claimed'. false. 'covered'. 0. 'tail'. 0 } ]
				ifNotEmpty: [ graph streamable first ]).
			'diagnostics'. graph diagnostics.
			'frames'. (self collectFramesFrom: itemRecords start: startItem end: endItem) }) ].
	^ Dictionary newFromPairs: {
		'segments'. segments asArray.
		'torn'. torn.
		'clean_end'. cleanEnd.
		'item_count'. itemRecords size }
]

{ #category : #json }
GtsReplication class >> headsJsonFor: inventory [
	| aggregateDigest clean fileHead heads segments torn |
	segments := inventory at: 'segments'.
	heads := segments collect: [ :segment | GtsHex encode: (segment at: 'head') ].
	fileHead := heads isEmpty ifTrue: [ nil ] ifFalse: [ heads last ].
	aggregateDigest := GtsHex encode: (self aggregateDigestFor: inventory).
	clean := (self hasProblems: inventory) not.
	torn := inventory at: 'torn'.
	^ String streamContents: [ :stream |
		stream
			nextPutAll: '{"schema":"gts-replication-heads-v1","clean":';
			nextPutAll: (self boolJson: clean);
			nextPutAll: ',"segment_heads":';
			nextPutAll: (self jsonStringArray: heads);
			nextPutAll: ',"aggregate":{"schema":"gts-segment-heads-v1","count":';
			nextPutAll: heads size asString;
			nextPutAll: ',"digest":"';
			nextPutAll: aggregateDigest;
			nextPutAll: '","file_head":';
			nextPutAll: (self nullableJsonString: fileHead);
			nextPutAll: '},"torn_at":';
			nextPutAll: (torn ifNil: [ 'null' ] ifNotNil: [ torn asString ]);
			nextPutAll: ',"fatal":null}';
			lf ]
]

{ #category : #inventory }
GtsReplication class >> hasProblems: inventory [
	| segments |
	(inventory at: 'torn') ifNotNil: [ ^ true ].
	segments := inventory at: 'segments'.
	segments isEmpty ifTrue: [ ^ true ].
	segments do: [ :segment |
		(segment at: 'diagnostics') isEmpty ifFalse: [ ^ true ] ].
	^ false
]

{ #category : #inventory }
GtsReplication class >> itemRecordsFrom: data [
	| items reader torn |
	reader := GtsCborReader on: data.
	items := OrderedCollection new.
	torn := nil.
	[ reader atEnd ] whileFalse: [
		| start |
		start := reader position - 1.
		[ | item |
			item := reader readItem.
			items add: (Dictionary newFromPairs: {
				'start'. start.
				'end'. reader position - 1.
				'item'. item }) ]
			on: Error
			do: [ :error |
				torn := start.
				reader skipToEnd ] ].
	^ Dictionary newFromPairs: {
		'items'. items asArray.
		'torn'. torn }
]

{ #category : #json }
GtsReplication class >> jsonByteRangeStart: start end: end [
	^ '{"start":', start asString,
		',"end":', end asString,
		',"length":', (end - start max: 0) asString,
		'}'
]

{ #category : #json }
GtsReplication class >> jsonString: string [
	| escaped |
	escaped := string asString.
	escaped := escaped copyReplaceAll: '\' with: '\\'.
	escaped := escaped copyReplaceAll: '"' with: '\"'.
	escaped := escaped copyReplaceAll: String lf with: '\n'.
	escaped := escaped copyReplaceAll: String cr with: '\r'.
	escaped := escaped copyReplaceAll: String tab with: '\t'.
	^ '"', escaped, '"'
]

{ #category : #json }
GtsReplication class >> jsonStringArray: strings [
	^ '[', ((strings collect: [ :string | self jsonString: string ]) joinUsing: ','), ']'
]

{ #category : #missing }
GtsReplication class >> missingFromHead: head in: inventory [
	| cleanEnd ranges segments |
	(self hasProblems: inventory) ifTrue: [
		^ Dictionary newFromPairs: {
			'status'. 'error'.
			'from_head'. head.
			'ranges'. #().
			'scan_required'. true.
			'detail'. (self problemDetailFor: inventory) } ].
	segments := inventory at: 'segments'.
	cleanEnd := inventory at: 'clean_end'.
	segments do: [ :segment |
		(segment at: 'head') = head ifTrue: [
			ranges := OrderedCollection new.
			(segment at: 'end') < cleanEnd ifTrue: [
				ranges add: { segment at: 'end'. cleanEnd } ].
			^ Dictionary newFromPairs: {
				'status'. ranges isEmpty ifTrue: [ 'complete' ] ifFalse: [ 'ranges' ].
				'from_head'. head.
				'ranges'. ranges asArray.
				'scan_required'. false.
				'detail'. nil } ].
		(segment at: 'frames') do: [ :frame |
			(((frame at: 'valid') = true) and: [ (frame at: 'id') = head ]) ifTrue: [
				ranges := OrderedCollection new.
				(frame at: 'end') < cleanEnd ifTrue: [
					ranges add: { frame at: 'end'. cleanEnd } ].
				^ Dictionary newFromPairs: {
					'status'. ranges isEmpty ifTrue: [ 'complete' ] ifFalse: [ 'ranges' ].
					'from_head'. head.
					'ranges'. ranges asArray.
					'scan_required'. false.
					'detail'. nil } ] ] ].
	^ Dictionary newFromPairs: {
		'status'. 'unknown'.
		'from_head'. head.
		'ranges'. #().
		'scan_required'. true.
		'detail'. 'unknown peer head; scan required' }
]

{ #category : #json }
GtsReplication class >> missingJson: result [
	^ String streamContents: [ :stream |
		stream
			nextPutAll: '{"schema":"gts-replication-missing-v1","status":"';
			nextPutAll: (result at: 'status');
			nextPutAll: '","from_head":"';
			nextPutAll: (GtsHex encode: (result at: 'from_head'));
			nextPutAll: '","ranges":['.
		(result at: 'ranges') withIndexDo: [ :range :index |
			index > 1 ifTrue: [ stream nextPut: $, ].
			stream nextPutAll: (self jsonByteRangeStart: (range at: 1) end: (range at: 2)) ].
		stream
			nextPutAll: '],"scan_required":';
			nextPutAll: (self boolJson: (result at: 'scan_required'));
			nextPutAll: ',"detail":';
			nextPutAll: (self nullableJsonString: (result at: 'detail'));
			nextPutAll: '}';
			lf ]
]

{ #category : #json }
GtsReplication class >> nullableJsonString: stringOrNil [
	stringOrNil ifNil: [ ^ 'null' ].
	^ self jsonString: stringOrNil
]

{ #category : #resume }
GtsReplication class >> resumeBytesAfter: frameId in: data [
	| cleanEnd inventory |
	inventory := self inventoryFor: data.
	(self hasProblems: inventory) ifTrue: [ self error: (self problemDetailFor: inventory) ].
	cleanEnd := inventory at: 'clean_end'.
	(inventory at: 'segments') do: [ :segment |
		(segment at: 'frames') do: [ :frame |
			(((frame at: 'valid') = true) and: [ (frame at: 'id') = frameId ])
				ifTrue: [ ^ data copyFrom: (frame at: 'end') + 1 to: cleanEnd ] ] ].
	self error: 'frame not found'
]

{ #category : #json }
GtsReplication class >> segmentsJsonFor: inventory [
	| clean segments torn |
	segments := inventory at: 'segments'.
	clean := (self hasProblems: inventory) not.
	torn := inventory at: 'torn'.
	^ String streamContents: [ :stream |
		stream
			nextPutAll: '{"schema":"gts-replication-segments-v1","clean":';
			nextPutAll: (self boolJson: clean);
			nextPutAll: ',"segments":['.
		segments withIndexDo: [ :segment :index |
			| layout |
			index > 1 ifTrue: [ stream nextPut: $, ].
			layout := segment at: 'layout'.
			stream
				nextPutAll: '{"index":';
				nextPutAll: (segment at: 'index') asString;
				nextPutAll: ',"byte_range":';
				nextPutAll: (self jsonByteRangeStart: (segment at: 'start') end: (segment at: 'end'));
				nextPutAll: ',"item_range":{"start":';
				nextPutAll: (segment at: 'item_start') asString;
				nextPutAll: ',"end":';
				nextPutAll: (segment at: 'item_end') asString;
				nextPutAll: '},"profile":"';
				nextPutAll: (segment at: 'profile');
				nextPutAll: '","head":';
				nextPutAll: (self nullableJsonString: (GtsHex encode: (segment at: 'head')));
				nextPutAll: ',"frame_count":';
				nextPutAll: (segment at: 'frame_count') asString;
				nextPutAll: ',"layout":{"claimed":';
				nextPutAll: (self boolJson: (layout at: 'claimed'));
				nextPutAll: ',"covered":';
				nextPutAll: (layout at: 'covered') asString;
				nextPutAll: ',"tail":';
				nextPutAll: (layout at: 'tail') asString;
				nextPutAll: ',"head":null},"diagnostics":[]}' ].
		stream
			nextPutAll: '],"item_count":';
			nextPutAll: (inventory at: 'item_count') asString;
			nextPutAll: ',"torn_at":';
			nextPutAll: (torn ifNil: [ 'null' ] ifNotNil: [ torn asString ]);
			nextPutAll: ',"fatal":null}';
			lf ]
]

{ #category : #inventory }
GtsReplication class >> problemDetailFor: inventory [
	(inventory at: 'torn') ifNotNil: [ :torn | ^ 'torn at offset ', torn asString ].
	(inventory at: 'segments') do: [ :segment |
		(segment at: 'diagnostics') isEmpty ifFalse: [
			| diagnostic |
			diagnostic := (segment at: 'diagnostics') first.
			^ diagnostic code, ': ', diagnostic detail ] ].
	^ 'input is not clean'
]
