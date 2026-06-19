Class {
	#name : #GtsCborTag,
	#superclass : #Object,
	#instVars : [
		'tag',
		'value'
	],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsCborTag class >> tag: tagNumber value: taggedValue [
	^ self new
		tag: tagNumber;
		value: taggedValue;
		yourself
]

{ #category : #accessing }
GtsCborTag >> tag [
	^ tag
]

{ #category : #accessing }
GtsCborTag >> tag: tagNumber [
	tag := tagNumber
]

{ #category : #accessing }
GtsCborTag >> value [
	^ value
]

{ #category : #accessing }
GtsCborTag >> value: taggedValue [
	value := taggedValue
]
