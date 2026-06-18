// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package main

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"testing"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/stream"
	"go.blackcatinformatics.ca/gts/wire"
	"go.blackcatinformatics.ca/gts/writer"
)

var binPath string

func TestMain(m *testing.M) {
	dir, err := os.MkdirTemp("", "gts-cli-test")
	if err != nil {
		fmt.Fprintf(os.Stderr, "cannot create temp dir: %v\n", err)
		os.Exit(1)
	}

	binName := "gts"
	if runtime.GOOS == "windows" {
		binName += ".exe" // Windows needs the extension to locate/exec the binary.
	}
	binPath = filepath.Join(dir, binName)
	//nolint:gosec // subprocess is intentional test scaffolding for the CLI binary.
	cmd := exec.Command("go", "build", "-o", binPath, "go.blackcatinformatics.ca/gts/cmd/gts")
	if out, err := cmd.CombinedOutput(); err != nil {
		_ = os.RemoveAll(dir)
		fmt.Fprintf(os.Stderr, "cannot build gts binary: %v\n%s\n", err, out)
		os.Exit(1)
	}

	code := m.Run()
	_ = os.RemoveAll(dir)
	os.Exit(code)
}

func run(t *testing.T, args ...string) (*exec.Cmd, *bytes.Buffer, *bytes.Buffer) {
	t.Helper()
	//nolint:gosec // subprocess is intentional test scaffolding for the compiled CLI.
	cmd := exec.Command(binPath, args...)
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	err := cmd.Run()
	if err != nil {
		if _, ok := err.(*exec.ExitError); !ok {
			t.Fatalf("failed to start command: %v", err)
		}
	}
	return cmd, &stdout, &stderr
}

func vectorsDir(t *testing.T) string {
	t.Helper()
	dir, err := filepath.Abs("../../../vectors")
	if err != nil {
		t.Fatal(err)
	}
	return dir
}

func vector(t *testing.T, name string) string {
	t.Helper()
	return filepath.Join(vectorsDir(t), name)
}

func proofVector(t *testing.T, name string) string {
	t.Helper()
	return filepath.Join(vectorsDir(t), "proofs", name)
}

func TestFoldEmitsNQuads(t *testing.T) {
	v := vector(t, "01-minimal.gts")
	cmd, stdout, stderr := run(t, "fold", v)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("expected exit 0, got %d", cmd.ProcessState.ExitCode())
	}
	if stderr.Len() > 0 {
		t.Fatalf("fold produced stderr: %s", stderr.String())
	}
	want := "<https://example.org/Cat> <http://www.w3.org/2000/01/rdf-schema#label> \"Cat\"@en .\n"
	if got := stdout.String(); got != want {
		t.Fatalf("fold output mismatch\ngot:  %q\nwant: %q", got, want)
	}
}

func TestVerifyFlagsDamageWithExit1(t *testing.T) {
	v := vector(t, "04-damaged-frame.gts")
	cmd, stdout, _ := run(t, "verify", v)
	if cmd.ProcessState.ExitCode() != 1 {
		t.Fatalf("expected exit 1, got %d", cmd.ProcessState.ExitCode())
	}
	if !bytes.Contains(stdout.Bytes(), []byte("DamagedFrame")) {
		t.Fatalf("ledger did not list DamagedFrame")
	}
}

func TestVerifyProofFixture(t *testing.T) {
	cmd, stdout, stderr := run(t, "verify-proof", proofVector(t, "mmr-basic-proof.json"))
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("expected exit 0, got %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	if !bytes.Contains(stdout.Bytes(), []byte("proof ok")) {
		t.Fatalf("stdout did not report proof ok: %s", stdout.String())
	}
}

func TestVerifyProofRejectsBadRoot(t *testing.T) {
	cmd, _, stderr := run(t, "verify-proof", proofVector(t, "mmr-basic-proof-bad-root.json"))
	if cmd.ProcessState.ExitCode() != 1 {
		t.Fatalf("expected exit 1, got %d", cmd.ProcessState.ExitCode())
	}
	if !bytes.Contains(stderr.Bytes(), []byte("invalid proof")) {
		t.Fatalf("stderr did not name invalid proof: %s", stderr.String())
	}
}

