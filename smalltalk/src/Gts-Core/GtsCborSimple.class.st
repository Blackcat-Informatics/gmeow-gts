Class {
	#name : #GtsCborSimple,
	#superclass : #Object,
	#instVars : [
		'additional',
		'argument'
	],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsCborSimple class >> additional: additionalInformation argument: argumentBits [
	^ self new
		additional: additionalInformation;
		argument: argumentBits;
		yourself
]

{ #category : #accessing }
GtsCborSimple >> additional [
	^ additional
]

{ #category : #accessing }
GtsCborSimple >> additional: additionalInformation [
	additional := additionalInformation
]

{ #category : #accessing }
GtsCborSimple >> argument [
	^ argument
]

{ #category : #accessing }
GtsCborSimple >> argument: argumentBits [
	argument := argumentBits
]

{ #category : #comparing }
GtsCborSimple >> = other [
	^ (other isKindOf: GtsCborSimple)
		and: [ additional = other additional and: [ argument = other argument ] ]
]

{ #category : #comparing }
GtsCborSimple >> hash [
	^ additional hash bitXor: argument hash
]
