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
		| kid |
		kid := GtsHex decode: entry key.
		kid size = 32 ifFalse: [ GtsArchiveError signal: 'GTS v2 --key needs a 32-byte hexadecimal key identifier' ].
		(heldKeys at: kid ifAbsentPut: [ Set new ]) add: entry value ].
	heldKeys keysAndValuesDo: [ :kid :keys |
		heldKeys at: kid put: ((keys select: [ :key | (self keyId: key) = kid ]) asArray
			sort: [ :left :right |
				left size = right size
					ifTrue: [ (GtsCose compareBytes: left with: right) <= 0 ]
					ifFalse: [ left size < right size ] ]) ].
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
	| keys key start message |
	(#(1 2) includes: algorithm) ifFalse: [ ^ 'unknown_signature_algorithm' ].
	keys := heldKeys at: kid ifAbsent: [ ^ 'key_not_held' ].
	(algorithm ~= 1 or: [ keys isEmpty ]) ifTrue: [ ^ 'key_mismatch' ].
	key := keys first.
	start := value byteBodyOfSize: nil.
	(key size = 32 and: [ value end - start = 64 ]) ifFalse: [ ^ 'value_length' ].
	message := GtsBlake3 deriveWith: 'gts2 signature subject' bytes:
		(GtsCborWriter encode: { algorithm. kid. header. frame }).
	^ ed25519 failureFor: (value bytesStartingAt: start) message: message publicKey: key
]

{ #category : #private }
GtsV2SignatureChecks >> keyId: key [
	^ GtsBlake3 deriveWith: 'gts2 key id' bytes: (GtsCborWriter encode: { 1. key })
]
