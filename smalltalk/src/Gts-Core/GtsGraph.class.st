Class {
	#name : #GtsGraph,
	#superclass : #Object,
	#instVars : [
		'terms',
		'quads',
		'reifiers',
		'annotations',
		'blobs',
		'blobMeta',
		'meta',
		'suppressions',
		'opaqueReasons',
		'diagnostics',
		'signatures',
		'profiles',
		'segmentHeads',
		'streamable'
	],
	#category : #'Gts-Core'
}

{ #category : #constants }
GtsGraph class >> rdfLangString [
	^ 'http://www.w3.org/1999/02/22-rdf-syntax-ns#langString'
]

{ #category : #constants }
GtsGraph class >> rdfDirLangString [
	^ 'http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString'
]

{ #category : #constants }
GtsGraph class >> xsdString [
	^ 'http://www.w3.org/2001/XMLSchema#string'
]

{ #category : #adding }
GtsGraph >> addAnnotation: triple [
	annotations add: triple
]

{ #category : #adding }
GtsGraph >> addBlob: data digest: digestString [
	blobs at: digestString put: data
]

{ #category : #adding }
GtsGraph >> addDiagnostic: aDiagnostic [
	diagnostics add: aDiagnostic
]

{ #category : #adding }
GtsGraph >> addOpaqueReason: reason [
	opaqueReasons add: reason
]

{ #category : #adding }
GtsGraph >> addProfile: profileName [
	profiles add: profileName
]

{ #category : #adding }
GtsGraph >> addQuad: aQuad [
	quads add: aQuad
]

{ #category : #adding }
GtsGraph >> addSegmentHead: segmentHead [
	segmentHeads add: segmentHead
]

{ #category : #adding }
GtsGraph >> addSignature: signature [
	signatures add: signature
]

{ #category : #adding }
GtsGraph >> addStreamableClaimed: claimed covered: covered tail: tail [
	^ self addStreamableClaimed: claimed covered: covered tail: tail head: nil
]

{ #category : #adding }
GtsGraph >> addStreamableClaimed: claimed covered: covered tail: tail head: head [
	streamable add: (Dictionary newFromPairs: {
		'claimed'. claimed.
		'covered'. covered.
		'tail'. tail.
		'head'. head })
]

{ #category : #adding }
GtsGraph >> addSuppression: suppression [
	suppressions add: suppression
]

{ #category : #adding }
GtsGraph >> addTerm: aTerm [
	terms add: aTerm
]

{ #category : #accessing }
GtsGraph >> annotations [
	^ annotations asArray
]

{ #category : #accessing }
GtsGraph >> blobMeta [
	^ blobMeta
]

{ #category : #accessing }
GtsGraph >> blobs [
	^ blobs
]

{ #category : #accessing }
GtsGraph >> datatypeIriForTerm: term [
	| datatypeId |
	datatypeId := term datatype.
	datatypeId ifNotNil: [
		((datatypeId >= 0) and: [ datatypeId < terms size ])
			ifTrue: [ ^ (self termAtId: datatypeId) value ].
		^ self class xsdString ].
	(term language notNil and: [ term language isEmpty not ])
		ifTrue: [
			(#('ltr' 'rtl') includes: term direction)
				ifTrue: [ ^ self class rdfDirLangString ].
			^ self class rdfLangString ].
	^ self class xsdString
]

{ #category : #accessing }
GtsGraph >> diagnostics [
	^ diagnostics asArray
]

{ #category : #initialization }
GtsGraph >> initialize [
	super initialize.
	terms := OrderedCollection new.
	quads := OrderedCollection new.
	reifiers := OrderedCollection new.
	annotations := OrderedCollection new.
	blobs := Dictionary new.
	blobMeta := Dictionary new.
	meta := Dictionary new.
	suppressions := OrderedCollection new.
	opaqueReasons := OrderedCollection new.
	diagnostics := OrderedCollection new.
	signatures := OrderedCollection new.
	profiles := OrderedCollection new.
	segmentHeads := OrderedCollection new.
	streamable := OrderedCollection new
]

{ #category : #accessing }
GtsGraph >> meta [
	^ meta
]

{ #category : #accessing }
GtsGraph >> opaqueReasons [
	^ opaqueReasons asArray
]

{ #category : #accessing }
GtsGraph >> profiles [
	^ profiles asArray
]

{ #category : #accessing }
GtsGraph >> quads [
	^ quads asArray
]

{ #category : #accessing }
GtsGraph >> reifierFor: reifierId [
	"The FIRST folded triple bound to reifierId, if any.

	rdf:reifies is not functional (§7.3), so this single-valued view exists only to
	resolve a 'tt'-less (legacy) triple term; use #reifierTriplesFor: for the full
	multi-valued statement layer."
	reifiers do: [ :entry |
		(entry at: 'rid') = reifierId ifTrue: [ ^ entry at: 'spo' ] ].
	^ nil
]

{ #category : #accessing }
GtsGraph >> reifierTriplesFor: reifierId [
	"Every DISTINCT triple bound to reifierId, in file order (§7.3)."
	| out |
	out := OrderedCollection new.
	reifiers do: [ :entry |
		| spo |
		(entry at: 'rid') = reifierId ifTrue: [
			spo := entry at: 'spo'.
			(out includes: spo) ifFalse: [ out add: spo ] ] ].
	^ out asArray
]

{ #category : #accessing }
GtsGraph >> tripleForTermId: termId [
	"The (s p o) a quoted-triple term denotes (§7.3).

	The term's own 'tt' is authoritative; a legacy 'tt'-less term falls back to the
	FIRST binding of its reifier so pre-'tt' files keep reading exactly as they did."
	| term |
	((termId isInteger) and: [ termId >= 0 and: [ termId < terms size ] ])
		ifFalse: [ ^ nil ].
	term := self termAtId: termId.
	term kind = #triple ifFalse: [ ^ nil ].
	term triple ifNotNil: [ :spo | ^ spo ].
	term reifier ifNotNil: [ :reifierId | ^ self reifierFor: reifierId ].
	^ nil
]

{ #category : #accessing }
GtsGraph >> reifiers [
	^ reifiers asArray
]

{ #category : #accessing }
GtsGraph >> segmentHeads [
	^ segmentHeads asArray
]

{ #category : #accessing }
GtsGraph >> signatures [
	^ signatures asArray
]

{ #category : #setting }
GtsGraph >> setBlobMeta: digestString metadata: metadataMap [
	blobMeta at: digestString put: metadataMap
]

{ #category : #setting }
GtsGraph >> setMeta: key value: value [
	meta at: key put: value
]

{ #category : #setting }
GtsGraph >> setReifier: reifierId to: spo [
	^ self setReifier: reifierId to: spo graph: nil
]

{ #category : #setting }
GtsGraph >> setReifier: reifierId to: spo graph: graphId [
	"Append a reifier row unless the identical row is already present.

	The reifies frame is a MULTI-VALUED statement layer: distinct triples on one
	reifier id all survive, each keeping its own graph slot. Only a byte-identical
	repeat collapses (§7.8 set semantics)."
	| entry |
	reifiers do: [ :existing |
		(((existing at: 'rid') = reifierId)
			and: [ (existing at: 'spo') = spo
			and: [ (existing at: 'graph' ifAbsent: [ nil ]) = graphId ] ])
			ifTrue: [ ^ self ] ].
	entry := Dictionary newFromPairs: {
		'rid'. reifierId.
		'spo'. spo }.
	graphId ifNotNil: [ entry at: 'graph' put: graphId ].
	reifiers add: entry
]

{ #category : #accessing }
GtsGraph >> streamable [
	^ streamable asArray
]

{ #category : #accessing }
GtsGraph >> suppressions [
	^ suppressions asArray
]

{ #category : #accessing }
GtsGraph >> termAtId: termId [
	^ terms at: termId + 1
]

{ #category : #accessing }
GtsGraph >> terms [
	^ terms asArray
]
