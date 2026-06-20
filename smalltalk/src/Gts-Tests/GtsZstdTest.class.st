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
GtsZstdTest >> testRejectsOversizedExpectedDecompression [
	self
		should: [ GtsZstd decompress: #[1 2 3 4] expectedSize: GtsZstd maxDecompressedBytes + 1 ]
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
