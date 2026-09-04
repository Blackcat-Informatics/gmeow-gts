Class {
	#name : #GtsReaderTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #private }
GtsReaderTest >> readMinimalVectorBytes [
	^ '/workspace/vectors/01-minimal.gts' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #private }
GtsReaderTest >> readZstdVectorBytes [
	^ '/workspace/vectors/02-zstd-frame.gts' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #private }
GtsReaderTest >> diagnosticCodesFor: aByteArray [
	^ (GtsReader read: aByteArray) diagnostics collect: [ :diagnostic | diagnostic code ]
]

{ #category : #private }
GtsReaderTest >> codesOf: graph [
	^ graph diagnostics collect: [ :diagnostic | diagnostic code ]
]

{ #category : #private }
GtsReaderTest >> statementLayerTerms [
	"Terms 0..4 shared by the §7.3 statement-layer tests: r, s, p, 'Cat'@en, 'Chat'@fr."
	^ {
		GtsTerm iri: 'https://ex/r'.
		GtsTerm iri: 'https://ex/s'.
		GtsTerm iri: 'https://ex/p'.
		GtsTerm literal: 'Cat' language: 'en' datatype: nil.
		GtsTerm literal: 'Chat' language: 'fr' datatype: nil }
]

{ #category : #tests }
GtsReaderTest >> testMultiValuedReifiesKeepsEveryRowWithoutDiagnostic [
	| graph writer |
	writer := GtsWriter forProfile: 'generic'.
	writer addTerms: self statementLayerTerms.
	writer addReifiers: {
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 3) }.
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 4) } }.
	graph := GtsReader read: writer bytes.
	self assert: graph diagnostics isEmpty.
	self assert: graph reifiers size equals: 2.
	self assert: (graph reifierTriplesFor: 0) equals: { #(1 2 3). #(1 2 4) }.
	self assert: (GtsNQuads toNQuads: graph) lines size equals: 2
]

{ #category : #tests }
GtsReaderTest >> testByteIdenticalReifiesRowsStillCollapse [
	| graph writer |
	writer := GtsWriter forProfile: 'generic'.
	writer addTerms: self statementLayerTerms.
	writer addReifiers: {
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 3) }.
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 3) } }.
	graph := GtsReader read: writer bytes.
	self assert: graph diagnostics isEmpty.
	self assert: graph reifiers size equals: 1.
	self assert: (graph reifierTriplesFor: 0) equals: { #(1 2 3) }
]

{ #category : #tests }
GtsReaderTest >> testConflictingReifierFlagsOnlyTheLegacyTripleTermAndDropsNoRow [
	| graph writer |
	writer := GtsWriter forProfile: 'generic'.
	writer addTerms: self statementLayerTerms, { GtsTerm tripleReifier: 0 }.
	writer addReifiers: {
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 3) }.
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 4) } }.
	writer addQuads: { #(5 2 1) }.
	graph := GtsReader read: writer bytes.
	self assert: (self codesOf: graph) equals: #('ConflictingReifier').
	self assert: graph diagnostics first frameIndex isNil.
	self assert: graph reifiers size equals: 2.
	self assert: (graph tripleForTermId: 5) equals: #(1 2 3)
]

{ #category : #tests }
GtsReaderTest >> testSelfDescribingTripleTermNeverRaisesConflictingReifier [
	| graph writer |
	writer := GtsWriter forProfile: 'generic'.
	writer addTerms: self statementLayerTerms, { GtsTerm tripleComponents: #(1 2 3) reifier: 0 }.
	writer addReifiers: {
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 3) }.
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 4) } }.
	writer addQuads: { #(5 2 1) }.
	graph := GtsReader read: writer bytes.
	self assert: graph diagnostics isEmpty.
	self assert: graph reifiers size equals: 2.
	self assert: (graph tripleForTermId: 5) equals: #(1 2 3)
]

