// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// Package xsd validates XML Schema lexical forms used by RDF import paths.
//
// GTS keeps literal lexical forms and datatype IRIs verbatim. This package is
// a syntax-side companion for RDF text importers: recognized invalid XSD
// literals are reported as IllTypedLiteral diagnostics and can be persisted in
// metadata without rewriting the stored terms.
//
// The durable sidecar key is gts:illTypedLiterals. Its value is a map:
//
//	{
//	    "version": 1,
//	    "items": [
//	        {
//	            "term": 5,
//	            "datatype": "http://www.w3.org/2001/XMLSchema#integer",
//	            "lexical": "12.0",
//	            "reason": "integer lexical form contains a non-digit character"
//	        }
//	    ]
//	}
package xsd

import (
	"fmt"
	"math/big"
	"regexp"
	"strconv"
	"strings"

	"go.blackcatinformatics.ca/gts/model"
)

// XML Schema namespace used by RDF typed literals.
const Namespace = "http://www.w3.org/2001/XMLSchema#"

// IllTypedLiteralCode is the stable diagnostic code for invalid recognized XSD literals.
const IllTypedLiteralCode = "IllTypedLiteral"

// IllTypedLiteralMetaKey is the metadata key for ill-typed literal sidecar rows.
const IllTypedLiteralMetaKey = "gts:illTypedLiterals"

// LexicalStatusKind classifies a datatype lexical validation result.
type LexicalStatusKind int

const (
	// StatusValid means the datatype is recognized and the lexical form is valid.
	StatusValid LexicalStatusKind = iota
	// StatusInvalid means the datatype is recognized, but the lexical form is invalid.
	StatusInvalid
	// StatusUnsupported means this syntax layer does not validate the datatype.
	StatusUnsupported
)

// LexicalStatus is the validation result for one literal/datatype pair.
type LexicalStatus struct {
	Kind      LexicalStatusKind
	Canonical string
	Reason    string
}

// IsValid reports whether the datatype is recognized and the lexical form is valid.
func (s LexicalStatus) IsValid() bool { return s.Kind == StatusValid }

// IsInvalid reports whether the datatype is recognized and the lexical form is invalid.
func (s LexicalStatus) IsInvalid() bool { return s.Kind == StatusInvalid }

// IsUnsupported reports whether this package does not cover the datatype.
func (s LexicalStatus) IsUnsupported() bool { return s.Kind == StatusUnsupported }

// IllTypedLiteral is one recognized invalid XSD literal observation.
type IllTypedLiteral struct {
	TermID      int
	DatatypeIRI string
	Lexical     string
	Reason      string
}

// Diagnostic builds the stable graph diagnostic for this observation.
func (i IllTypedLiteral) Diagnostic() model.Diagnostic {
	return model.Diagnostic{
		Code: IllTypedLiteralCode,
		Detail: fmt.Sprintf(
			"term %d literal %q is ill-typed for %s: %s",
			i.TermID,
			i.Lexical,
			i.DatatypeIRI,
			i.Reason,
		),
	}
}

// ValidateLexical validates one lexical form for an XSD datatype IRI.
//
// Unsupported datatypes return StatusUnsupported. Invalid recognized datatypes
// should be flagged by importers, not rejected or rewritten.
func ValidateLexical(datatypeIRI, lexical string) LexicalStatus {
	local, ok := strings.CutPrefix(datatypeIRI, Namespace)
	if !ok {
		return unsupported()
	}

	switch local {
	case "string", "anyURI":
		return valid(lexical)
	case "normalizedString":
		return valid(replaceXMLWhitespace(lexical))
	case "token":
		return valid(collapseXMLWhitespace(lexical))
	case "boolean":
		return validateBoolean(lexical)
	case "decimal":
		return validateDecimal(lexical)
	case "integer":
		return validateInteger(lexical)
	case "nonPositiveInteger", "negativeInteger", "long", "int", "short", "byte",
		"nonNegativeInteger", "unsignedLong", "unsignedInt", "unsignedShort",
		"unsignedByte", "positiveInteger":
		return validateIntegerFamily(local, lexical)
	case "float", "double":
		return validateFloat(local, lexical)
	case "dateTime", "date", "time", "gYearMonth", "gYear", "gMonthDay", "gMonth", "gDay":
		return validateDateTimeFamily(local, lexical)
	case "duration":
		return validateDuration(lexical)
	case "yearMonthDuration":
		return validateYearMonthDuration(lexical)
	case "dayTimeDuration":
		return validateDayTimeDuration(lexical)
	case "hexBinary":
		return validateHexBinary(lexical)
	default:
		return unsupported()
	}
}

