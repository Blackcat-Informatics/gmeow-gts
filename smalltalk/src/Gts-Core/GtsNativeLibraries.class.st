Class {
	#name : #GtsNativeLibraries,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #accessing }
GtsNativeLibraries class >> requiredLibraries [
	^ #('libblake3.so' 'libzstd.so' 'libsodium.so')
]

{ #category : #accessing }
GtsNativeLibraries class >> runtimeSummary [
	^ Dictionary newFromPairs: {
		'pharo'. Smalltalk version.
		'requiredLibraries'. self requiredLibraries }
]
