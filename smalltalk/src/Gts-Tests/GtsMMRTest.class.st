Class {
	#name : #GtsMMRTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #tests }
GtsMMRTest >> testBadRootProofFails [
	self
		should: [
			GtsMMR verifyProofJson: ('/workspace/vectors/proofs/mmr-basic-proof-bad-root.json' asFileReference
				binaryReadStreamDo: [ :stream | stream upToEnd ]) ]
		raise: Error
]

{ #category : #tests }
GtsMMRTest >> testBasicProofVerifies [
	| proof |
	proof := GtsMMR verifyProofJson: ('/workspace/vectors/proofs/mmr-basic-proof.json' asFileReference
		binaryReadStreamDo: [ :stream | stream upToEnd ]).
	self
		assert: (GtsHex encode: (proof at: 'frame_id'))
		equals: '0303030303030303030303030303030303030303030303030303030303030303'.
	self
		assert: (GtsHex encode: (proof at: 'root'))
		equals: 'c24901580895f06da6f51ee3d1ec215890c2ecb611a9e5729579bf967717a738'
]