// IllTypedLiterals returns invalid recognized XSD literals in a graph.
func IllTypedLiterals(graph *model.Graph) []IllTypedLiteral {
	if graph == nil {
		return nil
	}
	return IllTypedLiteralsInTerms(graph.Terms)
}

// IllTypedLiteralsInTerms returns invalid recognized XSD literals in term order.
func IllTypedLiteralsInTerms(terms []model.Term) []IllTypedLiteral {
	out := []IllTypedLiteral{}
	for termID := range terms {
		term := &terms[termID]
		if term.Kind != model.Literal {
			continue
		}
		datatypeIRI := effectiveDatatypeIRI(terms, term)
		status := ValidateLexical(datatypeIRI, term.Value)
		if status.IsInvalid() {
			out = append(out, IllTypedLiteral{
				TermID:      termID,
				DatatypeIRI: datatypeIRI,
				Lexical:     term.Value,
				Reason:      status.Reason,
			})
		}
	}
	return out
}

// AnnotateIllTypedLiterals appends diagnostics and metadata for invalid XSD literals.
func AnnotateIllTypedLiterals(graph *model.Graph) {
	if graph == nil {
		return
	}
	items := IllTypedLiterals(graph)
	if len(items) == 0 {
		return
	}
	for _, item := range items {
		graph.Diagnostics = append(graph.Diagnostics, item.Diagnostic())
	}
	graph.SetMeta(IllTypedLiteralMetaKey, IllTypedLiteralsMetadata(items))
}

// IllTypedLiteralsMetadata builds the durable sidecar value for invalid literals.
func IllTypedLiteralsMetadata(items []IllTypedLiteral) map[interface{}]interface{} {
	rows := make([]interface{}, len(items))
	for i, item := range items {
		rows[i] = map[interface{}]interface{}{
			"term":     int64(item.TermID),
			"datatype": item.DatatypeIRI,
			"lexical":  item.Lexical,
			"reason":   item.Reason,
		}
	}
	return map[interface{}]interface{}{
		"version": int64(1),
		"items":   rows,
	}
}

func valid(canonical string) LexicalStatus {
	return LexicalStatus{Kind: StatusValid, Canonical: canonical}
}

func invalid(reason string) LexicalStatus {
	return LexicalStatus{Kind: StatusInvalid, Reason: reason}
}

func unsupported() LexicalStatus {
	return LexicalStatus{Kind: StatusUnsupported}
}

func validateBoolean(lexical string) LexicalStatus {
	switch lexical {
	case "true", "1":
		return valid("true")
	case "false", "0":
		return valid("false")
	default:
		return invalid("invalid xsd:boolean lexical form")
	}
}

type canonicalInteger struct {
	lexical string
	sign    int
}

func validateInteger(lexical string) LexicalStatus {
	canonical, err := canonicalIntegerLexical(lexical)
	if err != nil {
		return invalid(err.Error())
	}
	return valid(canonical.lexical)
}

