"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2PayloadBudget,
	#superclass : #Object,
	#instVars : [ 'storedLimit', 'frameLimit', 'readLimit', 'itemLimit', 'total', 'exhausted' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2PayloadBudget class >> withReadLimit: bytes [
	^ self new readLimit: bytes
]

{ #category : #initialization }
GtsV2PayloadBudget >> initialize [
	super initialize.
	storedLimit := 1 bitShift: 26.
	frameLimit := 1 bitShift: 26.
	readLimit := 1 bitShift: 36.
	itemLimit := 1 bitShift: 26.
	total := 0.
	exhausted := false
]

{ #category : #initialization }
GtsV2PayloadBudget >> readLimit: bytes [
	readLimit := bytes.
	^ self
]

{ #category : #accessing }
GtsV2PayloadBudget >> readLimit [
	^ readLimit
]

{ #category : #accessing }
GtsV2PayloadBudget >> itemLimit [
	^ itemLimit
]

{ #category : #initialization }
GtsV2PayloadBudget >> itemLimit: limit [
	itemLimit := limit.
	^ self
]

{ #category : #validation }
GtsV2PayloadBudget >> decodedExhausted [
	^ exhausted
]

{ #category : #validation }
GtsV2PayloadBudget >> admitResync [
	^ self admit: 4096 stored: 0 content: 0 signatures: 0
]

{ #category : #validation }
GtsV2PayloadBudget >> admit: length stored: storedSize content: contentSize signatures: signaturesSize [
	| sum |
	exhausted ifFalse: [
		storedSize > storedLimit ifTrue: [ ^ self unchecked: 'stored_size' count: storedSize ].
		contentSize > storedLimit ifTrue: [ ^ self unchecked: 'stored_size' count: contentSize ].
		signaturesSize > storedLimit ifTrue: [ ^ self unchecked: 'stored_size' count: signaturesSize ].
		length > frameLimit ifTrue: [ ^ self unchecked: 'decoded_size' count: length ] ].
	sum := total + length.
	(exhausted or: [ sum > readLimit ]) ifTrue: [
		exhausted := true.
		^ self unchecked: 'total_decoded' count: (sum min: 18446744073709551615) ].
	total := sum.
	^ nil
]

{ #category : #private }
GtsV2PayloadBudget >> unchecked: reason count: count [
	^ GtsV2PayloadResult state: #unchecked reason: reason count: count
]
