#!/usr/bin/env sh
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
set -eu

cd /opt/pharo

pharo eval "[ | testClass result |
Metacello new
  baseline: 'Gts';
  repository: 'tonel:///workspace/smalltalk/src';
  load: #('tests').
testClass := Smalltalk globals at: #GtsCborWriterTest.
result := testClass suite run.
result printString crTrace.
result hasPassed
  ifTrue: [ Smalltalk exitSuccess ]
  ifFalse: [ Smalltalk exitFailure ] ] value"
