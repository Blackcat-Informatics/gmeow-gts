Class {
	#name : #GtsFilesTest,
	#superclass : #TestCase,
	#category : #'Gts-Tests'
}

{ #category : #tests }
GtsFilesTest >> testPackSourcesProducesReadableFilesProfileArchive [
	| archive graph root subdir |
	root := '/tmp/gts-smalltalk-files-test' asFileReference.
	root exists ifTrue: [ root ensureDeleteAll ].
	root ensureCreateDirectory.
	subdir := root / 'sub'.
	subdir ensureCreateDirectory.
	(root / 'a.txt') writeStreamDo: [ :stream | stream nextPutAll: 'hello gts'; lf ].
	(subdir / 'b.txt') writeStreamDo: [ :stream | stream nextPutAll: 'second file'; lf ].
	archive := GtsFiles packSources: { root fullName }.
	graph := GtsReader read: archive.
	self assert: graph diagnostics isEmpty.
	self assert: graph profiles equals: #('files').
	self assert: graph quads size equals: 14.
	self assert: graph blobs size equals: 2.
	root ensureDeleteAll
]

{ #category : #tests }
GtsFilesTest >> testUnsafeArchivePathSignalsArchiveError [
	self should: [ GtsFiles validateArchivePath: '../escape.txt' ] raise: GtsArchiveError
]
