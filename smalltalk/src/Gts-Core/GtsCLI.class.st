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
		command = 'verify-proof' ifTrue: [ ^ self runVerifyProof: args allButFirst ].
		command = 'extract-key' ifTrue: [ ^ self runExtractKey: args allButFirst ].
		command = 'from-nq' ifTrue: [ ^ self runFromNQuads: args allButFirst ].
		command = 'info' ifTrue: [ ^ self runInfo: args allButFirst ].
		command = 'verify' ifTrue: [ ^ self runVerify: args allButFirst ].
		command = 'ls' ifTrue: [ ^ self runLs: args allButFirst ].
		command = 'extract' ifTrue: [ ^ self runExtract: args allButFirst ].
		command = 'cat' ifTrue: [ ^ self runCat: args allButFirst ].
		command = 'compact' ifTrue: [ ^ self runCompact: args allButFirst ].
		self stderr: (self unknownCommandMessage: command).
		^ 2 ]
		on: Error
		do: [ :error |
			self stderr: 'gts: ', error messageText, String lf.
			^ 1 ]
]

{ #category : #commands }
GtsCLI class >> runCat: args [
	| combined outPath paths |
	outPath := nil.
	paths := OrderedCollection new.
	1 to: args size do: [ :index |
		((args at: index) = '-o' or: [ (args at: index) = '--out' ])
			ifTrue: [ outPath := args at: index + 1 ifAbsent: [ ^ 2 ] ]
			ifFalse: [
				(index = 1 or: [ ((args at: index - 1) ~= '-o') and: [ (args at: index - 1) ~= '--out' ] ])
					ifTrue: [ paths add: (args at: index) ] ] ].
	(outPath isNil or: [ paths size < 2 ]) ifTrue: [ self stderr: self usage. ^ 2 ].
	combined := ByteArray streamContents: [ :stream |
		paths do: [ :path |
			| data graph |
			data := self readFile: path.
			graph := GtsReader read: data.
			(self graphIsClean: graph) ifFalse: [
				self stderr: 'gts: refusing ', path, ': not a clean GTS input', String lf.
				^ 1 ].
			(self graphContributes: graph) ifFalse: [
				self stderr: 'gts: refusing ', path, ': segment folds to nothing', String lf.
				^ 1 ].
			stream nextPutAll: data ] ].
	(self allQuadsSuppressed: (GtsReader read: combined)) ifTrue: [
		self stderr: 'gts: refusing composition: suppressions hide every quad in the folded output', String lf.
		^ 1 ].
	self writeFile: outPath bytes: combined.
	^ 0
]

{ #category : #commands }
GtsCLI class >> runCompact: args [
	| outPath path sealOriginal streamable timestamp |
	outPath := nil.
	path := nil.
	sealOriginal := false.
	streamable := false.
	timestamp := '2026-01-01T00:00:00Z'.
	1 to: args size do: [ :index |
		(args at: index) = '--streamable' ifTrue: [ streamable := true ].
		(args at: index) = '--seal-original' ifTrue: [ sealOriginal := true ].
		((args at: index) = '-o' or: [ (args at: index) = '--out' ])
			ifTrue: [ outPath := args at: index + 1 ifAbsent: [ ^ 2 ] ].
		(args at: index) = '--timestamp'
			ifTrue: [ timestamp := args at: index + 1 ifAbsent: [ ^ 2 ] ].
		(((args at: index) beginsWith: '-') not
			and: [ index = 1 or: [
				(args at: index - 1) ~= '-o'
					and: [ (args at: index - 1) ~= '--out'
					and: [ (args at: index - 1) ~= '--timestamp' ] ] ] ])
			ifTrue: [ path := args at: index ] ].
	(path isNil or: [ outPath isNil ]) ifTrue: [ self stderr: self usage. ^ 2 ].
	streamable ifFalse: [
		self stderr: 'gts: compact requires --streamable', String lf.
		^ 2 ].
	[
		self
			writeFile: outPath
			bytes: (GtsCompact
				compactStreamable: (self readFile: path)
				timestamp: timestamp
				sealOriginal: sealOriginal).
		^ 0 ]
		on: Error
		do: [ :error |
			self stderr: 'gts: refusing compact: ', error messageText, String lf.
			^ 1 ]
]

{ #category : #commands }
GtsCLI class >> runDiff: args [
	| graph lines |
	args size = 2 ifFalse: [ self stderr: self usage. ^ 2 ].
	graph := GtsReader read: (self readFile: args first).
	self printDiagnosticsFor: graph.
	(self graphIsClean: graph) ifFalse: [
		self stderr: 'gts: refusing diff: archive did not read cleanly', String lf.
		^ 1 ].
	lines := GtsFiles diffGraph: graph withDirectory: args second.
	lines do: [ :line | self stdout: line, String lf ].
	^ lines isEmpty ifTrue: [ 0 ] ifFalse: [ 1 ]
]

{ #category : #commands }
GtsCLI class >> runExtract: args [
	| data digest graph includeSuppressed index mediaType outPath paths |
	outPath := nil.
	mediaType := ''.
	includeSuppressed := false.
	paths := OrderedCollection new.
	index := 1.
	[ index <= args size ] whileTrue: [
		| arg |
		arg := args at: index.
		((arg = '-o') or: [ arg = '--out' ])
			ifTrue: [
				index = args size ifTrue: [ self stderr: self usage. ^ 2 ].
				outPath := args at: index + 1.
				index := index + 2 ]
			ifFalse: [
				arg = '--mt'
					ifTrue: [
						index = args size ifTrue: [ self stderr: self usage. ^ 2 ].
						mediaType := args at: index + 1.
						index := index + 2 ]
					ifFalse: [
						arg = '--include-suppressed'
							ifTrue: [
								includeSuppressed := true.
								index := index + 1 ]
							ifFalse: [
								paths add: arg.
								index := index + 1 ] ] ] ].
	paths size = 2 ifFalse: [ self stderr: self usage. ^ 2 ].
	graph := GtsReader read: (self readFile: paths first).
	self printDiagnosticsFor: graph.
	(self graphIsClean: graph) ifFalse: [
		self stderr: 'gts: refusing extract: archive did not read cleanly', String lf.
		^ 1 ].
	digest := GtsFiles normalizeDigest: paths second.
	data := graph blobs
		at: digest
		ifAbsent: [
			self stderr: 'gts: no inline blob ', digest, ' in ', paths first, String lf.
			^ 1 ].
	(includeSuppressed not and: [ (GtsFiles suppressedBlobDigestsIn: graph) includesKey: digest ])
		ifTrue: [
			self stderr: 'gts: refusing ', digest, ': suppressed; pass --include-suppressed to extract anyway', String lf.
			^ 1 ].
	(mediaType isEmpty not and: [ (GtsFiles mediaTypeForDigest: digest in: graph) ~= mediaType ])
		ifTrue: [
			self stderr: 'gts: refusing ', digest, ': declared media type does not match asserted ', mediaType, String lf.
			^ 1 ].
	(GtsWriter digestString: data) = digest ifFalse: [
		self stderr: 'gts: integrity failure: ', digest, ' bytes re-hash differently', String lf.
		^ 1 ].
	outPath
		ifNil: [ self stdoutBytes: data ]
		ifNotNil: [ self writeFile: outPath bytes: data ].
	^ 0
]

{ #category : #commands }
GtsCLI class >> runExtractKey: args [
	| graph |
	args size = 1 ifFalse: [ self stderr: self usage. ^ 2 ].
	graph := GtsReader read: (self readFile: args first).
	self stdout: (GtsOpenPGP extractKeyTextFromGraph: graph).
	^ 0
]

{ #category : #commands }
GtsCLI class >> runFold: args [
	| graph bytes wire whole |
	whole := args size = 2 and: [ args second = '--whole' ].
	(args size = 1 or: [ whole ]) ifFalse: [ self stderr: self usage. ^ 2 ].
	bytes := self readFile: args first.
	wire := GtsV2Wire inspect: bytes.
	wire format = #v2 ifTrue: [
		graph := GtsV2RdfFold read: wire visible: whole not.
		self stdout: (GtsNQuads toNQuads: graph).
		^ 0 ].
	wire format = #unsupported ifTrue: [
		self stderr: 'gts fold: unsupported GTS generation ', wire generation printString, String lf.
		^ 1 ].
	whole ifTrue: [ self stderr: 'gts fold: --whole requires GTS v2', String lf. ^ 2 ].
	graph := GtsReader read: bytes.
	self stdout: (GtsNQuads toNQuads: graph).
	^ (graph diagnostics isEmpty and: [ graph segmentHeads notEmpty ])
		ifTrue: [ 0 ]
		ifFalse: [ 1 ]
]

{ #category : #commands }
GtsCLI class >> runFromNQuads: args [
	| outPath paths text bytes |
	outPath := nil.
	paths := OrderedCollection new.
	1 to: args size do: [ :index |
		((args at: index) = '-o' or: [ (args at: index) = '--out' ])
			ifTrue: [ outPath := args at: index + 1 ifAbsent: [ ^ 2 ] ]
			ifFalse: [
				(index = 1 or: [ ((args at: index - 1) ~= '-o') and: [ (args at: index - 1) ~= '--out' ] ])
					ifTrue: [ paths add: (args at: index) ] ] ].
	paths size = 1 ifFalse: [ self stderr: self usage. ^ 2 ].
	text := self readText: paths first.
	bytes := GtsFromNQuads v2FromString: text.
	outPath
		ifNil: [ self stdoutBytes: bytes ]
		ifNotNil: [ self writeFile: outPath bytes: bytes ].
	^ 0
]

{ #category : #commands }
GtsCLI class >> runHeads: args [
	| data inventory |
	args size = 1 ifFalse: [ self stderr: self usage. ^ 2 ].
	data := self readFile: args first.
	inventory := GtsReplication inventoryFor: data.
	self stdout: (GtsReplication headsJsonFor: inventory).
	^ (GtsReplication hasProblems: inventory) ifTrue: [ 1 ] ifFalse: [ 0 ]
]

{ #category : #commands }
GtsCLI class >> runInfo: args [
	| problems |
	args isEmpty ifTrue: [ self stderr: self usage. ^ 2 ].
	problems := false.
	args do: [ :path |
		| data wire clean |
		data := self readFile: path.
		wire := GtsV2Wire inspect: data.
		clean := wire format = #legacy
			ifTrue: [
				| graph |
				graph := GtsReader read: data.
				self printLedgerFor: path graph: graph.
				graph diagnostics isEmpty ]
			ifFalse: [ self printWireInfoFor: path wire: wire ].
		clean ifFalse: [ problems := true ] ].
	^ problems ifTrue: [ 1 ] ifFalse: [ 0 ]
]

{ #category : #printing }
GtsCLI class >> printWireInfoFor: path wire: wire [
	| format ordinal clean |
	format := wire format.
	format ~= #v2 ifTrue: [
		wire initialResyncRefusal ifNotNil: [ :refusal |
			self stderr: path, ': ', (self resyncRefusalMessageFor: refusal), String lf ] ].
	format = #empty ifTrue: [
		self stdout: path, ': empty input, 0 segments', String lf.
		^ false ].
	format = #framingError ifTrue: [
		self stdout: path, ': framing stopped at byte 0: ', wire stopReason, String lf.
		^ false ].
	format = #unsupported ifTrue: [
		self stderr: 'gts info: ', path, ': unsupported GTS generation ', wire generation printString, String lf.
		^ false ].
	format = #unknown ifTrue: [
		self stderr: 'gts info: ', path, ': input does not identify a GTS generation', String lf.
		^ false ].
	self stdout: path, ': GTS v2 framing ledger (byte ranges, frame integrity and closing records)', String lf.
	ordinal := 0.
	clean := true.
	wire eventsDo: [ :event |
		(event problem isNotNil or: [ event closingClaimFailure isNotNil ]) ifTrue: [ clean := false ].
		event kind = #stop
			ifTrue: [
				self stdout: '  framing stopped at byte ', event offset printString, ': ', event problem, String lf ]
			ifFalse: [
				| prefix kind details diagnostic |
				prefix := '    '.
				event startsSegment ifTrue: [
					prefix := '  segment ', ordinal printString, ': '.
					ordinal := ordinal + 1 ].
				kind := event frameType
					ifNil: [ event kind asString ]
					ifNotNil: [ :type | 'frame type ', type printString ].
				details := event opener ifNil: [ '' ] ifNotNil: [ :opener |
					', registry ', opener registry printString, ', hash algorithm ', opener hashAlgorithm printString ].
				event scanned ifTrue: [ details := details, ', found by recovery scan' ].
				diagnostic := self wireDiagnosticFor: event.
				diagnostic isNotEmpty ifTrue: [ clean := false ].
				self stdout: prefix, kind, ' bytes [', event offset printString, ', ',
					event end printString, ')', details, diagnostic, String lf.
				event link ifNotNil: [
					(self printWireIntegrityFor: event) ifFalse: [ clean := false ] ].
				(event opener isNotNil and: [ event opener registry > 1 ]) ifTrue: [
					clean := false.
					self stdout: '    unsupported registry version ', event opener registry printString, String lf ] ] ]
		decodedWhen: [ :event | false ]
		unprotectedTailsDo: [ :offset :length |
			self stdout: '    ', (self unprotectedTailAt: offset length: length), String lf ].
	^ clean
]

{ #category : #printing }
GtsCLI class >> wireDiagnosticFor: event [
	^ (self wireFindingFor: event) ifNil: [ '' ] ifNotNil: [ :finding | ': ', finding ]
]

{ #category : #printing }
GtsCLI class >> unprotectedTailAt: offset length: length [
	^ 'unprotected_tail at byte ', offset printString, ' (', length printString, ' bytes of the segment stream)'
]

{ #category : #printing }
GtsCLI class >> wireFindingFor: event [
	event kind = #resyncRefused ifTrue: [
		^ self resyncRefusalMessageFor: event ].
	event frameType = 16 ifTrue: [ ^ self parityFindingFor: event ].
	^ event problem ifNil: [
		event closingClaimFailure ifNotNil: [ :reason | 'index_claim_failed (', reason, ')' ] ]
]

{ #category : #printing }
GtsCLI class >> parityFindingFor: event [
	| messages |
	messages := OrderedCollection new.
	event isDamagedParity ifTrue: [ messages add: 'parity_damaged' ].
	event parityShardFindings do: [ :finding |
		| reason count noun |
		count := finding third.
		count
			ifNil: [ messages add: finding first, ' (', finding second, ')' ]
			ifNotNil: [
				reason := finding second ifNil: [ '' ] ifNotNil: [ :name | name, ', ' ].
				noun := count = 1 ifTrue: [ ' shard)' ] ifFalse: [ ' shards)' ].
				messages add: finding first, ' (', reason, count printString, noun ] ].
	event parityFinding ifNotNil: [ :finding | messages add: finding ].
	^ messages isEmpty ifFalse: [
		String streamContents: [ :output |
			messages do: [ :message | output nextPutAll: message ] separatedBy: [ output nextPutAll: ', ' ] ] ]
]

{ #category : #printing }
GtsCLI class >> resyncRefusalMessageFor: event [
	^ 'resync ', event problem, ' (', event payload count printString, ' bytes)'
]

{ #category : #printing }
GtsCLI class >> printWireIntegrityFor: event [
	| stored payload reason count |
	stored := event storedMatches ifNil: [ '' ] ifNotNil: [ :matches |
		matches ifTrue: [ ', stored check matches' ] ifFalse: [ ', stored check mismatch' ] ].
	payload := event payload.
	count := payload count ifNil: [ '' ] ifNotNil: [ :size | ', ', size printString, ' bytes' ].
	reason := payload reason ifNil: [ '' ] ifNotNil: [ :finding | ' (', finding, count, ')' ].
	event isDamagedParity ifTrue: [ reason := ''. stored := '' ].
	self stdout: '      link ', event link asString, stored, ', payload ', payload state asString, reason, String lf.
	^ event link = #linked and: [ event storedMatches ~= false and: [ payload state = #verified ] ]
]

{ #category : #private }
GtsCLI class >> allQuadsSuppressed: graph [
	| quadSup termSup |
	(graph quads isEmpty or: [ graph suppressions isEmpty ]) ifTrue: [ ^ false ].
	termSup := Dictionary new.
	quadSup := Dictionary new.
	graph suppressions do: [ :suppression |
		(suppression at: 'targets' ifAbsent: [ #() ]) do: [ :target |
			| kind |
			target isDictionary ifTrue: [
				kind := target at: 'kind' ifAbsent: [ '' ].
				((kind = 'term') or: [ kind = 'reifier' ]) ifTrue: [
					(target at: 'id' ifAbsent: [ nil ]) ifNotNil: [ :termId | termSup at: termId put: true ] ].
				kind = 'quad' ifTrue: [
					(target at: 'q' ifAbsent: [ nil ]) ifNotNil: [ :quad | quadSup at: quad put: true ] ] ] ] ].
	graph quads do: [ :quad |
		| hidden |
		hidden := quadSup includesKey: quad.
		quad do: [ :termId |
			(termSup includesKey: termId) ifTrue: [ hidden := true ] ].
		hidden ifFalse: [ ^ false ] ].
	^ true
]

{ #category : #private }
GtsCLI class >> graphContributes: graph [
	graph quads isEmpty ifFalse: [ ^ true ].
	graph blobs isEmpty ifFalse: [ ^ true ].
	graph reifiers isEmpty ifFalse: [ ^ true ].
	graph annotations isEmpty ifFalse: [ ^ true ].
	graph suppressions isEmpty ifFalse: [ ^ true ].
	^ false
]

{ #category : #private }
GtsCLI class >> graphIsClean: graph [
	^ graph diagnostics isEmpty and: [ graph segmentHeads notEmpty ]
]

{ #category : #private }
GtsCLI class >> parseVerifyKey: spec [
	| colon separator algorithm keyStart |
	colon := spec findString: ':' startingAt: 1.
	colon <= 1 ifTrue: [ GtsArchiveError signal: 'expected --key KEY_ID:PUBLIC_KEY or KEY_ID:ALGORITHM:PUBLIC_KEY' ].
	separator := spec findString: ':' startingAt: colon + 1.
	algorithm := 1.
	keyStart := colon + 1.
	separator > 0 ifTrue: [
		algorithm := #('ed25519' 'mldsa65_ed25519') indexOf: (spec copyFrom: colon + 1 to: separator - 1).
		algorithm = 0 ifTrue: [ GtsArchiveError signal: 'unknown --key signature algorithm' ].
		keyStart := separator + 1 ].
	^ { (spec copyFrom: 1 to: colon - 1). algorithm. (GtsHex decode: (spec copyFrom: keyStart to: spec size)) }
]

{ #category : #private }
GtsCLI class >> printDiagnosticsFor: graph [
	graph diagnostics do: [ :diagnostic |
		self stderr: 'gts: diagnostic ', diagnostic code, ': ', diagnostic detail, String lf ]
]

{ #category : #private }
GtsCLI class >> profileFindingsFor: graph [
	| declaredFiles findings usesFiles |
	declaredFiles := graph profiles includes: 'files'.
	usesFiles := false.
	graph terms do: [ :term |
		(term kind = #iri and: [ term value beginsWith: GtsFiles filesNamespace ])
			ifTrue: [ usesFiles := true ] ].
	findings := OrderedCollection new.
	(usesFiles and: [ declaredFiles not ]) ifTrue: [
		findings add: (Dictionary newFromPairs: {
			'severity'. 'error'.
			'message'. 'profile error: segment uses ', GtsFiles filesNamespace,
				' vocabulary but does not declare ''files''' }) ].
	(declaredFiles and: [ usesFiles not ]) ifTrue: [
		findings add: (Dictionary newFromPairs: {
			'severity'. 'warning'.
			'message'. 'profile warning: segment declares ''files'' but uses no ',
				GtsFiles filesNamespace, ' vocabulary' }) ].
	^ findings asArray
]

{ #category : #private }
GtsCLI class >> printLedgerFor: path graph: graph [
	self stdout: path, ': ', graph segmentHeads size asString, ' segment(s)', String lf.
	graph segmentHeads withIndexDo: [ :head :index |
		| signers |
		signers := graph signatures count: [ :signature | (signature at: 'status' ifAbsent: [ 'unverified' ]) ~= 'invalid' ].
		self stdout: '  segment ', (index - 1) asString,
			': head ', (GtsHex encode: head),
			' profile ', (graph profiles at: index ifAbsent: [ 'generic' ]),
			' terms ', graph terms size asString,
			' quads ', graph quads size asString,
			' reifies ', graph reifiers size asString,
			' annot ', graph annotations size asString,
			' blobs ', graph blobs size asString,
			' suppress ', graph suppressions size asString,
			' opaque ', graph opaqueReasons size asString,
			' sigs ', signers asString,
			String lf ].
	graph streamable withIndexDo: [ :layout :index |
		(layout at: 'claimed') ifTrue: [
			| headHex tail |
			headHex := (layout at: 'head' ifAbsent: [ nil ])
				ifNil: [ '<none>' ]
				ifNotNil: [ :layoutHead | GtsHex encode: layoutHead ].
			tail := (layout at: 'tail') > 0
				ifTrue: [ ', accretive tail ', (layout at: 'tail') asString, ' frame(s)' ]
				ifFalse: [ '' ].
			self stdout: '    layout: streamable through frame ',
				(layout at: 'covered') asString,
				' (head ', headHex, ')',
				tail,
				String lf ] ].
	graph diagnostics do: [ :diagnostic |
		| where |
		where := diagnostic frameIndex
			ifNil: [ '' ]
			ifNotNil: [ :frameIndex | ' [item ', frameIndex asString, ']' ].
		self stdout: '    diagnostic ', diagnostic code, ': ', diagnostic detail, where, String lf ]
]

{ #category : #commands }
GtsCLI class >> runLs: args [
	| problems |
	args isEmpty ifTrue: [ self stderr: self usage. ^ 2 ].
	problems := false.
	args do: [ :path |
		| graph |
		graph := GtsReader read: (self readFile: path).
		self printDiagnosticsFor: graph.
		(self graphIsClean: graph) ifFalse: [ problems := true ].
		graph blobs keys asSortedCollection do: [ :digest |
			| mediaType meta |
			meta := graph blobMeta at: digest ifAbsent: [ Dictionary new ].
			mediaType := meta at: 'mt' ifAbsent: [ '' ].
			self stdout: digest, ' ', (graph blobs at: digest) size asString, ' ', mediaType, String lf ] ].
	^ problems ifTrue: [ 1 ] ifFalse: [ 0 ]
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
	^ (result at: 'status') = 'error' ifTrue: [ 1 ] ifFalse: [ 0 ]
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
	^ (GtsReplication hasProblems: inventory) ifTrue: [ 1 ] ifFalse: [ 0 ]
]

{ #category : #commands }
GtsCLI class >> runUnpack: args [
	| destination graph includeSuppressed index path |
	destination := '.'.
	includeSuppressed := false.
	path := nil.
	index := 1.
	[ index <= args size ] whileTrue: [
		| arg |
		arg := args at: index.
		((arg = '-C') or: [ arg = '--directory' ])
			ifTrue: [
				index = args size ifTrue: [ self stderr: self usage. ^ 2 ].
				destination := args at: index + 1.
				index := index + 2 ]
			ifFalse: [
				arg = '--include-suppressed'
					ifTrue: [
						includeSuppressed := true.
						index := index + 1 ]
					ifFalse: [
						path ifNotNil: [ self stderr: self usage. ^ 2 ].
						path := arg.
						index := index + 1 ] ] ].
	path ifNil: [ self stderr: self usage. ^ 2 ].
	graph := GtsReader read: (self readFile: path).
	self printDiagnosticsFor: graph.
	(self graphIsClean: graph) ifFalse: [
		self stderr: 'gts: refusing unpack: archive did not read cleanly', String lf.
		^ 1 ].
	GtsFiles unpackGraph: graph to: destination includeSuppressed: includeSuppressed.
	^ 0
]

{ #category : #commands }
GtsCLI class >> runVerify: args [
	| keys trusted paths problems input |
	args isEmpty ifTrue: [ self stderr: self usage. ^ 2 ].
	keys := OrderedCollection new.
	trusted := Set new.
	paths := OrderedCollection new.
	input := args readStream.
	[ input atEnd ] whileFalse: [
		| argument |
		argument := input next.
		argument caseOf: {
			[ '--key' ] -> [
				input atEnd ifTrue: [ ^ 2 ].
				keys add: (self parseVerifyKey: input next) ].
			[ '--trust' ] -> [
				| kid |
				input atEnd ifTrue: [ ^ 2 ].
				kid := GtsHex decode: input next.
				kid size = 32 ifFalse: [ GtsArchiveError signal: '--trust needs a 32-byte hexadecimal key identifier' ].
				trusted add: kid ] }
			otherwise: [ paths add: argument ] ].
	paths isEmpty ifTrue: [ self stderr: self usage. ^ 2 ].
	problems := false.
	paths do: [ :path |
		| bytes wire clean |
		bytes := self readFile: path.
		wire := GtsV2Wire inspect: bytes.
		clean := wire format = #legacy
			ifTrue: [
				trusted isNotEmpty ifTrue: [ GtsArchiveError signal: '--trust requires GTS v2' ].
				self verifyLegacy: bytes path: path keys: keys ]
			ifFalse: [ self verifyWire: wire path: path keys: keys trusted: trusted ].
		clean ifFalse: [ problems := true ] ].
	^ problems ifTrue: [ 1 ] ifFalse: [ 0 ]
]

{ #category : #private }
GtsCLI class >> verifyLegacy: bytes path: path keys: entries [
	| graph clean keys |
	keys := Dictionary new.
	entries do: [ :entry |
		(entry second = 1 and: [ entry third size = GtsSodium ed25519PublicKeyBytes ]) ifFalse: [
			GtsArchiveError signal: 'archived GTS --key needs a 32-byte Ed25519 public key' ].
		keys at: entry first put: entry third ].
	graph := GtsReader read: bytes.
	self printLedgerFor: path graph: graph.
	clean := graph diagnostics isEmpty.
	(self profileFindingsFor: graph) do: [ :finding |
		self stderr: '  ', (finding at: 'severity'), ': ', (finding at: 'message'), String lf.
		(finding at: 'severity') = 'error' ifTrue: [ clean := false ] ].
	keys isEmpty ifFalse: [
		graph signatures do: [ :signature |
			| kid status |
			kid := signature at: 'kid' ifAbsent: [ nil ].
			status := self verifySignature: signature keys: keys.
			signature at: 'status' put: status.
			self stdout: '  signature ', (kid ifNil: [ '?' ]), ': ', status, String lf.
			status = 'invalid' ifTrue: [ clean := false ] ] ].
	^ clean
]

{ #category : #private }
GtsCLI class >> verifyWire: wire path: path keys: keys trusted: trusted [
	| format report clean ordinal checks verifier header authentication |
	format := wire format.
	clean := true.
	report := [ :message |
		self stderr: 'gts verify: ', path, ': ', message, String lf.
		clean := false ].
	format ~= #v2 ifTrue: [
		wire initialResyncRefusal ifNotNil: [ :refusal |
			report value: (self resyncRefusalMessageFor: refusal) ] ].
	format = #empty ifTrue: [ report value: 'empty input'. ^ false ].
	format = #framingError ifTrue: [
		report value: 'framing stopped at byte 0: ', wire stopReason.
		^ false ].
	format = #unsupported ifTrue: [
		report value: 'unsupported GTS generation ', wire generation printString.
		^ false ].
	format = #unknown ifTrue: [ report value: 'input does not identify a GTS generation'. ^ false ].
	verifier := GtsV2SignatureChecks held: keys.
	self stdout: path, ': GTS v2 frame, closing record, commit and index prefix integrity', String lf.
	ordinal := -1.
	wire eventsDo: [ :event |
		| location |
		location := 'byte ', event offset printString.
		(#(stop framingGap resyncRefused) includes: event kind)
			ifTrue: [
				event kind = #stop
					ifTrue: [ report value: 'framing stopped at ', location, ': ', event problem ]
					ifFalse: [ report value: location, ': ', (self wireFindingFor: event) ] ]
			ifFalse: [
				event startsSegment
					ifTrue: [
						checks ifNotNil: [ self printCommitSummary: checks authentication: authentication ordinal: ordinal path: path ].
						ordinal := ordinal + 1.
						header := event effectiveIdentifier.
						authentication := GtsV2Authentication trusted: trusted.
						checks := self commitChecksFor: event report: [ :finding |
							report value: location, ': ', finding ] ]
					ifFalse: [
						checks ifNotNil: [
							| commitValid |
							self reportIntegrityFor: event report: [ :finding |
								report value: location, ': ', finding ].
							commitValid := checks observe: event findingsDo: [ :finding :position :expected |
								| detail |
								detail := expected ifNil: [ '' ] ifNotNil: [
									' (expected ', expected printString, ')' ].
								report value: location, ', item ', position printString, ': ', finding, detail ].
							authentication observe: event validCommit: commitValid.
							verifier event: event header: header resultsDo: [ :algorithm :kid :status :reason |
								| detail result |
								detail := 'signature ', (GtsHex encode: kid), ' algorithm ', algorithm printString,
									' at ', location, ': ', status.
								reason ifNotNil: [ detail := detail, ' (', reason, ')' ].
								status = 'invalid'
									ifTrue: [ report value: detail ]
									ifFalse: [ self stdout: '  ', detail, String lf ].
								result := authentication coverageFor: status keyId: kid.
								result at: 'offset' put: event offset.
								result at: 'algorithm' put: algorithm.
								result at: 'key_id' put: (GtsHex encode: kid).
								result at: 'status' put: status.
								result at: 'reason' put: reason.
								self printAuthentication: result kind: 'signature' ordinal: ordinal path: path ] ] ] ] ]
		decodedWhen: [ :event | #(1 2 3 4 5 13 15) includes: event frameType ]
		unprotectedTailsDo: [ :offset :length |
			self stdout: '  ', (self unprotectedTailAt: offset length: length), String lf ].
	checks ifNotNil: [ self printCommitSummary: checks authentication: authentication ordinal: ordinal path: path ].
	^ clean
]

{ #category : #private }
GtsCLI class >> commitChecksFor: event report: report [
	event problem ifNotNil: [ :finding | report value: finding. ^ nil ].
	(event opener registry ~= 1 or: [ event effectiveIdentifier isNil ]) ifTrue: [
		report value: 'unsupported registry version ', event opener registry printString.
		^ nil ].
	^ GtsV2CommitChecks openedBy: event effectiveIdentifier
]

{ #category : #private }
GtsCLI class >> reportIntegrityFor: event report: report [
	| payload |
	(self wireFindingFor: event) ifNotNil: [ :finding | report value: finding ].
	(event kind ~= #frame or: [ event isDamagedParity ]) ifTrue: [ ^ self ].
	(event link = #healed or: [ event link = #broken and: [ event problem isNil ] ])
		ifTrue: [ report value: 'link ', event link asString ].
	event storedMatches = false ifTrue: [ report value: 'stored check mismatch' ].
	payload := event payload.
	(payload state ~= #verified and: [ payload reason isNotNil and: [ payload reason ~= 'codec_constraint' ] ])
		ifTrue: [
			| count |
			count := payload count ifNil: [ '' ] ifNotNil: [ :size | ' (', size printString, ' bytes)' ].
			report value: 'payload ', payload reason, count ]
]

{ #category : #printing }
GtsCLI class >> printCommitSummary: checks authentication: authentication ordinal: ordinal path: path [
	| prefix tail index |
	prefix := checks committedFrames = 0
		ifTrue: [ 'no_commit' ]
		ifFalse: [ 'committed_frames ', checks committedFrames printString ].
	tail := checks tailPosition ifNil: [ '' ] ifNotNil: [ :position |
		', uncommitted_tail ', checks tailFrames printString, ' (item ', position printString, ')' ].
	index := checks hasIndex ifTrue: [ ', index fold statistics and content digest unchecked' ] ifFalse: [ '' ].
	checks uncomparedDirectories do: [ :directory | index := index, ', index ', directory, ' unchecked' ].
	self stdout: '  segment ', ordinal printString, ': ', prefix, tail, index, String lf.
	self printAuthentication: authentication summary kind: 'segment' ordinal: ordinal path: path
]

{ #category : #printing }
GtsCLI class >> printAuthentication: result kind: kind ordinal: ordinal path: path [
	result at: 'kind' put: kind.
	result at: 'segment' put: ordinal.
	result at: 'path' put: path.
	self stdout: (STONJSON toString: result), String lf
]

{ #category : #commands }
GtsCLI class >> runVerifyProof: args [
	| proof |
	args size = 1 ifFalse: [ self stderr: self usage. ^ 2 ].
	proof := GtsMMR verifyProofJson: (self readFile: args first).
	self stdout: 'proof ok: root ', (GtsHex encode: (proof at: 'root')),
		' frame ', (GtsHex encode: (proof at: 'frame_id')), String lf.
	^ 0
]

{ #category : #private }
GtsCLI class >> verifySignature: signature keys: keys [
	"Dispatch on the algorithm declared in the protected header (GTS-SPEC section
	9.2): a malformed COSE_Sign1 is invalid, an unsupported algorithm or an
	unresolved key is unverified, and EdDSA is verified as Ed25519."

	| parsed publicKey |
	parsed := [ GtsCose parseSign1: (signature at: 'cose') ]
		on: Error
		do: [ :error | ^ 'invalid' ].
	(parsed at: 'alg') = GtsCose eddsaAlgorithm ifFalse: [ ^ 'unverified' ].
	publicKey := keys at: (parsed at: 'kid') ifAbsent: [ ^ 'unverified' ].
	(parsed at: 'signature') size = GtsSodium ed25519SignatureBytes
		ifFalse: [ ^ 'invalid' ].
	[
		^ (GtsCose
			verifySign1: (signature at: 'cose')
			frameId: (signature at: 'frame_id')
			publicKey: publicKey)
			ifTrue: [ 'valid' ]
			ifFalse: [ 'invalid' ] ]
		on: Error
		do: [ :error | ^ 'invalid' ]
]

{ #category : #io }
GtsCLI class >> readFile: path [
	^ path asFileReference binaryReadStreamDo: [ :stream | stream upToEnd ]
]

{ #category : #io }
GtsCLI class >> readText: path [
	| data |
	path = '-' ifTrue: [
		data := Stdio stdin upToEnd.
		(data isKindOf: ByteArray) ifTrue: [ ^ data utf8Decoded ].
		^ data asString ].
	^ (self readFile: path) utf8Decoded
]

{ #category : #io }
GtsCLI class >> stderr: text [
	Stdio stderr nextPutAll: text utf8Encoded
]

{ #category : #io }
GtsCLI class >> stdout: text [
	Stdio stdout nextPutAll: text utf8Encoded
]

{ #category : #io }
GtsCLI class >> stdoutBytes: bytes [
	Stdio stdout nextPutAll: bytes
]

{ #category : #usage }
GtsCLI class >> locale [
	| environment |
	environment := (Smalltalk globals at: #OSEnvironment ifAbsent: [ ^ #en ]) current.
	#('GTS_LANG' 'LC_ALL' 'LC_MESSAGES' 'LANG') do: [ :key |
		| raw |
		raw := (environment at: key ifAbsent: [ '' ]) trim.
		raw isEmpty ifFalse: [ ^ self localeFrom: raw ] ].
	^ #en
]

{ #category : #usage }
GtsCLI class >> localeFrom: raw [
	| value |
	value := ((raw trim) copyUpTo: $.) copyUpTo: $@.
	value := (value copyReplaceAll: '_' with: '-') asLowercase.
	((value = 'fr') or: [ value = 'fr-ca' ]) ifTrue: [ ^ #frCA ].
	((value = 'zh') or: [
		(value = 'zh-cn') or: [
			(value = 'zh-hans') or: [ value = 'zh-hans-cn' ] ] ])
		ifTrue: [ ^ #zhHans ].
	^ #en
]

{ #category : #usage }
GtsCLI class >> unknownCommandMessage: command [
	^ self unknownCommandMessage: command locale: self locale
]

{ #category : #usage }
GtsCLI class >> unknownCommandMessage: command locale: locale [
	locale = #frCA ifTrue: [
		^ 'gts: commande inconnue ', command, String lf, (self usageForLocale: locale) ].
	locale = #zhHans ifTrue: [
		^ 'gts: 未知命令 ', command, String lf, (self usageForLocale: locale) ].
	^ 'gts: unknown command ', command, String lf, (self usageForLocale: locale)
]

{ #category : #usage }
GtsCLI class >> usage [
	^ self usageForLocale: self locale
]

{ #category : #usage }
GtsCLI class >> usageForLocale: locale [
	locale = #frCA ifTrue: [
		^ 'utilisation: gts <command> [args]', String lf,
			'commandes:', String lf,
			'  info fold verify verify-proof heads segments missing resume extract-key', String lf,
			'  ls extract cat compact pack unpack diff from-nq', String lf ].
	locale = #zhHans ifTrue: [
		^ '用法: gts <command> [args]', String lf,
			'命令:', String lf,
			'  info fold verify verify-proof heads segments missing resume extract-key', String lf,
			'  ls extract cat compact pack unpack diff from-nq', String lf ].
	^ 'usage: gts <command> [args]', String lf,
		'commands:', String lf,
		'  info fold verify verify-proof heads segments missing resume extract-key', String lf,
		'  ls extract cat compact pack unpack diff from-nq', String lf
]

{ #category : #io }
GtsCLI class >> writeFile: path bytes: bytes [
	| target |
	target := path asFileReference.
	target parent ensureCreateDirectory.
	target exists ifFalse: [ target ensureCreateFile ].
	target binaryWriteStreamDo: [ :stream | stream nextPutAll: bytes ]
]
