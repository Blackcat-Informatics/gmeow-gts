Class {
	#name : #GtsMMR,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #private }
GtsMMR class >> expectedPeakHeightsFor: count [
	| heights remaining |
	heights := OrderedCollection new.
	remaining := count.
	[ remaining > 0 ] whileTrue: [
		| height |
		height := 0.
		[ (1 bitShift: height + 1) <= remaining ] whileTrue: [
			height := height + 1 ].
		heights add: height.
		remaining := remaining - (1 bitShift: height) ].
	^ heights asArray
]

{ #category : #private }
GtsMMR class >> leafHashIndex: index frameId: frameId [
	^ GtsBlake3 hash: (GtsCborWriter encode: { 'gts-mmr-leaf-v1'. index. frameId })
]

{ #category : #private }
GtsMMR class >> parentHashHeight: parentHeight left: leftHash right: rightHash [
	^ GtsBlake3 hash: (GtsCborWriter encode: {
		'gts-mmr-parent-v1'.
		parentHeight.
		leftHash.
		rightHash })
]

{ #category : #private }
GtsMMR class >> parseHex32: text [
	| raw |
	raw := text trimBoth.
	(raw beginsWith: 'blake3:') ifTrue: [ raw := raw allButFirst: 7 ].
	raw size = 64 ifFalse: [ GtsProofError signal: 'expected a 32-byte hex value' ].
	^ GtsHex decode: raw
]

{ #category : #private }
GtsMMR class >> peakIndexForLeaf: leafIndex count: count heights: heights [
	| start |
	leafIndex >= count ifTrue: [ GtsProofError signal: 'leaf_index is outside covered count' ].
	start := 0.
	heights withIndexDo: [ :height :index |
		| stop |
		stop := start + (1 bitShift: height).
		((leafIndex >= start) and: [ leafIndex < stop ]) ifTrue: [ ^ index - 1 ].
		start := stop ].
	GtsProofError signal: 'peak ranges do not cover leaf_index'
]

{ #category : #private }
GtsMMR class >> rootHashCount: count peaks: peaks [
	| peakValues |
	peakValues := peaks collect: [ :peak |
		{ peak at: 'height'. peak at: 'hash' } ].
	^ GtsBlake3 hash: (GtsCborWriter encode: {
		'gts-mmr-root-v1'.
		count.
		peakValues asArray })
]

{ #category : #verifying }
GtsMMR class >> verifyProof: proof [
	| actualHeights carried computedPeakIndex count expectedHeights frameId height leafIndex peak peakIndex peaks root |
	(proof at: 'schema') = 'gts-mmr-proof-v1'
		ifFalse: [ GtsProofError signal: 'unsupported proof schema' ].
	(proof at: 'hash') = 'blake3-256'
		ifFalse: [ GtsProofError signal: 'unsupported hash algorithm' ].
	(proof at: 'preimage') = 'gts-mmr-v1'
		ifFalse: [ GtsProofError signal: 'unsupported preimage version' ].
	count := proof at: 'count'.
	leafIndex := proof at: 'leaf_index'.
	peakIndex := proof at: 'peak_index'.
	frameId := self parseHex32: (proof at: 'frame_id').
	root := self parseHex32: (proof at: 'root').
	peaks := (proof at: 'peaks') collect: [ :rawPeak |
		Dictionary newFromPairs: {
			'height'. (rawPeak at: 'height').
			'hash'. (self parseHex32: (rawPeak at: 'hash')) } ].
	leafIndex >= count ifTrue: [ GtsProofError signal: 'leaf_index is outside covered count' ].
	peakIndex >= peaks size ifTrue: [ GtsProofError signal: 'peak_index is out of range' ].
	expectedHeights := self expectedPeakHeightsFor: count.
	actualHeights := peaks collect: [ :each | each at: 'height' ].
	actualHeights asArray = expectedHeights
		ifFalse: [ GtsProofError signal: 'peak heights do not match count' ].
	computedPeakIndex := self peakIndexForLeaf: leafIndex count: count heights: expectedHeights.
	computedPeakIndex = peakIndex
		ifFalse: [ GtsProofError signal: 'leaf_index belongs to a different peak' ].
	carried := self leafHashIndex: leafIndex frameId: frameId.
	height := 0.
	(proof at: 'path') do: [ :step |
		| parentHeight sibling side |
		parentHeight := step at: 'parent_height'.
		parentHeight = (height + 1)
			ifFalse: [ GtsProofError signal: 'path parent height does not follow carried height' ].
		side := step at: 'side'.
		sibling := self parseHex32: (step at: 'hash').
		side = 'left'
			ifTrue: [ carried := self parentHashHeight: parentHeight left: sibling right: carried ]
			ifFalse: [
				side = 'right'
					ifTrue: [ carried := self parentHashHeight: parentHeight left: carried right: sibling ]
					ifFalse: [ GtsProofError signal: 'unsupported proof side' ] ].
		height := parentHeight ].
	peak := peaks at: peakIndex + 1.
	height = (peak at: 'height')
		ifFalse: [ GtsProofError signal: 'path height does not reach peak height' ].
	carried = (peak at: 'hash')
		ifFalse: [ GtsProofError signal: 'proof path does not reconstruct the selected peak' ].
	(self rootHashCount: count peaks: peaks) = root
		ifFalse: [ GtsProofError signal: 'proof peaks do not reconstruct the declared root' ].
	^ Dictionary newFromPairs: {
		'root'. root.
		'frame_id'. frameId }
]

{ #category : #verifying }
GtsMMR class >> verifyProofJson: jsonBytes [
	^ self verifyProof: (STONJSON fromString: jsonBytes utf8Decoded)
]
