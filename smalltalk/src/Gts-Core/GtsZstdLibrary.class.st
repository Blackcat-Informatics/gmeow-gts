Class {
	#name : #GtsZstdLibrary,
	#superclass : #FFILibrary,
	#category : #'Gts-Core'
}

{ #category : #ffi }
GtsZstdLibrary >> compressBoundFor: inputLength [
	^ self ffiCall: #(#size_t #ZSTD_compressBound #(#size_t #inputLength))
]

{ #category : #ffi }
GtsZstdLibrary >> compressInto: outputBytes capacity: outputCapacity from: inputBytes length: inputLength level: compressionLevel [
	^ self ffiCall: #(#size_t #ZSTD_compress #(#void #* #outputBytes #, #size_t #outputCapacity #, #const #void #* #inputBytes #, #size_t #inputLength #, #int #compressionLevel))
]

{ #category : #ffi }
GtsZstdLibrary >> decompressInto: outputBytes capacity: outputCapacity from: inputBytes length: inputLength [
	^ self ffiCall: #(#size_t #ZSTD_decompress #(#void #* #outputBytes #, #size_t #outputCapacity #, #const #void #* #inputBytes #, #size_t #inputLength))
]

{ #category : #ffi }
GtsZstdLibrary >> isError: resultCode [
	^ self ffiCall: #(#uint #ZSTD_isError #(#size_t #resultCode))
]

{ #category : #'accessing platform' }
GtsZstdLibrary >> macLibraryName [
	^ 'libzstd.dylib'
]

{ #category : #'accessing platform' }
GtsZstdLibrary >> unixLibraryName [
	^ 'libzstd.so'
]

{ #category : #ffi }
GtsZstdLibrary >> versionNumber [
	^ self ffiCall: #(#uint #ZSTD_versionNumber #())
]

{ #category : #'accessing platform' }
GtsZstdLibrary >> win32LibraryName [
	^ 'zstd.dll'
]
