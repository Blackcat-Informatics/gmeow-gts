"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2CborHead,
	#superclass : #Object,
	#instVars : [ 'major', 'argument', 'end', 'shortest' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2CborHead class >> major: aMajor argument: aValue end: aPosition shortest: isShortest [
	^ self new initializeMajor: aMajor argument: aValue end: aPosition shortest: isShortest
]

{ #category : #initialization }
GtsV2CborHead >> initializeMajor: aMajor argument: aValue end: aPosition shortest: isShortest [
	major := aMajor.
	argument := aValue.
	end := aPosition.
	shortest := isShortest.
	^ self
]

{ #category : #accessing }
GtsV2CborHead >> argument [
	^ argument
]

{ #category : #accessing }
GtsV2CborHead >> end [
	^ end
]

{ #category : #accessing }
GtsV2CborHead >> major [
	^ major
]

{ #category : #testing }
GtsV2CborHead >> isShortest [
	^ shortest
]
