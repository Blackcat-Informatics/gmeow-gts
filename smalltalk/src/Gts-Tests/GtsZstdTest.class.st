Class {
	#name : #GtsZstdTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #tests }
GtsZstdTest >> testInvalidZstdBytesRaiseError [
	self
		should: [ GtsZstd decompress: #[1 2 3 4] expectedSize: 32 ]
		raise: Error
]

{ #category : #tests }
GtsZstdTest >> testNativeVersionIsAvailable [
	self assert: GtsZstd versionNumber > 0.
	self
		assert: (GtsNativeLibraries runtimeSummary at: 'zstdVersionNumber')
		equals: GtsZstd versionNumber
]

{ #category : #tests }
GtsZstdTest >> testRoundTripBytesThroughNativeZstd [
	| compressed payload restored |
	payload := 'Graph Transport Substrate zstd FFI smoke test' utf8Encoded.
	compressed := GtsZstd compress: payload.
	self assert: (GtsZstd frameContentSizeFor: compressed) equals: payload size.
	restored := GtsZstd decompress: compressed.
	self assert: restored equals: payload
]

{ #category : #tests }
GtsZstdTest >> testRoundTripBytesAboveFormerSafetyBound [
	| compressed payload restored |
	payload := ByteArray new: (16 * 1024 * 1024 + 1).
	compressed := GtsZstd compress: payload.
	self assert: (GtsZstd frameContentSizeFor: compressed) equals: payload size.
	restored := GtsZstd decompress: compressed.
	self assert: restored equals: payload
]

{ #category : #tests }
GtsZstdTest >> testConcatenatedFramesDecodeToTheEndOfThePayload [
	| first second third compressed |
	first := GtsZstd compress: 'alpha ' utf8Encoded.
	second := GtsZstd compress: 'beta ' utf8Encoded.
	third := GtsZstd compress: 'gamma' utf8Encoded.
	compressed := first , second , third.
	self assert: (GtsZstd frameContentSizeFor: compressed) equals: 16.
	self assert: (GtsZstd decompress: compressed) equals: 'alpha beta gamma' utf8Encoded
]

{ #category : #tests }
GtsZstdTest >> testConcatenatedFramesAboveOneUnit [
	| unit compressed restored |
	unit := ByteArray new: 65536 withAll: 7.
	compressed := (GtsZstd compress: unit) , (GtsZstd compress: unit) , (GtsZstd compress: unit).
	restored := GtsZstd decompress: compressed.
	self assert: restored size equals: unit size * 3
]
