Class {
	#name : #GtsReader,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #coercing }
GtsReader class >> asIndex: value [
	(value isInteger and: [ value >= 0 ]) ifTrue: [ ^ value ].
	^ nil
]

{ #category : #private }
GtsReader class >> blobDigestFor: data [
	^ GtsBlake3 digestString: data
]

{ #category : #codecs }
GtsReader class >> catalogFromHeader: header [
	| catalog rawCatalog |
	catalog := Dictionary new.
	rawCatalog := header at: 'cat' ifAbsent: [ ^ catalog ].
	rawCatalog isDictionary ifFalse: [ ^ catalog ].
	rawCatalog associationsDo: [ :association |
		association key isInteger ifTrue: [
			association value isDictionary ifTrue: [
				catalog at: association key put: association value ] ] ].
	^ catalog
]

{ #category : #private }
GtsReader class >> checkPositionsForQuad: row in: graph index: frameIndex [
	| graphId object predicate subject termCount valid |
	subject := row at: 1.
	predicate := row at: 2.
	object := row at: 3.
	graphId := row size >= 4 ifTrue: [ row at: 4 ] ifFalse: [ nil ].
	termCount := graph terms size.
	valid := subject < termCount
		and: [ predicate < termCount
		and: [ object < termCount
		and: [ graphId isNil or: [ graphId < termCount ] ] ] ].
	valid ifFalse: [
		graph addDiagnostic: (GtsDiagnostic
			code: 'PositionConstraint'
			detail: 'quad has out-of-range term ids'
			frameIndex: frameIndex).
		^ false ].
	valid := (graph termAtId: predicate) kind = #iri.
	(graph termAtId: subject) kind = #literal ifTrue: [ valid := false ].
	graphId ifNotNil: [
		| graphKind |
		graphKind := (graph termAtId: graphId) kind.
		((graphKind = #literal) or: [ graphKind = #triple ])
			ifTrue: [ valid := false ] ].
	valid ifFalse: [
		graph addDiagnostic: (GtsDiagnostic
			code: 'PositionConstraint'
			detail: 'quad violates term positions'
			frameIndex: frameIndex) ].
	^ valid
]

{ #category : #codecs }
GtsReader class >> decodeChain: codecs data: encodedData [
	| current |
	current := encodedData.
	codecs reverseDo: [ :codec |
		| codecClass codecName |
		codecClass := codec at: 'cls' ifAbsent: [ 'encode' ].
		codecClass = 'encrypt' ifTrue: [
			^ self payloadUnavailable: 'missing-key' code: 'MissingKey' detail: 'no key for encrypt codec' ].
		codecName := codec at: 'name' ifAbsent: [ '' ].
		codecName = 'gzip' ifTrue: [
			[ current := (GZipReadStream on: current readStream) upToEnd ]
				on: Error
				do: [ :error |
					^ self payloadDamaged: 'gzip decode failed: ', error messageText ] ].
		((codecName = 'zstd') or: [ codecName = 'zstd-rsyncable' ]) ifTrue: [
			[ current := GtsZstd decompress: current ]
				on: Error
				do: [ :error |
					^ self payloadDamaged: 'zstd decode failed: ', error messageText ] ].
		((codecName = 'identity')
			or: [ codecName = 'gzip'
			or: [ codecName = 'zstd'
			or: [ codecName = 'zstd-rsyncable' ] ] ]) ifFalse: [
			^ self
				payloadUnavailable: 'unknown-codec'
				code: 'UnknownCodec'
				detail: 'unknown codec ', codecName ] ].
	^ self payloadOk: current
]

{ #category : #folding }
GtsReader class >> foldAnnot: payload into: graph index: frameIndex [
	payload isArray ifTrue: [
		payload do: [ :row |
			| badIds object predicate reifier termCount |
			(row isArray and: [ row size = 3 ]) ifTrue: [
				reifier := self asIndex: (row at: 1).
				predicate := self asIndex: (row at: 2).
				object := self asIndex: (row at: 3).
				termCount := graph terms size.
				badIds := reifier isNil
					or: [ predicate isNil
					or: [ object isNil
					or: [ reifier >= termCount
					or: [ predicate >= termCount
					or: [ object >= termCount ] ] ] ] ].
				badIds
					ifTrue: [
						graph addDiagnostic: (GtsDiagnostic
							code: 'DamagedFrame'
							detail: 'annot row has bad/out-of-range ids'
							frameIndex: frameIndex) ]
					ifFalse: [
						(graph termAtId: predicate) kind = #iri
							ifFalse: [
								graph addDiagnostic: (GtsDiagnostic
									code: 'PositionConstraint'
									detail: 'annot predicate is not an IRI'
									frameIndex: frameIndex) ]
							ifTrue: [ graph addAnnotation: { reifier. predicate. object } ] ] ] ] ]
]

{ #category : #folding }
GtsReader class >> foldBlob: payload frame: frame into: graph index: frameIndex blobEvents: blobEvents described: described [
	| digest meta |
	(payload isKindOf: ByteArray) ifFalse: [ ^ self ].
	digest := self blobDigestFor: payload.
	meta := frame at: 'pub' ifAbsent: [ nil ].
	meta isDictionary ifTrue: [ graph setBlobMeta: digest metadata: meta ].
	graph addBlob: payload digest: digest.
	blobEvents add: (Dictionary newFromPairs: {
		'pos'. frameIndex.
		'digest'. digest.
		'described'. (described includesKey: digest) })
]

{ #category : #folding }
GtsReader class >> foldFrame: frame index: frameIndex into: graph catalog: catalog indexRecords: indexRecords blobEvents: blobEvents described: described [
	| frameType payloadResult payload |
	frameType := frame at: 't' ifAbsent: [ '' ].
	payloadResult := self payloadForFrame: frame catalog: catalog isBlob: frameType = 'blob'.
	(payloadResult at: 'ok') ifFalse: [
		graph addOpaqueReason: (payloadResult at: 'reason').
		graph addDiagnostic: (GtsDiagnostic
			code: (payloadResult at: 'code')
			detail: (payloadResult at: 'detail')
			frameIndex: frameIndex).
		^ self ].
	payload := payloadResult at: 'payload'.
	frameType = 'terms' ifTrue: [ ^ self foldTerms: payload into: graph index: frameIndex ].
	frameType = 'quads' ifTrue: [ ^ self foldQuads: payload into: graph index: frameIndex described: described ].
	frameType = 'reifies' ifTrue: [ ^ self foldReifies: payload into: graph index: frameIndex ].
	frameType = 'annot' ifTrue: [ ^ self foldAnnot: payload into: graph index: frameIndex ].
	frameType = 'blob' ifTrue: [
		^ self
			foldBlob: payload
			frame: frame
			into: graph
			index: frameIndex
			blobEvents: blobEvents
			described: described ].
	frameType = 'meta' ifTrue: [ ^ self foldMeta: payload into: graph ].
	frameType = 'suppress' ifTrue: [ ^ self foldSuppress: payload into: graph ].
	frameType = 'snapshot' ifTrue: [
		^ self
			foldSnapshot: payload
			into: graph
			index: frameIndex
			blobEvents: blobEvents
			described: described ].
	frameType = 'index' ifTrue: [ ^ self foldIndex: payload index: frameIndex into: indexRecords ].
	frameType = 'opaque' ifTrue: [ ^ self foldOpaque: payload into: graph ].
	graph addOpaqueReason: 'unknown-frame-type'.
	graph addDiagnostic: (GtsDiagnostic
		code: 'UnknownFrameType'
		detail: 'unsupported frame type'
		frameIndex: frameIndex)
]

{ #category : #folding }
GtsReader class >> foldIndex: payload index: frameIndex into: indexRecords [
	| count head |
	payload isDictionary ifFalse: [ ^ self ].
	count := self asIndex: (payload at: 'count' ifAbsent: [ nil ]).
	head := payload at: 'head' ifAbsent: [ nil ].
	(count notNil and: [ head isKindOf: ByteArray ])
		ifTrue: [
			indexRecords add: (Dictionary newFromPairs: {
				'pos'. frameIndex.
				'count'. count.
				'head'. head }) ]
]

{ #category : #folding }
GtsReader class >> foldMeta: payload into: graph [
	payload isDictionary ifFalse: [ ^ self ].
	payload associationsDo: [ :association |
		graph setMeta: association key asString value: association value ]
]

{ #category : #folding }
GtsReader class >> foldOpaque: payload into: graph [
	payload isDictionary ifFalse: [ ^ self ].
	graph addOpaqueReason: (payload at: 'reason' ifAbsent: [ 'unknown-codec' ])
]

{ #category : #folding }
GtsReader class >> foldQuads: rows into: graph index: frameIndex described: described [
	rows isArray ifFalse: [ ^ self ].
	rows do: [ :row |
		| object predicate subject graphId quad |
		(row isArray and: [ row size >= 3 ]) ifTrue: [
			subject := self asIndex: (row at: 1).
			predicate := self asIndex: (row at: 2).
			object := self asIndex: (row at: 3).
			graphId := row size >= 4 ifTrue: [ self asIndex: (row at: 4) ] ifFalse: [ nil ].
			(subject isNil
				or: [ predicate isNil
				or: [ object isNil
				or: [ row size >= 4 and: [ graphId isNil ] ] ] ])
				ifTrue: [
					graph addDiagnostic: (GtsDiagnostic
						code: 'DamagedFrame'
						detail: 'quad has non-integer term ids'
						frameIndex: frameIndex) ]
				ifFalse: [
					quad := graphId
						ifNil: [ { subject. predicate. object } ]
						ifNotNil: [ { subject. predicate. object. graphId } ].
					(self checkPositionsForQuad: quad in: graph index: frameIndex)
						ifTrue: [
							graph addQuad: quad.
							((graph termAtId: predicate) value = 'https://w3id.org/gts/stream#digest')
								ifTrue: [
									| digestValue |
									digestValue := (graph termAtId: object) value.
									(digestValue notNil and: [ digestValue isEmpty not ])
										ifTrue: [ described at: digestValue put: true ] ] ] ] ] ]
]

{ #category : #folding }
GtsReader class >> foldReifies: payload into: graph index: frameIndex [
	payload isDictionary ifFalse: [ ^ self ].
	payload associationsDo: [ :association |
		| existing object predicate reifier spo subject termCount |
		reifier := self asIndex: association key.
		spo := association value.
		(spo isArray and: [ spo size = 3 ]) ifTrue: [
			subject := self asIndex: (spo at: 1).
			predicate := self asIndex: (spo at: 2).
			object := self asIndex: (spo at: 3).
			termCount := graph terms size.
			(reifier isNil
				or: [ subject isNil
				or: [ predicate isNil
				or: [ object isNil
				or: [ reifier >= termCount
				or: [ subject >= termCount
				or: [ predicate >= termCount
				or: [ object >= termCount ] ] ] ] ] ] ])
				ifTrue: [
					graph addDiagnostic: (GtsDiagnostic
						code: 'DamagedFrame'
						detail: 'reifier has bad/out-of-range ids'
						frameIndex: frameIndex) ]
				ifFalse: [
					existing := graph reifierFor: reifier.
					(existing notNil and: [ existing ~= { subject. predicate. object } ])
						ifTrue: [
							graph addDiagnostic: (GtsDiagnostic
								code: 'ConflictingReifier'
								detail: 'reifier rebound'
								frameIndex: frameIndex) ]
						ifFalse: [ graph setReifier: reifier to: { subject. predicate. object } ] ] ] ]
]

{ #category : #folding }
GtsReader class >> foldSnapshot: payload into: graph index: frameIndex blobEvents: blobEvents described: described [
	| base |
	payload isDictionary ifFalse: [ ^ self ].
	base := graph terms size.
	(payload at: 'terms' ifAbsent: [ nil ]) ifNotNil: [ :termsPayload |
		termsPayload isArray ifTrue: [
			self foldTerms: (termsPayload collect: [ :termMap | self shiftedTermMap: termMap by: base ])
				into: graph
				index: frameIndex ] ].
	(payload at: 'quads' ifAbsent: [ nil ]) ifNotNil: [ :quadsPayload |
		quadsPayload isArray ifTrue: [
			self foldQuads: (quadsPayload collect: [ :row | self shiftedRow: row by: base ])
				into: graph
				index: frameIndex
				described: described ] ].
	(payload at: 'reifies' ifAbsent: [ nil ]) ifNotNil: [ :reifiesPayload |
		reifiesPayload isDictionary ifTrue: [
			| shifted |
			shifted := Dictionary new.
			reifiesPayload associationsDo: [ :association |
				shifted at: (self shiftedValue: association key by: base)
					put: (self shiftedRow: association value by: base) ].
			self foldReifies: shifted into: graph index: frameIndex ] ].
	(payload at: 'annot' ifAbsent: [ nil ]) ifNotNil: [ :annotPayload |
		annotPayload isArray ifTrue: [
			self foldAnnot: (annotPayload collect: [ :row | self shiftedRow: row by: base ])
				into: graph
				index: frameIndex ] ].
	(payload at: 'blobs' ifAbsent: [ nil ]) ifNotNil: [ :blobPayload |
		blobPayload isDictionary ifTrue: [
			blobPayload valuesDo: [ :data |
				(data isKindOf: ByteArray) ifTrue: [
					graph addBlob: data digest: (self blobDigestFor: data) ] ] ] ].
	(payload at: 'meta' ifAbsent: [ nil ]) ifNotNil: [ :metaPayload |
		self foldMeta: metaPayload into: graph ]
]

{ #category : #folding }
GtsReader class >> foldSuppress: payload into: graph [
	| targets |
	payload isDictionary ifFalse: [ ^ self ].
	targets := payload at: 'targets' ifAbsent: [ #() ].
	targets isArray ifFalse: [ targets := #() ].
	graph addSuppression: (Dictionary newFromPairs: {
		'targets'. (targets select: [ :each | each isDictionary ]).
		'reason'. (payload at: 'reason' ifAbsent: [ '' ]).
		'by'. (payload at: 'by' ifAbsent: [ nil ]) })
]

{ #category : #folding }
GtsReader class >> foldTerm: termMap into: graph index: frameIndex [
	| datatype datatypeRaw kind language reifier reifierRaw termId value |
	termMap isDictionary ifFalse: [ ^ self ].
	kind := termMap at: 'k' ifAbsent: [ 0 ].
	value := termMap at: 'v' ifAbsent: [ '' ].
	language := termMap at: 'l' ifAbsent: [ nil ].
	termId := graph terms size.
	datatypeRaw := termMap at: 'dt' ifAbsent: [ nil ].
	reifierRaw := termMap at: 'rf' ifAbsent: [ nil ].
	datatype := self sanitizeTermReference: datatypeRaw before: termId index: frameIndex graph: graph.
	reifier := self sanitizeTermReference: reifierRaw before: termId index: frameIndex graph: graph.
	kind = 1 ifTrue: [
		^ graph addTerm: (GtsTerm literal: value language: language datatype: datatype) ].
	kind = 2 ifTrue: [
		^ graph addTerm: (GtsTerm blankNode: value) ].
	kind = 3 ifTrue: [
		^ graph addTerm: (GtsTerm tripleReifier: reifier) ].
	graph addTerm: (GtsTerm iri: value)
]

{ #category : #folding }
GtsReader class >> foldTerms: termRows into: graph index: frameIndex [
	termRows isArray ifFalse: [ ^ self ].
	termRows do: [ :termMap | self foldTerm: termMap into: graph index: frameIndex ]
]

{ #category : #segments }
GtsReader class >> isHeaderItem: item [
	| header |
	[ header := GtsWire unwrapHeader: item ] on: Error do: [ :error | ^ false ].
	header isDictionary ifFalse: [ ^ false ].
	^ (header includesKey: 'gts') and: [ (header includesKey: 't') not ]
]

{ #category : #segments }
GtsReader class >> itemResultFrom: aByteArray [
	| items reader torn |
	reader := GtsCborReader on: aByteArray.
	items := OrderedCollection new.
	torn := false.
	[ reader atEnd ] whileFalse: [
		[ items add: reader readItem ]
			on: Error
			do: [ :error |
				torn := true.
				reader skipToEnd ] ].
	^ Dictionary newFromPairs: {
		'items'. items asArray.
		'torn'. torn }
]

{ #category : #segments }
GtsReader class >> layoutCheckHeader: header graph: graph indexRecords: indexRecords blobEvents: blobEvents frameIds: frameIds indexOffset: indexOffset [
	| claimed count head last relPos tail total |
	claimed := (header at: 'layout' ifAbsent: [ '' ]) = 'streamable'.
	total := frameIds size.
	claimed ifFalse: [
		graph addStreamableClaimed: false covered: 0 tail: 0.
		^ self ].
	indexRecords isEmpty ifTrue: [
		graph addDiagnostic: (GtsDiagnostic
			code: 'StreamableLayoutError'
			detail: 'streamable layout has no index footer'
			frameIndex: nil).
		graph addStreamableClaimed: true covered: 0 tail: total.
		^ self ].
	last := indexRecords last.
	count := last at: 'count'.
	head := last at: 'head'.
	relPos := (last at: 'pos') - indexOffset.
	tail := total - relPos.
	((count ~= (relPos - 1))
		or: [ count < 1
		or: [ (frameIds at: count ifAbsent: [ nil ]) ~= head ] ])
		ifTrue: [
			graph addDiagnostic: (GtsDiagnostic
				code: 'StreamableLayoutError'
				detail: 'index footer contradicts covered frames'
				frameIndex: (last at: 'pos')) ].
	blobEvents do: [ :event |
		(((event at: 'pos') - indexOffset) <= count
			and: [ (event at: 'described') not ])
			ifTrue: [
				graph addDiagnostic: (GtsDiagnostic
					code: 'StreamableLayoutError'
					detail: 'covered blob was not described before payload'
					frameIndex: (event at: 'pos')) ] ].
	graph addStreamableClaimed: true covered: count tail: tail head: head
]

{ #category : #union }
GtsReader class >> mapTerm: termId from: segment segmentIndex: segmentIndex into: output intern: intern [
	| datatype key mapped reifier term value |
	key := self termKeyFor: termId in: segment segmentIndex: segmentIndex output: output intern: intern.
	(intern includesKey: key) ifTrue: [ ^ intern at: key ].
	term := segment termAtId: termId.
	datatype := term datatype ifNotNil: [
		self
			mapTerm: term datatype
			from: segment
			segmentIndex: segmentIndex
			into: output
			intern: intern ].
	reifier := term reifier ifNotNil: [
		self
			mapTerm: term reifier
			from: segment
			segmentIndex: segmentIndex
			into: output
			intern: intern ].
	value := term value.
	term kind = #blankNode ifTrue: [
		(value notNil and: [ value isEmpty not ])
			ifTrue: [ value := 's', segmentIndex asString, '.', value ]
			ifFalse: [ value := 's', segmentIndex asString, '._anon', output terms size asString ] ].
	term kind = #literal
		ifTrue: [ output addTerm: (GtsTerm literal: value language: term language datatype: datatype) ]
		ifFalse: [
			term kind = #blankNode
				ifTrue: [ output addTerm: (GtsTerm blankNode: value) ]
				ifFalse: [
					term kind = #triple
						ifTrue: [ output addTerm: (GtsTerm tripleReifier: reifier) ]
						ifFalse: [ output addTerm: (GtsTerm iri: value) ] ] ].
	mapped := output terms size - 1.
	intern at: key put: mapped.
	^ mapped
]

{ #category : #payloads }
GtsReader class >> payloadDamaged: detail [
	^ Dictionary newFromPairs: {
		'ok'. false.
		'reason'. 'damaged'.
		'code'. 'DamagedFrame'.
		'detail'. detail }
]

{ #category : #payloads }
GtsReader class >> payloadForFrame: frame catalog: catalog isBlob: isBlob [
	| chainResult data decodedResult transforms |
	data := frame at: 'd' ifAbsent: [ nil ].
	transforms := frame at: 'x' ifAbsent: [ nil ].
	(transforms isNil or: [ transforms isArray and: [ transforms isEmpty ] ])
		ifTrue: [ ^ self payloadOk: data ].
	transforms isArray ifFalse: [
		^ self payloadDamaged: 'transform chain must be an array' ].
	(data isKindOf: ByteArray) ifFalse: [
		^ self payloadDamaged: 'transformed frame d must be a byte string' ].
	chainResult := self resolveCodecs: transforms catalog: catalog.
	(chainResult at: 'ok') ifFalse: [ ^ chainResult ].
	decodedResult := self decodeChain: (chainResult at: 'codecs') data: data.
	(decodedResult at: 'ok') ifFalse: [ ^ decodedResult ].
	isBlob ifTrue: [ ^ decodedResult ].
	[ ^ self payloadOk: (GtsCborReader decode: (decodedResult at: 'payload')) ]
		on: Error
		do: [ :error |
			^ self payloadDamaged: 'payload decode failed: ', error messageText ]
]

{ #category : #payloads }
GtsReader class >> payloadOk: payload [
	^ Dictionary newFromPairs: {
		'ok'. true.
		'payload'. payload }
]

{ #category : #payloads }
GtsReader class >> payloadUnavailable: reason code: diagnosticCode detail: detail [
	^ Dictionary newFromPairs: {
		'ok'. false.
		'reason'. reason.
		'code'. diagnosticCode.
		'detail'. detail }
]

{ #category : #reading }
GtsReader class >> read: aByteArray [
	^ self read: aByteArray allowSegments: true
]

{ #category : #reading }
GtsReader class >> read: aByteArray allowSegments: allowSegments [
	| bounds firstBoundary graph itemResult items torn |
	itemResult := self itemResultFrom: aByteArray.
	items := itemResult at: 'items'.
	torn := itemResult at: 'torn'.
	items isEmpty ifTrue: [
		graph := GtsGraph new.
		graph addDiagnostic: (GtsDiagnostic code: 'EmptyFile' detail: 'no CBOR items' frameIndex: 0).
		^ graph ].
	bounds := OrderedCollection new.
	1 to: items size do: [ :index |
		(self isHeaderItem: (items at: index))
			ifTrue: [ bounds add: index ] ].
	(bounds isEmpty or: [ bounds first ~= 1 ]) ifTrue: [
		graph := GtsGraph new.
		graph addDiagnostic: (GtsDiagnostic code: 'DamagedFrame' detail: 'first item is not a header' frameIndex: 0).
		^ graph ].
	((bounds size > 1) and: [ allowSegments not ]) ifTrue: [
		firstBoundary := bounds second.
		graph := self readSegment: (items copyFrom: 1 to: firstBoundary - 1) indexOffset: 0.
		graph addDiagnostic: (GtsDiagnostic
			code: 'SegmentBoundary'
			detail: 'segment boundary but reader is in pre-segment mode'
			frameIndex: firstBoundary - 1).
		^ graph ].
	graph := self readSegments: items bounds: bounds.
	torn ifTrue: [
		graph addDiagnostic: (GtsDiagnostic code: 'TornAppendError' detail: 'torn append' frameIndex: nil) ].
	^ graph
]

{ #category : #segments }
GtsReader class >> readSegment: items indexOffset: indexOffset [
	| blobEvents catalog described expectedPrev frameIds graph header indexRecords lastHead storedHeaderId |
	graph := GtsGraph new.
	[ header := GtsWire unwrapHeader: items first ]
		on: Error
		do: [ :error |
			graph addDiagnostic: (GtsDiagnostic code: 'DamagedFrame' detail: 'invalid header' frameIndex: indexOffset).
			^ graph ].
	header isDictionary ifFalse: [
		graph addDiagnostic: (GtsDiagnostic code: 'DamagedFrame' detail: 'invalid header' frameIndex: indexOffset).
		^ graph ].
	storedHeaderId := header at: 'id' ifAbsent: [ nil ].
	storedHeaderId = (GtsWire headerIdFor: header)
		ifFalse: [
			graph addDiagnostic: (GtsDiagnostic code: 'DamagedFrame' detail: 'header self-hash mismatch' frameIndex: indexOffset) ].
	(((header at: 'gts' ifAbsent: [ '' ]) = 'GTS1')
		and: [ (header at: 'v' ifAbsent: [ nil ]) = 1 ])
		ifFalse: [
			graph addDiagnostic: (GtsDiagnostic
				code: 'DamagedFrame'
				detail: 'unsupported header magic/version'
				frameIndex: indexOffset) ].
	graph addProfile: (header at: 'prof' ifAbsent: [ 'generic' ]).
	expectedPrev := storedHeaderId.
	lastHead := expectedPrev.
	catalog := self catalogFromHeader: header.
	indexRecords := OrderedCollection new.
	blobEvents := OrderedCollection new.
	described := Dictionary new.
	frameIds := OrderedCollection new.
	2 to: items size do: [ :itemIndex |
		| absIndex computed frame storedId |
		absIndex := indexOffset + itemIndex - 1.
		frame := items at: itemIndex.
		frame isDictionary ifFalse: [
			graph addDiagnostic: (GtsDiagnostic code: 'DamagedFrame' detail: 'frame is not a map' frameIndex: absIndex).
			frameIds add: ByteArray new ]
		ifTrue: [
			storedId := frame at: 'id' ifAbsent: [ nil ].
			computed := GtsWire contentIdForFrame: frame.
			storedId = computed
				ifFalse: [
					graph addDiagnostic: (GtsDiagnostic code: 'DamagedFrame' detail: 'frame self-hash mismatch' frameIndex: absIndex).
					graph addOpaqueReason: 'damaged'.
					expectedPrev := storedId ifNil: [ computed ].
					lastHead := expectedPrev.
					frameIds add: expectedPrev ]
				ifTrue: [
					(frame at: 'prev' ifAbsent: [ nil ]) = expectedPrev
						ifFalse: [
							graph addDiagnostic: (GtsDiagnostic code: 'BrokenChain' detail: 'prev does not match' frameIndex: absIndex) ].
					expectedPrev := computed.
					lastHead := computed.
					frameIds add: computed.
					self retainSignatureFrom: frame frameId: computed into: graph.
					self
						foldFrame: frame
						index: absIndex
						into: graph
						catalog: catalog
						indexRecords: indexRecords
						blobEvents: blobEvents
						described: described ] ] ].
	graph addSegmentHead: lastHead.
	self
		layoutCheckHeader: header
		graph: graph
		indexRecords: indexRecords
		blobEvents: blobEvents
		frameIds: frameIds asArray
		indexOffset: indexOffset.
	^ graph
]

{ #category : #folding }
GtsReader class >> retainSignatureFrom: frame frameId: frameId into: graph [
	| kid parsed signature |
	signature := frame at: 'sig' ifAbsent: [ nil ].
	(signature isKindOf: ByteArray) ifFalse: [ ^ self ].
	kid := nil.
	[ parsed := GtsCose parseSign1: signature.
	kid := parsed at: 'kid' ifAbsent: [ nil ] ]
		on: Error
		do: [ :error | kid := nil ].
	graph addSignature: (Dictionary newFromPairs: {
		'frame_id'. frameId.
		'kid'. kid.
		'status'. 'unverified'.
		'cose'. signature })
]

{ #category : #segments }
GtsReader class >> readSegments: items bounds: bounds [
	| segments |
	segments := OrderedCollection new.
	1 to: bounds size do: [ :index |
		| start stop |
		start := bounds at: index.
		stop := index < bounds size
			ifTrue: [ (bounds at: index + 1) - 1 ]
			ifFalse: [ items size ].
		segments add: (self
			readSegment: (items copyFrom: start to: stop)
			indexOffset: start - 1) ].
	segments size = 1 ifTrue: [ ^ segments first ].
	^ self unionSegments: segments asArray
]

{ #category : #codecs }
GtsReader class >> resolveCodecs: transformIds catalog: catalog [
	| codecs |
	codecs := OrderedCollection new.
	transformIds do: [ :transformId |
		| codec |
		transformId isInteger ifFalse: [
			^ self
				payloadUnavailable: 'unknown-codec'
				code: 'UnknownCodec'
				detail: 'codec id is not an integer' ].
		codec := catalog at: transformId ifAbsent: [
			^ self
				payloadUnavailable: 'unknown-codec'
				code: 'UnknownCodec'
				detail: 'codec id not in catalog' ].
		codec isDictionary ifFalse: [
			^ self
				payloadUnavailable: 'unknown-codec'
				code: 'UnknownCodec'
				detail: 'codec catalog entry is not a map' ].
		codecs add: codec ].
	^ Dictionary newFromPairs: {
		'ok'. true.
		'codecs'. codecs asArray }
]

{ #category : #folding }
GtsReader class >> sanitizeTermReference: value before: termId index: frameIndex graph: graph [
	value ifNil: [ ^ nil ].
	(value isInteger and: [ value >= 0 and: [ value < termId ] ])
		ifTrue: [ ^ value ].
	(value isInteger and: [ value >= termId ])
		ifTrue: [
			graph addDiagnostic: (GtsDiagnostic
				code: 'ForwardReference'
				detail: 'term has an out-of-range ref'
				frameIndex: frameIndex) ].
	^ nil
]

{ #category : #folding }
GtsReader class >> shiftedRow: row by: base [
	row isArray ifFalse: [ ^ row ].
	^ row collect: [ :each | self shiftedValue: each by: base ]
]

{ #category : #folding }
GtsReader class >> shiftedTermMap: termMap by: base [
	| shifted |
	termMap isDictionary ifFalse: [ ^ termMap ].
	shifted := Dictionary new.
	termMap associationsDo: [ :association |
		((association key = 'dt') or: [ association key = 'rf' ])
			ifTrue: [ shifted at: association key put: (self shiftedValue: association value by: base) ]
			ifFalse: [ shifted at: association key put: association value ] ].
	^ shifted
]

{ #category : #folding }
GtsReader class >> shiftedValue: value by: base [
	value isInteger ifTrue: [ ^ value + base ].
	^ value
]

{ #category : #union }
GtsReader class >> termKeyFor: termId in: segment segmentIndex: segmentIndex output: output intern: intern [
	| datatype term |
	term := segment termAtId: termId.
	term kind = #iri ifTrue: [ ^ 'iri|', term value ].
	term kind = #literal ifTrue: [
		datatype := segment datatypeIriForTerm: term.
		^ 'lit|', term value, '|', datatype, '|', (term language ifNil: [ '' ]) ].
	term kind = #blankNode ifTrue: [
		(term value notNil and: [ term value isEmpty not ])
			ifTrue: [ ^ 'bnode|', segmentIndex asString, '|', term value ].
		^ 'bnode-anon|', segmentIndex asString, '|', termId asString ].
	term kind = #triple ifTrue: [
		^ 'triple|', (term reifier ifNil: [ -1 ]) asString ].
	^ 'unknown|', termId asString
]

{ #category : #union }
GtsReader class >> unionSegments: segments [
	| intern output seenQuads |
	output := GtsGraph new.
	intern := Dictionary new.
	seenQuads := Dictionary new.
	segments withIndexDo: [ :segment :segmentIndexOneBased |
		| segmentIndex |
		segmentIndex := segmentIndexOneBased - 1.
		segment quads do: [ :quad |
			| mapped |
			mapped := quad collect: [ :termId |
				self
					mapTerm: termId
					from: segment
					segmentIndex: segmentIndex
					into: output
					intern: intern ].
			(seenQuads includesKey: mapped) ifFalse: [
				seenQuads at: mapped put: true.
				output addQuad: mapped ] ].
		segment reifiers do: [ :entry |
			output
				setReifier: (self mapTerm: (entry at: 'rid') from: segment segmentIndex: segmentIndex into: output intern: intern)
				to: ((entry at: 'spo') collect: [ :termId |
					self mapTerm: termId from: segment segmentIndex: segmentIndex into: output intern: intern ]) ].
		segment annotations do: [ :annotation |
			output addAnnotation: (annotation collect: [ :termId |
				self mapTerm: termId from: segment segmentIndex: segmentIndex into: output intern: intern ]) ].
		segment blobs associationsDo: [ :association |
			output addBlob: association value digest: association key ].
		segment blobMeta associationsDo: [ :association |
			output setBlobMeta: association key metadata: association value ].
		segment meta associationsDo: [ :association |
			output setMeta: association key value: association value ].
		segment suppressions do: [ :suppression | output addSuppression: suppression ].
		segment opaqueReasons do: [ :reason | output addOpaqueReason: reason ].
		segment diagnostics do: [ :diagnostic | output addDiagnostic: diagnostic ].
		segment signatures do: [ :signature | output addSignature: signature ].
		segment segmentHeads do: [ :head | output addSegmentHead: head ].
		segment profiles do: [ :profile | output addProfile: profile ].
		segment streamable do: [ :layout |
			output
				addStreamableClaimed: (layout at: 'claimed')
				covered: (layout at: 'covered')
				tail: (layout at: 'tail') ] ].
	^ output
]
