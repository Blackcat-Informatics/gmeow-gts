Class {
	#name : #GtsCLI,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #running }
GtsCLI class >> run: args [
	args isEmpty ifTrue: [ self stderr: self usage. ^ 2 ].
	[
		| command |
		command := args first.
		command = 'fold' ifTrue: [ ^ self runFold: args allButFirst ].
		command = 'pack' ifTrue: [ ^ self runPack: args allButFirst ].
		command = 'unpack' ifTrue: [ ^ self runUnpack: args allButFirst ].
		command = 'diff' ifTrue: [ ^ self runDiff: args allButFirst ].
		command = 'heads' ifTrue: [ ^ self runHeads: args allButFirst ].
		command = 'segments' ifTrue: [ ^ self runSegments: args allButFirst ].
		command = 'missing' ifTrue: [ ^ self runMissing: args allButFirst ].
		command = 'resume' ifTrue: [ ^ self runResume: args allButFirst ].
		command = 'info' ifTrue: [ ^ self runInfo: args allButFirst ].
		command = 'verify' ifTrue: [ ^ self runInfo: args allButFirst ].
		command = 'ls' ifTrue: [ ^ self runLs: args allButFirst ].
		command = 'extract' ifTrue: [ ^ self runExtract: args allButFirst ].
		command = 'cat' ifTrue: [ ^ self runCat: args allButFirst ].
		self stderr: 'gts: unknown command ', command, String lf, self usage.
		^ 2 ]
		on: Error
		do: [ :error |
			self stderr: 'gts: ', error messageText, String lf.
			^ 1 ]
]

{ #category : #commands }
GtsCLI class >> runCat: args [
	| outPath paths |
	outPath := nil.
	paths := OrderedCollection new.
	1 to: args size do: [ :index |
		((args at: index) = '-o' or: [ (args at: index) = '--out' ])
			ifTrue: [ outPath := args at: index + 1 ifAbsent: [ ^ 2 ] ]
			ifFalse: [
				(index = 1 or: [ ((args at: index - 1) ~= '-o') and: [ (args at: index - 1) ~= '--out' ] ])
					ifTrue: [ paths add: (args at: index) ] ] ].
	(outPath isNil or: [ paths isEmpty ]) ifTrue: [ self stderr: self usage. ^ 2 ].
	self writeFile: outPath bytes: (ByteArray streamContents: [ :stream |
		paths do: [ :path |
			| data graph |
			data := self readFile: path.
			graph := GtsReader read: data.
			graph diagnostics isEmpty ifFalse: [ self error: 'cat refuses input with diagnostics' ].
			stream nextPutAll: data ] ]).
	^ 0
]

{ #category : #commands }
GtsCLI class >> runDiff: args [
	| graph lines |
	args size = 2 ifFalse: [ self stderr: self usage. ^ 2 ].
	graph := GtsReader read: (self readFile: args first).
	lines := GtsFiles diffGraph: graph withDirectory: args second.
	lines do: [ :line | self stdout: line, String lf ].
	^ lines isEmpty ifTrue: [ 0 ] ifFalse: [ 1 ]
]

{ #category : #commands }
GtsCLI class >> runExtract: args [
	| digest graph outPath |
	args size < 2 ifTrue: [ self stderr: self usage. ^ 2 ].
	outPath := nil.
	graph := GtsReader read: (self readFile: args first).
	digest := args second.
	3 to: args size do: [ :index |
		((args at: index) = '-o' or: [ (args at: index) = '--out' ])
			ifTrue: [ outPath := args at: index + 1 ifAbsent: [ ^ 2 ] ] ].
	outPath
		ifNil: [ self stdoutBytes: (graph blobs at: digest) ]
		ifNotNil: [ self writeFile: outPath bytes: (graph blobs at: digest) ].
	^ 0
]

{ #category : #commands }
GtsCLI class >> runFold: args [
	| graph |
	args size = 1 ifFalse: [ self stderr: self usage. ^ 2 ].
	graph := GtsReader read: (self readFile: args first).
	self stdout: (GtsNQuads toNQuads: graph).
	^ (graph diagnostics isEmpty and: [ graph segmentHeads notEmpty ])
		ifTrue: [ 0 ]
		ifFalse: [ 1 ]
]

{ #category : #commands }
GtsCLI class >> runHeads: args [
	| data inventory |
	args size = 1 ifFalse: [ self stderr: self usage. ^ 2 ].
	data := self readFile: args first.
	inventory := GtsReplication inventoryFor: data.
	self stdout: (GtsReplication headsJsonFor: inventory).
	^ 0
]

{ #category : #commands }
GtsCLI class >> runInfo: args [
	| problems |
	args isEmpty ifTrue: [ self stderr: self usage. ^ 2 ].
	problems := false.
	args do: [ :path |
		| graph |
		graph := GtsReader read: (self readFile: path).
		self stdout: path, ': ', graph segmentHeads size asString, ' segment(s)', String lf.
		graph segmentHeads withIndexDo: [ :head :index |
			self stdout: '  segment ', (index - 1) asString,
				': head ', (GtsHex encode: head),
				' profile ', (graph profiles at: index ifAbsent: [ 'generic' ]),
				' terms ', graph terms size asString,
				' quads ', graph quads size asString,
				' blobs ', graph blobs size asString,
				' suppress ', graph suppressions size asString,
				' opaque ', graph opaqueReasons size asString,
				String lf ].
		graph diagnostics isEmpty ifFalse: [ problems := true ] ].
	^ problems ifTrue: [ 1 ] ifFalse: [ 0 ]
]

{ #category : #commands }
GtsCLI class >> runLs: args [
	args isEmpty ifTrue: [ self stderr: self usage. ^ 2 ].
	args do: [ :path |
		| graph |
		graph := GtsReader read: (self readFile: path).
		graph blobs keys asSortedCollection do: [ :digest |
			| mediaType meta |
			meta := graph blobMeta at: digest ifAbsent: [ Dictionary new ].
			mediaType := meta at: 'mt' ifAbsent: [ '' ].
			self stdout: digest, ' ', (graph blobs at: digest) size asString, ' ', mediaType, String lf ] ].
	^ 0
]

{ #category : #commands }
GtsCLI class >> runMissing: args [
	| data fromHead inventory result |
	(args size = 3 and: [ args first = '--from-head' ]) ifFalse: [ self stderr: self usage. ^ 2 ].
	fromHead := GtsHex decode: args second.
	data := self readFile: args third.
	inventory := GtsReplication inventoryFor: data.
	result := GtsReplication missingFromHead: fromHead in: inventory.
	self stdout: (GtsReplication missingJson: result).
	^ 0
]

{ #category : #commands }
GtsCLI class >> runPack: args [
	| outPath sources |
	outPath := nil.
	sources := OrderedCollection new.
	1 to: args size do: [ :index |
		((args at: index) = '-o' or: [ (args at: index) = '--out' ])
			ifTrue: [ outPath := args at: index + 1 ifAbsent: [ ^ 2 ] ]
			ifFalse: [
				(index = 1 or: [ ((args at: index - 1) ~= '-o') and: [ (args at: index - 1) ~= '--out' ] ])
					ifTrue: [ sources add: (args at: index) ] ] ].
	(outPath isNil or: [ sources isEmpty ]) ifTrue: [ self stderr: self usage. ^ 2 ].
	self writeFile: outPath bytes: (GtsFiles packSources: sources asArray).
	^ 0
]

{ #category : #commands }
GtsCLI class >> runResume: args [
	| data frameId |
	(args size = 3 and: [ args first = '--after' ]) ifFalse: [ self stderr: self usage. ^ 2 ].
	frameId := GtsHex decode: args second.
	data := self readFile: args third.
	self stdoutBytes: (GtsReplication resumeBytesAfter: frameId in: data).
	^ 0
]

{ #category : #commands }
GtsCLI class >> runSegments: args [
	| data inventory |
	args size = 1 ifFalse: [ self stderr: self usage. ^ 2 ].
	data := self readFile: args first.
	inventory := GtsReplication inventoryFor: data.
	self stdout: (GtsReplication segmentsJsonFor: inventory).
	^ 0
]

{ #category : #commands }
GtsCLI class >> runUnpack: args [
	| destination graph path |
	destination := '.'.
	path := nil.
	1 to: args size do: [ :index |
		((args at: index) = '-C' or: [ (args at: index) = '--directory' ])
			ifTrue: [ destination := args at: index + 1 ifAbsent: [ ^ 2 ] ]
			ifFalse: [
				(index = 1 or: [ ((args at: index - 1) ~= '-C') and: [ (args at: index - 1) ~= '--directory' ] ])
					ifTrue: [ path := args at: index ] ] ].
	path ifNil: [ self stderr: self usage. ^ 2 ].
	graph := GtsReader read: (self readFile: path).
	GtsFiles unpackGraph: graph to: destination includeSuppressed: false.
	^ 0
]

{ #category : #io }
GtsCLI class >> readFile: path [
	^ path asFileReference binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #io }
GtsCLI class >> stderr: text [
	Stdio stderr nextPutAll: text
]

{ #category : #io }
GtsCLI class >> stdout: text [
	Stdio stdout nextPutAll: text
]

{ #category : #io }
GtsCLI class >> stdoutBytes: bytes [
	Stdio stdout nextPutAll: bytes
]

{ #category : #usage }
GtsCLI class >> usage [
	^ 'usage: gts <command> [args]', String lf
]

{ #category : #io }
GtsCLI class >> writeFile: path bytes: bytes [
	| target |
	target := path asFileReference.
	target parent ensureCreateDirectory.
	target exists ifFalse: [ target ensureCreateFile ].
	target binaryWriteStreamDo: [ :stream | stream nextPutAll: bytes ]
]
