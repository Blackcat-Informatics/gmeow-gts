// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

package ca.blackcatinformatics.gts

private val DEFAULT_CATALOG =
    linkedMapOf(
        0L to Codec("identity", "encode"),
        1L to Codec("gzip", "compress"),
        2L to Codec("zstd", "compress"),
        3L to Codec("zstd-rsyncable", "compress"),
        7L to Codec("cose-encrypt0", "encrypt"),
    )

class Writer(
    profile: String = "dist",
    layout: String? = null,
) {
    private val nameToId: Map<String, Long>
    private var prev: ByteArray
    private val chunks: MutableList<ByteArray> = mutableListOf()
    private val offsets: MutableList<Int> = mutableListOf()
    private val types: MutableList<String> = mutableListOf()

    init {
        require(layout == null || layout == "streamable") { "unsupported layout claim '$layout' (§3.3)" }
        nameToId = DEFAULT_CATALOG.map { it.value.name to it.key }.toMap()
        val catalog =
            CborMap(
                DEFAULT_CATALOG.map { (id, codec) ->
                    uint(id) to cborMap(text("cls") to text(codec.cls), text("name") to text(codec.name))
                },
            )
        val headerEntries =
            mutableListOf<Pair<CborValue, CborValue>>(
                text("cat") to catalog,
                text("gts") to text(MAGIC),
                text("prof") to text(profile),
                text("v") to uint(VERSION),
            )
        layout?.let { headerEntries += text("layout") to text(it) }
        val unsignedHeader = CborMap(headerEntries)
        val id = headerId(unsignedHeader)
        val header = CborMap(unsignedHeader.value + (text("id") to bytes(id)))
        prev = id
        chunks += encode(CborTag(SELF_DESCRIBE_TAG, header))
    }

    fun head(): ByteArray = prev.copyOf()

    fun addFrame(
        frameType: String,
        payload: CborValue? = null,
        raw: ByteArray? = null,
        transform: List<String> = emptyList(),
        pubMeta: CborValue? = null,
    ): ByteArray {
        require(payload == null || raw == null) { "payload and raw are mutually exclusive" }
        val entries = mutableListOf<Pair<CborValue, CborValue>>(text("t") to text(frameType))
        val data: CborValue? =
            when {
                transform.isNotEmpty() -> {
                    val source = raw ?: payload?.let { encode(it) }
                        ?: error("transform requires a raw or payload source")
                    entries += text("x") to CborArray(transform.map { uint(nameToId[it] ?: error("unknown codec '$it'")) })
                    bytes(encodeChain(transform.map { Codec(it, "encode") }, source))
                }
                raw != null -> bytes(raw)
                payload != null -> payload
                else -> null
            }
        data?.let { entries += text("d") to it }
        pubMeta?.let { entries += text("pub") to it }
        entries += text("prev") to bytes(prev)
        val unsigned = CborMap(entries)
        val id = contentId(unsigned)
        val frame = CborMap(unsigned.value + (text("id") to bytes(id)))
        offsets += chunks.sumOf { it.size }
        types += frameType
        chunks += encode(frame)
        prev = id
        return id
    }

    fun addTerms(terms: List<Term>): ByteArray = addFrame("terms", CborArray(terms.map(::termToWire)))

    fun addQuads(quads: List<Quad>): ByteArray =
        addFrame(
            "quads",
            CborArray(
                quads.map { q ->
                    CborArray(listOfNotNull(uint(q.s), uint(q.p), uint(q.o), q.g?.let { uint(it) }))
                },
            ),
        )

    fun addReifies(bindings: List<ReifierEntry>): ByteArray =
        addFrame(
            "reifies",
            CborMap(bindings.map { uint(it.rid) to cborArray(uint(it.spo.s), uint(it.spo.p), uint(it.spo.o)) }),
        )

    fun addAnnot(rows: List<Triple>): ByteArray =
        addFrame("annot", CborArray(rows.map { cborArray(uint(it.s), uint(it.p), uint(it.o)) }))

    fun addBlob(data: ByteArray, mt: String? = null, rep: String? = null): ByteArray {
        val pub =
            CborMap(
                buildList {
                    add(text("digest") to text(digestStr(data)))
                    mt?.let { add(text("mt") to text(it)) }
                    rep?.let { add(text("rep") to text(it)) }
                },
            )
        return addFrame("blob", raw = data, pubMeta = pub)
    }

    fun addMeta(meta: CborMap): ByteArray = addFrame("meta", meta)

    fun addSuppress(targets: List<CborValue>, reason: String? = null, by: Int? = null): ByteArray =
        addFrame(
            "suppress",
            CborMap(
                buildList {
                    add(text("targets") to CborArray(targets))
                    reason?.let { add(text("reason") to text(it)) }
                    by?.let { add(text("by") to uint(it)) }
                },
            ),
        )

    fun addIndex(): ByteArray {
        val ti =
            types.withIndex().groupBy({ it.value }, { it.index }).map { (type, positions) ->
                text(type) to CborArray(positions.map { uint(it) })
            }
        val payload =
            CborMap(
                buildList {
                    add(text("count") to uint(types.size))
                    add(text("head") to bytes(prev))
                    if (offsets.isNotEmpty()) {
                        add(text("off") to CborArray(offsets.map { uint(it) }))
                        add(text("ti") to CborMap(ti))
                    }
                },
            )
        return addFrame("index", payload)
    }

    fun toBytes(): ByteArray {
        val out = ByteArray(chunks.sumOf { it.size })
        var offset = 0
        for (chunk in chunks) {
            chunk.copyInto(out, offset)
            offset += chunk.size
        }
        return out
    }
}

private fun termToWire(term: Term): CborMap =
    CborMap(
        buildList {
            add(text("k") to uint(term.kind.wire))
            if (term.value.isNotEmpty() || term.kind == TermKind.LITERAL) add(text("v") to text(term.value))
            term.datatype?.let { add(text("dt") to uint(it)) }
            term.lang?.takeIf { it.isNotEmpty() }?.let { add(text("l") to text(it)) }
            term.reifier?.let { add(text("rf") to uint(it)) }
        },
    )
