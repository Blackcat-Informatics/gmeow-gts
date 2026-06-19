Class {
	#name : #GtsBlake3Library,
	#superclass : #FFILibrary,
	#category : #'Gts-Core'
}

{ #category : #ffi }
GtsBlake3Library >> hashInput: inputBytes length: inputLength output: outputBytes [
	^ self ffiCall: #(#void #gts_blake3_hash #(#const #void #* #inputBytes #, #size_t #inputLength #, #void #* #outputBytes))
]

{ #category : #'accessing platform' }
GtsBlake3Library >> macLibraryName [
	^ 'libblake3.dylib'
]

{ #category : #ffi }
GtsBlake3Library >> outLength [
	^ self ffiCall: #(#size_t #gts_blake3_out_len #())
]

{ #category : #'accessing platform' }
GtsBlake3Library >> unixLibraryName [
	^ 'libblake3.so'
]

{ #category : #'accessing platform' }
GtsBlake3Library >> win32LibraryName [
	^ 'blake3.dll'
]