func TestReplicationVerbsEmitJSONShapesAndResumeBoundary(t *testing.T) {
	tmp := t.TempDir()
	first := writer.New("generic")
	firstHead := first.AddBlob([]byte("a"), "text/plain", "")
	firstBytes := first.ToBytes()
	second := writer.New("generic")
	secondHead := second.AddBlob([]byte("b"), "text/plain", "")
	secondBytes := second.ToBytes()
	data := append(append([]byte{}, firstBytes...), secondBytes...)
	path := filepath.Join(tmp, "replicated.gts")
	if err := os.WriteFile(path, data, 0o644); err != nil { //nolint:gosec // test fixture.
		t.Fatal(err)
	}

	cmd, stdout, stderr := run(t, "heads", path)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("heads exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	var heads struct {
		Schema       string   `json:"schema"`
		Clean        bool     `json:"clean"`
		SegmentHeads []string `json:"segment_heads"`
		Aggregate    struct {
			Schema   string  `json:"schema"`
			Count    int     `json:"count"`
			FileHead *string `json:"file_head"`
		} `json:"aggregate"`
		Fatal *struct{} `json:"fatal"`
	}
	if err := json.Unmarshal(stdout.Bytes(), &heads); err != nil {
		t.Fatal(err)
	}
	if heads.Schema != "gts-replication-heads-v1" || !heads.Clean {
		t.Fatalf("bad heads doc: %s", stdout.String())
	}
	wantHeads := []string{wire.Hex(firstHead), wire.Hex(secondHead)}
	if fmt.Sprint(heads.SegmentHeads) != fmt.Sprint(wantHeads) {
		t.Fatalf("segment heads = %v, want %v", heads.SegmentHeads, wantHeads)
	}
	if heads.Aggregate.Schema != "gts-segment-heads-v1" || heads.Aggregate.Count != 2 {
		t.Fatalf("bad aggregate: %+v", heads.Aggregate)
	}
	if heads.Aggregate.FileHead == nil || *heads.Aggregate.FileHead != wire.Hex(secondHead) {
		t.Fatalf("file head = %v, want %s", heads.Aggregate.FileHead, wire.Hex(secondHead))
	}
	if heads.Fatal != nil {
		t.Fatalf("fatal = %+v, want nil", heads.Fatal)
	}

	cmd, stdout, stderr = run(t, "segments", path)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("segments exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	var segments struct {
		Schema    string `json:"schema"`
		Clean     bool   `json:"clean"`
		ItemCount int    `json:"item_count"`
		Segments  []struct {
			ByteRange struct {
				Start  int `json:"start"`
				End    int `json:"end"`
				Length int `json:"length"`
			} `json:"byte_range"`
			FrameCount int `json:"frame_count"`
		} `json:"segments"`
	}
	if err := json.Unmarshal(stdout.Bytes(), &segments); err != nil {
		t.Fatal(err)
	}
	if segments.Schema != "gts-replication-segments-v1" || !segments.Clean || segments.ItemCount != 4 {
		t.Fatalf("bad segments doc: %s", stdout.String())
	}
	if got := segments.Segments[0].ByteRange; got.Start != 0 || got.End != len(firstBytes) || got.Length != len(firstBytes) {
		t.Fatalf("first range = %+v", got)
	}
	if got := segments.Segments[1].ByteRange; got.Start != len(firstBytes) || got.End != len(data) || got.Length != len(secondBytes) {
		t.Fatalf("second range = %+v", got)
	}
	if segments.Segments[0].FrameCount != 1 {
		t.Fatalf("frame count = %d, want 1", segments.Segments[0].FrameCount)
	}

	cmd, stdout, stderr = run(t, "missing", "--from-head", wire.Hex(firstHead), path)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("missing exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	var missing struct {
		Schema   string `json:"schema"`
		Status   string `json:"status"`
		FromHead string `json:"from_head"`
		Ranges   []struct {
			Start  int `json:"start"`
			End    int `json:"end"`
			Length int `json:"length"`
		} `json:"ranges"`
		ScanRequired bool    `json:"scan_required"`
		Detail       *string `json:"detail"`
	}
	if err := json.Unmarshal(stdout.Bytes(), &missing); err != nil {
		t.Fatal(err)
	}
	if missing.Schema != "gts-replication-missing-v1" || missing.Status != "ranges" {
		t.Fatalf("bad missing doc: %s", stdout.String())
	}
	if missing.FromHead != wire.Hex(firstHead) || missing.ScanRequired || missing.Detail != nil {
		t.Fatalf("bad missing metadata: %+v", missing)
	}
	if got := missing.Ranges[0]; got.Start != len(firstBytes) || got.End != len(data) || got.Length != len(secondBytes) {
		t.Fatalf("missing range = %+v", got)
	}

	cmd, stdout, stderr = run(t, "resume", "--after", wire.Hex(firstHead), path)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("resume exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	if !bytes.Equal(stdout.Bytes(), secondBytes) {
		t.Fatalf("resume bytes differ from second segment")
	}
}

