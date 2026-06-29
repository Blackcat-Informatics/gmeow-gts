// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package reader

import (
	"context"
	"errors"
	"fmt"
	"io"

	"github.com/fxamacker/cbor/v2"

	"go.blackcatinformatics.ca/gts/model"
	"go.blackcatinformatics.ca/gts/wire"
)

// StreamingEventKind identifies one event emitted by ReadToSink.
type StreamingEventKind string

// StreamingEventTerm and related constants identify ReadToSink event kinds.
const (
	StreamingEventTerm             StreamingEventKind = "term"
	StreamingEventQuad             StreamingEventKind = "quad"
	StreamingEventReifier          StreamingEventKind = "reifier"
	StreamingEventAnnotation       StreamingEventKind = "annotation"
	StreamingEventSuppression      StreamingEventKind = "suppression"
	StreamingEventBlob             StreamingEventKind = "blob"
	StreamingEventOpaque           StreamingEventKind = "opaque"
	StreamingEventSignature        StreamingEventKind = "signature"
	StreamingEventDiagnostic       StreamingEventKind = "diagnostic"
	StreamingEventSegmentHead      StreamingEventKind = "segment-head"
	StreamingEventStreamableLayout StreamingEventKind = "streamable-layout"
)

// StreamingEvent is a segment-local fold event emitted by ReadToSink.
//
// FrameIndex is -1 when the event is file-level rather than tied to one frame.
// SegmentIndex is -1 for file-level diagnostics such as expected-head mismatch
// and pre-segment-mode segment-boundary refusal.
type StreamingEvent struct {
	// Kind selects which payload field is populated.
	Kind StreamingEventKind
	// SegmentIndex is the zero-based segment being folded, or -1 for file-level events.
	SegmentIndex int
	// FrameIndex is the absolute CBOR item index for the source frame, or -1.
	FrameIndex int

	// TermID is the segment-local term id for Term events.
	TermID int
	// Term is the decoded term for StreamingEventTerm.
	Term model.Term
	// Quad is the decoded quad for StreamingEventQuad.
	Quad model.Quad
	// ReifierID is the segment-local reifier id for StreamingEventReifier.
	ReifierID int
	// Triple is the decoded triple for reifier events.
	Triple model.Triple3
	// ReifierGraph is the optional named graph term id for reifier events.
	ReifierGraph *int
	// Annotation is the decoded annotation row for StreamingEventAnnotation.
	Annotation model.AnnotationEntry
	// Suppression is the decoded suppress directive for StreamingEventSuppression.
	Suppression model.Suppression

	// BlobDigest is the content-addressed digest for blob events.
	BlobDigest string
	// BlobData is the decoded inline blob bytes for blob events.
	BlobData []byte
	// BlobMeta is the optional public blob metadata attached to a blob frame.
	BlobMeta interface{}

	// Opaque records an undecodable frame for unknown-codec, missing-key, or damage.
	Opaque model.OpaqueNode
	// Signature records the signature status observed for a signed frame.
	Signature model.Signature
	// Diagnostic is the reader diagnostic emitted at this point in the fold.
	Diagnostic model.Diagnostic
	// SegmentHead is the final id/prev head for a completed segment.
	SegmentHead []byte
	// Streamable records declared-vs-computed layout state for the completed segment.
	Streamable model.StreamableInfo
}

// StreamingSink receives fold events from ReadToSink; returning an error stops
// the stream and returns that error to the caller.
type StreamingSink interface {
	Accept(StreamingEvent) error
}

// StreamingSinkFunc adapts a function to StreamingSink.
type StreamingSinkFunc func(StreamingEvent) error

// Accept calls f(event).
func (f StreamingSinkFunc) Accept(event StreamingEvent) error {
	return f(event)
}