{ #category : #tests }
GtsReaderTest >> testDistinctTripleTermsSharingAReifierIdStayDistinct [
	| bytes quoted single union writer |
	writer := GtsWriter forProfile: 'generic'.
	writer addTerms: self statementLayerTerms, {
		GtsTerm tripleComponents: #(1 2 3) reifier: 0.
		GtsTerm tripleComponents: #(1 2 4) reifier: 0 }.
	writer addReifiers: {
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 3) }.
		Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 4) } }.
	writer addQuads: { #(5 2 1). #(6 2 1) }.
	bytes := writer bytes.
	single := GtsReader read: bytes.
	self assert: single diagnostics isEmpty.
	self assert: (single tripleForTermId: 5) equals: #(1 2 3).
	self assert: (single tripleForTermId: 6) equals: #(1 2 4).
	union := GtsReader read: bytes, bytes.
	self assert: union diagnostics isEmpty.
	self assert: union segmentHeads size equals: 2.
	quoted := (0 to: union terms size - 1) select: [ :termId |
		(union termAtId: termId) kind = #triple ].
	self assert: quoted size equals: 2.
	self
		assert: (quoted collect: [ :termId | union tripleForTermId: termId ]) asSet size
		equals: 2
]

{ #category : #private }
GtsReaderTest >> selfReachingSegmentBytes [
	"A `reifies` row that names the very TERM resolving through it.

	Built through the real writer, not hand-forged: the row (0, (2 1 1)) sits alongside
	term 2 = k:3 rf=0. Resolution MUST terminate (§7.3) — the union interns a triple term
	on its RESOLVED components and the N-Quads projection walks them, so both would
	recurse forever without a guard."
	| writer |
	writer := GtsWriter forProfile: 'generic'.
	writer addTerms: {
		GtsTerm iri: 'https://example.org/r1'.
		GtsTerm iri: 'https://example.org/p'.
		GtsTerm tripleReifier: 0 }.
	writer addReifiers: { Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(2 1 1) } }.
	writer addQuads: { #(2 1 1) }.
	^ writer bytes
]

{ #category : #tests }
GtsReaderTest >> testSelfReachingTripleTermFoldsAndProjects [
	| graph lines |
	graph := GtsReader read: self selfReachingSegmentBytes.
	self assert: graph diagnostics isEmpty.
	lines := (GtsNQuads toNQuads: graph) lines.
	self assert: lines size equals: 2.
	"The SELF-REACHING TERM ITSELF is the blank node (§7.3). Resolving one step first
	and degrading a nested occurrence renders a different graph."
	self
		assert: (lines at: 1)
		equals: '<https://example.org/r1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( _:unbound_triple_2 <https://example.org/p> <https://example.org/p> )>> .'.
	self
		assert: (lines at: 2)
		equals: '_:unbound_triple_2 <https://example.org/p> <https://example.org/p> .'
]

{ #category : #tests }
GtsReaderTest >> testSelfReachingTripleTermUnionTerminates [
	| bytes graph |
	bytes := self selfReachingSegmentBytes.
	graph := GtsReader read: bytes, bytes.
	"Both copies state no triple, so they intern to ONE term rather than two distinct
	terms sharing a reifier id; byte-identical rows then collapse under §7.8. The
	reifier is not over-bound, so there is no conflict either."
	self assert: graph quads size equals: 1.
	graph quads do: [ :quad |
		self assert: (graph termAtId: (quad at: 1)) kind equals: #triple ].
	self assert: (self codesOf: graph) isEmpty.
	self assert: (GtsNQuads toNQuads: graph) lines size equals: 2
]

{ #category : #tests }
GtsReaderTest >> testForwardReferencingTripleTermComponentsAreDropped [
	| graph writer |
	writer := GtsWriter forProfile: 'generic'.
	writer addTerms: {
		GtsTerm iri: 'https://ex/s'.
		GtsTerm tripleComponents: #(0 7 0) }.
	graph := GtsReader read: writer bytes.
	self assert: (self codesOf: graph) equals: #('ForwardReference').
	self assert: (graph termAtId: 1) triple isNil.
	self assert: (graph tripleForTermId: 1) isNil
]

{ #category : #tests }
GtsReaderTest >> testTripleTermPositionViolationDropsTtAndFallsBackToReifier [
	| graph writer |
	writer := GtsWriter forProfile: 'generic'.
	writer addTerms: self statementLayerTerms, { GtsTerm tripleComponents: #(1 3 4) reifier: 0 }.
	writer addReifiers: { Dictionary newFromPairs: { 'rid'. 0. 'spo'. #(1 2 3) } }.
	graph := GtsReader read: writer bytes.
	self assert: (self codesOf: graph) equals: #('PositionConstraint').
	self assert: (graph termAtId: 5) triple isNil.
	self assert: (graph tripleForTermId: 5) equals: #(1 2 3)
]

{ #category : #tests }
GtsReaderTest >> testMinimalVectorFoldsToExpectedGraph [
	| graph |
	graph := GtsReader read: self readMinimalVectorBytes.
	self assert: graph diagnostics isEmpty.
	self assert: graph terms size equals: 3.
	self assert: graph quads size equals: 1.
	self assert: graph profiles equals: #('dist').
	self
		assert: (GtsHex encode: graph segmentHeads first)
		equals: 'ec5a15cbe3b79c333712d64ed83a70e69a2d1be8c1316835727e5d5219823cd9'
]

{ #category : #tests }
GtsReaderTest >> testMinimalVectorProjectsToNQuads [
	| graph |
	graph := GtsReader read: self readMinimalVectorBytes.
	self
		assert: (GtsNQuads toNQuads: graph)
		equals: '<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> "Cat"@en .', String lf
]

{ #category : #tests }
GtsReaderTest >> testUnionSegmentsPreservesStreamableHead [
	| graph head union |
	graph := GtsGraph new.
	head := GtsHex decode: (String new: 64 withAll: $a).
	graph addStreamableClaimed: true covered: 3 tail: 12 head: head.
	union := GtsReader unionSegments: { graph }.
	self assert: ((union streamable first) at: 'head') equals: head
]

{ #category : #private }
GtsReaderTest >> segmentReifying: reifierIri object: objectIri [
	"One segment: <reifierIri> reifies (s p objectIri), quoted by a LEGACY 'tt'-less
	triple term (id 4) that the segment's only quad uses as its subject."
	| segment |
	segment := GtsGraph new.
	segment addTerm: (GtsTerm iri: reifierIri).
	segment addTerm: (GtsTerm iri: 'https://ex/s').
	segment addTerm: (GtsTerm iri: 'https://ex/p').
	segment addTerm: (GtsTerm iri: objectIri).
	segment addTerm: (GtsTerm tripleReifier: 0).
	segment setReifier: 0 to: #(1 2 3).
	segment addQuad: #(4 2 3).
	^ segment
]

{ #category : #tests }
GtsReaderTest >> testUnionSegmentsMergeTripleTermsWithTheSameResolvedTriple [
	"§7.8: triple-term equality IS equality of the RESOLVED (s p o), so two legacy terms
	that hang off DIFFERENT reifier ids but resolve to the SAME triple are one term."
	| tripleTerms union |
	union := GtsReader unionSegments: {
		self segmentReifying: 'https://ex/r1' object: 'https://ex/o'.
		self segmentReifying: 'https://ex/r2' object: 'https://ex/o' }.
	tripleTerms := union terms select: [ :term | term kind = #triple ].
	self assert: tripleTerms size equals: 1.
	self assert: union quads size equals: 1
]

{ #category : #tests }
GtsReaderTest >> testUnionSegmentsSeparateTripleTermsWithDifferentResolvedTriples [
	| tripleTerms union |
	union := GtsReader unionSegments: {
		self segmentReifying: 'https://ex/r' object: 'https://ex/o1'.
		self segmentReifying: 'https://ex/r' object: 'https://ex/o2' }.
	tripleTerms := union terms select: [ :term | term kind = #triple ].
	self assert: tripleTerms size equals: 2.
	self assert: union quads size equals: 2
]

{ #category : #tests }
GtsReaderTest >> testPublicReaderReportsMalformedInputDiagnostics [
	self assert: (self diagnosticCodesFor: ByteArray new) equals: #('EmptyFile').
	self assert: (self diagnosticCodesFor: (ByteArray with: 16r01)) equals: #('DamagedFrame').
	self
		assert: (self diagnosticCodesFor: self readMinimalVectorBytes , (ByteArray with: 16rA3))
		equals: #('TornAppendError')
]

{ #category : #tests }
GtsReaderTest >> testZstdVectorFoldsToExpectedGraph [
	| graph |
	graph := GtsReader read: self readZstdVectorBytes.
	self assert: graph diagnostics isEmpty.
	self assert: graph terms size equals: 3.
	self assert: graph quads size equals: 1.
	self assert: graph profiles equals: #('generic').
	self
		assert: (GtsHex encode: graph segmentHeads first)
		equals: '4bdbb88d5c994034359346c2c66fe46bb0f9c3670365086cdf3a8282c6bd5c9f'.
	self
		assert: (GtsNQuads toNQuads: graph)
		equals: '<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> "Cat"@en .', String lf
]
