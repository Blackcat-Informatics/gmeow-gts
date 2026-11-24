"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2Ed25519,
	#superclass : #Object,
	#instVars : [ 'prime', 'order', 'signBit', 'curve' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2Ed25519 >> initialize [
	super initialize.
	signBit := 1 bitShift: 255.
	prime := signBit - 19.
	order := (1 bitShift: 252) + 27742317777372353535851937790883648493.
	curve := 37095705934669439343138083508754565189542113879843219016388785533085940283555
]

{ #category : #verification }
GtsV2Ed25519 >> failureFor: signature message: message publicKey: key [
	| scalar nonce encodedNonce encodedKey point challenge multiple recovered |
	scalar := self littleEndian: (signature copyFrom: 33 to: 64).
	scalar >= order ifTrue: [ ^ 'non_canonical_scalar' ].
	encodedKey := self littleEndian: key.
	(self isSmallOrder: encodedKey) ifTrue: [ ^ 'small_order_key' ].
	nonce := signature copyFrom: 1 to: 32.
	encodedNonce := self littleEndian: nonce.
	(self isSmallOrder: encodedNonce) ifTrue: [ ^ 'small_order_nonce' ].
	point := self decodePoint: encodedKey.
	point ifNil: [ ^ 'equation_failed' ].
	challenge := (self littleEndian: (GtsSodium sha512: nonce, key, message)) \\ order.
	multiple := self multiply: point by: challenge.
	recovered := self add: (self multiply: self basePoint by: scalar)
		to: { multiple first negated. multiple second. multiple third. multiple fourth negated }.
	^ (self encodePoint: recovered) = encodedNonce ifFalse: [ 'equation_failed' ]
]

{ #category : #private }
GtsV2Ed25519 >> littleEndian: bytes [
	| result |
	result := 0.
	bytes size to: 1 by: -1 do: [ :index |
		result := (result bitShift: 8) + (bytes at: index) ].
	^ result
]

{ #category : #private }
GtsV2Ed25519 >> isSmallOrder: encoded [
	| y |
	y := encoded bitAnd: signBit - 1.
	^ y = 0 or: [
		encoded = 1 or: [ encoded = (prime - 1) or: [
			y = 2707385501144840649318225287225658788936804267575313519463743609750303402022 or: [
			y = 55188659117513257062467267217118295137698188065244968500265048394206261417927 ] ] ] ]
]

{ #category : #private }
GtsV2Ed25519 >> decodePoint: encoding [
	| y sign squareY denominator squareX x |
	y := encoding bitAnd: signBit - 1.
	sign := encoding bitShift: -255.
	y >= prime ifTrue: [ ^ nil ].
	squareY := y * y \\ prime.
	denominator := (curve * squareY + 1) \\ prime.
	squareX := (squareY - 1) * (self inverse: denominator) \\ prime.
	x := squareX raisedTo: (prime + 3) // 8 modulo: prime.
	x * x \\ prime = squareX ifFalse: [
		x := x * 19681161376707505956807079304988542015446066515923890162744021073123829784752 \\ prime ].
	(x * x \\ prime ~= squareX or: [ x = 0 and: [ sign = 1 ] ]) ifTrue: [ ^ nil ].
	(x bitAnd: 1) = sign ifFalse: [ x := prime - x ].
	^ { x. y. 1. x * y \\ prime }
]

{ #category : #private }
GtsV2Ed25519 >> encodePoint: point [
	| affine sign |
	affine := self inverse: point third.
	sign := (point first * affine \\ prime) bitAnd: 1.
	^ (point second * affine \\ prime) + (sign * signBit)
]

{ #category : #private }
GtsV2Ed25519 >> inverse: value [
	^ (value \\ prime) raisedTo: prime - 2 modulo: prime
]

{ #category : #private }
GtsV2Ed25519 >> basePoint [
	| x y |
	x := 15112221349535400772501151409588531511454012693041857206046113283949847762202.
	y := 46316835694926478169428394003475163141307993866256225615783033603165251855960.
	^ { x. y. 1. x * y \\ prime }
]

{ #category : #private }
GtsV2Ed25519 >> multiply: point by: scalar [
	| result addend remaining |
	"These operations take public inputs only."
	result := #(0 1 1 0).
	addend := point.
	remaining := scalar.
	[ remaining > 0 ] whileTrue: [
		remaining odd ifTrue: [ result := self add: result to: addend ].
		addend := self add: addend to: addend.
		remaining := remaining bitShift: -1 ].
	^ result
]

{ #category : #private }
GtsV2Ed25519 >> add: left to: right [
	| a b c d e f g h |
	"Complete Edwards addition on extended coordinates (RFC 8032 section 6)."
	a := (left second - left first) * (right second - right first).
	b := (left second + left first) * (right second + right first).
	c := 2 * curve * left fourth * right fourth.
	d := 2 * left third * right third.
	e := b - a.
	f := d - c.
	g := d + c.
	h := b + a.
	^ { e * f \\ prime. g * h \\ prime. f * g \\ prime. e * h \\ prime }
]
