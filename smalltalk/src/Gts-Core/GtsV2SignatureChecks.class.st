"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2SignatureChecks,
	#superclass : #Object,
	#instVars : [ 'heldKeys', 'ed25519' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2SignatureChecks class >> held: entries [
	^ self new initializeHeld: entries
]

{ #category : #initialization }
GtsV2SignatureChecks >> initializeHeld: entries [
	heldKeys := Dictionary new.
	entries do: [ :entry |
		| kid algorithm key candidates existing |
		kid := GtsHex decode: entry first.
		kid size = 32 ifFalse: [ GtsArchiveError signal: 'GTS v2 --key needs a 32-byte hexadecimal key identifier' ].
		algorithm := entry second.
		key := entry third.
		candidates := heldKeys at: kid ifAbsentPut: [ Dictionary new ].
		(self keyId: key algorithm: algorithm) = kid ifTrue: [
			existing := candidates at: algorithm ifAbsent: [ nil ].
			(existing isNil or: [ self key: key precedes: existing ]) ifTrue: [
				candidates at: algorithm put: key ] ] ].
	ed25519 := GtsV2Ed25519 new.
	^ self
]

{ #category : #verification }
GtsV2SignatureChecks >> event: event header: header resultsDo: report [
	| records cursor |
	(event kind = #frame and: [ event effectiveIdentifier isNotNil ]) ifFalse: [ ^ self ].
	event frame isTransparent ifTrue: [ ^ self ].
	records := event frame signatureRecords.
	records ifNil: [ ^ self ].
	cursor := records list.
	[ cursor hasNext ] whileTrue: [
		| fields algorithm kid value status reason |
		fields := cursor next fields: 3.
		algorithm := fields next uint.
		kid := GtsV2RdfColumns body: fields next size: 32.
		value := fields next.
		reason := self reasonFor: value algorithm: algorithm keyId: kid header: header frame: event effectiveIdentifier.
		status := reason ifNil: [ 'valid' ] ifNotNil: [
			(#('key_not_held' 'unknown_signature_algorithm') includes: reason)
				ifTrue: [ 'unverified' ]
				ifFalse: [ 'invalid' ] ].
		report value: algorithm value: kid value: status value: reason ]
]

{ #category : #private }
GtsV2SignatureChecks >> reasonFor: value algorithm: algorithm keyId: kid header: header frame: frame [
	| keys key start message signature keyLength signatureLength |
	(#(1 2) includes: algorithm) ifFalse: [ ^ 'unknown_signature_algorithm' ].
	keys := heldKeys at: kid ifAbsent: [ ^ 'key_not_held' ].
	key := keys at: algorithm ifAbsent: [ ^ 'key_mismatch' ].
	start := value byteBodyOfSize: nil.
	keyLength := #(32 1984) at: algorithm.
	signatureLength := #(64 3373) at: algorithm.
	(key size = keyLength and: [ value end - start = signatureLength ]) ifFalse: [ ^ 'value_length' ].
	message := GtsBlake3 deriveWith: 'gts2 signature subject' bytes:
		(GtsCborWriter encode: { algorithm. kid. header. frame }).
	signature := value bytesStartingAt: start.
	algorithm = 1 ifTrue: [ ^ ed25519 failureFor: signature message: message publicKey: key ].
	^ self compositeFailureFor: signature message: message publicKey: key
]

{ #category : #private }
GtsV2SignatureChecks >> key: candidate precedes: existing [
	"With a fixed key ID and algorithm, deterministic CBOR orders by byte-string length, then by octets."
	^ candidate size = existing size
		ifTrue: [ (GtsCose compareBytes: candidate with: existing) < 0 ]
		ifFalse: [ candidate size < existing size ]
]

{ #category : #private }
GtsV2SignatureChecks >> keyId: key algorithm: algorithm [
	^ GtsBlake3 deriveWith: 'gts2 key id' bytes: (GtsCborWriter encode: { algorithm. key })
]

{ #category : #private }
GtsV2SignatureChecks >> compositeFailureFor: signature message: message publicKey: key [
	| label representative failure |
	label := 'COMPSIG-MLDSA65-Ed25519-SHA512' utf8Encoded.
	representative := 'CompositeAlgorithmSignatures2025' utf8Encoded, label, #[0], (GtsSodium sha512: message).
	failure := ed25519 failureFor: (signature copyFrom: 3310 to: 3373)
		message: representative publicKey: (key copyFrom: 1953 to: 1984).
	failure ifNotNil: [ ^ 'composite_classical_failed' ].
	^ (GtsSodium verifyMlDsa65: (signature copyFrom: 1 to: 3309)
		message: representative publicKey: (key copyFrom: 1 to: 1952) context: label)
		ifFalse: [ 'composite_post_quantum_failed' ]
]