func validateIntegerFamily(local, lexical string) LexicalStatus {
	canonical, err := canonicalIntegerLexical(lexical)
	if err != nil {
		return invalid(err.Error())
	}

	validFacet := true
	switch local {
	case "nonPositiveInteger":
		validFacet = canonical.sign <= 0
	case "negativeInteger":
		validFacet = canonical.sign < 0
	case "nonNegativeInteger":
		validFacet = canonical.sign >= 0
	case "positiveInteger":
		validFacet = canonical.sign > 0
	case "long":
		validFacet = integerInRange(canonical.lexical, "-9223372036854775808", "9223372036854775807")
	case "int":
		validFacet = integerInRange(canonical.lexical, "-2147483648", "2147483647")
	case "short":
		validFacet = integerInRange(canonical.lexical, "-32768", "32767")
	case "byte":
		validFacet = integerInRange(canonical.lexical, "-128", "127")
	case "unsignedLong":
		validFacet = integerInRange(canonical.lexical, "0", "18446744073709551615")
	case "unsignedInt":
		validFacet = integerInRange(canonical.lexical, "0", "4294967295")
	case "unsignedShort":
		validFacet = integerInRange(canonical.lexical, "0", "65535")
	case "unsignedByte":
		validFacet = integerInRange(canonical.lexical, "0", "255")
	}
	if !validFacet {
		return invalid(fmt.Sprintf("xsd:%s facet violation", local))
	}
	return valid(canonical.lexical)
}

func canonicalIntegerLexical(lexical string) (canonicalInteger, error) {
	if lexical == "" {
		return canonicalInteger{}, fmt.Errorf("integer lexical form is empty")
	}
	negative := false
	digits := lexical
	switch lexical[0] {
	case '+':
		digits = lexical[1:]
	case '-':
		negative = true
		digits = lexical[1:]
	}
	if digits == "" {
		return canonicalInteger{}, fmt.Errorf("integer lexical form has no digits")
	}
	if !isASCIIDigits(digits) {
		return canonicalInteger{}, fmt.Errorf("integer lexical form contains a non-digit character")
	}
	trimmed := strings.TrimLeft(digits, "0")
	if trimmed == "" {
		return canonicalInteger{lexical: "0", sign: 0}, nil
	}
	if negative {
		return canonicalInteger{lexical: "-" + trimmed, sign: -1}, nil
	}
	return canonicalInteger{lexical: trimmed, sign: 1}, nil
}

func integerInRange(lexical, minText, maxText string) bool {
	value, ok := new(big.Int).SetString(lexical, 10)
	if !ok {
		return false
	}
	min, _ := new(big.Int).SetString(minText, 10)
	max, _ := new(big.Int).SetString(maxText, 10)
	return value.Cmp(min) >= 0 && value.Cmp(max) <= 0
}

func validateDecimal(lexical string) LexicalStatus {
	canonical, err := canonicalDecimalLexical(lexical)
	if err != nil {
		return invalid(err.Error())
	}
	return valid(canonical)
}

func canonicalDecimalLexical(lexical string) (string, error) {
	if lexical == "" {
		return "", fmt.Errorf("decimal lexical form is empty")
	}
	negative := false
	body := lexical
	switch lexical[0] {
	case '+':
		body = lexical[1:]
	case '-':
		negative = true
		body = lexical[1:]
	}
	if body == "" {
		return "", fmt.Errorf("decimal lexical form has no digits")
	}
	if strings.Count(body, ".") > 1 {
		return "", fmt.Errorf("decimal lexical form has more than one decimal point")
	}
	whole, fractional, _ := strings.Cut(body, ".")
	if whole == "" && fractional == "" {
		return "", fmt.Errorf("decimal lexical form has no digits")
	}
	if !isASCIIDigits(whole) || !isASCIIDigits(fractional) {
		return "", fmt.Errorf("decimal lexical form contains an invalid character")
	}

	whole = strings.TrimLeft(whole, "0")
	fractional = strings.TrimRight(fractional, "0")
	if whole == "" && fractional == "" {
		return "0.0", nil
	}
	var out strings.Builder
	if negative {
		out.WriteByte('-')
	}
	if whole == "" {
		out.WriteByte('0')
	} else {
		out.WriteString(whole)
	}
	out.WriteByte('.')
	if fractional == "" {
		out.WriteByte('0')
	} else {
		out.WriteString(fractional)
	}
	return out.String(), nil
}

