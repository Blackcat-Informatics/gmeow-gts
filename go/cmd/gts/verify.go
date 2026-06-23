// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package main

import (
	"crypto/ed25519"
	"encoding/hex"
	"fmt"
	"os"
	"strings"

	"go.blackcatinformatics.ca/gts/cose"
	"go.blackcatinformatics.ca/gts/emojihash"
	"go.blackcatinformatics.ca/gts/mmr"
	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/openpgp"
	"go.blackcatinformatics.ca/gts/reader"
	"go.blackcatinformatics.ca/gts/wire"
)

func parseKey(spec string) (string, ed25519.PublicKey, bool) {
	idx := strings.IndexByte(spec, ':')
	if idx <= 0 {
		return "", nil, false
	}
	raw, err := hex.DecodeString(spec[idx+1:])
	if err != nil || len(raw) != ed25519.PublicKeySize {
		return "", nil, false
	}
	return spec[:idx], ed25519.PublicKey(raw), true
}

func cmdVerify(args []string) int {
	var paths []string
	keys := map[string]ed25519.PublicKey{}
	for i := 0; i < len(args); i++ {
		if args[i] == "--key" {
			i++
			if i >= len(args) {
				fmt.Fprintln(os.Stderr, usage)
				return 2
			}
			kid, pub, ok := parseKey(args[i])
			if !ok {
				fmt.Fprintf(os.Stderr, "gts verify: bad --key %q (want kid:hexpubkey)\n", args[i])
				return 2
			}
			keys[kid] = pub
		} else {
			paths = append(paths, args[i])
		}
	}
	if len(paths) == 0 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	problems := false
	for _, path := range paths {
		data, code := load(path)
		if code != 0 {
			return code
		}
		fs := reader.ReadFileSegments(data)
		printLedger(path, fs)
		if hasProblems(fs) {
			problems = true
		}
		// §14.1: declared-vs-computed profile requirements + layout warnings.
		for idx, seg := range fs.Segments {
			for _, check := range profileCheck(seg) {
				prefix := "warning"
				if check.IsErr {
					prefix = "error"
					problems = true
				}
				fmt.Fprintf(os.Stderr, "  segment %d: %s: %s\n", idx, prefix, check.Msg)
			}
			for _, msg := range streamVocabCheck(seg) {
				fmt.Fprintf(os.Stderr, "  segment %d: warning: %s\n", idx, msg)
			}
		}
		// §9.2: COSE signature verification against the provided keys.
		if len(keys) > 0 {
			g := reader.Read(data, true, nil)
			cose.VerifySignatures(g.Signatures, func(kid string) (ed25519.PublicKey, bool) {
				k, ok := keys[kid]
				return k, ok
			})
			for _, sig := range g.Signatures {
				kid := sig.Kid
				if kid == "" {
					kid = "?"
				}
				fmt.Printf("  signature %s: %s\n", kid, sig.Status)
				if sig.Status == "invalid" {
					problems = true
				}
			}
		}
	}
	if problems {
		return 1
	}
	return 0
}

func cmdVerifyProof(args []string) int {
	if len(args) != 1 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	path := args[0]
	//nolint:gosec // CLI explicitly reads the user-supplied proof path.
	data, err := os.ReadFile(path)
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts verify-proof: cannot read %s: %v\n", path, err)
		return 2
	}
	proof, err := mmr.ProofFromJSON(data)
	if err != nil {
		fmt.Fprintf(os.Stderr, "gts verify-proof: invalid proof JSON: %v\n", err)
		return 1
	}
	if err := mmr.VerifyProof(proof); err != nil {
		fmt.Fprintf(os.Stderr, "gts verify-proof: invalid proof: %v\n", err)
		return 1
	}
	fmt.Printf("proof ok: root %s frame %s\n", wire.Hex(proof.Root), wire.Hex(proof.FrameID))
	return 0
}

// transportKey finds the embedded gts:transportKey (kid, gpg) in file-level meta.
func transportKey(g *model.Graph) (kid, gpg string, ok bool) {
	for _, e := range g.Meta {
		if e.Key != "gts:transportKey" {
			continue
		}
		m, isMap := e.Value.(map[interface{}]interface{})
		if !isMap {
			return "", "", false
		}
		k, kOK := m["kid"].(string)
		p, pOK := m["gpg"].(string)
		if kOK && pOK {
			return k, p, true
		}
		return "", "", false
	}
	return "", "", false
}

// formatFingerprint groups a hex fingerprint into space-separated 4-char blocks.
func formatFingerprint(fp string) string {
	compact := strings.ToUpper(strings.Join(strings.Fields(fp), ""))
	if compact == "" {
		return fp
	}
	for _, c := range compact {
		if !strings.ContainsRune("0123456789ABCDEF", c) {
			return fp
		}
	}
	var groups []string
	for i := 0; i < len(compact); i += 4 {
		end := i + 4
		if end > len(compact) {
			end = len(compact)
		}
		groups = append(groups, compact[i:end])
	}
	return strings.Join(groups, " ")
}

// cmdExtractKey prints the embedded transport (verification) key (§9.2).
func cmdExtractKey(args []string) int {
	if len(args) == 0 {
		fmt.Fprintln(os.Stderr, usage)
		return 2
	}
	path := args[0]
	data, code := load(path)
	if code != 0 {
		return code
	}
	g := reader.Read(data, true, nil)
	kid, gpg, ok := transportKey(g)
	if !ok {
		fmt.Fprintf(os.Stderr, "%s: no embedded transport key\n", path)
		return 1
	}
	fmt.Printf("kid:         %s\n", kid)
	if key, err := openpgp.ParseTransportKey(gpg); err == nil {
		fmt.Printf("fingerprint: %s\n", formatFingerprint(key.Fingerprint))
		fmt.Printf("emojihash:   %s\n", emojihash.Emojihash(key.RawPublic, 11))
	} else {
		// A malformed embedded key still prints the kid + armored block below.
		fmt.Printf("fingerprint: %s\n", formatFingerprint(kid))
	}
	fmt.Println(gpg)
	return 0
}