func TestCatComposesCleanInputs(t *testing.T) {
	a := vector(t, "01-minimal.gts")
	b := vector(t, "14-bnode-label.gts")
	cmd, stdout, _ := run(t, "cat", a, b)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("expected exit 0, got %d", cmd.ProcessState.ExitCode())
	}
	//nolint:gosec // test reads frozen conformance vectors by name.
	adata, err := os.ReadFile(a)
	if err != nil {
		t.Fatal(err)
	}
	//nolint:gosec // test reads frozen conformance vectors by name.
	bdata, err := os.ReadFile(b)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(stdout.Bytes(), append(adata, bdata...)) {
		t.Fatalf("cat output is not raw concatenation")
	}
}

func TestCatRefusesDamagedInput(t *testing.T) {
	a := vector(t, "01-minimal.gts")
	b := vector(t, "04-damaged-frame.gts")
	cmd, _, stderr := run(t, "cat", a, b)
	if cmd.ProcessState.ExitCode() != 1 {
		t.Fatalf("expected exit 1, got %d", cmd.ProcessState.ExitCode())
	}
	if !bytes.Contains(stderr.Bytes(), []byte("refusing")) {
		t.Fatalf("stderr did not name refusal: %s", stderr.String())
	}
}

func TestLsListsDigestSizeAndMediaType(t *testing.T) {
	v := vector(t, "22-inline-blob.gts")
	cmd, stdout, _ := run(t, "ls", v)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("expected exit 0, got %d", cmd.ProcessState.ExitCode())
	}
	out := stdout.String()

	var found bool
	for _, line := range strings.Split(strings.TrimSpace(out), "\n") {
		fields := strings.Fields(line)
		if len(fields) != 3 {
			continue
		}
		if !strings.HasPrefix(fields[0], "blake3:") {
			continue
		}
		found = true
		if fields[1] != "21" {
			t.Fatalf("size not 21: %s", line)
		}
		if fields[2] != "image/webp" {
			t.Fatalf("media type not image/webp: %s", line)
		}
	}
	if !found {
		t.Fatalf("no blob line found in: %s", out)
	}
}

