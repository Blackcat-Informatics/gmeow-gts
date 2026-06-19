Class {
	#name : #GtsConformanceTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #tests }
GtsConformanceTest >> testTopLevelGtsCorpusMatchesExpectedSummaries [
	| divergences |
	divergences := GtsConformanceSummary divergences.
	self assert: divergences isEmpty
]
