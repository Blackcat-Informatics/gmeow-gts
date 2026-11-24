"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ParityRepairSpan,
	#superclass : #Object,
	#instVars : [ 'offset', 'data', 'parity', 'failure' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2ParityRepairSpan class >> at: offset [
	^ self new initializeAt: offset
]

{ #category : #initialization }
GtsV2ParityRepairSpan >> initializeAt: inputOffset [
	offset := inputOffset.
	data := OrderedCollection new.
	parity := OrderedCollection new.
	^ self
]

{ #category : #accessing }
GtsV2ParityRepairSpan >> offset [
	^ offset
]

{ #category : #accessing }
GtsV2ParityRepairSpan >> failure [
	^ failure
]

{ #category : #accessing }
GtsV2ParityRepairSpan >> failure: reason [
	failure := reason
]

{ #category : #observing }
GtsV2ParityRepairSpan >> addData: patches conditional: conditional [
	data add: { patches. conditional }
]

{ #category : #observing }
GtsV2ParityRepairSpan >> addParity: shard at: start [
	parity add: { start. shard }
]

{ #category : #observing }
GtsV2ParityRepairSpan >> finishOpenerLink: passes [
	passes ifTrue: [ data := data reject: [ :entry | entry second ] ]
]

{ #category : #enumerating }
GtsV2ParityRepairSpan >> dataPatchesDo: aBlock [
	data do: [ :entry | entry first do: aBlock ]
]

{ #category : #enumerating }
GtsV2ParityRepairSpan >> patchesDo: aBlock [
	self dataPatchesDo: aBlock.
	parity do: aBlock
]

{ #category : #accessing }
GtsV2ParityRepairSpan >> shardCount [
	^ data size + parity size
]
