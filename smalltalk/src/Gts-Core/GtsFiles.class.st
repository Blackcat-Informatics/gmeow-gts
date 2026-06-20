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
GtsFiles class >> validateArchivePath: archivePath [
	(self hasUnsafePathComponent: archivePath)
		ifTrue: [ self error: 'unsafe archive path: ', archivePath ]
]
