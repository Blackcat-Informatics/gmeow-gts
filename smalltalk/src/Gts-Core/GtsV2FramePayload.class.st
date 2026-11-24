"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2FramePayload,
	#superclass : #Object,
	#instVars : [ 'decodedLength', 'digest', 'storage', 'stored', 'bodyStart', 'contentSize', 'signaturesSize' ],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsV2FramePayload class >> length: length digest: digest storage: storage stored: stored bodyStart: bodyStart contentSize: contentSize signaturesSize: signaturesSize [
	^ self new initializeLength: length digest: digest storage: storage stored: stored bodyStart: bodyStart contentSize: contentSize signaturesSize: signaturesSize
]

{ #category : #initialization }
GtsV2FramePayload >> initializeLength: length digest: aDigest storage: aStorage stored: aField bodyStart: start contentSize: contentBytes signaturesSize: signatureBytes [
	decodedLength := length.
	digest := aDigest.
	storage := aStorage.
	stored := aField.
	bodyStart := start.
	contentSize := contentBytes.
	signaturesSize := signatureBytes.
	^ self
]

{ #category : #validation }
GtsV2FramePayload >> storedMatches [
	^ storage ifNotNil: [ storage storedMatches: stored ]
]

{ #category : #validation }
GtsV2FramePayload >> checkWith: budget [
	| storedSize refusal computed |
	(decodedLength isNil or: [ digest isNil or: [ storage isNil or: [ bodyStart isNil ] ] ])
		ifTrue: [ ^ self damaged: nil ].
	storedSize := stored end - bodyStart.
	refusal := budget admit: decodedLength stored: storedSize content: contentSize signatures: signaturesSize.
	refusal ifNotNil: [ ^ refusal ].
	storage tag = 1 ifTrue: [
		storedSize = decodedLength ifFalse: [ ^ self damaged: 'decoded_length' ].
		computed := stored deriveWith: 'gts2 payload' prefix: #[] from: bodyStart to: stored end.
		^ self resultForDigest: computed ].
	storage isZstd ifTrue: [ ^ self checkZstd ].
	^ GtsV2PayloadResult state: #unchecked reason: 'unknown_codec' count: nil
]

{ #category : #validation }
GtsV2FramePayload >> checkZstd [
	| header decoded computed |
	decodedLength = 0 ifTrue: [ ^ self damaged: 'codec_constraint' ].
	header := self zstdHeader.
	(stored bytesStartingAt: bodyStart beginWith: header)
		ifFalse: [ ^ self damaged: 'codec_constraint' ].
	decoded := GtsZstd decompressSingleFrame: (stored bytesStartingAt: bodyStart) expectedSize: decodedLength.
	decoded ifNil: [ ^ self damaged: 'decode_failed' ].
	computed := GtsBlake3 deriveWith: 'gts2 payload' prefix: #[] bytes: decoded from: 1 to: decoded size + 1.
	^ self resultForDigest: computed
]

{ #category : #private }
GtsV2FramePayload >> zstdHeader [
	| prefix length width suffix |
	prefix := #[40 181 47 253].
	length := decodedLength.
	length < 256
		ifTrue: [ prefix := prefix, #[32]. width := 1 ]
		ifFalse: [
			length <= 65791
				ifTrue: [ prefix := prefix, #[96]. width := 2. length := length - 256 ]
				ifFalse: [ prefix := prefix, #[160]. width := 4 ] ].
	suffix := ByteArray new: width.
	1 to: width do: [ :index |
		suffix at: index put: ((length bitShift: (1 - index) * 8) bitAnd: 255) ].
	^ prefix, suffix
]

{ #category : #private }
GtsV2FramePayload >> damaged: reason [
	^ GtsV2PayloadResult state: #damaged reason: reason count: nil
]

{ #category : #private }
GtsV2FramePayload >> resultForDigest: computed [
	computed = digest ifFalse: [ ^ self damaged: 'payload_digest' ].
	^ GtsV2PayloadResult state: #verified reason: nil count: nil
]