// StreamingReadResult carries final reader sidecar state from a streaming fold.
type StreamingReadResult struct {
	// Diagnostics is the final diagnostic list, matching the total reader.
	Diagnostics []model.Diagnostic
	// SegmentHeads are final segment heads in file order.
	SegmentHeads [][]byte
	// SegmentProfiles are header profile names in file order.
	SegmentProfiles []string
	// SegmentMeta snapshots per-segment meta entries at segment completion.
	SegmentMeta [][]model.MetaEntry
	// SegmentStreamable records layout-state checks in file order.
	SegmentStreamable []model.StreamableInfo
	// Torn is the offset of an incomplete trailing CBOR item, or -1 when clean.
	Torn int
}

func emptyStreamingReadResult() *StreamingReadResult {
	return &StreamingReadResult{
		Diagnostics:       []model.Diagnostic{},
		SegmentHeads:      [][]byte{},
		SegmentProfiles:   []string{},
		SegmentMeta:       [][]model.MetaEntry{},
		SegmentStreamable: []model.StreamableInfo{},
		Torn:              -1,
	}
}

type contextReader struct {
	ctx context.Context
	r   io.Reader
}

func (r *contextReader) Read(p []byte) (int, error) {
	if err := r.ctx.Err(); err != nil {
		return 0, err
	}
	return r.r.Read(p)
}

func eventFrameIndex(index *int) int {
	if index == nil {
		return -1
	}
	return *index
}

func (r *StreamingReadResult) addDiagnostic(sink StreamingSink, diag model.Diagnostic) error {
	r.Diagnostics = append(r.Diagnostics, diag)
	if sink == nil {
		return nil
	}
	return sink.Accept(StreamingEvent{
		Kind:         StreamingEventDiagnostic,
		SegmentIndex: -1,
		FrameIndex:   eventFrameIndex(diag.FrameIndex),
		Diagnostic:   diag,
	})
}

func (r *StreamingReadResult) appendSegment(g *model.Graph) {
	r.Diagnostics = append(r.Diagnostics, g.Diagnostics...)
	r.SegmentHeads = append(r.SegmentHeads, g.SegmentHeads...)
	r.SegmentProfiles = append(r.SegmentProfiles, g.SegmentProfiles...)
	r.SegmentMeta = append(r.SegmentMeta, g.SegmentMeta...)
	r.SegmentStreamable = append(r.SegmentStreamable, g.SegmentStreamable...)
}

type streamingSegment struct {
	g            *model.Graph
	header       map[interface{}]interface{}
	expectedPrev []byte
	fld          *folder
	frameIDs     [][]byte
	indexOffset  int
	validHeader  bool
	finished     bool
}

func newStreamingSegment(rawHeader interface{}, indexOffset, segmentIndex int, sink StreamingSink) *streamingSegment {
	g := emptyGraph()
	fld := &folder{
		g:            g,
		segmentIndex: segmentIndex,
		sink:         sink,
		materialize:  false,
		described:    make(map[string]struct{}),
	}
	seg := &streamingSegment{g: g, fld: fld, indexOffset: indexOffset}
	header, err := wire.UnwrapHeader(rawHeader)
	if err != nil {
		idx := indexOffset
		fld.diag("DamagedFrame", fmt.Sprintf("invalid header: %v", err), &idx)
		return seg
	}
	seg.header = header
	seg.validHeader = true
	fld.catalog = catalogFrom(header)

	var storedHID []byte
	if v, ok := wire.MapGet(header, "id"); ok {
		storedHID, _ = v.([]byte)
	}
	if !bytesEqual(storedHID, wire.HeaderID(header)) {
		idx := indexOffset
		fld.diag("DamagedFrame", "header self-hash mismatch", &idx)
	}
	headerMagic, _ := wire.MapGet(header, "gts")
	headerVersion, _ := wire.MapGet(header, "v")
	version, versionOK := asInt64(headerVersion)
	if textOr(headerMagic, "") != wire.Magic || !versionOK || version != int64(wire.Version) {
		idx := indexOffset
		fld.diag(
			"DamagedFrame",
			fmt.Sprintf("unsupported header magic/version %v/%v", headerMagic, headerVersion),
			&idx,
		)
	}
	seg.expectedPrev = storedHID
	return seg
}

