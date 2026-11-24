"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2SegmentWriter,
	#superclass : #Object,
	#instVars : [ 'output', 'range', 'predecessor', 'decodedBytes' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2SegmentWriter >> initialize [
	| header |
	super initialize.
	output := ByteArray new writeStream.
	header := GtsCborWriter encode: #(1 2 1 1 (1) (1) (1)).
	output nextPutAll: header.
	predecessor := GtsBlake3 deriveWith: 'gts2 header' bytes: header.
	range := GtsV2Range new.
	range add: predecessor.
	decodedBytes := 0
]

{ #category : #accessing }
GtsV2SegmentWriter >> bytes [
	^ output contents
]

{ #category : #writing }
GtsV2SegmentWriter >> appendType: type content: content payload: payload [
	(#(1 2 8 11) includes: type) ifFalse: [
		GtsArchiveError signal: 'unsupported RDF import frame type' ].
	self writeType: type content: content payload: payload
]

{ #category : #private }
GtsV2SegmentWriter >> writeType: type content: content payload: payload [
	| encoded identity digest |
	(range leafCount < 67108864 and: [ decodedBytes + payload size <= 68719476736 ])
		ifFalse: [ GtsArchiveError signal: 'GTS v2 segment exceeds reader capacity' ].
	encoded := GtsCborWriter encode: content.
	(payload size <= 67108864 and: [ encoded size <= 67108864 ])
		ifFalse: [ GtsArchiveError signal: 'GTS v2 frame exceeds reader capacity' ].
	digest := GtsBlake3 deriveWith: 'gts2 payload' bytes: payload.
	identity := ByteArray streamContents: [ :stream |
		GtsCborWriter write: type on: stream.
		GtsCborWriter write: type < 14 on: stream.
		GtsCborWriter write: false on: stream.
		stream nextPutAll: encoded.
		GtsCborWriter write: payload size on: stream.
		GtsCborWriter write: digest on: stream ].
	predecessor := GtsBlake3 deriveWith: 'gts2 frame'
		prefix: #[135 88 32], predecessor bytes: identity from: 1 to: identity size + 1.
	output nextPutAll: #[139 2].
	output nextPutAll: identity.
	GtsCborWriter write: #(1) on: output.
	GtsCborWriter write: payload on: output.
	GtsCborWriter write: predecessor on: output.
	GtsCborWriter write: #() on: output.
	range add: predecessor.
	decodedBytes := decodedBytes + payload size
]

{ #category : #writing }
GtsV2SegmentWriter >> commit [
	self writeType: 14 content: { range leafCount - 1. range root } payload: #[]
]
