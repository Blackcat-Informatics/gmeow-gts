Class {
	#name : #GtsBlake3Test,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #tests }
GtsBlake3Test >> testDigestStringUsesBlake3Prefix [
	self
		assert: (GtsBlake3 digestString: 'gts' utf8Encoded)
		equals: 'blake3:c397ac220374334c4469a26ce4018a4bde7f573f64873738b75f899879b47dd7'
]

{ #category : #tests }
GtsBlake3Test >> testHashMatchesKnownVectors [
	self
		assert: (GtsBlake3 hexDigest: ByteArray new)
		equals: 'af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262'.
	self
		assert: (GtsBlake3 hexDigest: 'abc' utf8Encoded)
		equals: '6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85'
]

{ #category : #tests }
GtsBlake3Test >> testNativeDigestSize [
	self assert: GtsBlake3 digestSize equals: 32
]