func (s *streamingSegment) processFrame(item interface{}, absIndex int) {
	if !s.validHeader || s.fld.eventErr != nil {
		return
	}
	frame, ok := item.(map[interface{}]interface{})
	if !ok {
		s.fld.diag("DamagedFrame", "frame is not a map", &absIndex)
		s.frameIDs = append(s.frameIDs, []byte{})
		return
	}
	var storedID []byte
	if v, ok := frame["id"]; ok {
		storedID, _ = v.([]byte)
	}
	computed := wire.ContentID(frame)
	if !bytesEqual(storedID, computed) {
		s.fld.diag("DamagedFrame", "frame self-hash mismatch", &absIndex)
		ftype := textOr(frame["t"], "")
		s.fld.opaque(frame, ftype, "damaged", absIndex)
		if storedID != nil {
			s.expectedPrev = storedID
		} else {
			s.expectedPrev = computed
		}
		s.frameIDs = append(s.frameIDs, s.expectedPrev)
		return
	}
	prevOk := false
	if v, ok := frame["prev"]; ok {
		if b, ok := v.([]byte); ok {
			prevOk = bytesEqual(b, s.expectedPrev)
		}
	}
	if !prevOk {
		s.fld.diag("BrokenChain", "prev does not match", &absIndex)
	}
	s.expectedPrev = computed
	s.frameIDs = append(s.frameIDs, s.expectedPrev)
	if sig, ok := frame["sig"]; ok {
		if cose, ok := sig.([]byte); ok {
			s.fld.pushSignature(model.Signature{FrameID: computed, Status: "unverified", Cose: cose}, absIndex)
		} else {
			s.fld.pushSignature(model.Signature{FrameID: computed, Status: "invalid"}, absIndex)
		}
	}
	s.fld.foldFrame(frame, absIndex)
}

func (s *streamingSegment) finish() *model.Graph {
	if s.finished || !s.validHeader {
		return s.g
	}
	s.finished = true
	head := append([]byte(nil), s.expectedPrev...)
	s.g.SegmentHeads = append(s.g.SegmentHeads, head)
	segMeta := make([]model.MetaEntry, len(s.g.Meta))
	copy(segMeta, s.g.Meta)
	s.g.SegmentMeta = append(s.g.SegmentMeta, segMeta)
	s.g.SegmentProfiles = append(s.g.SegmentProfiles, textOr(s.header["prof"], "generic"))
	info := layoutCheck(s.header, s.fld, s.frameIDs, s.indexOffset)
	s.g.SegmentStreamable = append(s.g.SegmentStreamable, info)
	s.fld.emit(StreamingEvent{
		Kind:        StreamingEventSegmentHead,
		FrameIndex:  -1,
		SegmentHead: head,
	})
	s.fld.emit(StreamingEvent{
		Kind:       StreamingEventStreamableLayout,
		FrameIndex: -1,
		Streamable: info,
	})
	return s.g
}

func streamingSource(ctx context.Context, r io.Reader, maxBytes int64) (io.Reader, error) {
	if maxBytes < 0 {
		return nil, errors.New("gts reader: MaxBytes must be >= 0")
	}
	source := io.Reader(&contextReader{ctx: ctx, r: r})
	if maxBytes > 0 {
		source = io.LimitReader(source, maxBytes+1)
	}
	return source, nil
}

