"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ResyncScan,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #scanning }
GtsV2ResyncScan class >> on: walker from: start budget: budget [
	| bytes pattern matched |
	budget decodedExhausted ifTrue: [ ^ nil ].
	bytes := walker bytes.
	pattern := #[139 5 2 80 252 243 235 50 162 240 38 125 209 106 83 240 155 255 135 146].
	matched := 0.
	start to: bytes size do: [ :position |
		| octet candidate |
		octet := bytes at: position.
		matched := octet = (pattern at: matched + 1)
			ifTrue: [ matched + 1 ]
			ifFalse: [ octet = pattern first ifTrue: [ 1 ] ifFalse: [ 0 ] ].
		matched = pattern size ifTrue: [
			candidate := self candidateOn: walker at: position + 1 - matched budget: budget.
			candidate ifNotNil: [ ^ candidate ].
			matched := 0 ] ].
	^ nil
]

{ #category : #private }
GtsV2ResyncScan class >> candidateOn: walker at: start budget: budget [
	| item refusal opener |
	item := GtsV2Field walker: walker from: start to: (start + 4096 min: walker bytes size + 1).
	[ opener := GtsV2Opener candidate: item charging: [
		refusal := budget admitResync.
		refusal isNil ] ]
		on: GtsV2FramingError, GtsV2EnvelopeError
		do: [ :error | ^ nil ].
	refusal ifNotNil: [
		^ (GtsV2WireEvent kind: #resyncRefused from: start - 1 to: start + 4095 problem: refusal reason)
			resyncRefusal: refusal ].
	^ opener ifNotNil: [
		(GtsV2WireEvent kind: #resync from: start - 1 to: opener end - 1 problem: nil)
			opener: opener frame: nil;
			markScanned ]
]
