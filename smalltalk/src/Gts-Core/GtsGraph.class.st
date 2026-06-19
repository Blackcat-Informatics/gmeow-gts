Class {
	#name : #GtsGraph,
	#superclass : #Object,
	#instVars : [
		'terms',
		'quads',
		'diagnostics',
		'profiles',
		'segmentHeads'
	],
	#category : #'Gts-Core'
}

{ #category : #adding }
GtsGraph >> addDiagnostic: aDiagnostic [
	diagnostics add: aDiagnostic
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
GtsGraph >> addTerm: aTerm [
	terms add: aTerm
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
	diagnostics := OrderedCollection new.
	profiles := OrderedCollection new.
	segmentHeads := OrderedCollection new
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
GtsGraph >> segmentHeads [
	^ segmentHeads asArray
]

{ #category : #accessing }
GtsGraph >> termAtId: termId [
	^ terms at: termId + 1
]

{ #category : #accessing }
GtsGraph >> terms [
	^ terms asArray
]
