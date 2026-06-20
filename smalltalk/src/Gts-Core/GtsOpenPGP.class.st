Class {
	#name : #GtsOpenPGP,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #private }
GtsOpenPGP class >> base64Decode: text [
	^ (Base64MimeConverter mimeDecodeToChars: text readStream) contents asByteArray
]

{ #category : #emojihash }
GtsOpenPGP class >> emojihash: data length: length [
	| acc bits digest indices nbytes wanted |
	wanted := length max: 1.
	nbytes := ((wanted * 6) + 7) // 8.
	digest := GtsBlake3 hash: data.
	nbytes > digest size ifTrue: [ self error: 'emojihash length exceeds available digest bytes' ].
	indices := OrderedCollection new.
	acc := 0.
	bits := 0.
	1 to: nbytes do: [ :index |
		acc := (acc bitShift: 8) + (digest at: index).
		bits := bits + 8.
		[ bits >= 6 and: [ indices size < wanted ] ] whileTrue: [
			bits := bits - 6.
			indices add: ((acc bitShift: bits negated) bitAnd: 16r3F) ].
		acc := acc bitAnd: ((1 bitShift: bits) - 1) ].
	^ String streamContents: [ :stream |
		indices withIndexDo: [ :each :index |
			index > 1 ifTrue: [ stream nextPut: Character space ].
			stream nextPutAll: (self emojiAlphabet at: each + 1) ] ]
]

{ #category : #emojihash }
GtsOpenPGP class >> emojiAlphabet [
	^ {
		'🐵'. '🐶'. '🐺'. '🦊'. '🐱'. '🦁'. '🐯'. '🐴'.
		'🦄'. '🦓'. '🦌'. '🐮'. '🐷'. '🐗'. '🐭'. '🐹'.
		'🐰'. '🐻'. '🐼'. '🐨'. '🐸'. '🐲'. '🐔'. '🐧'.
		'🦆'. '🦅'. '🦉'. '🦇'. '🐢'. '🐍'. '🦎'. '🐊'.
		'🐳'. '🐬'. '🐟'. '🐠'. '🐡'. '🦈'. '🐙'. '🦑'.
		'🦀'. '🦞'. '🦐'. '🦋'. '🐌'. '🐞'. '🐝'. '🐜'.
		'🦂'. '🍎'. '🍐'. '🍊'. '🍋'. '🍌'. '🍉'. '🍇'.
		'🍓'. '🍒'. '🍍'. '🥝'. '🍑'. '🥥'. '🥕'. '🌽' }
]

{ #category : #extracting }
GtsOpenPGP class >> extractKeyTextFromGraph: graph [
	| armored fingerprint key parsed |
	key := self transportKeyFromGraph: graph.
	armored := key at: 'gpg'.
	^ String streamContents: [ :stream |
		stream
			nextPutAll: 'kid:         ';
			nextPutAll: (key at: 'kid');
			nextPut: Character lf.
		[
			parsed := self parseTransportKey: armored.
			fingerprint := parsed at: 'fingerprint'.
			stream
				nextPutAll: 'fingerprint: ';
				nextPutAll: (self formatFingerprint: fingerprint);
				nextPut: Character lf;
				nextPutAll: 'emojihash:   ';
				nextPutAll: (self emojihash: (parsed at: 'rawPublic') length: 11);
				nextPut: Character lf ]
			on: Error
			do: [ :error |
				stream
					nextPutAll: 'fingerprint: ';
					nextPutAll: (self formatFingerprint: (key at: 'kid'));
					nextPut: Character lf ].
		stream
			nextPutAll: armored;
			nextPut: Character lf ]
]

{ #category : #fingerprint }
GtsOpenPGP class >> fingerprintFor: publicKeyBody [
	| preimage |
	publicKeyBody size > 16rFFFF ifTrue: [ self error: 'OpenPGP public key packet too large' ].
	preimage := ByteArray streamContents: [ :stream |
		stream nextPut: 16r99.
		self writeUint16: publicKeyBody size on: stream.
		stream nextPutAll: publicKeyBody ].
	^ (GtsHex encode: (SHA1 hashMessage: preimage)) asUppercase
]

{ #category : #fingerprint }
GtsOpenPGP class >> formatFingerprint: fingerprint [
	| compact |
	compact := fingerprint select: [ :character | character isSeparator not ].
	compact := compact asUppercase.
	compact isEmpty ifTrue: [ ^ fingerprint ].
	(compact allSatisfy: [ :character | self isHexDigitCharacter: character ])
		ifFalse: [ ^ fingerprint ].
	^ String streamContents: [ :stream |
		1 to: compact size by: 4 do: [ :index |
			index > 1 ifTrue: [ stream nextPut: Character space ].
			stream nextPutAll: (compact copyFrom: index to: (index + 3 min: compact size)) ] ]
]

{ #category : #private }
GtsOpenPGP class >> isHexDigitCharacter: character [
	| code |
	code := character asciiValue.
	^ (code between: $0 asciiValue and: $9 asciiValue)
		or: [ (code between: $A asciiValue and: $F asciiValue)
		or: [ code between: $a asciiValue and: $f asciiValue ] ]
]

{ #category : #private }
GtsOpenPGP class >> iterPackets: data [
	| offset packets |
	offset := 1.
	packets := OrderedCollection new.
	[ offset <= data size ] whileTrue: [
		| packet |
		packet := self nextPacketIn: data offset: offset.
		packets add: packet.
		offset := packet at: 'next' ].
	^ packets asArray
]

{ #category : #private }
GtsOpenPGP class >> nextPacketIn: data offset: startOffset [
	| body bodyEnd header length lengthOctet lengthType offset tag |
	startOffset > data size ifTrue: [ self error: 'truncated packet header' ].
	header := data at: startOffset.
	(header bitAnd: 16r80) = 0 ifTrue: [ self error: 'invalid packet tag octet' ].
	offset := startOffset + 1.
	(header bitAnd: 16r40) ~= 0
		ifTrue: [
			tag := header bitAnd: 16r3F.
			offset > data size ifTrue: [ self error: 'truncated new-format length octet' ].
			lengthOctet := data at: offset.
			lengthOctet < 192
				ifTrue: [
					length := lengthOctet.
					offset := offset + 1 ]
				ifFalse: [
					lengthOctet < 224
						ifTrue: [
							offset + 1 > data size ifTrue: [ self error: 'truncated new-format 2-octet length' ].
							length := ((lengthOctet - 192) bitShift: 8) + (data at: offset + 1) + 192.
							offset := offset + 2 ]
						ifFalse: [
							lengthOctet = 255
								ifTrue: [
									offset + 4 > data size ifTrue: [ self error: 'truncated new-format 4-octet length' ].
									length := self uint32At: offset + 1 in: data.
									offset := offset + 5 ]
								ifFalse: [ self error: 'partial body lengths are not supported' ] ] ] ]
		ifFalse: [
			tag := (header bitShift: -2) bitAnd: 16r0F.
			lengthType := header bitAnd: 3.
			lengthType = 0
				ifTrue: [
					offset > data size ifTrue: [ self error: 'truncated old-format length octet' ].
					length := data at: offset.
					offset := offset + 1 ]
				ifFalse: [
					lengthType = 1
						ifTrue: [
							offset + 1 > data size ifTrue: [ self error: 'truncated old-format 2-octet length' ].
							length := self uint16At: offset in: data.
							offset := offset + 2 ]
						ifFalse: [
							lengthType = 2
								ifTrue: [
									offset + 3 > data size ifTrue: [ self error: 'truncated old-format 4-octet length' ].
									length := self uint32At: offset in: data.
									offset := offset + 4 ]
								ifFalse: [ self error: 'indeterminate-length packets are not supported' ] ] ] ].
	bodyEnd := offset + length - 1.
	bodyEnd > data size ifTrue: [ self error: 'packet body exceeds input' ].
	body := length = 0
		ifTrue: [ ByteArray new ]
		ifFalse: [ data copyFrom: offset to: bodyEnd ].
	^ Dictionary newFromPairs: {
		'tag'. tag.
		'body'. body.
		'next'. bodyEnd + 1 }
]

{ #category : #parsing }
GtsOpenPGP class >> parseEd25519PublicMaterial: body [
	| end mpi oid oidLength offset raw result |
	(body size < 6 or: [ (body at: 1) ~= 4 ])
		ifTrue: [ self error: 'only OpenPGP v4 public keys are supported' ].
	(body at: 6) = 22
		ifFalse: [ self error: 'unsupported public-key algorithm ', (body at: 6) asString ].
	offset := 7.
	offset > body size ifTrue: [ self error: 'truncated public-key packet' ].
	oidLength := body at: offset.
	offset := offset + 1.
	offset + oidLength - 1 > body size ifTrue: [ self error: 'truncated OID' ].
	oid := body copyFrom: offset to: offset + oidLength - 1.
	offset := offset + oidLength.
	oid = self supportedEd25519Oid
		ifFalse: [ self error: 'unsupported curve OID ', (GtsHex encode: oid) ].
	result := self readMpiFrom: body offset: offset.
	mpi := result at: 'bytes'.
	end := result at: 'next'.
	mpi size = 33
		ifTrue: [ raw := mpi copyFrom: 2 to: 33 ]
		ifFalse: [
			mpi size = 32
				ifTrue: [ raw := mpi ]
				ifFalse: [ self error: 'unexpected Ed25519 public MPI length ', mpi size asString ] ].
	^ Dictionary newFromPairs: {
		'rawPublic'. raw.
		'end'. end }
]

{ #category : #parsing }
GtsOpenPGP class >> parseTransportKey: armoredText [
	| packets |
	packets := self iterPackets: (self stripArmor: armoredText).
	packets do: [ :packet |
		| body material publicBody tag |
		tag := packet at: 'tag'.
		body := packet at: 'body'.
		(tag = 6 or: [ tag = 5 ]) ifTrue: [
			material := self parseEd25519PublicMaterial: body.
			publicBody := tag = 6
				ifTrue: [ body ]
				ifFalse: [ body copyFrom: 1 to: (material at: 'end') - 1 ].
			^ Dictionary newFromPairs: {
				'rawPublic'. (material at: 'rawPublic').
				'fingerprint'. (self fingerprintFor: publicBody) } ] ].
	self error: 'no public-key packet found'
]

{ #category : #private }
GtsOpenPGP class >> readMpiFrom: data offset: offset [
	| bits end length |
	offset + 1 > data size ifTrue: [ self error: 'truncated MPI length' ].
	bits := self uint16At: offset in: data.
	length := (bits + 7) // 8.
	end := offset + 2 + length.
	end - 1 > data size ifTrue: [ self error: 'truncated MPI payload' ].
	^ Dictionary newFromPairs: {
		'bytes'. (data copyFrom: offset + 2 to: end - 1).
		'next'. end }
]

{ #category : #private }
GtsOpenPGP class >> stripArmor: text [
	| body end index lines scan start |
	lines := text lines.
	start := nil.
	end := nil.
	scan := 1.
	[ scan <= lines size and: [ end isNil ] ] whileTrue: [
		| line |
		line := lines at: scan.
		(start isNil and: [ line beginsWith: '-----BEGIN PGP' ])
			ifTrue: [ start := scan ].
		(start notNil and: [ scan > start and: [ line beginsWith: '-----END PGP' ] ])
			ifTrue: [ end := scan ].
		scan := scan + 1 ].
	start ifNil: [ self error: 'missing armor BEGIN line' ].
	end ifNil: [ self error: 'missing armor END line' ].
	index := start + 1.
	[ index < end
		and: [ (lines at: index) trimBoth isEmpty not
		and: [ (lines at: index) includes: $: ] ] ]
		whileTrue: [ index := index + 1 ].
	body := String streamContents: [ :stream |
		[ index < end ] whileTrue: [
			| line |
			line := lines at: index.
			(line beginsWith: '=') ifTrue: [ index := end ] ifFalse: [
				line isEmpty ifFalse: [ stream nextPutAll: line ].
				index := index + 1 ] ] ].
	body isEmpty ifTrue: [ self error: 'empty armor body' ].
	^ self base64Decode: body
]

{ #category : #private }
GtsOpenPGP class >> supportedEd25519Oid [
	^ #[16r2B 16r06 16r01 16r04 16r01 16rDA 16r47 16r0F 16r01]
]

{ #category : #extracting }
GtsOpenPGP class >> transportKeyFromGraph: graph [
	| key |
	key := graph meta at: 'gts:transportKey' ifAbsent: [ self error: 'no embedded transport key' ].
	key isDictionary ifFalse: [ self error: 'malformed embedded transport key' ].
	((key includesKey: 'kid') and: [ key includesKey: 'gpg' ])
		ifFalse: [ self error: 'malformed embedded transport key' ].
	^ key
]

{ #category : #private }
GtsOpenPGP class >> uint16At: offset in: data [
	^ ((data at: offset) bitShift: 8) + (data at: offset + 1)
]

{ #category : #private }
GtsOpenPGP class >> uint32At: offset in: data [
	^ (((data at: offset) bitShift: 24)
		+ ((data at: offset + 1) bitShift: 16)
		+ ((data at: offset + 2) bitShift: 8)
		+ (data at: offset + 3))
]

{ #category : #private }
GtsOpenPGP class >> writeUint16: value on: stream [
	stream
		nextPut: ((value bitShift: -8) bitAnd: 16rFF);
		nextPut: (value bitAnd: 16rFF)
]
