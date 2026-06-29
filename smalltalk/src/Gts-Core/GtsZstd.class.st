Class {
	#name : #GtsZstd,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #compression }
GtsZstd class >> compress: aByteArray [
	^ self compress: aByteArray level: 3
]

{ #category : #compression }
GtsZstd class >> compress: aByteArray level: compressionLevel [
	| inputBytes outputBytes outputSize |
	inputBytes := aByteArray asByteArray.
	outputBytes := ByteArray new: (GtsZstdLibrary uniqueInstance compressBoundFor: inputBytes size).
	outputSize := GtsZstdLibrary uniqueInstance
		compressInto: outputBytes
		capacity: outputBytes size
		from: inputBytes
		length: inputBytes size
		level: compressionLevel.
	self ensureOk: outputSize.
	^ outputBytes copyFrom: 1 to: outputSize
]

{ #category : #decompression }
GtsZstd class >> decompress: aByteArray [
	^ self decompress: aByteArray expectedSize: (self frameContentSizeFor: aByteArray)
]

{ #category : #decompression }
GtsZstd class >> decompress: aByteArray expectedSize: expectedSize [
	| inputBytes outputBytes outputSize |
	(expectedSize isInteger and: [ expectedSize >= 0 ])
		ifFalse: [ self error: 'zstd frame content size is unavailable' ].
	inputBytes := aByteArray asByteArray.
	outputBytes := ByteArray new: expectedSize.
	outputSize := GtsZstdLibrary uniqueInstance
		decompressInto: outputBytes
		capacity: expectedSize
		from: inputBytes
		length: inputBytes size.
	self ensureOk: outputSize.
	^ outputBytes copyFrom: 1 to: outputSize
]

{ #category : #private }
GtsZstd class >> ensureOk: resultCode [
	(GtsZstdLibrary uniqueInstance isError: resultCode) = 0
		ifFalse: [ self error: 'zstd operation failed' ]
]

{ #category : #private }
GtsZstd class >> frameContentSizeError [
	^ 16rFFFFFFFFFFFFFFFE
]

{ #category : #accessing }
GtsZstd class >> frameContentSizeFor: aByteArray [
	| inputBytes size |
	inputBytes := aByteArray asByteArray.
	size := GtsZstdLibrary uniqueInstance
		frameContentSizeOf: inputBytes
		length: inputBytes size.
	((size = self frameContentSizeUnknown) or: [ size = self frameContentSizeError ])
		ifTrue: [ self error: 'zstd frame content size is unavailable' ].
	^ size
]

{ #category : #private }
GtsZstd class >> frameContentSizeUnknown [
	^ 16rFFFFFFFFFFFFFFFF
]

{ #category : #accessing }
GtsZstd class >> versionNumber [
	^ GtsZstdLibrary uniqueInstance versionNumber
]
