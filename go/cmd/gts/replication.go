// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package main

import (
	"fmt"
	"os"

	"go.blackcatinformatics.ca/gts/mmr"
	"go.blackcatinformatics.ca/gts/replication"
)

func cmdHeads(args []string) int {
	if len(args) != 1 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	data, code := load(args[0])
	if code != 0 {
		return code
	}
	inv := replication.InventoryFor(data)
	fmt.Print(replication.HeadsJSON(inv))
	if inv.HasProblems() {
		return 1
	}
	return 0
}

func cmdSegments(args []string) int {
	if len(args) != 1 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	data, code := load(args[0])
	if code != 0 {
		return code
	}
	inv := replication.InventoryFor(data)
	fmt.Print(replication.SegmentsJSON(inv))
	if inv.HasProblems() {
		return 1
	}
	return 0
}

func cmdMissing(args []string) int {
	if len(args) != 3 || args[0] != "--from-head" {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	fromHead, err := mmr.ParseHex32(args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts missing: invalid peer head: %v\n", err)
		return 2
	}
	data, code := load(args[2])
	if code != 0 {
		return code
	}
	result := replication.Missing(replication.InventoryFor(data), fromHead)
	fmt.Print(replication.MissingJSON(result))
	if result.Status == replication.MissingError {
		return 1
	}
	return 0
}

func cmdResume(args []string) int {
	if len(args) != 3 || args[0] != "--after" {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	frameID, err := mmr.ParseHex32(args[1])
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts resume: invalid frame id: %v\n", err)
		return 2
	}
	data, code := load(args[2])
	if code != 0 {
		return code
	}
	tail, err := replication.ResumeAfter(data, frameID)
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts resume: %v\n", err)
		return 1
	}
	if _, err := os.Stdout.Write(tail); err != nil {
		fmt.Fprintf(os.Stderr, "gts resume: cannot write stdout: %v\n", err)
		return 2
	}
	return 0
}
