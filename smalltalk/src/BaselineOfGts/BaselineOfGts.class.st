Class {
	#name : #BaselineOfGts,
	#superclass : #BaselineOf,
	#category : #BaselineOfGts
}

{ #category : #baselines }
BaselineOfGts >> baseline: spec [
	<baseline>
	spec for: #common do: [
		spec
			package: 'Gts-Core';
			package: 'Gts-Tests' with: [ spec requires: #('Gts-Core') ].
		spec
			group: 'default' with: #('Gts-Core');
			group: 'tests' with: #('Gts-Core' 'Gts-Tests') ]
]
