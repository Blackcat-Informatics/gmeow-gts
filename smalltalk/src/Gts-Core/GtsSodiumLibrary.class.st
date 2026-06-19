Class {
	#name : #GtsSodiumLibrary,
	#superclass : #FFILibrary,
	#category : #'Gts-Core'
}

{ #category : #ffi }
GtsSodiumLibrary >> aes256GcmDecrypt: ciphertext length: ciphertextLength additionalData: additionalData additionalDataLength: additionalDataLength nonce: nonce key: key output: message [
	^ self ffiCall: #(#int #gts_aes256gcm_decrypt #(#const #void #* #ciphertext #, #size_t #ciphertextLength #, #const #void #* #additionalData #, #size_t #additionalDataLength #, #const #void #* #nonce #, #const #void #* #key #, #void #* #message))
]

{ #category : #ffi }
GtsSodiumLibrary >> aes256GcmEncrypt: message length: messageLength additionalData: additionalData additionalDataLength: additionalDataLength nonce: nonce key: key output: ciphertext [
	^ self ffiCall: #(#int #gts_aes256gcm_encrypt #(#const #void #* #message #, #size_t #messageLength #, #const #void #* #additionalData #, #size_t #additionalDataLength #, #const #void #* #nonce #, #const #void #* #key #, #void #* #ciphertext))
]

{ #category : #ffi }
GtsSodiumLibrary >> aes256GcmIsAvailable [
	^ self ffiCall: #(#int #gts_aes256gcm_is_available #())
]

{ #category : #ffi }
GtsSodiumLibrary >> aes256GcmKeyBytes [
	^ self ffiCall: #(#size_t #gts_aes256gcm_keybytes #())
]

{ #category : #ffi }
GtsSodiumLibrary >> aes256GcmMacBytes [
	^ self ffiCall: #(#size_t #gts_aes256gcm_macbytes #())
]

{ #category : #ffi }
GtsSodiumLibrary >> aes256GcmNonceBytes [
	^ self ffiCall: #(#size_t #gts_aes256gcm_noncebytes #())
]

{ #category : #ffi }
GtsSodiumLibrary >> ed25519PublicKeyBytes [
	^ self ffiCall: #(#size_t #gts_ed25519_publickeybytes #())
]

{ #category : #ffi }
GtsSodiumLibrary >> ed25519SecretKeyBytes [
	^ self ffiCall: #(#size_t #gts_ed25519_secretkeybytes #())
]

{ #category : #ffi }
GtsSodiumLibrary >> ed25519SeedBytes [
	^ self ffiCall: #(#size_t #gts_ed25519_seedbytes #())
]

{ #category : #ffi }
GtsSodiumLibrary >> ed25519SignatureBytes [
	^ self ffiCall: #(#size_t #gts_ed25519_signaturebytes #())
]

{ #category : #ffi }
GtsSodiumLibrary >> ed25519Sign: message length: messageLength secretKey: secretKey output: signature [
	^ self ffiCall: #(#int #gts_ed25519_sign_detached #(#const #void #* #message #, #size_t #messageLength #, #const #void #* #secretKey #, #void #* #signature))
]

{ #category : #ffi }
GtsSodiumLibrary >> ed25519Verify: signature message: message length: messageLength publicKey: publicKey [
	^ self ffiCall: #(#int #gts_ed25519_verify_detached #(#const #void #* #signature #, #const #void #* #message #, #size_t #messageLength #, #const #void #* #publicKey))
]

{ #category : #ffi }
GtsSodiumLibrary >> initializeSodium [
	^ self ffiCall: #(#int #gts_sodium_init #())
]

{ #category : #'accessing platform' }
GtsSodiumLibrary >> macLibraryName [
	^ 'libgts_sodium.dylib'
]

{ #category : #ffi }
GtsSodiumLibrary >> seedKeyPairFrom: seed publicKey: publicKey secretKey: secretKey [
	^ self ffiCall: #(#int #gts_ed25519_seed_keypair #(#const #void #* #seed #, #void #* #publicKey #, #void #* #secretKey))
]

{ #category : #'accessing platform' }
GtsSodiumLibrary >> unixLibraryName [
	^ 'libgts_sodium.so'
]

{ #category : #'accessing platform' }
GtsSodiumLibrary >> win32LibraryName [
	^ 'gts_sodium.dll'
]
