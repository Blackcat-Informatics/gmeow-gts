Class {
	#name : #GtsNativeLibraries,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #accessing }
GtsNativeLibraries class >> requiredLibraries [
	^ #('libblake3.so' 'libzstd.so' 'libsodium.so' 'libgts_sodium.so')
]

{ #category : #accessing }
GtsNativeLibraries class >> runtimeSummary [
	^ Dictionary newFromPairs: {
		'pharo'. Smalltalk version.
		'requiredLibraries'. self requiredLibraries.
		'blake3DigestSize'. GtsBlake3 digestSize.
		'zstdVersionNumber'. GtsZstd versionNumber.
		'ed25519SignatureBytes'. GtsSodium ed25519SignatureBytes.
		'aes256GcmAvailable'. GtsSodium aes256GcmIsAvailable }
]