// ReadToSink reads a GTS CBOR Sequence from r and emits fold events without
// constructing the final union Graph.
//
// The stream still retains each segment's term dictionary and validation
// sidecar state needed to preserve the same diagnostics and segment heads as
// Read. Options has the same meaning as ReadFrom: AllowSegments controls
// segment-boundary handling, ExpectedHead checks the final segment head, and
// MaxBytes bounds bytes consumed from r. The emitted events use segment-local
// term ids; callers that need a cross-segment union should continue using Read.
func ReadToSink(ctx context.Context, r io.Reader, opts Options, sink StreamingSink) (*StreamingReadResult, error) {
	if ctx == nil {
		ctx = context.Background()
	}
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	if r == nil {
		return nil, errors.New("gts reader: nil reader")
	}
	if sink == nil {
		return nil, errors.New("gts reader: nil streaming sink")
	}
	source, err := streamingSource(ctx, r, opts.MaxBytes)
	if err != nil {
		return nil, err
	}
	dec := cbor.NewDecoder(source)
	result := emptyStreamingReadResult()
	var current *streamingSegment
	itemIndex := 0
	segmentIndex := 0

	finishCurrent := func() error {
		if current == nil {
			return nil
		}
		result.appendSegment(current.finish())
		if current.fld.eventErr != nil {
			return current.fld.eventErr
		}
		return nil
	}

	for {
		if err := ctx.Err(); err != nil {
			return nil, err
		}
		itemStart := dec.NumBytesRead()
		var item interface{}
		err := dec.Decode(&item)
		if opts.MaxBytes > 0 && int64(dec.NumBytesRead()) > opts.MaxBytes {
			return nil, ErrReadLimitExceeded
		}
		if err != nil {
			if errors.Is(err, io.EOF) {
				break
			}
			if ctxErr := ctx.Err(); ctxErr != nil {
				return nil, ctxErr
			}
			if itemIndex == 0 {
				idx := 0
				diag := model.Diagnostic{Code: "EmptyFile", Detail: "no CBOR items", FrameIndex: &idx}
				if err := result.addDiagnostic(sink, diag); err != nil {
					return nil, err
				}
				return result, nil
			}
			result.Torn = itemStart
			break
		}

		if isHeaderItem(item) {
			if current != nil {
				if err := finishCurrent(); err != nil {
					return nil, err
				}
				if !opts.AllowSegments {
					idx := itemIndex
					diag := model.Diagnostic{
						Code:       "SegmentBoundary",
						Detail:     fmt.Sprintf("segment boundary at item %d but reader is in pre-segment mode; remainder of file NOT folded", idx),
						FrameIndex: &idx,
					}
					if err := result.addDiagnostic(sink, diag); err != nil {
						return nil, err
					}
					return result, nil
				}
				segmentIndex++
			}
			current = newStreamingSegment(item, itemIndex, segmentIndex, sink)
			if current.fld.eventErr != nil {
				return nil, current.fld.eventErr
			}
		} else {
			if current == nil {
				idx := 0
				diag := model.Diagnostic{Code: "DamagedFrame", Detail: "first item is not a header", FrameIndex: &idx}
				if err := result.addDiagnostic(sink, diag); err != nil {
					return nil, err
				}
				return result, nil
			}
			current.processFrame(item, itemIndex)
			if current.fld.eventErr != nil {
				return nil, current.fld.eventErr
			}
		}
		itemIndex++
	}

	if itemIndex == 0 {
		idx := 0
		diag := model.Diagnostic{Code: "EmptyFile", Detail: "no CBOR items", FrameIndex: &idx}
		if err := result.addDiagnostic(sink, diag); err != nil {
			return nil, err
		}
		return result, nil
	}
	if err := finishCurrent(); err != nil {
		return nil, err
	}
	if opts.ExpectedHead != nil {
		var lastHead []byte
		if len(result.SegmentHeads) > 0 {
			lastHead = result.SegmentHeads[len(result.SegmentHeads)-1]
		}
		if !bytesEqual(lastHead, opts.ExpectedHead) {
			diag := model.Diagnostic{Code: "TruncatedLog", Detail: "observed head does not match expected head"}
			if err := result.addDiagnostic(sink, diag); err != nil {
				return nil, err
			}
		}
	}
	if result.Torn >= 0 {
		diag := model.Diagnostic{Code: "TornAppendError", Detail: fmt.Sprintf("torn at offset %d", result.Torn)}
		if err := result.addDiagnostic(sink, diag); err != nil {
			return nil, err
		}
	}
	return result, nil
}
