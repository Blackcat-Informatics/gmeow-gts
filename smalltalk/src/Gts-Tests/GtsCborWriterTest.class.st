Class {
	#name : #GtsCborWriterTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #asserting }
GtsCborWriterTest >> assertEncoding: anObject equalsHex: expectedHex [
	self assert: (GtsHex encode: (GtsCborWriter encode: anObject)) equals: expectedHex
]

{ #category : #tests }
GtsCborWriterTest >> testArraysUseDefiniteLength [
	self assertEncoding: #(1 2 3) equalsHex: '83010203'
]

{ #category : #tests }
GtsCborWriterTest >> testByteStringsUseDefiniteLength [
	self assertEncoding: #[1 2 255] equalsHex: '430102ff'
]

{ #category : #tests }
GtsCborWriterTest >> testIntegersUseShortestForm [
	self assertEncoding: 0 equalsHex: '00'.
	self assertEncoding: 23 equalsHex: '17'.
	self assertEncoding: 24 equalsHex: '1818'.
	self assertEncoding: 255 equalsHex: '18ff'.
	self assertEncoding: 256 equalsHex: '190100'.
	self assertEncoding: -1 equalsHex: '20'.
	self assertEncoding: -24 equalsHex: '37'.
	self assertEncoding: -25 equalsHex: '3818'
]

{ #category : #tests }
GtsCborWriterTest >> testMapKeysSortByEncodedBytes [
	| map |
	map := Dictionary new.
	map at: 'b' put: 1.
	map at: 'a' put: 2.
	self assertEncoding: map equalsHex: 'a2616102616201'
]

{ #category : #tests }
GtsCborWriterTest >> testTextStringsUseUtf8 [
	self assertEncoding: 'gts' equalsHex: '63677473'.
	self assertEncoding: 'cat' equalsHex: '63636174'
]

{ #category : #tests }
GtsCborWriterTest >> testRuntimeSummaryNamesNativeLibraries [
	| libraries |
	libraries := GtsNativeLibraries runtimeSummary at: 'requiredLibraries'.
	self assert: (libraries includes: 'libblake3.so').
	self assert: (libraries includes: 'libzstd.so').
	self assert: (libraries includes: 'libsodium.so')
]
