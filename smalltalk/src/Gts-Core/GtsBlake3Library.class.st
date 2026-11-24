Class {
	#name : #GtsBlake3Library,
	#superclass : #FFILibrary,
	#category : #'Gts-Core'
}

{ #category : #ffi }
GtsBlake3Library >> hashInput: inputBytes length: inputLength output: outputBytes [
	^ self ffiCall: #(#void #gts_blake3_hash #(#const #void #* #inputBytes #, #size_t #inputLength #, #void #* #outputBytes))
]

{ #category : #ffi }
GtsBlake3Library >> deriveWith: context contextLength: contextLength prefix: prefix prefixLength: prefixLength input: input start: start length: length output: output [
	^ self ffiCall: #(void gts_blake3_derive (
		const void * context, size_t contextLength, const void * prefix, size_t prefixLength,
		const void * input, size_t start, size_t length, void * output))
]

{ #category : #ffi }
GtsBlake3Library >> deriveStateFor: context length: contextLength [
	^ self ffiCall: #(void * gts_blake3_derive_new (const void * context, size_t contextLength))
]

{ #category : #ffi }
GtsBlake3Library >> update: state input: input start: start length: length [
	^ self ffiCall: #(void gts_blake3_derive_update (void * state, const void * input, size_t start, size_t length))
]

{ #category : #ffi }
GtsBlake3Library >> finish: state output: output [
	^ self ffiCall: #(void gts_blake3_derive_finish (const void * state, void * output))
]

{ #category : #ffi }
GtsBlake3Library >> free: state [
	^ self ffiCall: #(void gts_blake3_derive_free (void * state))
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
