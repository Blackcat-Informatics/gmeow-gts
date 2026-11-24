"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2TermsContent,
	#superclass : #Object,
	#instVars : [ 'runs', 'reservedCount', 'runProblem', 'problem', 'restart' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2TermsContent class >> read: content expecting: base newer: newer [
	^ self new read: content expecting: base newer: newer
]

{ #category : #decoding }
GtsV2TermsContent >> read: content expecting: base newer: newer [
	| fields |
	reservedCount := 0.
	[
		fields := content arrayPrefix: 2.
		fields next.
		self readRuns: fields next expecting: base newer: newer
	] on: GtsV2EnvelopeError do: [ :error | runProblem := error messageText ].
	[
		fields := content fields: 4.
		fields next uint = base ifFalse: [ GtsV2EnvelopeError signal: 'inconsistent_counts' ].
		fields next.
		problem := runProblem ifNil: [ self checkOrderAndRestart: fields newer: newer ]
	] on: GtsV2EnvelopeError do: [ :error | problem := error messageText ].
	^ self
]

{ #category : #private }
GtsV2TermsContent >> readRuns: field expecting: base newer: newer [
	| cursor previous total |
	[
		cursor := field list.
		runs := OrderedCollection new.
		previous := nil.
		total := 0.
		[ cursor hasNext ] whileTrue: [
			| run kind count |
			run := cursor next fields: 2.
			kind := run next uint.
			(kind = 0 or: [ kind > 7 and: [ newer not ] ]) ifTrue: [
				GtsV2EnvelopeError signal: 'unregistered_value' ].
			kind > 7 ifTrue: [ runProblem := #unsupported_critical ].
			kind = previous ifTrue: [ GtsV2EnvelopeError signal: 'not_deterministic' ].
			count := run next uint.
			count = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
			runs add: { kind. count }.
			total := total + count.
			previous := kind ].
		(total = 0 or: [ base + total > 18446744073709551616 ]) ifTrue: [
			GtsV2EnvelopeError signal: 'out_of_range' ].
		reservedCount := total
	] on: GtsV2EnvelopeError do: [ :error |
		runProblem ifNil: [ runProblem := error messageText ] ]
]

{ #category : #private }
GtsV2TermsContent >> checkOrderAndRestart: fields newer: newer [
	| order |
	order := fields next uint.
	(order = 0 or: [ order > 2 and: [ newer not ] ]) ifTrue: [
		GtsV2EnvelopeError signal: 'unregistered_value' ].
	order > 2 ifTrue: [ ^ #unsupported_critical ].
	restart := fields next uint.
	restart = 0 ifTrue: [ GtsV2EnvelopeError signal: 'out_of_range' ].
	^ nil
]

{ #category : #accessing }
GtsV2TermsContent >> runs [
	^ runs
]

{ #category : #accessing }
GtsV2TermsContent >> reservedCount [
	^ reservedCount
]

{ #category : #accessing }
GtsV2TermsContent >> problem [
	^ problem
]

{ #category : #accessing }
GtsV2TermsContent >> restart [
	^ restart
]
