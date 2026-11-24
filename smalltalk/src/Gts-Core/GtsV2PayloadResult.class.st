"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2PayloadResult,
	#superclass : #Object,
	#instVars : [ 'state', 'reason', 'count' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2PayloadResult class >> state: state reason: reason count: count [
	^ self new initializeState: state reason: reason count: count
]

{ #category : #initialization }
GtsV2PayloadResult >> initializeState: aState reason: aReason count: resultCount [
	state := aState.
	reason := aReason.
	count := resultCount.
	^ self
]

{ #category : #accessing }
GtsV2PayloadResult >> state [
	^ state
]

{ #category : #accessing }
GtsV2PayloadResult >> reason [
	^ reason
]

{ #category : #accessing }
GtsV2PayloadResult >> count [
	^ count
]
