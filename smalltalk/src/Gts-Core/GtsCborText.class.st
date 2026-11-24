"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsCborText,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #validation }
GtsCborText class >> isValidUtf8: bytes from: start to: limit [
	"True if bytes are well-formed UTF-8 by the Unicode table of well-formed byte
	sequences: no overlong form, no encoded surrogate, nothing above U+10FFFF, and
	no stray continuation byte or truncated sequence. Explicit, so it does not
	depend on how a particular decoder treats these cases."

	| i n b need lo hi |
	i := start.
	n := limit - 1.
	[ i <= n ] whileTrue: [
		b := bytes at: i.
		b < 16r80
			ifTrue: [ i := i + 1 ]
			ifFalse: [
				need := nil.
				lo := 16r80.
				hi := 16rBF.
				(b between: 16rC2 and: 16rDF) ifTrue: [ need := 1 ].
				(b between: 16rE0 and: 16rEF) ifTrue: [ need := 2 ].
				(b between: 16rF0 and: 16rF4) ifTrue: [ need := 3 ].
				need isNil ifTrue: [ ^ false ].
				b = 16rE0 ifTrue: [ lo := 16rA0 ].
				b = 16rED ifTrue: [ hi := 16r9F ].
				b = 16rF0 ifTrue: [ lo := 16r90 ].
				b = 16rF4 ifTrue: [ hi := 16r8F ].
				i + need > n ifTrue: [ ^ false ].
				((bytes at: i + 1) between: lo and: hi) ifFalse: [ ^ false ].
				2 to: need do: [ :k |
					((bytes at: i + k) between: 16r80 and: 16rBF) ifFalse: [ ^ false ] ].
				i := i + need + 1 ] ].
	^ true
]