func validateFloat(local, lexical string) LexicalStatus {
	if lexical == "INF" || lexical == "-INF" || lexical == "NaN" {
		return valid(lexical)
	}
	if !isDecimalWithOptionalExponent(lexical) {
		return invalid(fmt.Sprintf("invalid xsd:%s lexical form", local))
	}
	value, err := strconv.ParseFloat(lexical, 64)
	if err != nil {
		return invalid(fmt.Sprintf("invalid xsd:%s lexical form: %v", local, err))
	}
	return valid(strconv.FormatFloat(value, 'g', -1, 64))
}

func isDecimalWithOptionalExponent(lexical string) bool {
	if strings.Count(lexical, "e")+strings.Count(lexical, "E") > 1 {
		return false
	}
	mantissa := lexical
	exponent := ""
	hasExponent := false
	if idx := strings.IndexAny(lexical, "eE"); idx >= 0 {
		mantissa = lexical[:idx]
		exponent = lexical[idx+1:]
		hasExponent = true
	}
	if !isDecimalMantissa(mantissa) {
		return false
	}
	return !hasExponent || isSignedDigits(exponent)
}

func isDecimalMantissa(value string) bool {
	if value == "" {
		return false
	}
	body := value
	if value[0] == '+' || value[0] == '-' {
		body = value[1:]
	}
	if body == "" || strings.Count(body, ".") > 1 {
		return false
	}
	whole, fractional, _ := strings.Cut(body, ".")
	return (whole != "" || fractional != "") && isASCIIDigits(whole) && isASCIIDigits(fractional)
}

func isSignedDigits(value string) bool {
	if value == "" {
		return false
	}
	digits := value
	if value[0] == '+' || value[0] == '-' {
		digits = value[1:]
	}
	return digits != "" && isASCIIDigits(digits)
}

var (
	dateTimeRE = regexp.MustCompile(`^(-?[0-9]{4,})-([0-9]{2})-([0-9]{2})T([0-9]{2}):([0-9]{2}):([0-9]{2})(\.[0-9]+)?(Z|[+-][0-9]{2}:[0-9]{2})?$`)
	dateRE     = regexp.MustCompile(`^(-?[0-9]{4,})-([0-9]{2})-([0-9]{2})(Z|[+-][0-9]{2}:[0-9]{2})?$`)
	timeRE     = regexp.MustCompile(`^([0-9]{2}):([0-9]{2}):([0-9]{2})(\.[0-9]+)?(Z|[+-][0-9]{2}:[0-9]{2})?$`)
	gYearMonth = regexp.MustCompile(`^(-?[0-9]{4,})-([0-9]{2})(Z|[+-][0-9]{2}:[0-9]{2})?$`)
	gYear      = regexp.MustCompile(`^(-?[0-9]{4,})(Z|[+-][0-9]{2}:[0-9]{2})?$`)
	gMonthDay  = regexp.MustCompile(`^--([0-9]{2})-([0-9]{2})(Z|[+-][0-9]{2}:[0-9]{2})?$`)
	gMonth     = regexp.MustCompile(`^--([0-9]{2})(?:--)?(Z|[+-][0-9]{2}:[0-9]{2})?$`)
	gDay       = regexp.MustCompile(`^---([0-9]{2})(Z|[+-][0-9]{2}:[0-9]{2})?$`)
)

