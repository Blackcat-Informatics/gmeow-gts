#!/usr/bin/env sh
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -eu

cd /opt/pharo

pharo eval "[ | testClasses results |
Metacello new
  baseline: 'Gts';
  repository: 'tonel:///workspace/smalltalk/src';
  load: #('tests').
testClasses := {
  Smalltalk globals at: #GtsCborWriterTest.
  Smalltalk globals at: #GtsBlake3Test.
  Smalltalk globals at: #GtsMinimalVectorTest.
  Smalltalk globals at: #GtsZstdTest.
  Smalltalk globals at: #GtsSodiumTest }.
results := testClasses collect: [ :testClass | testClass suite run ].
results do: [ :result | result printString crTrace ].
(results allSatisfy: [ :result | result hasPassed ])
  ifTrue: [ Smalltalk exitSuccess ]
  ifFalse: [ Smalltalk exitFailure ] ] value"
