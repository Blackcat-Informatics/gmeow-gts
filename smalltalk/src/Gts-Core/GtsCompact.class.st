Class {
	#name : #GtsCompact,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #private }
GtsCompact class >> addLiteral: value datatype: datatype toTerms: terms [
	| id |
	id := terms size.
	terms add: (GtsTerm literal: value language: nil datatype: datatype).
	^ id
]

{ #category : #private }
GtsCompact class >> addTerm: term toTerms: terms [
	| id |
	id := terms size.
	terms add: term.
	^ id
]

{ #category : #private }
GtsCompact class >> base64Url: bytes [
	| encoded |
	encoded := (Base64MimeConverter mimeEncode: bytes readStream multiLine: false) contents.
	encoded := encoded copyReplaceAll: '+' with: '-'.
	encoded := encoded copyReplaceAll: '/' with: '_'.
	[ encoded endsWith: '=' ] whileTrue: [ encoded := encoded allButLast ].
	^ encoded
]

{ #category : #private }
GtsCompact class >> blobOrderFor: graph sealedDigest: sealedDigest [
	| order |
	order := graph blobs keys asSortedCollection: [ :left :right |
		| leftSize rightSize |
		leftSize := (graph blobs at: left) size.
		rightSize := (graph blobs at: right) size.
		leftSize = rightSize
			ifTrue: [ left < right ]
			ifFalse: [ leftSize < rightSize ] ].
	sealedDigest ifNotNil: [
		order := order reject: [ :digest | digest = sealedDigest ].
		order := order copyWith: sealedDigest ].
	^ order asArray
]

{ #category : #converting }
GtsCompact class >> compactStreamable: data timestamp: timestamp sealOriginal: sealOriginal [
	| base blobOrder graph profile sealedDigest writer |
	graph := self verifiedGraphFor: data sealOriginal: sealOriginal.
	profile := graph profiles ifEmpty: [ 'generic' ] ifNotEmpty: [ graph profiles first ].
	sealOriginal ifTrue: [ sealedDigest := GtsWriter digestString: data ].
	blobOrder := self blobOrderFor: graph sealedDigest: sealedDigest.
	writer := GtsWriter forProfile: profile layout: 'streamable'.
	base := self
		writeStreamingIndexFor: graph
		blobOrder: blobOrder
		timestamp: timestamp
		sealedDigest: sealedDigest
		sealedSize: (sealOriginal ifTrue: [ data size ] ifFalse: [ nil ])
		on: writer.
	self writeGraph: graph shiftedBy: base on: writer.
	self writeBlobsFor: graph order: blobOrder sealedDigest: sealedDigest sourceBytes: data on: writer.
	writer addIndex.
	^ writer bytes
]

{ #category : #constants }
GtsCompact class >> rdfType [
	^ 'http://www.w3.org/1999/02/22-rdf-syntax-ns#type'
]

{ #category : #constants }
GtsCompact class >> streamNamespace [
	^ 'https://w3id.org/gts/stream#'
]

{ #category : #private }
GtsCompact class >> shiftSuppression: suppression by: base [
	| by reason targets |
	targets := (suppression at: 'targets' ifAbsent: [ #() ]) collect: [ :target |
		| copy kind q tid |
		copy := Dictionary new.
		target associationsDo: [ :association | copy at: association key put: association value ].
		kind := copy at: 'kind' ifAbsent: [ '' ].
		tid := copy at: 'id' ifAbsent: [ nil ].
		q := copy at: 'q' ifAbsent: [ nil ].
		((kind = 'term' or: [ kind = 'reifier' ]) and: [ tid isInteger ])
			ifTrue: [ copy at: 'id' put: tid + base ].
		(kind = 'quad' and: [ q isArray ])
			ifTrue: [ copy at: 'q' put: (q collect: [ :each | each isInteger ifTrue: [ each + base ] ifFalse: [ each ] ]) ].
		copy ].
	reason := suppression at: 'reason' ifAbsent: [ nil ].
	by := suppression at: 'by' ifAbsent: [ nil ].
	^ Dictionary newFromPairs: {
		'targets'. targets asArray.
		'reason'. reason.
		'by'. (by isInteger ifTrue: [ by + base ] ifFalse: [ by ]) }
]

{ #category : #private }
GtsCompact class >> shiftTerm: term by: base [
	term kind = #literal ifTrue: [
		^ GtsTerm
			literal: term value
			language: term language
			direction: term direction
			datatype: (term datatype ifNotNil: [ term datatype + base ]) ].
	term kind = #blankNode ifTrue: [ ^ GtsTerm blankNode: term value ].
	term kind = #triple ifTrue: [
		^ GtsTerm tripleReifier: (term reifier ifNotNil: [ term reifier + base ]) ].
	^ GtsTerm iri: term value
]

{ #category : #private }
GtsCompact class >> verifiedGraphFor: data sealOriginal: sealOriginal [
	| graph profiles |
	graph := GtsReader read: data.
	graph diagnostics isEmpty ifFalse: [
		self error: 'input is not a clean GTS file: ', graph diagnostics first code ].
	profiles := graph profiles asSet.
	profiles size > 1 ifTrue: [ self error: 'mixed segment profiles are not compactable (v1)' ].
	((profiles includes: 'evidence') and: [ sealOriginal not ])
		ifTrue: [
			self error: 'an evidence artifact''s signed chain is the artifact; refusing to re-order it without --seal-original' ].
	graph suppressions do: [ :suppression |
		(suppression at: 'targets' ifAbsent: [ #() ]) do: [ :target |
			(target at: 'kind' ifAbsent: [ '' ]) = 'frame'
				ifTrue: [ self error: 'input carries a frame-addressed suppression' ] ] ].
	^ graph
]

{ #category : #private }
GtsCompact class >> writeBlobsFor: graph order: blobOrder sealedDigest: sealedDigest sourceBytes: data on: writer [
	blobOrder do: [ :digest |
		digest = sealedDigest
			ifTrue: [
				writer
					addBlob: data
					mediaType: 'application/vnd.blackcat.gts+cbor-seq'
					representation: 'source' ]
			ifFalse: [
				| meta |
				meta := graph blobMeta at: digest ifAbsent: [ Dictionary new ].
				writer
					addBlob: (graph blobs at: digest)
					mediaType: (meta at: 'mt' ifAbsent: [ '' ])
					representation: (meta at: 'rep' ifAbsent: [ '' ]) ] ]
]

{ #category : #private }
GtsCompact class >> writeGraph: graph shiftedBy: base on: writer [
	graph terms isEmpty ifFalse: [
		writer addTerms: (graph terms collect: [ :term | self shiftTerm: term by: base ]) ].
	graph quads isEmpty ifFalse: [
		writer addQuads: (graph quads collect: [ :quad |
			quad collect: [ :termId | termId + base ] ]) ].
	graph reifiers isEmpty ifFalse: [
		writer addReifiers: (graph reifiers collect: [ :entry |
			Dictionary newFromPairs: {
				'rid'. (entry at: 'rid') + base.
				'spo'. ((entry at: 'spo') collect: [ :termId | termId + base ]) } ]) ].
	graph annotations isEmpty ifFalse: [
		writer addAnnotations: (graph annotations collect: [ :row |
			row collect: [ :termId | termId + base ] ]) ].
	graph suppressions do: [ :suppression |
		| shifted |
		shifted := self shiftSuppression: suppression by: base.
		writer
			addSuppressTargets: (shifted at: 'targets')
			reason: (shifted at: 'reason')
			by: (shifted at: 'by') ]
]

{ #category : #private }
GtsCompact class >> writeStreamingIndexFor: graph blobOrder: blobOrder timestamp: timestamp sealedDigest: sealedDigest sealedSize: sealedSize on: writer [
	| c terms quads tAgent tCompaction tCose tDetachedSignature tDigest tDt tInt tManifestation tMediaType tOrder tRole tSealedSource tSize tSourceFrame tSourceHead tTimestamp tType |
	terms := OrderedCollection new.
	quads := OrderedCollection new.
	tType := self addTerm: (GtsTerm iri: self rdfType) toTerms: terms.
	tInt := self addTerm: (GtsTerm iri: 'http://www.w3.org/2001/XMLSchema#integer') toTerms: terms.
	tDt := self addTerm: (GtsTerm iri: 'http://www.w3.org/2001/XMLSchema#dateTime') toTerms: terms.
	tManifestation := self addTerm: (GtsTerm iri: self streamNamespace, 'Manifestation') toTerms: terms.
	tDigest := self addTerm: (GtsTerm iri: self streamNamespace, 'digest') toTerms: terms.
	tMediaType := self addTerm: (GtsTerm iri: self streamNamespace, 'mediaType') toTerms: terms.
	tSize := self addTerm: (GtsTerm iri: self streamNamespace, 'size') toTerms: terms.
	tRole := self addTerm: (GtsTerm iri: self streamNamespace, 'role') toTerms: terms.
	tOrder := self addTerm: (GtsTerm iri: self streamNamespace, 'order') toTerms: terms.
	tCompaction := self addTerm: (GtsTerm iri: self streamNamespace, 'Compaction') toTerms: terms.
	tAgent := self addTerm: (GtsTerm iri: self streamNamespace, 'agent') toTerms: terms.
	tTimestamp := self addTerm: (GtsTerm iri: self streamNamespace, 'timestamp') toTerms: terms.
	tSourceHead := self addTerm: (GtsTerm iri: self streamNamespace, 'sourceHead') toTerms: terms.
	tSealedSource := self addTerm: (GtsTerm iri: self streamNamespace, 'sealedSource') toTerms: terms.
	tDetachedSignature := self addTerm: (GtsTerm iri: self streamNamespace, 'DetachedSignature') toTerms: terms.
	tSourceFrame := self addTerm: (GtsTerm iri: self streamNamespace, 'sourceFrame') toTerms: terms.
	tCose := self addTerm: (GtsTerm iri: self streamNamespace, 'cose') toTerms: terms.
	blobOrder withIndexDo: [ :digest :index |
		| isSealed m mediaType metadata order size |
		order := index - 1.
		isSealed := digest = sealedDigest.
		m := self addTerm: (GtsTerm blankNode: 'm', order asString) toTerms: terms.
		size := isSealed ifTrue: [ sealedSize ] ifFalse: [ (graph blobs at: digest) size ].
		metadata := graph blobMeta at: digest ifAbsent: [ Dictionary new ].
		mediaType := isSealed
			ifTrue: [ 'application/vnd.blackcat.gts+cbor-seq' ]
			ifFalse: [ metadata at: 'mt' ifAbsent: [ nil ] ].
		quads add: { m. tType. tManifestation }.
		quads add: { m. tDigest. (self addLiteral: digest datatype: nil toTerms: terms) }.
		mediaType ifNotNil: [ quads add: { m. tMediaType. (self addLiteral: mediaType datatype: nil toTerms: terms) } ].
		size ifNotNil: [ quads add: { m. tSize. (self addLiteral: size asString datatype: tInt toTerms: terms) } ].
		quads add: { m. tRole. (self addLiteral: (isSealed ifTrue: [ 'source' ] ifFalse: [ 'primary' ]) datatype: nil toTerms: terms) }.
		quads add: { m. tOrder. (self addLiteral: order asString datatype: tInt toTerms: terms) } ].
	c := self addTerm: (GtsTerm blankNode: 'c') toTerms: terms.
	quads add: { c. tType. tCompaction }.
	quads add: { c. tAgent. (self addLiteral: 'gts-compact' datatype: nil toTerms: terms) }.
	quads add: { c. tTimestamp. (self addLiteral: timestamp datatype: tDt toTerms: terms) }.
	graph segmentHeads do: [ :head |
		quads add: { c. tSourceHead. (self addLiteral: 'blake3:', (GtsHex encode: head) datatype: nil toTerms: terms) } ].
	sealedDigest ifNotNil: [
		quads add: { c. tSealedSource. (self addLiteral: sealedDigest datatype: nil toTerms: terms) } ].
	graph signatures withIndexDo: [ :signature :index |
		| node |
		node := self addTerm: (GtsTerm blankNode: 's', (index - 1) asString) toTerms: terms.
		quads add: { node. tType. tDetachedSignature }.
		quads add: { node. tSourceFrame. (self addLiteral: 'blake3:', (GtsHex encode: (signature at: 'frame_id')) datatype: nil toTerms: terms) }.
		quads add: { node. tCose. (self addLiteral: (self base64Url: (signature at: 'cose')) datatype: nil toTerms: terms) } ].
	writer addTerms: terms asArray.
	writer addQuads: quads asArray.
	^ terms size
]