func validateDateTimeFamily(local, lexical string) LexicalStatus {
	switch local {
	case "dateTime":
		m := dateTimeRE.FindStringSubmatch(lexical)
		if m == nil {
			return invalid("invalid xsd:dateTime lexical form")
		}
		if !validDate(m[1], m[2], m[3]) || !validTime(m[4], m[5], m[6]) || !validTimezone(m[8]) {
			return invalid("invalid xsd:dateTime lexical form")
		}
	case "date":
		m := dateRE.FindStringSubmatch(lexical)
		if m == nil {
			return invalid("invalid xsd:date lexical form")
		}
		if !validDate(m[1], m[2], m[3]) || !validTimezone(m[4]) {
			return invalid("invalid xsd:date lexical form")
		}
	case "time":
		m := timeRE.FindStringSubmatch(lexical)
		if m == nil {
			return invalid("invalid xsd:time lexical form")
		}
		if !validTime(m[1], m[2], m[3]) || !validTimezone(m[5]) {
			return invalid("invalid xsd:time lexical form")
		}
	case "gYearMonth":
		m := gYearMonth.FindStringSubmatch(lexical)
		if m == nil {
			return invalid("invalid xsd:gYearMonth lexical form")
		}
		if !validMonth(m[2]) || !validTimezone(m[3]) {
			return invalid("invalid xsd:gYearMonth lexical form")
		}
	case "gYear":
		m := gYear.FindStringSubmatch(lexical)
		if m == nil {
			return invalid("invalid xsd:gYear lexical form")
		}
		if !validTimezone(m[2]) {
			return invalid("invalid xsd:gYear lexical form")
		}
	case "gMonthDay":
		m := gMonthDay.FindStringSubmatch(lexical)
		if m == nil {
			return invalid("invalid xsd:gMonthDay lexical form")
		}
		if !validMonthDay(m[1], m[2]) || !validTimezone(m[3]) {
			return invalid("invalid xsd:gMonthDay lexical form")
		}
	case "gMonth":
		m := gMonth.FindStringSubmatch(lexical)
		if m == nil {
			return invalid("invalid xsd:gMonth lexical form")
		}
		if !validMonth(m[1]) || !validTimezone(m[2]) {
			return invalid("invalid xsd:gMonth lexical form")
		}
	case "gDay":
		m := gDay.FindStringSubmatch(lexical)
		if m == nil {
			return invalid("invalid xsd:gDay lexical form")
		}
		if !validDay(m[1], 31) || !validTimezone(m[2]) {
			return invalid("invalid xsd:gDay lexical form")
		}
	}
	return valid(lexical)
}

func validDate(year, monthText, dayText string) bool {
	month, ok := parseTwoDigits(monthText)
	if !ok || month < 1 || month > 12 {
		return false
	}
	maxDay := daysInMonth(year, month)
	return validDay(dayText, maxDay)
}

func validMonthDay(monthText, dayText string) bool {
	month, ok := parseTwoDigits(monthText)
	if !ok || month < 1 || month > 12 {
		return false
	}
	maxDay := 31
	switch month {
	case 2:
		maxDay = 29
	case 4, 6, 9, 11:
		maxDay = 30
	}
	return validDay(dayText, maxDay)
}

func daysInMonth(year string, month int) int {
	switch month {
	case 2:
		if isLeapYear(year) {
			return 29
		}
		return 28
	case 4, 6, 9, 11:
		return 30
	default:
		return 31
	}
}

func isLeapYear(year string) bool {
	year = strings.TrimPrefix(year, "-")
	n, ok := new(big.Int).SetString(year, 10)
	if !ok {
		return false
	}
	return mod(n, 4) == 0 && (mod(n, 100) != 0 || mod(n, 400) == 0)
}

func mod(n *big.Int, divisor int64) int64 {
	return new(big.Int).Mod(n, big.NewInt(divisor)).Int64()
}

func validMonth(monthText string) bool {
	month, ok := parseTwoDigits(monthText)
	return ok && month >= 1 && month <= 12
}

func validDay(dayText string, maxDay int) bool {
	day, ok := parseTwoDigits(dayText)
	return ok && day >= 1 && day <= maxDay
}

func validTime(hourText, minuteText, secondText string) bool {
	hour, ok := parseTwoDigits(hourText)
	if !ok {
		return false
	}
	minute, ok := parseTwoDigits(minuteText)
	if !ok || minute > 59 {
		return false
	}
	second, ok := parseTwoDigits(secondText)
	if !ok || second > 59 {
		return false
	}
	if hour == 24 {
		return minute == 0 && second == 0
	}
	return hour <= 23
}

