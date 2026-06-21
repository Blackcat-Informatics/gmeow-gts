// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

const val XSD_STRING = "http://www.w3.org/2001/XMLSchema#string"
const val RDF_LANG_STRING = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
const val RDF_DIR_LANG_STRING = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString"

fun isLiteralDirection(direction: String?): Boolean = direction == "ltr" || direction == "rtl"

enum class TermKind(val wire: Int) {
    IRI(0),
    LITERAL(1),
    BNODE(2),
    TRIPLE(3),
}

fun termKindFromWire(k: Int): TermKind =
    when (k) {
        1 -> TermKind.LITERAL
        2 -> TermKind.BNODE
        3 -> TermKind.TRIPLE
        else -> TermKind.IRI
    }

data class Term(
    val kind: TermKind,
    val value: String,
    val datatype: Int? = null,
    val lang: String? = null,
    val reifier: Int? = null,
    val direction: String? = null,
)

data class Quad(
    val s: Int,
    val p: Int,
    val o: Int,
    val g: Int? = null,
)

data class Triple(
    val s: Int,
    val p: Int,
    val o: Int,
)

data class OpaqueNode(
    val id: ByteArray,
    val frameType: String,
    val reason: String,
    val sigStat: String = "none",
    val pubMeta: CborValue? = null,
    val recipients: List<CborValue> = emptyList(),
) {
    override fun equals(other: Any?): Boolean =
        other is OpaqueNode &&
            id.contentEquals(other.id) &&
            frameType == other.frameType &&
            reason == other.reason &&
            sigStat == other.sigStat &&
            pubMeta == other.pubMeta &&
            recipients == other.recipients

    override fun hashCode(): Int {
        var result = id.contentHashCode()
        result = 31 * result + frameType.hashCode()
        result = 31 * result + reason.hashCode()
        result = 31 * result + sigStat.hashCode()
        result = 31 * result + (pubMeta?.hashCode() ?: 0)
        result = 31 * result + recipients.hashCode()
        return result
    }
}

data class Suppression(
    val targets: List<CborValue>,
    val reason: String,
    val by: Int? = null,
)

data class Diagnostic(
    val code: String,
    val detail: String,
    val frameIndex: Int? = null,
)

data class Signature(
    val frameId: ByteArray,
    val kid: String,
    val status: String,
    val cose: ByteArray? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is Signature &&
            frameId.contentEquals(other.frameId) &&
            kid == other.kid &&
            status == other.status &&
            ((cose == null && other.cose == null) || (cose != null && other.cose != null && cose.contentEquals(other.cose)))

    override fun hashCode(): Int {
        var result = frameId.contentHashCode()
        result = 31 * result + kid.hashCode()
        result = 31 * result + status.hashCode()
        result = 31 * result + (cose?.contentHashCode() ?: 0)
        return result
    }
}

data class StreamableInfo(
    val claimed: Boolean = false,
    val covered: Int = 0,
    val tail: Int = 0,
    val head: ByteArray? = null,
) {
    override fun equals(other: Any?): Boolean =
        other is StreamableInfo &&
            claimed == other.claimed &&
            covered == other.covered &&
            tail == other.tail &&
            ((head == null && other.head == null) || (head != null && other.head != null && head.contentEquals(other.head)))

    override fun hashCode(): Int {
        var result = claimed.hashCode()
        result = 31 * result + covered
        result = 31 * result + tail
        result = 31 * result + (head?.contentHashCode() ?: 0)
        return result
    }
}

data class MetaEntry(val key: String, val value: CborValue)

data class BlobEntry(val digest: String, val data: ByteArray) {
    override fun equals(other: Any?): Boolean = other is BlobEntry && digest == other.digest && data.contentEquals(other.data)

    override fun hashCode(): Int = 31 * digest.hashCode() + data.contentHashCode()
}

data class BlobMetaEntry(val digest: String, val meta: CborValue)

data class ReifierEntry(val rid: Int, val spo: Triple)

class Graph {
    val terms: MutableList<Term> = mutableListOf()
    val quads: MutableList<Quad> = mutableListOf()
    val reifiers: MutableList<ReifierEntry> = mutableListOf()
    val annotations: MutableList<Triple> = mutableListOf()
    val blobs: MutableList<BlobEntry> = mutableListOf()
    val blobMeta: MutableList<BlobMetaEntry> = mutableListOf()
    val meta: MutableList<MetaEntry> = mutableListOf()
    val suppressions: MutableList<Suppression> = mutableListOf()
    val opaque: MutableList<OpaqueNode> = mutableListOf()
    val signatures: MutableList<Signature> = mutableListOf()
    val diagnostics: MutableList<Diagnostic> = mutableListOf()
    val segmentHeads: MutableList<ByteArray> = mutableListOf()
    val segmentProfiles: MutableList<String> = mutableListOf()
    val segmentMeta: MutableList<List<MetaEntry>> = mutableListOf()
    val segmentStreamable: MutableList<StreamableInfo> = mutableListOf()

    fun reifier(rid: Int): Triple? = reifiers.firstOrNull { it.rid == rid }?.spo

    fun setReifier(rid: Int, spo: Triple) {
        val idx = reifiers.indexOfFirst { it.rid == rid }
        if (idx >= 0) {
            reifiers[idx] = ReifierEntry(rid, spo)
        } else {
            reifiers += ReifierEntry(rid, spo)
        }
    }

    fun setMeta(key: String, value: CborValue) {
        val idx = meta.indexOfFirst { it.key == key }
        if (idx >= 0) {
            meta[idx] = MetaEntry(key, value)
        } else {
            meta += MetaEntry(key, value)
        }
    }

    fun setBlobMeta(digest: String, value: CborValue) {
        val idx = blobMeta.indexOfFirst { it.digest == digest }
        if (idx >= 0) {
            blobMeta[idx] = BlobMetaEntry(digest, value)
        } else {
            blobMeta += BlobMetaEntry(digest, value)
        }
    }

    fun setBlob(digest: String, data: ByteArray) {
        val idx = blobs.indexOfFirst { it.digest == digest }
        if (idx >= 0) {
            blobs[idx] = BlobEntry(digest, data.copyOf())
        } else {
            blobs += BlobEntry(digest, data.copyOf())
        }
    }

    fun datatypeIri(term: Term): String {
        term.datatype?.let { dt ->
            terms.getOrNull(dt)?.value?.takeIf { it.isNotEmpty() }?.let { return it }
            return XSD_STRING
        }
        return if (!term.lang.isNullOrEmpty()) {
            if (isLiteralDirection(term.direction)) RDF_DIR_LANG_STRING else RDF_LANG_STRING
        } else {
            XSD_STRING
        }
    }
}
