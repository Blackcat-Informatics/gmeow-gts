Class {
	#name : #GtsDiagnostic,
	#superclass : #Object,
	#instVars : [
		'code',
		'detail',
		'frameIndex'
	],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsDiagnostic class >> code: codeString detail: detailString frameIndex: anInteger [
	^ self new
		code: codeString;
		detail: detailString;
		frameIndex: anInteger;
		yourself
]

{ #category : #accessing }
GtsDiagnostic >> code [
	^ code
]

{ #category : #accessing }
GtsDiagnostic >> code: codeString [
	code := codeString
]

{ #category : #accessing }
GtsDiagnostic >> detail [
	^ detail
]

{ #category : #accessing }
GtsDiagnostic >> detail: detailString [
	detail := detailString
]

{ #category : #accessing }
GtsDiagnostic >> frameIndex [
	^ frameIndex
]

{ #category : #accessing }
GtsDiagnostic >> frameIndex: anInteger [
	frameIndex := anInteger
]
