Class {
	#name : #GtsFiles,
	#superclass : #Object,
	#category : #'Gts-Core'
}

{ #category : #constants }
GtsFiles class >> filesNamespace [
	^ 'https://w3id.org/gts/files#'
]

{ #category : #constants }
GtsFiles class >> rdfType [
	^ 'http://www.w3.org/1999/02/22-rdf-syntax-ns#type'
]

{ #category : #constants }
GtsFiles class >> xsdDateTime [
	^ 'http://www.w3.org/2001/XMLSchema#dateTime'
]

{ #category : #constants }
GtsFiles class >> xsdInteger [
	^ 'http://www.w3.org/2001/XMLSchema#integer'
]

{ #category : #private }
GtsFiles class >> archiveEntriesForSources: sourcePaths [
	| entries seen |
	entries := OrderedCollection new.
	seen := Dictionary new.
	sourcePaths do: [ :path |
		| source |
		source := path asFileReference.
		source exists ifFalse: [ self error: 'source does not exist: ', path ].
		source isSymlink ifTrue: [ self error: 'symlink not supported: ', path ].
		source isDirectory
			ifTrue: [
				(source allChildren select: [ :child | child isFile ]) do: [ :child |
					| relpath |
					child isSymlink ifTrue: [ self error: 'symlink not supported: ', child fullName ].
					relpath := (child relativeTo: source) pathString.
					self validateArchivePath: relpath.
					(seen includesKey: relpath) ifTrue: [ self error: 'duplicate archive path: ', relpath ].
					seen at: relpath put: true.
					entries add: { child. relpath } ] ]
			ifFalse: [
				source isFile ifFalse: [ self error: 'unsupported source type: ', path ].
				self validateArchivePath: source basename.
				(seen includesKey: source basename) ifTrue: [ self error: 'duplicate archive path: ', source basename ].
				seen at: source basename put: true.
				entries add: { source. source basename } ] ].
	^ entries asSortedCollection: [ :left :right | (left at: 2) < (right at: 2) ]
]

{ #category : #private }
GtsFiles class >> formatDateTime: dateAndTime [
	^ (dateAndTime asUTC printString copyReplaceAll: '+00:00' with: 'Z')
]

{ #category : #diffing }
GtsFiles class >> diffGraph: graph withDirectory: directoryPath [
	| archiveDigests diskDigests lines |
	archiveDigests := Dictionary new.
	(self fileEntriesIn: graph) associationsDo: [ :association |
		archiveDigests at: association key put: (association value at: 'digest') ].
	diskDigests := Dictionary new.
	(self archiveEntriesForSources: { directoryPath }) do: [ :entry |
		| data |
		data := (entry at: 1) binaryReadStreamDo: [ :stream | stream upToEnd ].
		diskDigests at: (entry at: 2) put: (GtsWriter digestString: data) ].
	lines := OrderedCollection new.
	archiveDigests keysDo: [ :path |
		(diskDigests includesKey: path) ifFalse: [ lines add: 'removed: ', path ] ].
	diskDigests keysDo: [ :path |
		(archiveDigests includesKey: path) ifFalse: [ lines add: 'added: ', path ] ].
	archiveDigests associationsDo: [ :association |
		| diskDigest |
		diskDigest := diskDigests at: association key ifAbsent: [ nil ].
		(diskDigest notNil and: [ diskDigest ~= association value ])
			ifTrue: [ lines add: 'modified: ', association key ] ].
	^ lines asSortedCollection asArray
]

{ #category : #private }
GtsFiles class >> digestFromValue: value [
	value isString ifTrue: [ ^ self normalizeDigest: value ].
	(value isKindOf: ByteArray) ifTrue: [ ^ 'blake3:', (GtsHex encode: value) ].
	^ ''
]

{ #category : #private }
GtsFiles class >> fileEntriesIn: graph [
	| entries fileEntryId fieldIds typeId |
	fieldIds := Dictionary new.
	graph terms withIndexDo: [ :term :indexOneBased |
		| termId value |
		termId := indexOneBased - 1.
		value := term value.
		term kind = #iri ifTrue: [
			value = self rdfType ifTrue: [ typeId := termId ].
			value = (self filesNamespace, 'FileEntry') ifTrue: [ fileEntryId := termId ].
			(value beginsWith: self filesNamespace) ifTrue: [
				fieldIds
					at: (value copyFrom: self filesNamespace size + 1 to: value size)
					put: termId ] ] ].
	(typeId isNil or: [ fileEntryId isNil ])
		ifTrue: [ self error: 'not a files-profile archive' ].
	entries := Dictionary new.
	graph quads do: [ :quad |
		| object predicate subject |
		subject := quad at: 1.
		predicate := quad at: 2.
		object := quad at: 3.
		(predicate = typeId and: [ object = fileEntryId ])
			ifTrue: [
				entries
					at: subject
					ifAbsentPut: [ Dictionary new ] ]
			ifFalse: [
				fieldIds associationsDo: [ :field |
					field value = predicate ifTrue: [
						(entries at: subject ifAbsentPut: [ Dictionary new ])
							at: field key
							put: (graph termAtId: object) value ] ] ] ].
	^ Dictionary newFrom: ((entries values select: [ :entry | entry includesKey: 'path' ])
		collect: [ :entry | (entry at: 'path') -> entry ])
]

{ #category : #private }
GtsFiles class >> guessMediaTypeFor: fileReference [
	| ext |
	ext := fileReference extension asLowercase.
	ext = 'txt' ifTrue: [ ^ 'text/plain' ].
	((ext = 'html') or: [ ext = 'htm' ]) ifTrue: [ ^ 'text/html' ].
	ext = 'json' ifTrue: [ ^ 'application/json' ].
	ext = 'xml' ifTrue: [ ^ 'application/xml' ].
	ext = 'png' ifTrue: [ ^ 'image/png' ].
	((ext = 'jpg') or: [ ext = 'jpeg' ]) ifTrue: [ ^ 'image/jpeg' ].
	ext = 'gif' ifTrue: [ ^ 'image/gif' ].
	ext = 'webp' ifTrue: [ ^ 'image/webp' ].
	ext = 'pdf' ifTrue: [ ^ 'application/pdf' ].
	ext = 'zip' ifTrue: [ ^ 'application/zip' ].
	ext = 'gz' ifTrue: [ ^ 'application/gzip' ].
	ext = 'tar' ifTrue: [ ^ 'application/x-tar' ].
	^ 'application/octet-stream'
]

{ #category : #private }
GtsFiles class >> guardSafeWriteTarget: target under: destinationRef archivePath: archivePath [
	| parent |
	target exists ifTrue: [
		target isSymlink ifTrue: [ self error: 'refusing to write through symlink: ', archivePath ] ].
	parent := target parent.
	[ parent exists and: [ parent ~= destinationRef ] ] whileTrue: [
		parent isSymlink ifTrue: [ self error: 'path escapes destination: ', archivePath ].
		parent := parent parent ]
]

{ #category : #private }
GtsFiles class >> hasUnsafePathComponent: archivePath [
	| parts |
	archivePath isEmpty ifTrue: [ ^ true ].
	(archivePath includesSubstring: '\') ifTrue: [ ^ true ].
	(archivePath beginsWith: '/') ifTrue: [ ^ true ].
	(archivePath size >= 2 and: [ (archivePath at: 2) = $: ]) ifTrue: [ ^ true ].
	parts := archivePath findTokens: '/'.
	parts isEmpty ifTrue: [ ^ true ].
	parts do: [ :part |
		((part isEmpty) or: [ part = '.' or: [ part = '..' ] ])
			ifTrue: [ ^ true ] ].
	^ false
]

{ #category : #private }
GtsFiles class >> mediaTypeForDigest: digest in: graph [
	| meta |
	meta := graph blobMeta at: digest ifAbsent: [ ^ '' ].
	meta isDictionary ifFalse: [ ^ '' ].
	^ meta at: 'mt' ifAbsent: [ '' ]
]

{ #category : #private }
GtsFiles class >> normalizeDigest: digest [
	(digest beginsWith: 'blake3:') ifTrue: [ ^ digest ].
	^ 'blake3:', digest
]

{ #category : #packing }
GtsFiles class >> packSources: sourcePaths [
	| blobOrder blobs entries fileTerms quads shared writer xsdDateTimeId xsdIntegerId |
	writer := GtsWriter forProfile: 'files'.
	shared := {
		GtsTerm iri: self filesNamespace, 'FileEntry'.
		GtsTerm iri: self filesNamespace, 'path'.
		GtsTerm iri: self filesNamespace, 'digest'.
		GtsTerm iri: self filesNamespace, 'size'.
		GtsTerm iri: self filesNamespace, 'mode'.
		GtsTerm iri: self filesNamespace, 'modified'.
		GtsTerm iri: self filesNamespace, 'mediaType'.
		GtsTerm iri: self rdfType.
		GtsTerm iri: self xsdInteger.
		GtsTerm iri: self xsdDateTime }.
	writer addTerms: shared.
	xsdIntegerId := 8.
	xsdDateTimeId := 9.
	entries := self archiveEntriesForSources: sourcePaths.
	fileTerms := OrderedCollection new.
	quads := OrderedCollection new.
	blobs := Dictionary new.
	blobOrder := OrderedCollection new.
	entries withIndexDo: [ :entry :indexOneBased |
		| base data digest entryId fileReference mediaType mode modified relpath size |
		fileReference := entry at: 1.
		relpath := entry at: 2.
		data := fileReference binaryReadStreamDo: [ :stream | stream upToEnd ].
		digest := GtsWriter digestString: data.
		size := data size.
		mode := fileReference permissions posixPermission.
		modified := self formatDateTime: fileReference modificationTime.
		mediaType := self guessMediaTypeFor: fileReference.
		base := shared size + fileTerms size.
		entryId := base.
		fileTerms
			add: (GtsTerm blankNode: 'f', (indexOneBased - 1) asString);
			add: (GtsTerm literal: relpath language: nil datatype: nil);
			add: (GtsTerm literal: digest language: nil datatype: nil);
			add: (GtsTerm literal: size asString language: nil datatype: xsdIntegerId);
			add: (GtsTerm literal: mode asString language: nil datatype: xsdIntegerId);
			add: (GtsTerm literal: modified language: nil datatype: xsdDateTimeId);
			add: (GtsTerm literal: mediaType language: nil datatype: nil).
		quads
			add: { entryId. 7. 0 };
			add: { entryId. 1. base + 1 };
			add: { entryId. 2. base + 2 };
			add: { entryId. 3. base + 3 };
			add: { entryId. 4. base + 4 };
			add: { entryId. 5. base + 5 };
			add: { entryId. 6. base + 6 }.
		(blobs includesKey: digest) ifFalse: [
			blobs at: digest put: (Dictionary newFromPairs: {
				'data'. data.
				'mt'. mediaType }).
			blobOrder add: digest ] ].
	fileTerms isEmpty ifFalse: [ writer addTerms: fileTerms asArray ].
	quads isEmpty ifFalse: [ writer addQuads: quads asArray ].
	blobOrder do: [ :digest |
		| blob |
		blob := blobs at: digest.
		writer
			addBlob: (blob at: 'data')
			mediaType: (blob at: 'mt')
			representation: '' ].
	^ writer bytes
]

{ #category : #private }
GtsFiles class >> pathFor: archivePath under: destination [
	| destinationRef target |
	self validateArchivePath: archivePath.
	destinationRef := destination asFileReference.
	target := destinationRef.
	(archivePath findTokens: '/') do: [ :part | target := target / part ].
	^ target
]

{ #category : #private }
GtsFiles class >> suppressedBlobDigestsIn: graph [
	| out |
	out := Dictionary new.
	graph suppressions do: [ :suppression |
		(suppression at: 'targets' ifAbsent: [ #() ]) do: [ :target |
			| digest kind |
			target isDictionary ifTrue: [
				kind := target at: 'kind' ifAbsent: [ '' ].
				digest := self digestFromValue: (target at: 'digest' ifAbsent: [ nil ]).
				(kind = 'blob' and: [ digest isEmpty not ])
					ifTrue: [ out at: digest put: true ] ] ] ].
	^ out
]

{ #category : #unpacking }
GtsFiles class >> unpackGraph: graph to: destination includeSuppressed: includeSuppressed [
	| blobByDigest destinationRef entries suppressed |
	destinationRef := destination asFileReference.
	destinationRef ensureCreateDirectory.
	blobByDigest := graph blobs.
	entries := self fileEntriesIn: graph.
	suppressed := includeSuppressed
		ifTrue: [ Dictionary new ]
		ifFalse: [ self suppressedBlobDigestsIn: graph ].
	entries associationsDo: [ :association |
		| data digest target |
		digest := association value at: 'digest' ifAbsent: [ self error: 'missing digest for ', association key ].
		(suppressed includesKey: digest) ifFalse: [
			data := blobByDigest at: digest ifAbsent: [ self error: 'missing inline blob for ', association key ].
			(GtsWriter digestString: data) = digest
				ifFalse: [ self error: 'integrity failure for ', association key ].
			target := self pathFor: association key under: destination.
			target parent ensureCreateDirectory.
			self guardSafeWriteTarget: target under: destinationRef archivePath: association key.
			target exists ifFalse: [ target ensureCreateFile ].
			target binaryWriteStreamDo: [ :stream | stream nextPutAll: data ] ] ]
]

{ #category : #private }
GtsFiles class >> validateArchivePath: archivePath [
	(self hasUnsafePathComponent: archivePath)
		ifTrue: [ self error: 'unsafe archive path: ', archivePath ]
]
