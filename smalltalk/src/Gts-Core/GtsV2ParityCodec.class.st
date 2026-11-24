"SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0"

Class {
	#name : #GtsV2ParityCodec,
	#superclass : #Object,
	#instVars : [ 'powers', 'logs', 'products', 'matrix' ],
	#category : #'Gts-Core'
}

{ #category : #initialization }
GtsV2ParityCodec >> initialize [
	| value |
	super initialize.
	powers := ByteArray new: 255.
	logs := ByteArray new: 256.
	value := 1.
	0 to: 254 do: [ :power |
		powers at: power + 1 put: value.
		logs at: value + 1 put: power.
		value := value bitShift: 1.
		value >= 256 ifTrue: [ value := value bitXor: 285 ] ].
	products := Array new: 256.
	matrix := (0 to: 3) collect: [ :row |
		(0 to: 63) collect: [ :column | self inverse: ((255 - row) bitXor: column) ] ]
]

{ #category : #private }
GtsV2ParityCodec >> multiply: left by: right [
	(left = 0 or: [ right = 0 ]) ifTrue: [ ^ 0 ].
	^ powers at: ((logs at: left + 1) + (logs at: right + 1) \\ 255) + 1
]

{ #category : #private }
GtsV2ParityCodec >> inverse: value [
	value = 0 ifTrue: [ GtsArchiveError signal: 'zero parity field pivot' ].
	^ powers at: ((255 - (logs at: value + 1)) \\ 255) + 1
]

{ #category : #private }
GtsV2ParityCodec >> add: source scaledBy: coefficient to: target [
	| table |
	coefficient = 0 ifTrue: [ ^ self ].
	table := products at: coefficient + 1.
	table ifNil: [
		table := ByteArray new: 256.
		1 to: 255 do: [ :value | table at: value + 1 put: (self multiply: coefficient by: value) ].
		products at: coefficient + 1 put: table ].
	1 to: source size do: [ :index |
		target at: index put: ((target at: index) bitXor: (table at: (source at: index) + 1)) ]
]

{ #category : #recovery }
GtsV2ParityCodec >> solveData: data parity: parity shardSize: size [
	"Use nil for erasures. Omitted data columns contribute zero."
	| missingData missingParity rows recovered restored |
	missingData := (1 to: data size) select: [ :index | (data at: index) isNil ].
	missingParity := (1 to: parity size) select: [ :index | (parity at: index) isNil ].
	missingData size + missingParity size > 4 ifTrue: [ ^ nil ].
	rows := ((1 to: 4) reject: [ :index | missingParity includes: index ]) first: missingData size.
	recovered := data copy.
	missingData ifNotEmpty: [
		self restore: missingData of: recovered rows: rows parity: parity shardSize: size ].
	restored := parity copy.
	missingParity do: [ :row |
		restored at: row put: (self accumulate: recovered row: row into: (ByteArray new: size)) ].
	^ { recovered. restored }
]

{ #category : #private }
GtsV2ParityCodec >> accumulate: data row: row into: target [
	| coefficients |
	coefficients := matrix at: row.
	data withIndexDo: [ :shard :column |
		shard ifNotNil: [ self add: shard scaledBy: (coefficients at: column) to: target ] ].
	^ target
]

{ #category : #private }
GtsV2ParityCodec >> restore: columns of: data rows: rows parity: parity shardSize: size [
	| system residual inverse solved |
	system := rows collect: [ :row | columns collect: [ :column | (matrix at: row) at: column ] ].
	residual := rows collect: [ :row | self accumulate: data row: row into: (parity at: row) copy ].
	inverse := self invert: system.
	solved := inverse collect: [ :coefficients |
		| shard |
		shard := ByteArray new: size.
		coefficients withIndexDo: [ :coefficient :index |
			self add: (residual at: index) scaledBy: coefficient to: shard ].
		shard ].
	columns withIndexDo: [ :column :index | data at: column put: (solved at: index) ]
]

{ #category : #private }
GtsV2ParityCodec >> invert: system [
	| size width augmented |
	size := system size.
	width := size * 2.
	augmented := system withIndexCollect: [ :row :index |
		row asArray, ((1 to: size) collect: [ :column | column = index ifTrue: [ 1 ] ifFalse: [ 0 ] ]) ].
	"Each leading square submatrix is again Cauchy, so pivots cannot vanish."
	1 to: size do: [ :index |
		| current scale |
		current := augmented at: index.
		scale := self inverse: (current at: index).
		1 to: width do: [ :column |
			current at: column put: (self multiply: (current at: column) by: scale) ].
		augmented withIndexDo: [ :row :other |
			| coefficient |
			other ~= index ifTrue: [
				coefficient := row at: index.
				1 to: width do: [ :column |
					row at: column put: ((row at: column) bitXor:
						(self multiply: coefficient by: (current at: column))) ] ] ] ].
	^ augmented collect: [ :row | row allButFirst: size ]
]