func validTimezone(value string) bool {
	if value == "" || value == "Z" {
		return true
	}
	if len(value) != 6 || (value[0] != '+' && value[0] != '-') || value[3] != ':' {
		return false
	}
	hour, ok := parseTwoDigits(value[1:3])
	if !ok {
		return false
	}
	minute, ok := parseTwoDigits(value[4:6])
	if !ok || minute > 59 || hour > 14 {
		return false
	}
	return hour != 14 || minute == 0
}

func parseTwoDigits(value string) (int, bool) {
	if len(value) != 2 || !isASCIIDigits(value) {
		return 0, false
	}
	n, err := strconv.Atoi(value)
	return n, err == nil
}

var (
	durationRE         = regexp.MustCompile(`^-?P(?:(\d+)Y)?(?:(\d+)M)?(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+(?:\.\d+)?)S)?)?$`)
	yearMonthDuration  = regexp.MustCompile(`^-?P(?:(\d+)Y)?(?:(\d+)M)?$`)
	dayTimeDuration    = regexp.MustCompile(`^-?P(?:(\d+)D)?(?:T(?:(\d+)H)?(?:(\d+)M)?(?:(\d+(?:\.\d+)?)S)?)?$`)
	durationTimeMarker = regexp.MustCompile(`T`)
)

func validateDuration(lexical string) LexicalStatus {
	m := durationRE.FindStringSubmatch(lexical)
	if m == nil || !hasAny(m[1:]) || hasBareTimeMarker(lexical, m[4:]) {
		return invalid("invalid xsd:duration lexical form")
	}
	return valid(lexical)
}

func validateYearMonthDuration(lexical string) LexicalStatus {
	m := yearMonthDuration.FindStringSubmatch(lexical)
	if m == nil || !hasAny(m[1:]) {
		return invalid("invalid xsd:yearMonthDuration lexical form")
	}
	return valid(lexical)
}

func validateDayTimeDuration(lexical string) LexicalStatus {
	m := dayTimeDuration.FindStringSubmatch(lexical)
	if m == nil || !hasAny(m[1:]) || hasBareTimeMarker(lexical, m[2:]) {
		return invalid("invalid xsd:dayTimeDuration lexical form")
	}
	return valid(lexical)
}

func hasBareTimeMarker(lexical string, timeParts []string) bool {
	return durationTimeMarker.MatchString(lexical) && !hasAny(timeParts)
}

func hasAny(values []string) bool {
	for _, value := range values {
		if value != "" {
			return true
		}
	}
	return false
}

func validateHexBinary(lexical string) LexicalStatus {
	if len(lexical)%2 != 0 {
		return invalid("xsd:hexBinary lexical form has an odd number of digits")
	}
	for i := 0; i < len(lexical); i++ {
		if !isASCIIHexDigit(lexical[i]) {
			return invalid("xsd:hexBinary lexical form contains a non-hex digit")
		}
	}
	return valid(strings.ToUpper(lexical))
}

func replaceXMLWhitespace(value string) string {
	return strings.Map(func(ch rune) rune {
		switch ch {
		case '\t', '\n', '\r':
			return ' '
		default:
			return ch
		}
	}, value)
}

func collapseXMLWhitespace(value string) string {
	return strings.Join(strings.Fields(replaceXMLWhitespace(value)), " ")
}

func effectiveDatatypeIRI(terms []model.Term, term *model.Term) string {
	if term.Datatype != nil {
		dt := *term.Datatype
		if dt >= 0 && dt < len(terms) {
			return terms[dt].Value
		}
		return model.XSDString
	}
	if term.Lang != "" {
		if model.IsLiteralDirection(term.Direction) {
			return model.RDFDirLangString
		}
		return model.RDFLangString
	}
	return model.XSDString
}

func isASCIIDigits(value string) bool {
	for i := 0; i < len(value); i++ {
		if value[i] < '0' || value[i] > '9' {
			return false
		}
	}
	return true
}

func isASCIIHexDigit(value byte) bool {
	return (value >= '0' && value <= '9') ||
		(value >= 'a' && value <= 'f') ||
		(value >= 'A' && value <= 'F')
}
