Class {
	#name : #GtsWire,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #hashing }
GtsWire class >> contentIdForFrame: frame [
	| preimage |
	preimage := Dictionary new.
	frame associationsDo: [ :association |
		((#('id' 'sig') includes: association key) not)
			ifTrue: [ preimage at: association key put: association value ] ].
	^ GtsBlake3 hash: (GtsCborWriter encode: preimage)
]

{ #category : #hashing }
GtsWire class >> headerIdFor: header [
	| preimage |
	preimage := Dictionary new.
	header associationsDo: [ :association |
		association key = 'id'
			ifFalse: [ preimage at: association key put: association value ] ].
	^ GtsBlake3 hash: (GtsCborWriter encode: preimage)
]

{ #category : #headers }
GtsWire class >> unwrapHeader: item [
	(item isKindOf: GtsCborTag) ifTrue: [
		item tag = 55799 ifFalse: [ self error: 'unexpected CBOR tag on GTS header' ].
		^ item value ].
	^ item
]