func TestPackUnpackRoundTrip(t *testing.T) {
	tmp := t.TempDir()
	src := filepath.Join(tmp, "src")
	//nolint:gosec // test fixtures need world-readable permissions.
	if err := os.MkdirAll(filepath.Join(src, "subdir"), 0o755); err != nil {
		t.Fatal(err)
	}
	//nolint:gosec // test fixtures need world-readable permissions.
	if err := os.WriteFile(filepath.Join(src, "a.txt"), []byte("hello"), 0o644); err != nil {
		t.Fatal(err)
	}
	//nolint:gosec // test fixtures need world-readable permissions.
	if err := os.WriteFile(filepath.Join(src, "subdir", "b.txt"), []byte("world"), 0o644); err != nil {
		t.Fatal(err)
	}

	archive := filepath.Join(tmp, "out.gts")
	cmd, _, stderr := run(t, "pack", src, "-o", archive)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("pack exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}

	dst := filepath.Join(tmp, "dst")
	cmd, _, stderr = run(t, "unpack", archive, "-C", dst)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("unpack exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	if got := readFile(t, filepath.Join(dst, "a.txt")); got != "hello" {
		t.Fatalf("a.txt: got %q", got)
	}
	if got := readFile(t, filepath.Join(dst, "subdir", "b.txt")); got != "world" {
		t.Fatalf("subdir/b.txt: got %q", got)
	}

	archive2 := filepath.Join(tmp, "out2.gts")
	cmd, _, stderr = run(t, "pack", dst, "-o", archive2)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("re-pack exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	//nolint:gosec // test reads back the archive it just wrote to a temp path.
	orig, err := os.ReadFile(archive)
	if err != nil {
		t.Fatal(err)
	}
	//nolint:gosec // test reads back the archive it just wrote to a temp path.
	repack, err := os.ReadFile(archive2)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(orig, repack) {
		t.Fatalf("re-packed archive differs")
	}
}

func readFile(t *testing.T, path string) string {
	t.Helper()
	//nolint:gosec // test helper reads files from temp directories.
	b, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return string(b)
}

// --------------------------------------------------------------------------
// gts compact --streamable (§10.1, §14.1) + layout reporting (§3.3)
// --------------------------------------------------------------------------

func accretiveFile(t *testing.T, dir string) string {
	t.Helper()
	w := writer.New("generic")
	w.AddBlob(bytes.Repeat([]byte("Z"), 64), "application/octet-stream", "") // blob before catalog
	w.AddTerms([]model.Term{
		{Kind: model.Iri, Value: "https://example.org/Cat"},
		{Kind: model.Iri, Value: "http://www.w3.org/2000/01/rdf-schema#label"},
		{Kind: model.Literal, Value: "Cat", Lang: "en"},
	})
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	path := filepath.Join(dir, "accretive.gts")
	//nolint:gosec // test fixture written to a temp path.
	if err := os.WriteFile(path, w.ToBytes(), 0o644); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestCompactRequiresStreamableFlag(t *testing.T) {
	tmp := t.TempDir()
	path := accretiveFile(t, tmp)
	cmd, _, stderr := run(t, "compact", path, "-o", filepath.Join(tmp, "x.gts"))
	if cmd.ProcessState.ExitCode() != 2 {
		t.Fatalf("expected exit 2, got %d", cmd.ProcessState.ExitCode())
	}
	if !bytes.Contains(stderr.Bytes(), []byte("compact requires --streamable")) {
		t.Fatalf("stderr did not name the missing mode: %s", stderr.String())
	}
}

func TestCompactVerifyInfoRoundTrip(t *testing.T) {
	tmp := t.TempDir()
	path := accretiveFile(t, tmp)
	out := filepath.Join(tmp, "streamable.gts")
	cmd, _, stderr := run(t, "compact", path, "-o", out, "--streamable", "--timestamp", "2026-01-01T00:00:00Z")
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("compact exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	cmd, stdout, stderr := run(t, "verify", out)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("verify exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	if !strings.Contains(stdout.String(), "layout: streamable through frame") {
		t.Fatalf("ledger missing layout line: %s", stdout.String())
	}
	if strings.Contains(stdout.String(), "accretive tail") {
		t.Fatalf("clean compaction must not report a tail: %s", stdout.String())
	}
	if strings.Contains(stderr.String(), "warning") {
		t.Fatalf("unexpected warning: %s", stderr.String())
	}
}

func TestCompactReproducesFrozenVectorViaCLI(t *testing.T) {
	tmp := t.TempDir()
	out := filepath.Join(tmp, "compacted.gts")
	cmd, _, stderr := run(t, "compact", vector(t, "25-streamable-source.gts"),
		"-o", out, "--streamable", "--timestamp", "2026-01-01T00:00:00Z")
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("compact exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	//nolint:gosec // test reads back the CLI output and a frozen vector.
	got, err := os.ReadFile(out)
	if err != nil {
		t.Fatal(err)
	}
	//nolint:gosec // test reads a frozen conformance vector by name.
	expected, err := os.ReadFile(vector(t, "25b-streamable-compacted.gts"))
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(got, expected) {
		t.Fatalf("compacted bytes diverge from the frozen oracle (§14.1)")
	}
}

func TestVerifyRefusesStreamableLie(t *testing.T) {
	cmd, stdout, _ := run(t, "verify", vector(t, "26-streamable-lie.gts"))
	if cmd.ProcessState.ExitCode() != 1 {
		t.Fatalf("expected exit 1, got %d", cmd.ProcessState.ExitCode())
	}
	if !strings.Contains(stdout.String(), "StreamableLayoutError") {
		t.Fatalf("ledger did not list StreamableLayoutError: %s", stdout.String())
	}
}

func TestInfoReportsAccretiveTail(t *testing.T) {
	cmd, stdout, _ := run(t, "info", vector(t, "27-streamable-tail.gts"))
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("expected exit 0, got %d", cmd.ProcessState.ExitCode())
	}
	if !strings.Contains(stdout.String(), "layout: streamable through frame") {
		t.Fatalf("ledger missing layout line: %s", stdout.String())
	}
	if !strings.Contains(stdout.String(), "accretive tail 2 frame(s)") {
		t.Fatalf("ledger missing the accretive tail: %s", stdout.String())
	}
}

func TestVerifyWarnsOnStreamVocabWithoutClaim(t *testing.T) {
	// §13.3: stream# provenance in an unclaimed segment is a warning, never
	// an error — it legitimately survives nq → gts round trips.
	tmp := t.TempDir()
	w := writer.New("generic")
	w.AddTerms([]model.Term{
		{Kind: model.Bnode, Value: "c"},
		{Kind: model.Iri, Value: stream.Compaction},
		{Kind: model.Literal, Value: stream.CompactAgent},
	})
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	path := filepath.Join(tmp, "unclaimed-stream.gts")
	//nolint:gosec // test fixture written to a temp path.
	if err := os.WriteFile(path, w.ToBytes(), 0o644); err != nil {
		t.Fatal(err)
	}
	cmd, _, stderr := run(t, "verify", path)
	if cmd.ProcessState.ExitCode() != 0 { // warning, exit stays 0
		t.Fatalf("expected exit 0, got %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	if !strings.Contains(stderr.String(), "layout warning") {
		t.Fatalf("stderr missing the layout warning: %s", stderr.String())
	}
}

func TestCompactRefusalExitsOne(t *testing.T) {
	tmp := t.TempDir()
	w := writer.New("evidence")
	w.AddTerms([]model.Term{
		{Kind: model.Iri, Value: "https://example.org/Cat"},
		{Kind: model.Iri, Value: "http://www.w3.org/2000/01/rdf-schema#label"},
		{Kind: model.Literal, Value: "Cat", Lang: "en"},
	})
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	path := filepath.Join(tmp, "evidence.gts")
	//nolint:gosec // test fixture written to a temp path.
	if err := os.WriteFile(path, w.ToBytes(), 0o644); err != nil {
		t.Fatal(err)
	}
	out := filepath.Join(tmp, "out.gts")
	cmd, _, stderr := run(t, "compact", path, "-o", out, "--streamable")
	if cmd.ProcessState.ExitCode() != 1 {
		t.Fatalf("expected exit 1, got %d", cmd.ProcessState.ExitCode())
	}
	if !strings.Contains(stderr.String(), "seal-original") {
		t.Fatalf("refusal did not name seal-original: %s", stderr.String())
	}
	cmd, _, stderr = run(t, "compact", path, "-o", out, "--streamable", "--seal-original")
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("seal-original compact exit %d: %s", cmd.ProcessState.ExitCode(), stderr.String())
	}
}

func TestVerifyEnforcesDeclaredVsComputedProfiles(t *testing.T) {
	tmp := t.TempDir()
	// files# vocabulary in a generic segment: an error, exit 1 (§14.1).
	w := writer.New("generic")
	w.AddTerms([]model.Term{
		{Kind: model.Bnode, Value: "f0"},
		{Kind: model.Iri, Value: "https://w3id.org/gts/files#path"},
		{Kind: model.Literal, Value: "a.txt"},
	})
	w.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	undeclared := filepath.Join(tmp, "undeclared.gts")
	//nolint:gosec // test fixture written to a temp path.
	if err := os.WriteFile(undeclared, w.ToBytes(), 0o644); err != nil {
		t.Fatal(err)
	}
	cmd, _, stderr := run(t, "verify", undeclared)
	if cmd.ProcessState.ExitCode() != 1 {
		t.Fatalf("verify exit = %d, want 1\n%s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	if !strings.Contains(stderr.String(), "profile error: segment uses https://w3id.org/gts/files#") {
		t.Fatalf("missing profile error, got:\n%s", stderr.String())
	}

	// declared-but-unused profile: a warning, exit stays 0.
	w2 := writer.New("files")
	w2.AddTerms([]model.Term{
		{Kind: model.Iri, Value: "https://example.org/Cat"},
		{Kind: model.Iri, Value: "http://www.w3.org/2000/01/rdf-schema#label"},
		{Kind: model.Literal, Value: "Cat", Lang: "en"},
	})
	w2.AddQuads([]model.Quad{{S: 0, P: 1, O: 2}})
	unused := filepath.Join(tmp, "unused.gts")
	//nolint:gosec // test fixture written to a temp path.
	if err := os.WriteFile(unused, w2.ToBytes(), 0o644); err != nil {
		t.Fatal(err)
	}
	cmd, _, stderr = run(t, "verify", unused)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("verify exit = %d, want 0\n%s", cmd.ProcessState.ExitCode(), stderr.String())
	}
	if !strings.Contains(stderr.String(), "profile warning: segment declares 'files'") {
		t.Fatalf("missing profile warning, got:\n%s", stderr.String())
	}
}

// TestExtractKeyMatchesFrozenStdout pins `gts extract-key` output (kid,
// fingerprint, emojihash, armored key) to the Python-generated vector.
func TestExtractKeyMatchesFrozenStdout(t *testing.T) {
	raw, err := os.ReadFile(vector(t, filepath.Join("openpgp", "extract-key.json")))
	if err != nil {
		t.Fatalf("vectors/openpgp/extract-key.json must exist: %v", err)
	}
	var c struct {
		GTS    string `json:"gts"`
		Stdout string `json:"stdout"`
	}
	if err := json.Unmarshal(raw, &c); err != nil {
		t.Fatal(err)
	}
	data, err := hex.DecodeString(c.GTS)
	if err != nil {
		t.Fatal(err)
	}
	f := filepath.Join(t.TempDir(), "signed.gts")
	if err := os.WriteFile(f, data, 0o644); err != nil { //nolint:gosec // test fixture.
		t.Fatal(err)
	}

	cmd, stdout, _ := run(t, "extract-key", f)
	if cmd.ProcessState.ExitCode() != 0 {
		t.Fatalf("exit = %d, want 0", cmd.ProcessState.ExitCode())
	}
	if stdout.String() != c.Stdout {
		t.Errorf("stdout mismatch:\n got %q\nwant %q", stdout.String(), c.Stdout)
	}
}

func TestExtractKeyMissingExits1(t *testing.T) {
	cmd, _, _ := run(t, "extract-key", vector(t, "01-minimal.gts"))
	if cmd.ProcessState.ExitCode() != 1 {
		t.Errorf("exit = %d, want 1", cmd.ProcessState.ExitCode())
	}
}
