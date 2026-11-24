"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2Range,
	#superclass : #Object,
	#instVars : [ 'peaks', 'leafCount' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2Range >> initialize [
	super initialize.
	peaks := OrderedCollection new.
	leafCount := 0
]

{ #category : #accessing }
GtsV2Range >> leafCount [
	^ leafCount
]

{ #category : #adding }
GtsV2Range >> add: identifier [
	| height digest |
	height := 0.
	digest := self hash: { identifier } context: 'gts2 range leaf'.
	[ peaks isNotEmpty and: [ peaks last key = height ] ] whileTrue: [
		digest := self hash: { peaks removeLast value. digest } context: 'gts2 range node'.
		height := height + 1 ].
	peaks add: height -> digest.
	leafCount := leafCount + 1
]

{ #category : #accessing }
GtsV2Range >> root [
	^ self hash: { leafCount. (peaks collect: [ :peak | peak value ]) asArray } context: 'gts2 range root'
]

{ #category : #private }
GtsV2Range >> hash: elements context: context [
	| encoded |
	encoded := GtsCborWriter encode: elements.
	^ GtsBlake3 deriveWith: context prefix: #[] bytes: encoded from: 1 to: encoded size + 1
]
