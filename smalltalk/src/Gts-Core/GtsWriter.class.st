Class {
	#name : #GtsWriter,
	#superclass : #Object,
	#instVars : [
		'nameToId',
		'previousId',
		'bytes',
		'offsets',
		'types'
	],
	#category : #'Gts-Core'
}

{ #category : #'instance creation' }
GtsWriter class >> forProfile: profileName [
	^ self forProfile: profileName layout: ''
]

{ #category : #'instance creation' }
GtsWriter class >> forProfile: profileName layout: layoutName [
	^ self new initializeForProfile: profileName layout: layoutName
]

{ #category : #adding }
GtsWriter >> addBlob: data mediaType: mediaType representation: representation [
	| pub |
	pub := Dictionary new.
	pub at: 'digest' put: (self class digestString: data).
	mediaType isEmpty ifFalse: [ pub at: 'mt' put: mediaType ].
	representation isEmpty ifFalse: [ pub at: 'rep' put: representation ].
	^ self
		addFrameType: 'blob'
		payload: nil
		raw: data
		transform: #()
		publicMetadata: pub
]

{ #category : #adding }
GtsWriter >> addAnnotations: annotations [
	| rows |
	rows := annotations collect: [ :annotation |
		{ annotation at: 1. annotation at: 2. annotation at: 3 } ].
	^ self addFrameType: 'annot' payload: rows raw: nil transform: #() publicMetadata: nil
]

{ #category : #adding }
GtsWriter >> addFrameType: frameType payload: payload raw: rawBytes transform: transformNames publicMetadata: publicMetadata [
	| data frame frameBytes frameId source |
	((payload notNil) and: [ rawBytes notNil ])
		ifTrue: [ self error: 'payload and raw are mutually exclusive' ].
	frame := Dictionary new.
	frame at: 't' put: frameType.
	transformNames isEmpty
		ifFalse: [
			((rawBytes isNil) and: [ payload isNil ])
				ifTrue: [ self error: 'transform requires data' ].
			transformNames do: [ :name |
				name = 'identity' ifFalse: [ self error: 'non-identity transforms are not supported by Smalltalk writer yet' ] ].
			source := rawBytes ifNil: [ GtsCborWriter encode: payload ].
			frame at: 'x' put: (transformNames collect: [ :name | nameToId at: name ]).
			data := source ]
		ifTrue: [
			rawBytes ifNotNil: [ data := rawBytes ].
			payload ifNotNil: [ data := payload ] ].
	data ifNotNil: [ frame at: 'd' put: data ].
	publicMetadata ifNotNil: [ frame at: 'pub' put: publicMetadata ].
	frame at: 'prev' put: previousId.
	frameId := GtsWire contentIdForFrame: frame.
	frame at: 'id' put: frameId.
	offsets add: bytes size.
	types add: frameType.
	frameBytes := GtsCborWriter encode: frame.
	bytes := bytes, frameBytes.
	previousId := frameId.
	^ frameId
]

{ #category : #adding }
GtsWriter >> addIndex [
	| payload typeIndex |
	typeIndex := Dictionary new.
	types withIndexDo: [ :frameType :index |
		(typeIndex at: frameType ifAbsentPut: [ OrderedCollection new ])
			add: index - 1 ].
	typeIndex keysAndValuesDo: [ :key :value |
		typeIndex at: key put: value asArray ].
	payload := Dictionary newFromPairs: {
		'count'. types size.
		'head'. previousId }.
	offsets isEmpty ifFalse: [
		payload at: 'off' put: offsets asArray.
		payload at: 'ti' put: typeIndex ].
	^ self addFrameType: 'index' payload: payload raw: nil transform: #() publicMetadata: nil
]

{ #category : #adding }
GtsWriter >> addMeta: metadata [
	^ self addFrameType: 'meta' payload: metadata raw: nil transform: #() publicMetadata: nil
]

{ #category : #adding }
GtsWriter >> addQuads: quads [
	| rows |
	rows := quads collect: [ :quad |
		quad size >= 4
			ifTrue: [ { quad at: 1. quad at: 2. quad at: 3. quad at: 4 } ]
			ifFalse: [ { quad at: 1. quad at: 2. quad at: 3 } ] ].
	^ self addFrameType: 'quads' payload: rows raw: nil transform: #() publicMetadata: nil
]

{ #category : #adding }
GtsWriter >> addReifiers: reifiers [
	| payload |
	payload := Dictionary new.
	reifiers do: [ :entry | payload at: (entry at: 'rid') put: (entry at: 'spo') ].
	^ self addFrameType: 'reifies' payload: payload raw: nil transform: #() publicMetadata: nil
]

{ #category : #adding }
GtsWriter >> addSuppressTargets: targets reason: reason by: by [
	| payload |
	payload := Dictionary new.
	payload at: 'targets' put: targets.
	reason ifNotNil: [ reason isEmpty ifFalse: [ payload at: 'reason' put: reason ] ].
	by ifNotNil: [ payload at: 'by' put: by ].
	^ self addFrameType: 'suppress' payload: payload raw: nil transform: #() publicMetadata: nil
]

{ #category : #adding }
GtsWriter >> addTerms: terms [
	^ self
		addFrameType: 'terms'
		payload: (terms collect: [ :term | self wireTermFor: term ])
		raw: nil
		transform: #()
		publicMetadata: nil
]

{ #category : #accessing }
GtsWriter >> bytes [
	^ bytes copy
]

{ #category : #initialization }
GtsWriter >> initializeForProfile: profileName layout: layoutName [
	| catalog header |
	nameToId := Dictionary newFromPairs: {
		'identity'. 0.
		'gzip'. 1.
		'zstd'. 2.
		'zstd-rsyncable'. 3.
		'cose-encrypt0'. 7 }.
	catalog := Dictionary newFromPairs: {
		0. (self codecNamed: 'identity' classifiedAs: 'encode').
		1. (self codecNamed: 'gzip' classifiedAs: 'compress').
		2. (self codecNamed: 'zstd' classifiedAs: 'compress').
		3. (self codecNamed: 'zstd-rsyncable' classifiedAs: 'compress').
		7. (self codecNamed: 'cose-encrypt0' classifiedAs: 'encrypt') }.
	header := Dictionary newFromPairs: {
		'gts'. 'GTS1'.
		'v'. 1.
		'prof'. profileName.
		'cat'. catalog }.
	layoutName isEmpty ifFalse: [ header at: 'layout' put: layoutName ].
	header at: 'id' put: (GtsWire headerIdFor: header).
	previousId := header at: 'id'.
	bytes := GtsCborWriter encodeSelfDescribed: header.
	offsets := OrderedCollection new.
	types := OrderedCollection new.
	^ self
]

{ #category : #private }
GtsWriter >> codecNamed: codecName classifiedAs: codecClass [
	^ Dictionary newFromPairs: {
		'name'. codecName.
		'cls'. codecClass }
]

{ #category : #private }
GtsWriter >> wireTermFor: term [
	| fields |
	fields := Dictionary new.
	term kind = #literal
		ifTrue: [ fields at: 'k' put: 1 ]
		ifFalse: [
			term kind = #blankNode
				ifTrue: [ fields at: 'k' put: 2 ]
				ifFalse: [
					term kind = #triple
						ifTrue: [ fields at: 'k' put: 3 ]
						ifFalse: [ fields at: 'k' put: 0 ] ] ].
	((term value notNil and: [ term value isEmpty not ]) or: [ term kind = #literal ])
		ifTrue: [ fields at: 'v' put: term value ].
	term datatype ifNotNil: [ fields at: 'dt' put: term datatype ].
	(term language notNil and: [ term language isEmpty not ])
		ifTrue: [ fields at: 'l' put: term language ].
	(term direction notNil and: [ term direction isEmpty not ])
		ifTrue: [ fields at: 'dir' put: term direction ].
	term reifier ifNotNil: [ fields at: 'rf' put: term reifier ].
	^ fields
]

{ #category : #digesting }
GtsWriter class >> digestString: data [
	^ GtsBlake3 digestString: data
]
