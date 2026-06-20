Class {
	#name : #GtsCliParityTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsCliParityTest >> vectorBytes: name [
	^ ('/workspace/vectors/', name) asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #tests }
GtsCliParityTest >> testAllQuadsSuppressedDetectsHiddenOutput [
	| graph |
	graph := GtsGraph new.
	graph addQuad: #(0 1 2).
	graph addSuppression: (Dictionary newFromPairs: {
		'targets'. { Dictionary newFromPairs: {
			'kind'. 'quad'.
			'q'. #(0 1 2) } } }).
	self assert: (GtsCLI allQuadsSuppressed: graph)
]

{ #category : #tests }
GtsCliParityTest >> testCatRequiresAtLeastTwoInputs [
	self
		assert: (GtsCLI runCat: {
			'-o'.
			'/tmp/gts-smalltalk-cat-single.gts'.
			'/workspace/vectors/01-minimal.gts' })
		equals: 2
]

{ #category : #tests }
GtsCliParityTest >> testProfileFindingReportsFilesVocabularyWithoutDeclaration [
	| findings graph |
	graph := GtsGraph new.
	graph addProfile: 'generic'.
	graph addTerm: (GtsTerm iri: GtsFiles filesNamespace, 'FileEntry').
	findings := GtsCLI profileFindingsFor: graph.
	self assert: findings size equals: 1.
	self assert: ((findings first at: 'severity') = 'error')
]

{ #category : #tests }
GtsCliParityTest >> testReplicationDirtyInventoryReportsUnclean [
	| inventory result |
	inventory := GtsReplication inventoryFor: (self vectorBytes: '05-torn-append.gts').
	self assert: (GtsReplication hasProblems: inventory).
	self assert: ((GtsReplication headsJsonFor: inventory) includesSubstring: '"clean":false').
	result := GtsReplication
		missingFromHead: (GtsHex decode: (String new: 64 withAll: $0))
		in: inventory.
	self assert: (result at: 'status') equals: 'error'
]

{ #category : #tests }
GtsCliParityTest >> testUnpackSkipsSuppressedBlobByDefault [
	| archive digest graph root suppressedTarget withSuppressed withoutSuppressed |
	root := '/tmp/gts-smalltalk-suppressed-source' asFileReference.
	withoutSuppressed := '/tmp/gts-smalltalk-suppressed-off' asFileReference.
	withSuppressed := '/tmp/gts-smalltalk-suppressed-on' asFileReference.
	{
		root.
		withoutSuppressed.
		withSuppressed } do: [ :dir | dir exists ifTrue: [ dir ensureDeleteAll ] ].
	[
		root ensureCreateDirectory.
		(root / 'a.txt') writeStreamDo: [ :stream | stream nextPutAll: 'suppressed payload'; lf ].
		archive := GtsFiles packSources: { root fullName }.
		graph := GtsReader read: archive.
		digest := graph blobs keys asArray first.
		suppressedTarget := Dictionary newFromPairs: {
			'kind'. 'blob'.
			'digest'. digest }.
		graph addSuppression: (Dictionary newFromPairs: {
			'targets'. { suppressedTarget } }).
		GtsFiles unpackGraph: graph to: withoutSuppressed fullName includeSuppressed: false.
		self deny: (withoutSuppressed / 'a.txt') exists.
		GtsFiles unpackGraph: graph to: withSuppressed fullName includeSuppressed: true.
		self assert: (withSuppressed / 'a.txt') exists ]
		ensure: [
			{
				root.
				withoutSuppressed.
				withSuppressed } do: [ :dir | dir exists ifTrue: [ dir ensureDeleteAll ] ] ]
]
