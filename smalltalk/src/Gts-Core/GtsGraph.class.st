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
GtsGraph >> addStreamableClaimed: claimed covered: covered tail: tail [
	streamable add: (Dictionary newFromPairs: {
		'claimed'. claimed.
		'covered'. covered.
		'tail'. tail })
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
		ifTrue: [ ^ self class rdfLangString ].
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
	reifiers do: [ :entry |
		(entry at: 'rid') = reifierId ifTrue: [ ^ entry at: 'spo' ] ].
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
	1 to: reifiers size do: [ :index |
		((reifiers at: index) at: 'rid') = reifierId
			ifTrue: [
				reifiers at: index put: (Dictionary newFromPairs: {
					'rid'. reifierId.
					'spo'. spo }).
				^ self ] ].
	reifiers add: (Dictionary newFromPairs: {
		'rid'. reifierId.
		'spo'. spo })
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
